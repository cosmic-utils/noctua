// SPDX-License-Identifier: GPL-3.0-only
// ui/cosmic/src/widget/annotation_strip.rs
//
// Page list of an annotation tab: one selectable tile per page, with its
// number and a right-click menu built by the caller.

use cosmic::iced::Length;
use cosmic::prelude::*;
use cosmic::widget::segmented_button::Entity;
use cosmic::widget::{self, menu};

use crate::message::Message;
use crate::model::AnnotationUiState;
use crate::widget::strip;

/// Build the page strip of an annotation tab. Each tile publishes
/// `Message::AnnotationPageSelected`; `page_menu` supplies its context menu.
pub(crate) fn annotation_strip<'a>(
    state: &AnnotationUiState,
    tab: Entity,
    page_menu: &dyn Fn(u32) -> Vec<menu::Tree<Message>>,
) -> Element<'a, Message> {
    let space = cosmic::theme::spacing();
    let mut column = strip::column(state.page_sizes.len());

    for index in 0..state.page_sizes.len() {
        let page = index as u32 + 1;
        let thumb = state.thumbs.get(index).and_then(|thumb| thumb.as_ref());
        let body = strip::preview(
            thumb,
            widget::text::caption(page.to_string())
                .width(Length::Fill)
                .height(Length::Fill),
        );
        let tile = strip::tile(body, strip::TILE_SIZE, state.selected == page);

        let entry: Element<'_, Message> = widget::container(
            widget::column::with_capacity(2)
                .spacing(space.space_xxs)
                .push(tile)
                .push(widget::text::caption(page.to_string())),
        )
        .into();

        let interactive =
            widget::mouse_area(entry).on_release(Message::AnnotationPageSelected { tab, page });
        let interactive: Element<'_, Message> =
            cosmic::widget::context_menu(interactive, Some(page_menu(page)))
                .on_surface_action(Message::Surface)
                .into();
        // Stable identity per page, so this tile keeps its own widget tree even
        // when the page list is reordered or the tab switches strips.
        column = column.push(widget::id_container(
            interactive,
            widget::Id::new(format!("annotation:{tab:?}:page:{page}")),
        ));
    }

    strip::panel(widget::scrollable(column))
}
