// SPDX-License-Identifier: GPL-3.0-only
// core/src/pdfium_ops/text.rs
//
// Free-text layout and appearance-stream generation. Pure functions with no
// pdfium calls, so the same metrics drive the rendered annotation and the
// caret the UI places inside form fields. The manager writes the produced
// strings into `/DA` and `/AP`; the `NoctuaStyle` entry carries the style
// attributes a `/DA` string cannot express.

use super::model::{AnnotationColor, FontFamily, FreeTextStyle, TextAlign};

/// Rectangle in page points: `(x, y, width, height)`, origin bottom-left.
pub(crate) type CaretRect = (f32, f32, f32, f32);

/// Helvetica line advance as a multiple of the font size.
const LINE_HEIGHT: f32 = 1.2;

/// Inset between the free-text box and its text, in points.
const PADDING: f32 = 2.0;

/// Helvetica ascent and descent in em units (Adobe AFM metrics). The same
/// values stand in for Times and Courier, which only differ by a fraction of a
/// point and do not affect the box or caret in a visible way.
const ASCENT: f32 = 0.718;
const DESCENT: f32 = 0.207;

/// Width of the insertion marker, in points.
const CARET_WIDTH: f32 = 0.8;

/// Italic slant, applied as a shear to the text matrix (≈ 15°). The shear does
/// not change the advance widths, so the box and caret stay on the baseline.
const ITALIC_SHEAR: f32 = 0.25;

/// Stroke width for the fake-bold rendering mode, relative to the font size.
const BOLD_STROKE_RATIO: f32 = 0.03;

/// Thickness of the underline and strikethrough lines, relative to the font size.
const DECORATION_THICKNESS_RATIO: f32 = 0.05;

/// Annotation entry that carries the style a `/DA` string cannot express.
pub(crate) const DECORATIONS_KEY: &str = "NoctuaStyle";

/// Advance widths for printable ASCII (32..=126), in 1/1000 em. Helvetica and
/// Times are taken from the metric-compatible URW base-35 AFM files; Courier is
/// monospaced, so every glyph advances 600.
const HELVETICA_ASCII_WIDTHS: [u16; 95] = [
    278, 278, 355, 556, 556, 889, 667, 191, 333, 333, 389, 584, 278, 333, 278, 278, // 32..=47
    556, 556, 556, 556, 556, 556, 556, 556, 556, 556, 278, 278, 584, 584, 584, 556, // 48..=63
    1015, 667, 667, 722, 722, 667, 611, 778, 722, 278, 500, 667, 556, 833, 722, 778, 667, 778, 722,
    667, 611, 722, 667, 944, 667, 667, 611, 278, 278, 278, 469, 556, 333, 556, 556, 500, 556, 556,
    278, 556, 556, 222, 222, 500, 222, 833, 556, 556, 556, 556, 333, 500, 278, 556, 500, 722, 500,
    500, 500, 334, 260, 334, 584,
];

const TIMES_ASCII_WIDTHS: [u16; 95] = [
    250, 333, 408, 500, 500, 833, 778, 333, 333, 333, 500, 564, 250, 333, 250, 278, // 32..=47
    500, 500, 500, 500, 500, 500, 500, 500, 500, 500, 278, 278, 564, 564, 564, 444, // 48..=63
    921, 722, 667, 667, 722, 611, 556, 722, 722, 333, 389, 722, 611, 889, 722, 722, 556, 722, 667,
    556, 611, 722, 722, 944, 722, 722, 611, 333, 278, 333, 469, 500, 333, 444, 500, 444, 500, 444,
    333, 500, 500, 278, 278, 500, 278, 778, 500, 500, 500, 500, 333, 389, 278, 500, 500, 722, 500,
    500, 444, 480, 200, 480, 541,
];

/// Courier's constant advance, in 1/1000 em.
const COURIER_WIDTH: u16 = 600;

/// Advance width of one character in 1/1000 em. Characters outside the ASCII
/// table use a typical width for the family; the full text stays in `/Contents`.
fn width_units(family: FontFamily, ch: char) -> u16 {
    let code = ch as u32;
    if (32..=126).contains(&code) {
        let index = (code - 32) as usize;
        return match family {
            FontFamily::Helvetica => HELVETICA_ASCII_WIDTHS[index],
            FontFamily::Times => TIMES_ASCII_WIDTHS[index],
            FontFamily::Courier => COURIER_WIDTH,
        };
    }
    match family {
        FontFamily::Helvetica => match ch {
            'Ä' => 667,
            'Ö' => 778,
            'Ü' => 722,
            'ç' => 500,
            '°' => 400,
            _ => 556,
        },
        FontFamily::Times => 500,
        FontFamily::Courier => COURIER_WIDTH,
    }
}

/// Advance width of one character at the style's font size, in points.
fn char_width(ch: char, style: &FreeTextStyle) -> f32 {
    f32::from(width_units(style.family, ch)) * style.font_size / 1000.0
}

/// Rendered width of `text` at the style's font size, in points.
pub(crate) fn text_width(text: &str, style: &FreeTextStyle) -> f32 {
    text.chars().map(|ch| char_width(ch, style)).sum()
}

/// Rendered width of `text` in Helvetica at `font_size` points.
///
/// Public because the UI places the caret inside a form field with the same
/// metrics the appearance uses; the width table lives in this module.
pub fn helvetica_text_width(text: &str, font_size: f32) -> f32 {
    let style = FreeTextStyle {
        font_size,
        ..FreeTextStyle::default()
    };
    text_width(text, &style)
}

/// The PDF font resource name for a family.
fn font_name(family: FontFamily) -> &'static str {
    match family {
        FontFamily::Helvetica => "/Helv",
        FontFamily::Times => "/TiRo",
        FontFamily::Courier => "/Cour",
    }
}

/// Greedy wrap: break at spaces, split words that are wider than a line. Each
/// line carries the character offset it starts at in the source text, so a
/// caret index can be mapped back to a position, and whether it is the last
/// line of its paragraph.
fn wrap_text(text: &str, style: &FreeTextStyle, max_width: f32) -> Vec<(String, usize, bool)> {
    // Even the narrowest usable box must fit one character per line.
    let max_width = max_width.max(char_width('m', style));
    let mut lines = Vec::new();
    let mut paragraph_start = 0;

    for paragraph in text.split('\n') {
        let characters: Vec<char> = paragraph.chars().collect();
        let mut line = String::new();
        let mut line_start = 0;
        let mut line_width = 0.0_f32;

        for (index, ch) in characters.iter().copied().enumerate() {
            let advance = char_width(ch, style);
            if line_width + advance > max_width && !line.is_empty() {
                match line.rfind(' ') {
                    // Keep the word that no longer fits for the next line.
                    Some(space) => {
                        let carry = line.split_off(space + 1);
                        let carry_start = line_start + line[..=space].chars().count();
                        line_width = text_width(&carry, style);
                        lines.push((
                            std::mem::take(&mut line).trim_end().to_string(),
                            paragraph_start + line_start,
                            false,
                        ));
                        line = carry;
                        line_start = carry_start;
                    }
                    None => {
                        lines.push((
                            std::mem::take(&mut line),
                            paragraph_start + line_start,
                            false,
                        ));
                        line_start = index;
                        line_width = 0.0;
                    }
                }
            }
            line.push(ch);
            line_width += advance;
        }

        lines.push((line, paragraph_start + line_start, true));
        paragraph_start += characters.len() + 1;
    }

    lines
}

/// Laid-out free-text: the wrapped lines plus the box that holds them.
pub(crate) struct TextLayout {
    pub(crate) lines: Vec<String>,
    /// Character offset of each line's first character in the source text.
    pub(crate) starts: Vec<usize>,
    /// Extra word spacing for justified lines, zero elsewhere.
    pub(crate) justify: Vec<f32>,
    /// Box width: the widest line, or the wrap width when justified.
    pub(crate) width: f32,
    pub(crate) height: f32,
}

/// Lay out `text` at the style's font size, wrapped at `max_width`.
pub(crate) fn layout(text: &str, style: &FreeTextStyle, max_width: f32) -> TextLayout {
    let wrapped = wrap_text(text, style, max_width);
    let widest = wrapped
        .iter()
        .map(|(line, _, _)| text_width(line, style))
        .fold(0.0_f32, f32::max)
        // An empty box still needs a clickable width.
        .max(char_width('n', style));

    // Justified lines fill the wrap width; the last line of every paragraph
    // stays left-aligned, as in a word processor.
    let line_count = wrapped.len();
    let mut justify = vec![0.0; line_count];
    let mut width = widest;
    if style.align == TextAlign::Justify {
        width = max_width;
        for (index, (line, _, is_last_of_paragraph)) in wrapped.iter().enumerate() {
            if !is_last_of_paragraph && line.contains(' ') {
                let words = line.split(' ').count();
                let extra = (max_width - text_width(line, style)) / (words - 1) as f32;
                justify[index] = extra.max(0.0);
            }
        }
    }

    TextLayout {
        width: width + 2.0 * PADDING,
        height: line_count.max(1) as f32 * style.font_size * LINE_HEIGHT + 2.0 * PADDING,
        starts: wrapped.iter().map(|(_, start, _)| *start).collect(),
        lines: wrapped.into_iter().map(|(line, _, _)| line).collect(),
        justify,
    }
}

/// Escape `text` as a PDF literal string. pdfium re-encodes the whole stream
/// through PDFDocEncoding, so characters outside Latin-1 would corrupt the
/// operators; those are replaced (the real text stays in `/Contents`).
fn pdf_text_string(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    out.push('(');
    for ch in text.chars() {
        match ch {
            '(' => out.push_str("\\("),
            ')' => out.push_str("\\)"),
            '\\' => out.push_str("\\\\"),
            ch if (ch as u32) < 0x20 || (0x7F..=0x9F).contains(&(ch as u32)) => out.push('?'),
            ch if (ch as u32) > 0xFF => out.push('?'),
            ch => out.push(ch),
        }
    }
    out.push(')');
    out
}

/// Baseline of the first line of a box whose top edge is `top`.
fn first_baseline(top: f32, font_size: f32) -> f32 {
    top - PADDING - font_size * ASCENT
}

/// Horizontal offset of one line from the box's left inner edge, for the
/// alignment styles that do not fill the line.
fn line_offset(style: &FreeTextStyle, line: &str, inner_width: f32) -> f32 {
    match style.align {
        TextAlign::Left => 0.0,
        TextAlign::Center => ((inner_width - text_width(line, style)) / 2.0).max(0.0),
        TextAlign::Right => (inner_width - text_width(line, style)).max(0.0),
        TextAlign::Justify => 0.0,
    }
}

/// Build the normal appearance stream of a free-text annotation. Writing the
/// stream ourselves keeps the frame off (pdfium's own generator always draws
/// one), gives us line breaks, and applies the style a `/DA` string cannot.
pub(crate) fn appearance_stream(
    layout: &TextLayout,
    style: &FreeTextStyle,
    top_left: (f32, f32),
) -> String {
    let (left, top) = top_left;
    let r = style.color.red as f32 / 255.0;
    let g = style.color.green as f32 / 255.0;
    let b = style.color.blue as f32 / 255.0;
    let inner_width = (layout.width - 2.0 * PADDING).max(0.0);
    let line_step = style.font_size * LINE_HEIGHT;
    let mut baseline = first_baseline(top, style.font_size);

    let mut content = format!("q\n{r:.3} {g:.3} {b:.3} rg\n");
    // Background fill under the text, when requested.
    if let Some(bg) = style.background {
        let br = bg.red as f32 / 255.0;
        let bgc = bg.green as f32 / 255.0;
        let bb = bg.blue as f32 / 255.0;
        content.push_str(&format!(
            "{br:.3} {bgc:.3} {bb:.3} rg\n{left:.3} {:.3} {:.3} {:.3} re f\n",
            top - layout.height,
            layout.width,
            layout.height,
        ));
        content.push_str(&format!("{r:.3} {g:.3} {b:.3} rg\n"));
    }

    content.push_str("BT\n");
    content.push_str(&format!(
        "{} {:.3} Tf\n",
        font_name(style.family),
        style.font_size
    ));
    // Fake bold strokes the glyphs as well as filling them; the advance widths
    // stay the family's, so layout and caret are unaffected.
    if style.bold {
        content.push_str(&format!(
            "{r:.3} {g:.3} {b:.3} RG\n{:.3} w\n2 Tr\n",
            style.font_size * BOLD_STROKE_RATIO
        ));
    }
    for (index, line) in layout.lines.iter().enumerate() {
        let justify = layout.justify.get(index).copied().unwrap_or(0.0);
        if justify > 0.0 {
            content.push_str(&format!("{justify:.3} Tw\n"));
        }
        let offset = line_offset(style, line, inner_width);
        let x = left + PADDING + offset;
        let shear = if style.italic { ITALIC_SHEAR } else { 0.0 };
        content.push_str(&format!("1 0 {shear:.3} 1 {x:.3} {baseline:.3} Tm\n"));
        content.push_str(&format!("{} Tj\n", pdf_text_string(line)));
        if justify > 0.0 {
            content.push_str("0 Tw\n");
        }
        baseline -= line_step;
    }
    content.push_str("ET\n");

    // Underline and strikethrough are strokes drawn under/through each line,
    // outside the text block. A line spans the laid-out text, or the full
    // inner width when justified.
    if style.underline || style.strikethrough {
        let thickness = (style.font_size * DECORATION_THICKNESS_RATIO).max(0.5);
        content.push_str(&format!("{r:.3} {g:.3} {b:.3} RG\n{thickness:.3} w\n"));
        let mut line_baseline = first_baseline(top, style.font_size);
        for (index, line) in layout.lines.iter().enumerate() {
            let justify = layout.justify.get(index).copied().unwrap_or(0.0);
            let offset = line_offset(style, line, inner_width);
            let x = left + PADDING + offset;
            let width = if justify > 0.0 {
                inner_width
            } else {
                text_width(line, style)
            };
            if style.underline {
                let y = line_baseline - style.font_size * DESCENT - thickness;
                content.push_str(&format!("{x:.3} {y:.3} m {:.3} {y:.3} l S\n", x + width));
            }
            if style.strikethrough {
                let y = line_baseline + style.font_size * 0.3;
                content.push_str(&format!("{x:.3} {y:.3} m {:.3} {y:.3} l S\n", x + width));
            }
            line_baseline -= line_step;
        }
    }

    content.push('Q');
    content
}

/// Rectangle of the insertion marker at the end of the laid-out text, in page
/// points with the origin at the bottom-left.
pub(crate) fn caret_rect(
    layout: &TextLayout,
    style: &FreeTextStyle,
    top_left: (f32, f32),
    cursor: usize,
) -> CaretRect {
    let (left, top) = top_left;

    // The cursor sits on the last line that starts at or before it.
    let mut line = 0;
    for (index, start) in layout.starts.iter().enumerate() {
        if *start <= cursor {
            line = index;
        }
    }

    let prefix: String = layout
        .lines
        .get(line)
        .map(|text| {
            text.chars()
                .take(cursor.saturating_sub(layout.starts[line]))
                .collect()
        })
        .unwrap_or_default();
    let inner_width = (layout.width - 2.0 * PADDING).max(0.0);
    let full_line = layout.lines.get(line).map(String::as_str).unwrap_or("");
    let baseline =
        first_baseline(top, style.font_size) - line as f32 * style.font_size * LINE_HEIGHT;
    // The cursor's line is the one being typed, so it is left-aligned (or
    // centered/right when the whole box is); justify does not stretch it.
    let x =
        left + PADDING + line_offset(style, full_line, inner_width) + text_width(&prefix, style);

    (
        x,
        baseline - style.font_size * DESCENT,
        CARET_WIDTH,
        style.font_size * (ASCENT + DESCENT),
    )
}

/// Build the `/DA` string: the standard colour, font and size. Bold, italic and
/// the remaining attributes are drawn in the appearance stream and stored in
/// [`DECORATIONS_KEY`], so the font name always names a regular face.
pub(crate) fn default_appearance(style: &FreeTextStyle) -> String {
    let r = style.color.red as f32 / 255.0;
    let g = style.color.green as f32 / 255.0;
    let b = style.color.blue as f32 / 255.0;
    format!(
        "{r:.3} {g:.3} {b:.3} rg {} {:.3} Tf",
        font_name(style.family),
        style.font_size
    )
}

/// Parse the font family, colour and font size out of a `/DA` string. Falls
/// back to Helvetica, black and 12 pt, matching the PDF defaults.
pub(crate) fn parse_default_appearance(da: &str) -> (FontFamily, AnnotationColor, f32) {
    let tokens: Vec<&str> = da.split_whitespace().collect();
    let number = |index: usize| {
        tokens
            .get(index)
            .and_then(|token| token.parse::<f32>().ok())
    };
    let mut rgb = [0.0f32; 3];
    let mut size = 12.0f32;
    let mut family = FontFamily::Helvetica;
    for (index, token) in tokens.iter().enumerate() {
        match *token {
            "rg" if index >= 3 => {
                if let (Some(r), Some(g), Some(b)) =
                    (number(index - 3), number(index - 2), number(index - 1))
                {
                    rgb = [r, g, b];
                }
            }
            "g" if index >= 1 => {
                if let Some(grey) = number(index - 1) {
                    rgb = [grey; 3];
                }
            }
            "Tf" if index >= 1 => {
                if let Some(value) = number(index - 1).filter(|value| *value > 0.0) {
                    size = value;
                }
                family = tokens
                    .get(index - 2)
                    .and_then(|name| parse_font_name(name))
                    .unwrap_or(FontFamily::Helvetica);
            }
            _ => {}
        }
    }
    let channel = |value: f32| (value.clamp(0.0, 1.0) * 255.0).round() as u8;
    (
        family,
        AnnotationColor {
            red: channel(rgb[0]),
            green: channel(rgb[1]),
            blue: channel(rgb[2]),
            alpha: 255,
        },
        size,
    )
}

/// Map a standard-14 resource name back to a [`FontFamily`].
fn parse_font_name(name: &str) -> Option<FontFamily> {
    let base = name
        .trim_start_matches('/')
        .split_once('-')
        .map(|(base, _)| base)
        .unwrap_or(name.trim_start_matches('/'));
    match base {
        "Helv" => Some(FontFamily::Helvetica),
        "TiRo" | "Times" => Some(FontFamily::Times),
        "Cour" | "Courier" => Some(FontFamily::Courier),
        _ => None,
    }
}

/// The style attributes a `/DA` string cannot carry. Serialized under
/// [`DECORATIONS_KEY`] so they survive reopening the document.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub(crate) struct Decorations {
    pub(crate) bold: bool,
    pub(crate) italic: bool,
    pub(crate) underline: bool,
    pub(crate) strikethrough: bool,
    pub(crate) background: Option<AnnotationColor>,
    pub(crate) align: TextAlign,
}

impl Decorations {
    pub(crate) fn from_style(style: &FreeTextStyle) -> Self {
        Self {
            bold: style.bold,
            italic: style.italic,
            underline: style.underline,
            strikethrough: style.strikethrough,
            background: style.background,
            align: style.align,
        }
    }

    pub(crate) fn apply(self, style: &mut FreeTextStyle) {
        style.bold = self.bold;
        style.italic = self.italic;
        style.underline = self.underline;
        style.strikethrough = self.strikethrough;
        style.background = self.background;
        style.align = self.align;
    }
}

/// Serialize the decorations into a compact, stable string.
pub(crate) fn serialize_decorations(decorations: Decorations) -> String {
    let bg = decorations
        .background
        .map(|color| {
            format!(
                "{}-{}-{}-{}",
                color.red, color.green, color.blue, color.alpha
            )
        })
        .unwrap_or_else(|| "-".to_string());
    let align = match decorations.align {
        TextAlign::Left => "l",
        TextAlign::Center => "c",
        TextAlign::Right => "r",
        TextAlign::Justify => "j",
    };
    format!(
        "b{}.i{}.u{}.s{}.a{}.bg{}",
        flag(decorations.bold),
        flag(decorations.italic),
        flag(decorations.underline),
        flag(decorations.strikethrough),
        align,
        bg,
    )
}

/// Parse the decorations serialized by [`serialize_decorations`]. Missing or
/// malformed fields fall back to their defaults.
pub(crate) fn parse_decorations(value: &str) -> Decorations {
    let mut decorations = Decorations::default();
    for part in value.split('.') {
        if let Some(rest) = part.strip_prefix("bg") {
            decorations.background = parse_color(rest);
        } else if let Some(flag) = part.strip_prefix('b') {
            decorations.bold = flag == "1";
        } else if let Some(flag) = part.strip_prefix('i') {
            decorations.italic = flag == "1";
        } else if let Some(flag) = part.strip_prefix('u') {
            decorations.underline = flag == "1";
        } else if let Some(flag) = part.strip_prefix('s') {
            decorations.strikethrough = flag == "1";
        } else if let Some(align) = part.strip_prefix('a') {
            decorations.align = match align {
                "c" => TextAlign::Center,
                "r" => TextAlign::Right,
                "j" => TextAlign::Justify,
                _ => TextAlign::Left,
            };
        }
    }
    decorations
}

fn flag(on: bool) -> &'static str {
    if on { "1" } else { "0" }
}

fn parse_color(value: &str) -> Option<AnnotationColor> {
    let channels: Vec<u8> = value
        .split('-')
        .filter_map(|part| part.parse::<u8>().ok())
        .collect();
    match channels.as_slice() {
        [red, green, blue, alpha] => Some(AnnotationColor {
            red: *red,
            green: *green,
            blue: *blue,
            alpha: *alpha,
        }),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn style() -> FreeTextStyle {
        FreeTextStyle::default()
    }

    #[test]
    fn courier_is_monospaced() {
        let courier = FreeTextStyle {
            family: FontFamily::Courier,
            ..FreeTextStyle::default()
        };
        assert_eq!(text_width("iii", &courier), text_width("mmm", &courier));
    }

    #[test]
    fn times_is_proportional() {
        let times = FreeTextStyle {
            family: FontFamily::Times,
            ..FreeTextStyle::default()
        };
        assert!(text_width("iii", &times) < text_width("mmm", &times));
    }

    #[test]
    fn decorations_round_trip() {
        let decorations = Decorations {
            bold: true,
            italic: true,
            underline: false,
            strikethrough: true,
            background: Some(AnnotationColor::rgb(1, 2, 3)),
            align: TextAlign::Justify,
        };
        assert_eq!(
            parse_decorations(&serialize_decorations(decorations)),
            decorations
        );
    }

    #[test]
    fn default_appearance_round_trips_family_color_size() {
        let style = FreeTextStyle {
            family: FontFamily::Times,
            font_size: 36.0,
            color: AnnotationColor::rgb(10, 20, 30),
            ..FreeTextStyle::default()
        };
        let (family, color, size) = parse_default_appearance(&default_appearance(&style));
        assert_eq!(family, FontFamily::Times);
        assert_eq!(size, 36.0);
        assert_eq!(color, AnnotationColor::rgb(10, 20, 30));
    }

    #[test]
    fn layout_width_is_full_for_justify() {
        let style = FreeTextStyle {
            align: TextAlign::Justify,
            ..style()
        };
        let layout = layout("two words", &style, 200.0);
        assert!((layout.width - (200.0 + 2.0 * PADDING)).abs() < 0.01);
    }
}
