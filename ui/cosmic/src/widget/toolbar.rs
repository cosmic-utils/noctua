// SPDX-License-Identifier: GPL-3.0-only
// ui/cosmic/src/widget/toolbar.rs
//
// Responsive toolbar with `start`, `center` and `end` sections. Lays out in a
// single row when everything fits, otherwise stacks the sections or wraps
// them so nothing is ever clipped. Adapted from pop-os/cosmic-viewer's
// viewer-toolbar (GPL-3.0-only).

use cosmic::iced::advanced::layout::{self, Node};
use cosmic::iced::advanced::renderer;
use cosmic::iced::advanced::widget::Widget;
use cosmic::iced::advanced::widget::tree::Tree;
use cosmic::iced::advanced::{Clipboard, Layout, Shell};
use cosmic::iced::mouse;
use cosmic::iced::{Event, Length, Point, Rectangle, Size};
use cosmic::{Element, Renderer, Theme};

/// Builder for a responsive toolbar.
pub(crate) struct ResponsiveToolbar<'a, Message> {
    start: Vec<Element<'a, Message>>,
    center: Vec<Element<'a, Message>>,
    end: Vec<Element<'a, Message>>,
    spacing: u16,
}

impl<'a, Message: Clone + 'static> ResponsiveToolbar<'a, Message> {
    pub(crate) fn new() -> Self {
        Self {
            start: Vec::new(),
            center: Vec::new(),
            end: Vec::new(),
            spacing: cosmic::theme::spacing().space_xxs,
        }
    }

    #[must_use]
    pub(crate) fn start(mut self, item: impl Into<Element<'a, Message>>) -> Self {
        self.start.push(item.into());
        self
    }

    #[must_use]
    pub(crate) fn center(mut self, item: impl Into<Element<'a, Message>>) -> Self {
        self.center.push(item.into());
        self
    }

    #[must_use]
    pub(crate) fn end(mut self, item: impl Into<Element<'a, Message>>) -> Self {
        self.end.push(item.into());
        self
    }

    pub(crate) fn view(self) -> Element<'a, Message> {
        let space = cosmic::theme::spacing();
        let reflow = ReflowToolbar::new(
            self.start,
            self.center,
            self.end,
            f32::from(self.spacing),
            f32::from(space.space_xxs),
        );

        cosmic::widget::container(reflow)
            .padding([
                space.space_xxs,
                space.space_s,
                space.space_xxs,
                space.space_s,
            ])
            .width(Length::Shrink)
            .height(Length::Shrink)
            .class(cosmic::style::Container::Primary)
            .into()
    }
}

/// Backing layout widget: children are stored flat in `[start.., center..,
/// end..]` order so the list maps 1:1 onto `tree.children`. `layout()` decides
/// between one row, two rows and a wrapping flow based on the available width.
struct ReflowToolbar<'a, Message> {
    children: Vec<Element<'a, Message>>,
    n_start: usize,
    n_center: usize,
    spacing: f32,
    row_spacing: f32,
}

impl<'a, Message> ReflowToolbar<'a, Message> {
    fn new(
        start: Vec<Element<'a, Message>>,
        center: Vec<Element<'a, Message>>,
        end: Vec<Element<'a, Message>>,
        spacing: f32,
        row_spacing: f32,
    ) -> Self {
        let n_start = start.len();
        let n_center = center.len();
        let mut children = start;
        children.extend(center);
        children.extend(end);
        Self {
            children,
            n_start,
            n_center,
            spacing,
            row_spacing,
        }
    }
}

/// Total width of a contiguous run of items placed with `spacing` between them.
fn run_width(nodes: &[Node], spacing: f32) -> f32 {
    let mut width = 0.0;
    for (i, node) in nodes.iter().enumerate() {
        if i > 0 {
            width += spacing;
        }
        width += node.size().width;
    }
    width
}

/// Tallest item in a run.
fn run_height(nodes: &[Node]) -> f32 {
    nodes.iter().map(|n| n.size().height).fold(0.0, f32::max)
}

/// Place items left-to-right from `x0`, vertically centered in `row_height`.
/// Returns the x just past the last item (no trailing spacing).
fn place_run(nodes: &mut [Node], x0: f32, y0: f32, row_height: f32, spacing: f32) -> f32 {
    let mut x = x0;
    for node in nodes.iter_mut() {
        let height = node.size().height;
        node.move_to_mut(Point::new(x, y0 + (row_height - height) / 2.0));
        x += node.size().width + spacing;
    }
    if nodes.is_empty() { x0 } else { x - spacing }
}

/// Two-row design: `start | end` on the top row, `center` centered below.
fn place_two_rows(
    nodes: &mut [Node],
    n_start: usize,
    center_end: usize,
    spacing: f32,
    row_spacing: f32,
) -> Size {
    let start_w = run_width(&nodes[..n_start], spacing);
    let center_w = run_width(&nodes[n_start..center_end], spacing);
    let has_start = n_start > 0;
    let has_end = nodes.len() > center_end;
    let has_center = center_end > n_start;

    let top_height = run_height(&nodes[..n_start]).max(run_height(&nodes[center_end..]));
    place_run(&mut nodes[..n_start], 0.0, 0.0, top_height, spacing);
    let end_x0 = if has_start { start_w + spacing } else { 0.0 };
    let top_w = place_run(&mut nodes[center_end..], end_x0, 0.0, top_height, spacing).max(start_w);

    let total_w = top_w.max(center_w);
    let center_height = run_height(&nodes[n_start..center_end]);
    let bottom_y = if has_start || has_end {
        top_height + row_spacing
    } else {
        0.0
    };
    place_run(
        &mut nodes[n_start..center_end],
        (total_w - center_w) / 2.0,
        bottom_y,
        center_height,
        spacing,
    );

    let total_h = if has_center {
        bottom_y + center_height
    } else {
        top_height
    };
    Size::new(total_w, total_h)
}

/// Flow every item left-to-right, wrapping to a new row when the next item
/// would exceed `available`. Guarantees nothing is clipped.
fn place_wrapped(nodes: &mut [Node], available: f32, spacing: f32, row_spacing: f32) -> Size {
    let mut rows: Vec<std::ops::Range<usize>> = Vec::new();
    let mut heights: Vec<f32> = Vec::new();
    let mut row_start = 0;
    let mut x = 0.0;
    let mut row_height = 0.0_f32;
    for (i, node) in nodes.iter().enumerate() {
        let size = node.size();
        if x > 0.0 && x + size.width > available {
            rows.push(row_start..i);
            heights.push(row_height);
            row_start = i;
            x = 0.0;
            row_height = 0.0;
        }
        x += size.width + spacing;
        row_height = row_height.max(size.height);
    }
    rows.push(row_start..nodes.len());
    heights.push(row_height);

    let mut y = 0.0;
    let mut total_w = 0.0_f32;
    for (range, height) in rows.into_iter().zip(heights) {
        total_w = total_w.max(place_run(&mut nodes[range], 0.0, y, height, spacing));
        y += height + row_spacing;
    }
    Size::new(total_w, (y - row_spacing).max(0.0))
}

impl<Message> Widget<Message, Theme, Renderer> for ReflowToolbar<'_, Message> {
    fn children(&self) -> Vec<Tree> {
        self.children.iter().map(Tree::new).collect()
    }

    fn diff(&mut self, tree: &mut Tree) {
        tree.diff_children(&mut self.children);
    }

    fn size(&self) -> Size<Length> {
        Size::new(Length::Shrink, Length::Shrink)
    }

    fn layout(&mut self, tree: &mut Tree, renderer: &Renderer, limits: &layout::Limits) -> Node {
        let available = limits.max().width;
        let spacing = self.spacing;
        let row_spacing = self.row_spacing;
        let n_start = self.n_start;
        let center_end = n_start + self.n_center;

        let child_limits = limits.loose();
        let mut nodes: Vec<Node> = self
            .children
            .iter_mut()
            .zip(&mut tree.children)
            .map(|(child, state)| child.as_widget_mut().layout(state, renderer, &child_limits))
            .collect();

        let start_w = run_width(&nodes[..n_start], spacing);
        let center_w = run_width(&nodes[n_start..center_end], spacing);
        let end_w = run_width(&nodes[center_end..], spacing);

        let has_start = n_start > 0;
        let has_center = center_end > n_start;
        let has_end = nodes.len() > center_end;

        let present = u16::from(has_start) + u16::from(has_center) + u16::from(has_end);
        let single_w = spacing.mul_add(
            f32::from(present.saturating_sub(1)),
            start_w + center_w + end_w,
        );

        let top_gap = if has_start && has_end { spacing } else { 0.0 };
        let top_w = start_w + end_w + top_gap;

        let size = if single_w <= available {
            let row_height = run_height(&nodes);
            let width = place_run(&mut nodes, 0.0, 0.0, row_height, spacing);
            Size::new(width, row_height)
        } else if top_w <= available && center_w <= available {
            place_two_rows(&mut nodes, n_start, center_end, spacing, row_spacing)
        } else {
            place_wrapped(&mut nodes, available, spacing, row_spacing)
        };

        Node::with_children(size, nodes)
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        for ((child, state), child_layout) in self
            .children
            .iter()
            .zip(&tree.children)
            .zip(layout.children())
        {
            child.as_widget().draw(
                state,
                renderer,
                theme,
                style,
                child_layout,
                cursor,
                viewport,
            );
        }
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        for ((child, state), child_layout) in self
            .children
            .iter_mut()
            .zip(&mut tree.children)
            .zip(layout.children())
        {
            child.as_widget_mut().update(
                state,
                event,
                child_layout,
                cursor,
                renderer,
                clipboard,
                shell,
                viewport,
            );
        }
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        self.children
            .iter()
            .zip(&tree.children)
            .zip(layout.children())
            .map(|((child, state), child_layout)| {
                child
                    .as_widget()
                    .mouse_interaction(state, child_layout, cursor, viewport, renderer)
            })
            .max()
            .unwrap_or_default()
    }
}

impl<'a, Message: 'a> From<ReflowToolbar<'a, Message>> for Element<'a, Message> {
    fn from(widget: ReflowToolbar<'a, Message>) -> Self {
        Element::new(widget)
    }
}
