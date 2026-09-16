// SPDX-License-Identifier: GPL-3.0-or-later
// ui/cosmic/src/view.rs
//
// Pure view: builds widgets from the application model. Never changes state.

use cosmic::iced::{Alignment, ContentFit, Length};
use cosmic::prelude::*;
use cosmic::widget::{self, tab_bar};

use noctua_core::storage;

use crate::fl;
use crate::message::Message;
use crate::model::{AppModel, CurrentTarget};
use crate::widget::document_preview::document_preview;
use crate::widget::empty_state::empty_state;
use crate::widget::image_viewer::Viewer;
use crate::widget::thumbnail_strip::thumbnail_strip;
use crate::widget::zoom_controls::zoom_controls;

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
        row = row.push(content_view(app));
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
    if let Some(tab) = app.active_tab()
        && let Some(state) = app.tab_ui.get(&tab)
        && let Some(preview) = state.preview.as_ref()
        && matches!(&app.current_target, Some(CurrentTarget::File { path }) if path == &preview.path)
    {
        return document_preview(preview);
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

            widget::container(
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
            )
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

    let row = widget::row::with_capacity(3)
        .spacing(space.space_s)
        .align_y(Alignment::Center)
        .push(widget::space().width(Length::Fill))
        .push(info)
        .push(zoom_controls_view(app));

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

    // PDF previews keep their zoom in the preview itself.
    if let CurrentTarget::File { path } = target
        && storage::document::is_pdf(path).unwrap_or(false)
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

/// Zoom controls reflecting the current zoom of the active target.
fn zoom_controls_view(app: &AppModel) -> Element<'_, Message> {
    let (fit, scale) = current_zoom(app);
    zoom_controls(fit, scale, &app.key_binds)
}
