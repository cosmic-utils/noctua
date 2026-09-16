// SPDX-License-Identifier: GPL-3.0-or-later
// ui/cosmic/src/widget/document_preview.rs
//
// Reusable continuous multi-page preview of a PDF inside a folder tab.

use cosmic::iced::alignment::Horizontal;
use cosmic::iced::{ContentFit, Length};
use cosmic::prelude::*;
use cosmic::widget;

use crate::message::Message;
use crate::model::{DocumentPreview, PREVIEW_SCROLL_ID, PageSlot};

/// Build the continuous multi-page preview. Emits `Message::PreviewScrolled`.
pub(crate) fn document_preview<'a>(preview: &'a DocumentPreview) -> Element<'a, Message> {
    let space = cosmic::theme::spacing();

    let mut column = widget::column::with_capacity(preview.pages.len())
        .spacing(space.space_xs)
        .padding(space.space_m);

    for (index, slot) in preview.pages.iter().enumerate() {
        let (_, height) = preview
            .page_sizes
            .get(index)
            .copied()
            .unwrap_or((600.0, 800.0));
        let box_height = height * preview.zoom;
        let page: Element<'_, Message> = match slot {
            PageSlot::Empty => widget::space()
                .width(Length::Fill)
                .height(Length::Fixed(box_height))
                .into(),
            PageSlot::Thumb { handle } | PageSlot::Full { handle } => widget::container(
                widget::Image::new(handle.clone()).content_fit(ContentFit::ScaleDown),
            )
            .width(Length::Fill)
            .height(Length::Fixed(box_height))
            .align_x(Horizontal::Center)
            .into(),
        };
        column = column.push(page);
    }

    widget::scrollable(column)
        .id(widget::Id::new(PREVIEW_SCROLL_ID))
        .on_scroll(|viewport| Message::PreviewScrolled {
            path: preview.path.clone(),
            offset_y: viewport.absolute_offset().y,
            viewport_height: viewport.bounds().height,
        })
        .into()
}
