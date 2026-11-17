use crate::geom::{PhysicalOffset, PhysicalRect, PhysicalSize};
use crate::layout::inline::collect::PreparedInlineItemKind;
use crate::layout::inline::line_breaker::Line;
use crate::layout::{
    FragmentArena, FragmentKind, LayoutContext, LayoutFragment, LayoutFragmentId, TextMeasurer,
};
use blink_dom::DomNodeId;

mod atomic;
mod collect;
mod line_breaker;
mod prepare;

fn build_line_fragments<M: TextMeasurer + ?Sized>(
    lines: Vec<Line>,
    content_origin: PhysicalOffset,
    current_y_cursor: &mut f32,
    ctx: &mut LayoutContext<M>,
    arena: &mut FragmentArena,
    child_fragments: &mut Vec<LayoutFragment>,
) -> f32 {
    let mut run_max_width: f32 = 0.0;

    for line in lines {
        run_max_width = run_max_width.max(line.width);

        let mut line_x = 0.0;
        let mut positioned_items = Vec::with_capacity(line.items.len());

        for item in line.items {
            let width = item.width;
            positioned_items.push((line_x, item));
            line_x += width;
        }

        let mut next_child_id: Option<LayoutFragmentId> = None;
        // we're iterating in reverse order to avoid mutating the fragment to insert the
        // next_sibling link after it's already been set by a previous iteration
        for (x, item) in positioned_items.into_iter().rev() {
            match item.kind {
                PreparedInlineItemKind::TextSegment { dom_node, text } => {
                    let text_rect = PhysicalRect::new(
                        PhysicalOffset::new(x, 0.0),
                        PhysicalSize::new(item.fit_width, item.height),
                    );
                    let (text_style_id, _) =
                        ctx.style_cache
                            .get_or_compute(dom_node, ctx.dom, ctx.stylesheet);
                    let fragment = LayoutFragment::new(
                        Some(dom_node),
                        text_rect,
                        Some(text_style_id),
                        FragmentKind::Text(text.clone()),
                    )
                    .with_next_sibling(next_child_id);

                    next_child_id = Some(arena.add_fragment(fragment));
                }
                PreparedInlineItemKind::AtomicInline { fragment } => {
                    let margin = fragment
                        .style_id()
                        .and_then(|style_id| ctx.style_cache.get(style_id))
                        .map(|s| s.margin)
                        .unwrap_or_default();
                    let new_origin = PhysicalOffset::new(x + margin.left, margin.top);
                    next_child_id = Some(
                        arena.add_fragment(
                            fragment
                                .with_origin(new_origin)
                                .with_next_sibling(next_child_id),
                        ),
                    );
                }
                PreparedInlineItemKind::ForcedBreak => {}
            }
        }
        // at the end, next_child_id is the first child of the line fragment

        let line_rect = PhysicalRect::new(
            PhysicalOffset::new(content_origin.x, content_origin.y + *current_y_cursor),
            PhysicalSize::new(line.width, line.height),
        );
        let line_fragment = LayoutFragment::new(None, line_rect, None, FragmentKind::Line)
            .with_first_child(next_child_id);

        child_fragments.push(line_fragment);

        *current_y_cursor += line.height;
    }

    run_max_width
}

pub(super) fn layout_inline_run<M: TextMeasurer + ?Sized>(
    inline_run: &mut Vec<DomNodeId>,
    max_width: f32,
    content_origin: PhysicalOffset,
    current_y_cursor: &mut f32,
    ctx: &mut LayoutContext<M>,
    arena: &mut FragmentArena,
    child_fragments: &mut Vec<LayoutFragment>,
) -> f32 {
    let items = collect::collect_inline_items(inline_run, ctx);
    let prepared_items = prepare::prepare_inline_items(items, ctx, max_width, arena);

    let min_line_height = ctx.text_measurer.line_height();
    let lines = line_breaker::break_lines(prepared_items, max_width, min_line_height);

    let run_max_width = build_line_fragments(
        lines,
        content_origin,
        current_y_cursor,
        ctx,
        arena,
        child_fragments,
    );
    inline_run.clear();

    run_max_width
}
