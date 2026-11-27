// SPDX-License-Identifier: GPL-3.0-only
// ui/cosmic/src/view.rs
//
// Pure view: builds widgets from the application model. Never changes state.

use cosmic::iced::alignment::Alignment;
use cosmic::iced::keyboard::key::Named;
use cosmic::iced::mouse;
use cosmic::iced::{Length, Point, Size};
use cosmic::prelude::*;
use cosmic::widget::{self, tab_bar};

use std::path::Path;

use noctua_core::pdfium_ops::model::AnnotationColor;
use noctua_core::storage;

use crate::fl;
use crate::message::{AnnotationMessage, AppMessage, BrowserMessage, Message, ViewMessage};
use crate::model::{AnnotationTool, AppModel, TabContent, Target};
use crate::widget::annotation_strip::annotation_strip;
use crate::widget::document_preview::document_preview;
use crate::widget::empty_placeholder::empty_placeholder;
use crate::widget::focus_scope::focus_scope;
use crate::widget::image_viewer::TextCaret;
use crate::widget::scroll_zoom::ScrollZoom;
use crate::widget::thumbnail_strip::thumbnail_strip;
use crate::widget::toolbar::ResponsiveToolbar;
use crate::widget::viewer_frame::viewer_frame;

/// Describes the interface based on the current state of the application model.
///
/// Application events will be processed through the view. Any messages emitted by
/// events received by widgets will be passed to the update method.
pub(crate) fn view(app: &AppModel) -> Element<'_, Message> {
    let space = cosmic::theme::spacing();

    let mut column = widget::column::with_capacity(3).spacing(space.space_xxs);

    // A single tab is the whole window; only show the tab bar with more,
    // matching COSMIC Terminal.
    if app.tabs.len() > 1 {
        let tab_strip = tab_bar::horizontal(&app.tab_model)
            .on_activate(|id| Message::Browser(BrowserMessage::TabActivated(id)))
            .on_close(|id| Message::Browser(BrowserMessage::TabCloseRequested(id)))
            .button_height(32)
            .button_spacing(space.space_xxs);
        column = column.push(tab_strip);
    }

    if app.tabs.is_empty() {
        column = column.push(content_view(app));
    } else {
        let mut row = widget::row::with_capacity(2).spacing(space.space_xxs);
        if app.show_nav_panel {
            row = row.push(strip_view(app));
        }
        // Content side: the view toolbar floats bottom-center like
        // cosmic-viewer's, below the viewer. Shown for a selected folder
        // entry or an active annotation tab.
        let is_annotation = app
            .active_tab()
            .is_some_and(|tab| matches!(app.tabs.get(&tab), Some(TabContent::Annotation { .. })));
        let has_toolbar = app.displayed_target.is_some() || is_annotation;
        let mut content = widget::column::with_capacity(3).spacing(space.space_xxs);
        content = content.push(content_view(app));
        if has_toolbar {
            content = content.push(
                widget::container(view_toolbar(app))
                    .center_x(Length::Fill)
                    .padding([space.space_xxs, 0, space.space_xxs, 0]),
            );
        }
        row = row.push(content.width(Length::Fill).height(Length::Fill));
        column = column.push(row.height(Length::Fill));
    }

    column = column.push(status_bar(app));

    widget::container(column)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

/// The rename dialog of an annotation tab, if one is open. The field edits the
/// target file name without its extension; the extension is kept.
pub(crate) fn rename_dialog(app: &AppModel) -> Option<Element<'_, Message>> {
    let draft = app.rename.as_ref()?;

    let field = widget::text_input("", draft.text.as_str())
        .on_input(|text| Message::Annotation(AnnotationMessage::RenameAnnotationChanged(text)))
        .on_submit(|_| Message::Annotation(AnnotationMessage::RenameAnnotationConfirmed));

    Some(
        widget::dialog()
            .title(fl!("rename-annotation"))
            .body(fl!("rename-annotation-body"))
            .control(field)
            .primary_action(
                widget::button::suggested(fl!("rename")).on_press(Message::Annotation(
                    AnnotationMessage::RenameAnnotationConfirmed,
                )),
            )
            .secondary_action(widget::button::standard(fl!("cancel")).on_press(
                Message::Annotation(AnnotationMessage::RenameAnnotationCancelled),
            ))
            .into(),
    )
}

/// The thumbnail strip of the active tab: image tiles, lazy thumbnails.
fn strip_view(app: &AppModel) -> Element<'_, Message> {
    let Some(tab) = app.active_tab() else {
        return widget::container(widget::text::body("")).into();
    };
    if matches!(app.tabs.get(&tab), Some(TabContent::Annotation { .. })) {
        let Some(state) = app.annotation_ui.get(&tab) else {
            return widget::container(widget::text::body("")).into();
        };
        return annotation_strip(state, tab, &|page| {
            crate::menu::page_context_menu(tab, page)
        });
    }
    let Some(state) = app.tab_ui.get(&tab) else {
        return widget::container(widget::text::body("")).into();
    };

    thumbnail_strip(
        state.entries(),
        state.selected(),
        state.expanded(),
        tab,
        &|_| crate::menu::file_context_menu(app),
    )
}

/// Wrap `content` in a right-click context menu for `path`, if applicable.
///
/// Workaround: no `on_surface_action`, because this libcosmic revision places
/// a Wayland popup surface away from the click. Without it the menu is drawn
/// inside the window and cannot extend past its edge.
fn with_file_context_menu<'a>(
    app: &AppModel,
    path: &Path,
    content: Element<'a, Message>,
) -> Element<'a, Message> {
    let Some(menu) = crate::menu::file_context_menu(app) else {
        return content;
    };
    cosmic::widget::context_menu(content, Some(menu))
        .on_open(Message::Browser(BrowserMessage::ContextOpened {
            path: path.to_path_buf(),
        }))
        .on_surface_action(|action| Message::App(AppMessage::Surface(action)))
        .into()
}

/// The content view of an annotation tab: the selected page, fitted. The text
/// being typed lives in the document itself, so no overlay is drawn here.
fn annotation_view(app: &AppModel) -> Element<'_, Message> {
    let Some(tab) = app.active_tab() else {
        return empty_placeholder("image-x-generic-symbolic", fl!("select-hint"));
    };
    let Some(state) = app.annotation_ui.get(&tab) else {
        return empty_placeholder("image-x-generic-symbolic", fl!("select-hint"));
    };
    let Some(current) = state.current() else {
        return empty_placeholder("image-x-generic-symbolic", fl!("select-hint"));
    };

    let cursor = if state.active_tool() == Some(AnnotationTool::Text) {
        mouse::Interaction::Text
    } else {
        mouse::Interaction::Grab
    };

    // The core reports the caret in page points with the origin at the
    // bottom-left; the viewer draws in image pixels from the top-left. The page
    // is rendered at one pixel per point.
    let image_height = current.rgba.1 as f32;
    let caret = if let Some(pending) = state.pending_text().filter(|_| state.caret_visible()) {
        let (x, y, width, height) = pending.caret;
        let color = state.annotation_color();
        Some(TextCaret {
            position: Point::new(x, image_height - (y + height)),
            size: Size::new(width, height),
            color: cosmic::iced::Color::from_rgba8(
                color.red,
                color.green,
                color.blue,
                f32::from(color.alpha) / 255.0,
            ),
        })
    } else {
        // While a form field is edited, the caret lives inside the field.
        state.form_focus().and_then(|index| {
            let field = state.form_fields().get(index)?;
            if !crate::widget::form_fields::accepts_text(field) {
                return None;
            }
            let (x, bottom, height) = crate::widget::form_fields::caret_in_field(
                field,
                state.form_draft(),
                state.form_caret(),
            );
            Some(TextCaret {
                position: Point::new(x, image_height - (bottom + height)),
                size: Size::new(1.0, height),
                color: cosmic::iced::Color::from_rgba8(0, 0, 0, 1.0),
            })
        })
    };

    // While a field or free text is edited, Tab and Escape belong to the page.
    // An open dialog keeps them for itself.
    let editing =
        (state.form_focus().is_some() || state.pending_text().is_some()) && app.rename.is_none();
    let element: Element<'static, Message> = focus_scope(viewer_frame(
        &current.handle,
        state.view(),
        app.keyboard_modifiers,
        app.viewer_bounds.clone(),
        cursor,
        move |scale, offset_x, offset_y| {
            Message::Annotation(AnnotationMessage::AnnotationViewerStateChanged {
                tab,
                scale,
                offset_x,
                offset_y,
            })
        },
        Some(Box::new(move |native, anchor| {
            Message::Annotation(AnnotationMessage::AnnotationPointerClicked {
                tab,
                x: native.x,
                y: native.y,
                anchor,
            })
        })),
        Some(Box::new({
            let fields = state.form_fields().to_vec();
            move |point: Point| crate::widget::form_fields::cursor_for(&fields, point, image_height)
        })),
        caret,
    ))
    .active(editing)
    .capture(&[Named::Tab, Named::Escape])
    .on_key(|key| Message::Annotation(AnnotationMessage::AnnotationTextKey(key)))
    .into();

    // The option list of a choice field is drawn over the page where the field
    // was clicked. The viewer sits inside its frame's padding, so the click
    // position moves into that frame before it anchors the popup.
    let Some(anchor) = state.form_menu() else {
        return element;
    };
    let Some(field) = state
        .form_focus()
        .and_then(|index| state.form_fields().get(index))
        .filter(|field| crate::widget::form_fields::has_options(field))
    else {
        return element;
    };
    let padding = f32::from(cosmic::theme::spacing().space_m);
    widget::popover(element)
        .position(widget::popover::Position::Point(Point::new(
            anchor.x + padding,
            anchor.y + padding,
        )))
        .popup(crate::widget::form_fields::form_option_list(tab, field))
        .on_close(Message::Annotation(
            AnnotationMessage::AnnotationFormMenuClosed,
        ))
        .into()
}

/// Free-text font size presets, in points.
const FONT_SIZE_PRESETS: [f32; 8] = [12.0, 16.0, 20.0, 24.0, 32.0, 48.0, 64.0, 96.0];

/// Preset text colours for the free-text tool (label, colour).
fn text_color_presets() -> Vec<(String, AnnotationColor)> {
    use noctua_core::pdfium_ops::model::palette;
    vec![
        (fl!("color-black"), AnnotationColor::rgb(0, 0, 0)),
        (fl!("color-white"), AnnotationColor::rgb(255, 255, 255)),
        (fl!("color-red"), palette::RED),
        (fl!("color-orange"), palette::ORANGE),
        (fl!("color-yellow"), palette::YELLOW),
        (fl!("color-green"), palette::GREEN),
        (fl!("color-blue"), palette::BLUE),
    ]
}

/// A toolbar icon button with a tooltip, publishing the message when pressed.
fn icon_button(name: &'static str, tooltip: String, message: Message) -> Element<'static, Message> {
    widget::button::icon(widget::icon::from_name(name))
        .tooltip(tooltip)
        .on_press(message)
        .into()
}

/// A toolbar icon button that is disabled (no `on_press`) when the action does
/// not apply, for example moving the first page up.
fn page_button(
    name: &'static str,
    tooltip: String,
    message: Message,
    enabled: bool,
) -> Element<'static, Message> {
    let mut button = widget::button::icon(widget::icon::from_name(name)).tooltip(tooltip);
    if enabled {
        button = button.on_press(message);
    }
    button.into()
}

/// The view toolbar: navigation, transform (rotate/flip) and zoom actions for
/// the active document, matching cosmic-viewer's bottom-center panel. In
/// annotation mode it additionally leads with the annotation tools, so a
/// single toolbar covers both modes instead of a second panel. Pure UI —
/// every button is a message routed to the core.
fn view_toolbar(app: &AppModel) -> Element<'_, Message> {
    let space = cosmic::theme::spacing();
    let zoom_label = format!("{:.0}%", app.effective_zoom() * 100.0);

    // In an annotation tab, rotation edits the selected page in the document
    // (persistent); everywhere else it only turns the view. The flip buttons
    // stay view-only, since PDF pages have no flip operation.
    let annotation_page = app.active_tab().and_then(|tab| match app.tabs.get(&tab) {
        Some(TabContent::Annotation { .. }) => app
            .annotation_ui
            .get(&tab)
            .map(|state| (tab, state.selected())),
        _ => None,
    });
    let (rotate_left, rotate_right) = match annotation_page {
        Some((tab, page)) => (
            Message::Annotation(AnnotationMessage::AnnotationRotatePage {
                tab,
                page,
                clockwise: false,
            }),
            Message::Annotation(AnnotationMessage::AnnotationRotatePage {
                tab,
                page,
                clockwise: true,
            }),
        ),
        None => (
            Message::View(ViewMessage::RotateCounterClockwise),
            Message::View(ViewMessage::RotateClockwise),
        ),
    };

    let mut toolbar = ResponsiveToolbar::new();

    // Annotation tools (text + font size) lead the toolbar in annotation mode.
    if let Some(state) = app.active_tab().and_then(|tab| match app.tabs.get(&tab) {
        Some(TabContent::Annotation { .. }) => app.annotation_ui.get(&tab),
        _ => None,
    }) {
        let text_active = state.active_tool() == Some(AnnotationTool::Text);
        toolbar = toolbar.start(
            widget::button::icon(widget::icon::from_name("insert-text-symbolic"))
                .tooltip(fl!("text-tool"))
                .selected(text_active)
                .on_press(Message::Annotation(
                    AnnotationMessage::AnnotationToolToggled(AnnotationTool::Text),
                )),
        );
        if text_active {
            let labels: Vec<String> = FONT_SIZE_PRESETS
                .iter()
                .map(|size| format!("{size:.0} pt"))
                .collect();
            let selected = FONT_SIZE_PRESETS
                .iter()
                .position(|size| (*size - state.font_size_pt()).abs() < f32::EPSILON);
            toolbar = toolbar.start(cosmic::widget::dropdown(labels, selected, |index| {
                Message::Annotation(AnnotationMessage::AnnotationFontSizeChanged(
                    FONT_SIZE_PRESETS[index],
                ))
            }));

            let colors = text_color_presets();
            let color_labels: Vec<String> = colors.iter().map(|(name, _)| name.clone()).collect();
            let color_selected = colors
                .iter()
                .position(|(_, color)| *color == state.annotation_color());
            toolbar = toolbar.start(cosmic::widget::dropdown(
                color_labels,
                color_selected,
                move |index| {
                    Message::Annotation(AnnotationMessage::AnnotationColorChanged(colors[index].1))
                },
            ));
        }
    }

    toolbar = toolbar
        .start(icon_button(
            "go-previous-symbolic",
            fl!("previous"),
            Message::Browser(BrowserMessage::PrevEntry),
        ))
        .start(icon_button(
            "go-next-symbolic",
            fl!("next"),
            Message::Browser(BrowserMessage::NextEntry),
        ))
        .start(icon_button(
            "object-rotate-left-symbolic",
            fl!("rotate-counter-clockwise"),
            rotate_left,
        ))
        .start(icon_button(
            "object-rotate-right-symbolic",
            fl!("rotate-clockwise"),
            rotate_right,
        ));

    // Page operations belong to the annotation tab's own toolbar section.
    if let Some((tab, page)) = annotation_page {
        let page_count = app
            .annotation_ui
            .get(&tab)
            .map(|state| state.page_sizes().len() as u32)
            .unwrap_or(0);
        toolbar = toolbar
            .start(page_button(
                "go-up-symbolic",
                fl!("move-page-up"),
                Message::Annotation(AnnotationMessage::AnnotationMovePage {
                    tab,
                    page,
                    up: true,
                }),
                page > 1,
            ))
            .start(page_button(
                "go-down-symbolic",
                fl!("move-page-down"),
                Message::Annotation(AnnotationMessage::AnnotationMovePage {
                    tab,
                    page,
                    up: false,
                }),
                page < page_count,
            ))
            .start(page_button(
                "user-trash-symbolic",
                fl!("delete-page"),
                Message::Annotation(AnnotationMessage::AnnotationDeletePage { tab, page }),
                // A document needs at least one page.
                page_count > 1,
            ));
    }

    toolbar = toolbar
        .start(icon_button(
            "object-flip-horizontal-symbolic",
            fl!("flip-horizontal"),
            Message::View(ViewMessage::FlipHorizontal),
        ))
        .start(icon_button(
            "object-flip-vertical-symbolic",
            fl!("flip-vertical"),
            Message::View(ViewMessage::FlipVertical),
        ))
        .center(
            widget::row::with_capacity(3)
                .spacing(space.space_xxs)
                .align_y(Alignment::Center)
                .push(icon_button(
                    "zoom-out-symbolic",
                    fl!("zoom-out"),
                    Message::View(ViewMessage::ZoomOut),
                ))
                .push(
                    ScrollZoom::new(
                        widget::button::custom(widget::text::body(zoom_label))
                            .class(widget::button::ButtonClass::Icon)
                            .force_enabled(true),
                    )
                    .require_ctrl(false)
                    .on_zoom(|delta| Message::View(ViewMessage::ZoomBy(scroll_steps(delta)))),
                )
                .push(icon_button(
                    "zoom-in-symbolic",
                    fl!("zoom-in"),
                    Message::View(ViewMessage::ZoomIn),
                )),
        )
        .end(icon_button(
            "view-actual-size-symbolic",
            fl!("zoom-100"),
            Message::View(ViewMessage::Zoom100),
        ))
        .end(icon_button(
            "view-fit-symbolic",
            fl!("zoom-fit"),
            Message::View(ViewMessage::ZoomToFit),
        ))
        .end(icon_button(
            "view-fullscreen-symbolic",
            fl!("fullscreen"),
            Message::View(ViewMessage::ToggleFullscreen),
        ));

    toolbar.view()
}

/// The content area view.
fn content_view(app: &AppModel) -> Element<'_, Message> {
    if app.tabs.is_empty() {
        let hint = match &app.start_error {
            Some(path) => fl!("start-error", path = path),
            None => fl!("open-folder-hint"),
        };
        return empty_placeholder("folder-open-symbolic", hint);
    }

    // Annotation tab: render its pages.
    if let Some(tab) = app.active_tab()
        && matches!(app.tabs.get(&tab), Some(TabContent::Annotation { .. }))
    {
        return annotation_view(app);
    }

    // Continuous preview of the selected multi-page PDF.
    if let Some(preview) = app.active_preview() {
        let path = preview.path.clone();
        let viewer = document_preview(preview, app.keyboard_modifiers);
        return with_file_context_menu(app, &path, viewer);
    }

    match &app.displayed_image {
        Some(image) => {
            let Some(target) = app.displayed_target.clone() else {
                return empty_placeholder("image-x-generic-symbolic", fl!("select-hint"));
            };
            let path = target.path().to_path_buf();
            let state = app
                .displayed_entry()
                .map(|entry| entry.view)
                .unwrap_or_default();
            let viewer = viewer_frame(
                &image.handle,
                state,
                app.keyboard_modifiers,
                app.viewer_bounds.clone(),
                mouse::Interaction::Grab,
                move |scale, offset_x, offset_y| {
                    Message::Browser(BrowserMessage::ViewerStateChanged {
                        target: target.clone(),
                        scale,
                        offset_x,
                        offset_y,
                    })
                },
                None,
                None,
                None,
            );
            with_file_context_menu(app, &path, viewer)
        }
        None => empty_placeholder("image-x-generic-symbolic", fl!("select-hint")),
    }
}

/// The status bar view.
fn status_bar(app: &AppModel) -> Element<'_, Message> {
    let space = cosmic::theme::spacing();

    let mut info = widget::row::with_capacity(3)
        .spacing(space.space_s)
        .align_y(Alignment::Center);

    let position = position_label(app);
    if !position.is_empty() {
        info = info.push(widget::text::body(position));
    }

    if let Some(target) = &app.displayed_target {
        let path = match target {
            Target::File { path } => path,
            Target::Page { path, .. } => path,
        };
        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            info = info.push(widget::text::body(name.to_string()));
        }
    }

    if let Some(size) = app.displayed_size {
        info = info.push(widget::text::body(storage::document::format_size(size)));
    }

    let row = widget::row::with_capacity(2)
        .spacing(space.space_s)
        .align_y(Alignment::Center)
        .push(widget::space().width(Length::Fill))
        .push(info);

    widget::container(row)
        .width(Length::Fill)
        .padding([
            space.space_xxs,
            space.space_s,
            space.space_xxs,
            space.space_s,
        ])
        .into()
}

/// Status bar text describing the current strip position.
fn position_label(app: &AppModel) -> String {
    let Some(tab) = app.active_tab() else {
        return String::new();
    };
    let Some(state) = app.tab_ui.get(&tab) else {
        return String::new();
    };
    let total = state.entries().len();
    let Some(position) = state.selected() else {
        return String::new();
    };

    format!("{} / {total}", position + 1)
}

/// Convert a wheel scroll delta into zoom steps (lines; ~50 px per line).
fn scroll_steps(delta: mouse::ScrollDelta) -> f32 {
    match delta {
        mouse::ScrollDelta::Lines { y, .. } => y,
        mouse::ScrollDelta::Pixels { y, .. } => y / 50.0,
    }
}
