use crate::engine::LayoutContext;
use crate::inline::{InlineItem, InlineItemKind, strip_trailing_space};
use crate::style::compute_style;

pub struct Line {
    pub items: Vec<InlineItem>,
    pub width: f32,
    pub height: f32,
}

impl Line {
    pub fn new(items: Vec<InlineItem>, width: f32, height: f32) -> Self {
        Self {
            items,
            width,
            height,
        }
    }
}

pub fn break_lines(items: Vec<InlineItem>, max_width: f32, ctx: &LayoutContext) -> Vec<Line> {
    let mut completed_lines = Vec::new();
    let mut current_line_items = Vec::new();
    let mut current_x = 0.0;
    let mut current_height = 0.0;
    let space_width = ctx.text_measurer.measure(" ");

    for item in items {
        if item.is_forced_break() {
            break_line(
                space_width,
                &mut current_line_items,
                &mut current_x,
                &mut current_height,
                &mut completed_lines,
            );
            continue;
        }

        if item.is_atomic_inline() {
            let item_style = compute_style(item.dom_node, ctx.dom, ctx.stylesheet);
            if current_x > 0.0 && current_x + item_style.width.unwrap_or(0.0) > max_width {
                break_line(
                    space_width,
                    &mut current_line_items,
                    &mut current_x,
                    &mut current_height,
                    &mut completed_lines,
                );
            }
            current_line_items.push(item);
            current_x += item_style.width.unwrap_or(0.0);
            current_height = current_height.max(item_style.height.unwrap_or(0.0));
            continue;
        }

        if let Some(text) = item.get_text() {
            for segment in text.split_inclusive(' ') {
                let advance_width = ctx.text_measurer.measure(segment);
                let fit_width = ctx.text_measurer.measure(segment.trim_end_matches(' '));
                if current_x + fit_width > max_width && !current_line_items.is_empty() {
                    break_line(
                        space_width,
                        &mut current_line_items,
                        &mut current_x,
                        &mut current_height,
                        &mut completed_lines,
                    );
                }
                if let Some(last_item) = current_line_items.last_mut()
                    && last_item.is_text()
                {
                    last_item.append_text(segment);
                } else {
                    current_line_items.push(InlineItem {
                        dom_node: item.dom_node,
                        kind: InlineItemKind::Text(segment.to_string()),
                    });
                }
                current_x += advance_width;
                current_height = current_height.max(ctx.text_measurer.line_height());
            }
        }
    }

    if !current_line_items.is_empty() {
        break_line(
            space_width,
            &mut current_line_items,
            &mut current_x,
            &mut current_height,
            &mut completed_lines,
        );
    }

    completed_lines
}

fn break_line(
    space_width: f32,
    current_line_items: &mut Vec<InlineItem>,
    current_x: &mut f32,
    current_height: &mut f32,
    completed_lines: &mut Vec<Line>,
) {
    if strip_trailing_space(current_line_items) {
        *current_x -= space_width;
    }
    completed_lines.push(Line::new(
        std::mem::take(current_line_items),
        *current_x,
        *current_height,
    ));
    *current_x = 0.0;
    *current_height = 0.0;
}
