use crate::layout::engine::LayoutContext;
use crate::layout::fragment::FragmentArena;
use crate::layout::inline::atomic::layout_atomic_inline;
use crate::layout::inline::collect::{InlineItem, PreparedInlineItem, PreparedInlineItemKind};
use crate::layout::text::TextMeasurer;

pub(crate) fn prepare_inline_items<T: TextMeasurer + ?Sized>(
    items: Vec<InlineItem>,
    ctx: &mut LayoutContext<T>,
    max_width: f32,
    arena: &mut FragmentArena,
) -> Vec<PreparedInlineItem> {
    let mut prepared_items: Vec<PreparedInlineItem> = Vec::new();
    let space_width = ctx.text_measurer.measure(" ");

    for item in items {
        if let Some(text) = item.get_text() {
            let mut current_text = String::new();

            for c in text.chars() {
                match c {
                    ' ' => {
                        let fit_width = ctx.text_measurer.measure(&current_text);
                        if current_text.is_empty() {
                            if let Some(last) = prepared_items.last_mut()
                                && !last.is_forced_break()
                                && last.width == last.fit_width
                            {
                                last.width += space_width;
                            }
                        } else {
                            let word = std::mem::take(&mut current_text);
                            prepared_items.push(PreparedInlineItem {
                                kind: PreparedInlineItemKind::TextSegment {
                                    dom_node: item.dom_node,
                                    text: word,
                                },
                                width: fit_width + space_width,
                                fit_width,
                                height: ctx.text_measurer.line_height(),
                            });
                            current_text.clear();
                        }
                    }
                    _ => current_text.push(c),
                }
            }
            if !current_text.is_empty() {
                let width = ctx.text_measurer.measure(&current_text);
                prepared_items.push(PreparedInlineItem {
                    kind: PreparedInlineItemKind::TextSegment {
                        dom_node: item.dom_node,
                        text: current_text.clone(),
                    },
                    width,
                    fit_width: width,
                    height: ctx.text_measurer.line_height(),
                });
            }
        } else if item.is_forced_break() {
            prepared_items.push(PreparedInlineItem {
                kind: PreparedInlineItemKind::ForcedBreak,
                width: 0.0,
                fit_width: 0.0,
                height: 0.0,
            });
        } else if item.is_atomic_inline()
            && let Some(fragment) = layout_atomic_inline(item.dom_node, max_width, ctx, arena)
        {
            let margin = fragment
                .style_id()
                .and_then(|style_id| ctx.style_cache.get(style_id))
                .map(|s| s.margin)
                .unwrap_or_default();
            let outer_width = fragment.width() + margin.horizontal();
            let outer_height = fragment.height() + margin.vertical();
            prepared_items.push(PreparedInlineItem {
                width: outer_width,
                fit_width: outer_width,
                height: outer_height,
                kind: PreparedInlineItemKind::AtomicInline { fragment },
            });
        }
    }

    prepared_items
}
