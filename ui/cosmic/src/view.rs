// SPDX-License-Identifier: GPL-3.0-only
// ui/cosmic/src/view.rs
//
// Pure view: builds widgets from the application model. Never changes state.

use cosmic::iced::mouse;
use cosmic::iced::{Alignment, ContentFit, Length};
use cosmic::prelude::*;
use cosmic::widget::{self, tab_bar};

use noctua_core::storage;

use crate::fl;
use crate::message::Message;
use crate::model::{AppModel, CurrentTarget, DocumentPreview};
use crate::widget::document_preview::document_preview;
use crate::widget::empty_state::empty_state;
use crate::widget::fresh_image::FreshImage;
use crate::widget::image_viewer::Viewer;
use crate::widget::scroll_zoom::ScrollZoom;
use crate::widget::thumbnail_strip::thumbnail_strip;
use crate::widget::toolbar::ResponsiveToolbar;

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
            .on_activate(Message::TabActivated)
            .on_close(Message::TabCloseRequested)
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
        // cosmic-viewer's, below the viewer.
        let mut content = widget::column::with_capacity(2).spacing(space.space_xxs);
        content = content.push(content_view(app));
        if app.current_target.is_some() {
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

/// The thumbnail strip of the active tab: image tiles, lazy thumbnails.
fn strip_view(app: &AppModel) -> Element<'_, Message> {
    let Some(tab) = app.active_tab() else {
        return widget::container(widget::text::body("")).into();
    };
    let Some(state) = app.tab_ui.get(&tab) else {
        return widget::container(widget::text::body("")).into();
    };

    thumbnail_strip(&state.strip, state.selected, state.expanded.as_ref(), tab)
}

/// The active tab's continuous PDF preview, if it matches the current target.
fn active_preview(app: &AppModel) -> Option<&DocumentPreview> {
    let tab = app.active_tab()?;
    let state = app.tab_ui.get(&tab)?;
    let preview = state.preview.as_ref()?;
    matches!(&app.current_target, Some(CurrentTarget::File { path }) if path == &preview.path)
        .then_some(preview)
}

/// A toolbar icon button with a tooltip, publishing the message when pressed.
fn icon_button(name: &'static str, tooltip: String, message: Message) -> Element<'static, Message> {
    widget::button::icon(widget::icon::from_name(name))
        .tooltip(tooltip)
        .on_press(message)
        .into()
}

/// The view toolbar: navigation, transform (rotate/flip) and zoom actions for
/// the active document, matching cosmic-viewer's bottom-center panel. Pure UI
/// — every button is a message routed to the core. The panel is identical for
/// single images and the continuous PDF preview.
fn view_toolbar(app: &AppModel) -> Element<'_, Message> {
    let space = cosmic::theme::spacing();
    let (fit, scale) = current_zoom(app);
    let zoom_label = if fit {
        fl!("fit")
    } else {
        format!("{:.0}%", scale * 100.0)
    };

    ResponsiveToolbar::new()
        .start(icon_button(
            "go-previous-symbolic",
            fl!("previous"),
            Message::PrevEntry,
        ))
        .start(icon_button(
            "go-next-symbolic",
            fl!("next"),
            Message::NextEntry,
        ))
        .start(icon_button(
            "object-rotate-left-symbolic",
            fl!("rotate-counter-clockwise"),
            Message::RotateCounterClockwise,
        ))
        .start(icon_button(
            "object-rotate-right-symbolic",
            fl!("rotate-clockwise"),
            Message::RotateClockwise,
        ))
        .start(icon_button(
            "object-flip-horizontal-symbolic",
            fl!("flip-horizontal"),
            Message::FlipHorizontal,
        ))
        .start(icon_button(
            "object-flip-vertical-symbolic",
            fl!("flip-vertical"),
            Message::FlipVertical,
        ))
        .center(
            widget::row::with_capacity(3)
                .spacing(space.space_xxs)
                .align_y(Alignment::Center)
                .push(icon_button(
                    "zoom-out-symbolic",
                    fl!("zoom-out"),
                    Message::ZoomOut,
                ))
                .push(
                    ScrollZoom::new(
                        widget::button::custom(widget::text::body(zoom_label))
                            .class(widget::button::ButtonClass::Icon)
                            .force_enabled(true),
                    )
                    .require_ctrl(false)
                    .on_zoom(|delta| Message::ZoomBy(scroll_steps(delta))),
                )
                .push(icon_button(
                    "zoom-in-symbolic",
                    fl!("zoom-in"),
                    Message::ZoomIn,
                )),
        )
        .end(icon_button(
            "view-actual-size-symbolic",
            fl!("zoom-100"),
            Message::Zoom100,
        ))
        .end(icon_button(
            "view-fit-symbolic",
            fl!("zoom-fit"),
            Message::ZoomToFit,
        ))
        .end(icon_button(
            "view-fullscreen-symbolic",
            fl!("fullscreen"),
            Message::ToggleFullscreen,
        ))
        .view()
}

/// The content area view.
fn content_view(app: &AppModel) -> Element<'_, Message> {
    let space = cosmic::theme::spacing();

    if app.tabs.is_empty() {
        let hint = match &app.start_error {
            Some(path) => fl!("start-error", path = path),
            None => fl!("open-folder-hint"),
        };
        return empty_state("folder-open-symbolic", hint);
    }

    // Continuous preview of the selected multi-page PDF.
    if let Some(preview) = active_preview(app) {
        return document_preview(preview, app.keyboard_modifiers);
    }

    match &app.current_image {
        Some(image) => {
            // Known upstream issue: the iced image atlas splits images
            // larger than its 2048 px layers into fragments and its
            // upload bounds check drops the bottom-right fragment, so
            // such images render with a missing block. Deliberately
            // not worked around — fix belongs in iced/libcosmic.
            let Some(target) = app.current_target.clone() else {
                return empty_state("image-x-generic-symbolic", fl!("select-hint"));
            };
            let state = app.zoom_states.get(&target).copied().unwrap_or_default();

            let content_fit = if state.fit {
                ContentFit::ScaleDown
            } else {
                ContentFit::None
            };
            let (scale, offset_x, offset_y) = if state.fit {
                (1.0, 0.0, 0.0)
            } else {
                (state.scale, state.offset_x, state.offset_y)
            };

            let handle_id = image.handle.id();

            widget::container(FreshImage::new(
                Viewer::new(image.handle.clone())
                    .content_fit(content_fit)
                    .modifiers(app.keyboard_modifiers)
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .with_state(scale, offset_x, offset_y)
                    .on_state_change(move |scale, offset_x, offset_y| {
                        Message::ViewerStateChanged {
                            target: target.clone(),
                            scale,
                            offset_x,
                            offset_y,
                        }
                    }),
                handle_id,
            ))
            .padding(space.space_m)
            .into()
        }
        None => empty_state("image-x-generic-symbolic", fl!("select-hint")),
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

    if let Some(target) = &app.current_target {
        let path = match target {
            CurrentTarget::File { path } => path,
            CurrentTarget::Page { path, .. } => path,
        };
        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            info = info.push(widget::text::body(name.to_string()));
        }
    }

    if let Some(size) = app.current_size {
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
    let total = state.strip.len();
    let Some(position) = state.selected else {
        return String::new();
    };

    format!("{} / {total}", position + 1)
}

/// The current zoom of the active target: `(fit, scale)`.
fn current_zoom(app: &AppModel) -> (bool, f32) {
    let Some(target) = &app.current_target else {
        return (true, 1.0);
    };

    // PDF previews keep their zoom in the preview itself. A preview exists
    // only for PDFs, so matching its path is enough — no need to re-read the
    // file's magic bytes on every frame.
    if let CurrentTarget::File { path } = target
        && let Some(zoom) = app
            .active_tab()
            .and_then(|tab| app.tab_ui.get(&tab))
            .and_then(|state| state.preview.as_ref())
            .filter(|preview| &preview.path == path)
            .map(|preview| preview.zoom)
    {
        return (false, zoom);
    }

    let state = app.zoom_states.get(target).copied().unwrap_or_default();
    (state.fit, state.scale)
}

/// Convert a wheel scroll delta into zoom steps (lines; ~50 px per line).
fn scroll_steps(delta: mouse::ScrollDelta) -> f32 {
    match delta {
        mouse::ScrollDelta::Lines { y, .. } => y,
        mouse::ScrollDelta::Pixels { y, .. } => y / 50.0,
    }
}
