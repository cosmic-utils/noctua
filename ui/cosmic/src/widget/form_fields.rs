// SPDX-License-Identifier: GPL-3.0-only
// ui/cosmic/src/widget/form_fields.rs
//
// Hit-testing, field order and the option list of interactive form fields.
// The geometry part is pure: the page is rendered at one pixel per point, so a
// pointer position in image pixels maps onto a field rectangle in page points
// without needing the viewer's zoom or pan state. No state is kept here.
//
// Reused by every view that shows a page with form fields.

use std::borrow::Cow;
use std::sync::{Arc, Mutex};

use cosmic::Element;
use cosmic::iced::Point;
use cosmic::iced::mouse;
use cosmic::widget::dropdown::menu::{Menu, State};
use cosmic::widget::segmented_button::Entity;

use noctua_core::pdfium_ops::{FormField, FormFieldKind};

use crate::message::{AnnotationMessage, Message};

/// Field flag `kChoiceEdit`: a combo box the user may also type into.
const CHOICE_EDIT: i32 = 1 << 18;

/// Width of the option list, in logical pixels.
const OPTION_LIST_WIDTH: f32 = 220.0;

/// Index of the field under `point`.
///
/// `point` is in image pixels with the origin at the top left; `image_height`
/// is the page height in the same unit. Page points have their origin at the
/// bottom left, so the vertical axis is flipped here once.
pub(crate) fn field_at(fields: &[FormField], point: Point, image_height: f32) -> Option<usize> {
    fields.iter().position(|field| {
        let left = field.x;
        let right = field.x + field.width;
        let top = image_height - (field.y + field.height);
        let bottom = image_height - field.y;
        point.x >= left && point.x <= right && point.y >= top && point.y <= bottom
    })
}

/// Pointer shape over `point`, or `None` when no field is under it so the
/// caller keeps its own cursor.
pub(crate) fn cursor_for(
    fields: &[FormField],
    point: Point,
    image_height: f32,
) -> Option<mouse::Interaction> {
    let kind = fields.get(field_at(fields, point, image_height)?)?.kind;
    Some(match kind {
        FormFieldKind::Text => mouse::Interaction::Text,
        // Choice fields open a list on a click, so they read as clickable; a
        // text caret would promise typing they do not accept.
        FormFieldKind::Combo
        | FormFieldKind::ListBox
        | FormFieldKind::Checkbox
        | FormFieldKind::Radio => mouse::Interaction::Pointer,
        FormFieldKind::PushButton | FormFieldKind::Signature | FormFieldKind::Unknown => {
            return None;
        }
    })
}

/// Whether the interaction can put focus on this kind of field.
pub(crate) fn is_focusable(kind: FormFieldKind) -> bool {
    matches!(
        kind,
        FormFieldKind::Text
            | FormFieldKind::Combo
            | FormFieldKind::ListBox
            | FormFieldKind::Checkbox
            | FormFieldKind::Radio
    )
}

/// Whether clicking the field edits text, instead of offering options or
/// toggling.
///
/// A combo box is free text when it has no options, and a combo box that allows
/// editing keeps the text input as well; a list box only offers its options.
pub(crate) fn accepts_text(field: &FormField) -> bool {
    match field.kind {
        FormFieldKind::Text => true,
        FormFieldKind::Combo => field.options.is_empty() || field.flags & CHOICE_EDIT != 0,
        FormFieldKind::ListBox
        | FormFieldKind::Checkbox
        | FormFieldKind::Radio
        | FormFieldKind::PushButton
        | FormFieldKind::Signature
        | FormFieldKind::Unknown => false,
    }
}

/// Whether the field offers a list of options to pick from.
pub(crate) fn has_options(field: &FormField) -> bool {
    matches!(field.kind, FormFieldKind::Combo | FormFieldKind::ListBox) && !field.options.is_empty()
}

/// The option list of a choice field, drawn with cosmic's own dropdown list.
///
/// `Menu` is the list, not a widget that places itself, so `popup` is only the
/// way to turn it into an element; the caller decides where it appears. The
/// list opens whether or not the click was on a dropdown arrow — the whole
/// field is the click target.
pub(crate) fn form_option_list<'a>(tab: Entity, field: &FormField) -> Element<'a, Message> {
    let space = cosmic::theme::spacing();
    let annotation = field.index;
    Menu::new(
        State::new(),
        Cow::Owned(field.options.clone()),
        Cow::Owned(Vec::new()),
        Arc::new(Mutex::new(None)),
        field.selected,
        move |option| {
            Message::Annotation(AnnotationMessage::AnnotationFormOptionPicked {
                tab,
                field: annotation,
                option,
            })
        },
        None,
        Some(Message::Annotation(
            AnnotationMessage::AnnotationFormMenuClosed,
        )),
    )
    .width(OPTION_LIST_WIDTH)
    .padding(space.space_xxs)
    .popup(Point::ORIGIN, 0.0)
}

/// Caret position inside a text field, in page points: `(x, bottom, height)`.
///
/// The caret is clamped to the field so it never leaves the box, and it uses the
/// same Helvetica metrics as the field's appearance.
pub(crate) fn caret_in_field(field: &FormField, draft: &str, caret: usize) -> (f32, f32, f32) {
    /// Padding between the field border and its text, as in the appearance.
    const PADDING: f32 = 2.0;

    let prefix: String = draft.chars().take(caret).collect();
    let advance = noctua_core::pdfium_ops::helvetica_text_width(&prefix, field.font_size);
    let height = field.font_size.max(1.0);
    let x = (field.x + PADDING + advance).min(field.x + field.width - 1.0);
    (x, field.y + PADDING, height)
}

/// Next focusable field after `focused`, wrapping around.
///
/// Fields the interaction cannot drive are skipped, including choice fields:
/// their option list opens on a click, which a Tab press cannot provide.
/// `focused == None` starts at the first field when moving forward and at the
/// last when moving backward.
pub(crate) fn next_field(
    fields: &[FormField],
    focused: Option<usize>,
    forward: bool,
) -> Option<usize> {
    let usable: Vec<usize> = fields
        .iter()
        .enumerate()
        .filter(|(_, field)| is_focusable(field.kind) && !has_options(field))
        .map(|(index, _)| index)
        .collect();
    if usable.is_empty() {
        return None;
    }

    let Some(focused) = focused else {
        return if forward {
            usable.first().copied()
        } else {
            usable.last().copied()
        };
    };

    // Fields are in page order, so the neighbour in that order is the next one.
    let position = usable.iter().position(|index| *index == focused);
    let next = match position {
        Some(position) if forward => (position + 1) % usable.len(),
        Some(position) => (position + usable.len() - 1) % usable.len(),
        // The focused field disappeared: start over from the nearest end.
        None if forward => 0,
        None => usable.len() - 1,
    };
    Some(usable[next])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn field(kind: FormFieldKind, x: f32, y: f32, width: f32, height: f32) -> FormField {
        FormField {
            index: 0,
            name: String::new(),
            kind,
            flags: 0,
            value: String::new(),
            options: Vec::new(),
            selected: None,
            font_size: 12.0,
            x,
            y,
            width,
            height,
        }
    }

    /// A 400 pt high page: a field at y 300..330 sits 70..100 px from the top.
    #[test]
    fn hit_test_flips_the_vertical_axis() {
        let fields = [field(FormFieldKind::Text, 100.0, 300.0, 200.0, 30.0)];

        assert_eq!(field_at(&fields, Point::new(150.0, 85.0), 400.0), Some(0));
        assert_eq!(field_at(&fields, Point::new(150.0, 50.0), 400.0), None);
        assert_eq!(field_at(&fields, Point::new(50.0, 85.0), 400.0), None);
    }

    #[test]
    fn cursor_follows_the_field_kind() {
        let fields = [
            field(FormFieldKind::Text, 0.0, 300.0, 100.0, 20.0),
            field(FormFieldKind::Checkbox, 200.0, 300.0, 20.0, 20.0),
            field(FormFieldKind::Signature, 300.0, 300.0, 100.0, 20.0),
        ];

        assert!(is_focusable(fields[0].kind));
        assert!(is_focusable(fields[1].kind));
        assert!(!is_focusable(fields[2].kind));
    }

    #[test]
    fn caret_starts_at_the_text_origin_and_stays_inside() {
        let field = field(FormFieldKind::Text, 100.0, 300.0, 200.0, 30.0);

        let (x, bottom, height) = caret_in_field(&field, "", 0);
        assert!((x - 102.0).abs() < 0.01, "x {x}");
        assert!((bottom - 302.0).abs() < 0.01, "bottom {bottom}");
        assert!((height - 12.0).abs() < 0.01, "height {height}");

        let (wide, _, _) = caret_in_field(&field, "AAAAAAAAAAAAAAAA", 16);
        assert!(wide > x, "advance {wide}");
        assert!(wide <= 299.0, "clamped {wide}");
    }

    #[test]
    fn hover_cursor_follows_the_field_kind() {
        let fields = [
            field(FormFieldKind::Text, 0.0, 300.0, 100.0, 20.0),
            field(FormFieldKind::Checkbox, 200.0, 300.0, 20.0, 20.0),
            field(FormFieldKind::Signature, 300.0, 300.0, 100.0, 20.0),
        ];

        assert_eq!(
            cursor_for(&fields, Point::new(10.0, 90.0), 400.0),
            Some(mouse::Interaction::Text)
        );
        assert_eq!(
            cursor_for(&fields, Point::new(210.0, 90.0), 400.0),
            Some(mouse::Interaction::Pointer)
        );
        assert_eq!(cursor_for(&fields, Point::new(310.0, 90.0), 400.0), None);
        assert_eq!(cursor_for(&fields, Point::new(10.0, 200.0), 400.0), None);
    }

    #[test]
    fn choice_fields_offer_options_instead_of_typing() {
        let mut combo = field(FormFieldKind::Combo, 0.0, 0.0, 100.0, 20.0);
        combo.options = vec!["United States".to_string(), "Germany".to_string()];
        assert!(has_options(&combo));
        assert!(!accepts_text(&combo));
        // A choice field reads as clickable; it opens a list, not a caret.
        assert_eq!(
            cursor_for(&[combo.clone()], Point::new(10.0, 390.0), 400.0),
            Some(mouse::Interaction::Pointer)
        );

        // A combo box that allows editing keeps the text input.
        combo.flags = CHOICE_EDIT;
        assert!(has_options(&combo));
        assert!(accepts_text(&combo));

        // Without options a combo box is free text.
        combo.options.clear();
        assert!(!has_options(&combo));
        assert!(accepts_text(&combo));

        // A list box only offers its options.
        let mut listbox = field(FormFieldKind::ListBox, 0.0, 0.0, 100.0, 20.0);
        listbox.options = vec!["One".to_string()];
        assert!(has_options(&listbox));
        assert!(!accepts_text(&listbox));

        assert!(accepts_text(&field(
            FormFieldKind::Text,
            0.0,
            0.0,
            1.0,
            1.0
        )));
    }

    #[test]
    fn tab_order_skips_fields_the_keyboard_cannot_drive() {
        let mut combo = field(FormFieldKind::Combo, 0.0, 0.0, 10.0, 10.0);
        combo.options = vec!["One".to_string()];
        let fields = [
            field(FormFieldKind::Text, 0.0, 0.0, 10.0, 10.0),
            field(FormFieldKind::PushButton, 0.0, 0.0, 10.0, 10.0),
            combo,
            field(FormFieldKind::Checkbox, 0.0, 0.0, 10.0, 10.0),
            field(FormFieldKind::Unknown, 0.0, 0.0, 10.0, 10.0),
        ];

        assert_eq!(next_field(&fields, None, true), Some(0));
        assert_eq!(next_field(&fields, None, false), Some(3));
        assert_eq!(next_field(&fields, Some(0), true), Some(3));
        assert_eq!(next_field(&fields, Some(3), true), Some(0));
        assert_eq!(next_field(&fields, Some(0), false), Some(3));
        assert_eq!(
            next_field(
                &[field(FormFieldKind::Signature, 0.0, 0.0, 1.0, 1.0)],
                None,
                true
            ),
            None
        );
    }
}
