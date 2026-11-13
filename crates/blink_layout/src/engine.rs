use crate::atomic_inline::layout_atomic_inline;
use crate::fragment::{FragmentArena, FragmentKind, LayoutFragment, LayoutFragmentId};
use crate::geom::{PhysicalOffset, PhysicalRect, PhysicalSize};
use crate::inline::{InlineItem, PreparedInlineItem, PreparedInlineItemKind, collect_inline_items};
use crate::line_breaker::{Line, break_lines};
use crate::space::ConstraintSpace;
use crate::style::{ComputedStyle, Display, DisplayOutside, compute_style};
use crate::text::TextMeasurer;
use blink_css::CSSStyleSheet;
use blink_dom::{Dom, DomNodeId};

pub struct LayoutContext<'a> {
    pub dom: &'a Dom,
    pub stylesheet: &'a CSSStyleSheet,
    pub text_measurer: Box<dyn TextMeasurer>,
}

pub fn build_layout_tree(
    root_id: DomNodeId,
    ctx: &mut LayoutContext,
    initial_space: ConstraintSpace,
) -> FragmentArena {
    let mut arena = FragmentArena::new();

    let root_fragment_id = walk_node(root_id, ctx, initial_space, &mut arena);
    arena.set_root(root_fragment_id);

    arena
}

pub(crate) fn layout_node(
    node_id: DomNodeId,
    ctx: &mut LayoutContext,
    space: ConstraintSpace,
    arena: &mut FragmentArena,
) -> Option<LayoutFragment> {
    if ctx.dom.get_node(node_id).is_some_and(|node| node.is_text()) {
        return None;
    }

    let computed_style = compute_style(node_id, ctx.dom, ctx.stylesheet);

    if computed_style.display.is_none() {
        return None;
    }

    let node_width = computed_style.width;
    let node_height = computed_style.height;

    let is_inline_level = computed_style.display.outside() == Some(DisplayOutside::Inline);

    let initial_width = node_width.unwrap_or(if is_inline_level {
        0.0
    } else {
        space.available_size.width
    });

    let available_width = node_width.unwrap_or(space.available_size.width);
    let child_space = ConstraintSpace {
        available_size: PhysicalSize::new(available_width, space.available_size.height),
    };

    let (first_child_id, total_child_height, max_line_width) =
        layout_flow_children(node_id, ctx, &child_space, arena);

    let node_size = PhysicalSize::new(
        if node_width.is_none() && is_inline_level {
            max_line_width
        } else {
            initial_width
        },
        if node_height.is_none() {
            total_child_height
        } else {
            node_height.unwrap_or_default()
        },
    );
    let bounding_rect = PhysicalRect::new(PhysicalOffset::default(), node_size);
    Some(
        LayoutFragment::new(
            Some(node_id),
            bounding_rect,
            computed_style,
            FragmentKind::Box,
        )
        .with_first_child(first_child_id),
    )
}

pub(crate) fn walk_node(
    node_id: DomNodeId,
    ctx: &mut LayoutContext,
    space: ConstraintSpace,
    arena: &mut FragmentArena,
) -> Option<LayoutFragmentId> {
    layout_node(node_id, ctx, space, arena).map(|fragment| arena.add_fragment(fragment))
}

fn layout_flow_children(
    node_id: DomNodeId,
    ctx: &mut LayoutContext,
    child_space: &ConstraintSpace,
    arena: &mut FragmentArena,
) -> (Option<LayoutFragmentId>, f32, f32) {
    let mut current_y_cursor = 0.0;
    let mut max_line_width: f32 = 0.0;
    let mut child_fragments: Vec<LayoutFragment> = Vec::new();

    let mut inline_run: Vec<DomNodeId> = Vec::new();
    let mut current_dom_child = ctx.dom.get_first_child(node_id);

    while let Some(dom_child) = current_dom_child {
        let child_style = compute_style(dom_child, ctx.dom, ctx.stylesheet);

        if child_style.display.is_none() {
            current_dom_child = ctx.dom.get_next_sibling(dom_child);
            continue;
        }

        let is_inline = ctx.dom.get_node(dom_child).unwrap().is_text()
            || child_style.display.outside() == Some(DisplayOutside::Inline);

        if is_inline {
            inline_run.push(dom_child);
        } else {
            if !inline_run.is_empty() {
                let run_width = flush_inline_run(
                    &mut inline_run,
                    ctx,
                    child_space.available_size.width,
                    &mut current_y_cursor,
                    arena,
                    &mut child_fragments,
                );
                max_line_width = max_line_width.max(run_width);
            }

            if let Some(block_fragment) = layout_node(dom_child, ctx, *child_space, arena) {
                let height = block_fragment.get_height();
                let width = block_fragment.get_width();
                let new_origin = block_fragment.get_origin().with_y(current_y_cursor);

                child_fragments.push(block_fragment.with_origin(new_origin));

                current_y_cursor += height;
                max_line_width = max_line_width.max(width);
            }
        }

        current_dom_child = ctx.dom.get_next_sibling(dom_child);
    }

    if !inline_run.is_empty() {
        let run_width = flush_inline_run(
            &mut inline_run,
            ctx,
            child_space.available_size.width,
            &mut current_y_cursor,
            arena,
            &mut child_fragments,
        );

        max_line_width = max_line_width.max(run_width);
    }

    let mut first_child_fragment_id: Option<LayoutFragmentId> = None;
    for fragment in child_fragments.into_iter().rev() {
        let id = arena.add_fragment(fragment.with_next_sibling(first_child_fragment_id));
        first_child_fragment_id = Some(id);
    }

    (first_child_fragment_id, current_y_cursor, max_line_width)
}

fn prepare_inline_items(
    items: Vec<InlineItem>,
    max_width: f32,
    ctx: &mut LayoutContext,
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
            prepared_items.push(PreparedInlineItem {
                width: fragment.get_width(),
                fit_width: fragment.get_width(),
                height: fragment.get_height(),
                kind: PreparedInlineItemKind::AtomicInline { fragment },
            });
        }
    }

    prepared_items
}

fn build_line_fragments(
    lines: Vec<Line>,
    current_y_cursor: &mut f32,
    ctx: &mut LayoutContext,
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
                    let text_style = compute_style(dom_node, ctx.dom, ctx.stylesheet);
                    let fragment = LayoutFragment::new(
                        Some(dom_node),
                        text_rect,
                        text_style,
                        FragmentKind::Text(text.clone()),
                    )
                    .with_next_sibling(next_child_id);

                    next_child_id = Some(arena.add_fragment(fragment));
                }
                PreparedInlineItemKind::AtomicInline { fragment } => {
                    let new_origin = fragment.get_origin().with_x(x);
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
            PhysicalOffset::new(0.0, *current_y_cursor),
            PhysicalSize::new(line.width, line.height),
        );
        let line_style = ComputedStyle::new(Display::BLOCK, Some(line.width), Some(line.height));
        let line_fragment = LayoutFragment::new(None, line_rect, line_style, FragmentKind::Line)
            .with_first_child(next_child_id);

        child_fragments.push(line_fragment);

        *current_y_cursor += line.height;
    }

    run_max_width
}

fn flush_inline_run(
    inline_run: &mut Vec<DomNodeId>,
    ctx: &mut LayoutContext,
    max_width: f32,
    current_y_cursor: &mut f32,
    arena: &mut FragmentArena,
    child_fragments: &mut Vec<LayoutFragment>,
) -> f32 {
    let items = collect_inline_items(inline_run, ctx);
    let prepared_items = prepare_inline_items(items, max_width, ctx, arena);

    let min_line_height = ctx.text_measurer.line_height();
    let lines = break_lines(prepared_items, max_width, min_line_height);

    let run_max_width = build_line_fragments(lines, current_y_cursor, ctx, arena, child_fragments);
    inline_run.clear();

    run_max_width
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::text::FixedWidth;
    use blink_css::parse_css_into_stylesheet;
    use blink_dom::get_tag_index;

    #[test]
    fn inline_element_without_explicit_width_shrink_wraps_children() {
        let mut dom = Dom::new();

        let parent = dom.add_element(get_tag_index("div"), None);
        let child = dom.add_element(get_tag_index("span"), None);

        dom.set_parent(child, parent).expect("Failed to set parent");

        dom.set_first_child(parent, Some(child))
            .expect("Failed to set first child");

        let mut stylesheet = CSSStyleSheet::new();
        parse_css_into_stylesheet(
            r#"
                div { display: inline-block; }
                span { display: inline-block; width: 40px; height: 10px; }
            "#,
            &mut stylesheet,
        );

        let space = ConstraintSpace {
            available_size: PhysicalSize::new(800.0, 600.0),
        };

        let mut ctx = LayoutContext {
            dom: &dom,
            stylesheet: &stylesheet,
            text_measurer: Box::new(FixedWidth {}),
        };

        let arena = build_layout_tree(parent, &mut ctx, space);

        let parent_fragment = arena
            .get_fragment_by_dom_node(parent)
            .expect("Expected parent fragment");

        assert_eq!(
            parent_fragment.get_width(),
            40.0,
            "Inline element should shrink-wrap its child's width"
        );
    }

    #[test]
    fn test_line_wraps() {
        let mut dom = Dom::new();

        let parent = dom.add_element(get_tag_index("div"), None);

        let child1 = dom.add_element(get_tag_index("span"), None);
        let child2 = dom.add_element(get_tag_index("span"), None);
        let child3 = dom.add_element(get_tag_index("span"), None);

        dom.set_parent(child1, parent)
            .expect("Failed to set parent");
        dom.set_parent(child2, parent)
            .expect("Failed to set parent");
        dom.set_parent(child3, parent)
            .expect("Failed to set parent");

        dom.set_first_child(parent, Some(child1))
            .expect("Failed to set first child");
        dom.set_next_sibling(child1, Some(child2))
            .expect("Failed to set next sibling");
        dom.set_next_sibling(child2, Some(child3))
            .expect("Failed to set next sibling");

        let mut stylesheet = CSSStyleSheet::new();
        parse_css_into_stylesheet(
            r#"
                div { display: block; width: 100px; }
                span { display: inline-block; width: 40px; height: 20px; }
            "#,
            &mut stylesheet,
        );

        let space = ConstraintSpace {
            available_size: PhysicalSize::new(800.0, 600.0),
        };

        let mut ctx = LayoutContext {
            dom: &dom,
            stylesheet: &stylesheet,
            text_measurer: Box::new(FixedWidth {}),
        };

        let arena = build_layout_tree(parent, &mut ctx, space);

        let child1_fragment = arena
            .get_fragment_by_dom_node(child1)
            .expect("Expected child1 fragment");

        assert_eq!(
            *child1_fragment.get_origin(),
            (0.0, 0.0),
            "Inline element should shrink-wrap its child's width"
        );

        let child2_fragment = arena
            .get_fragment_by_dom_node(child2)
            .expect("Expected child2 fragment");

        assert_eq!(
            *child2_fragment.get_origin(),
            (40.0, 0.0),
            "Inline element should shrink-wrap its child's width"
        );

        let child3_fragment = arena
            .get_fragment_by_dom_node(child3)
            .expect("Expected child3 fragment");

        assert_eq!(
            *child3_fragment.get_origin(),
            (0.0, 0.0),
            "Inline element should shrink-wrap its child's width"
        );

        let parent_fragment = arena
            .get_fragment_by_dom_node(parent)
            .expect("Expected parent fragment");

        assert_eq!(
            parent_fragment.get_height(),
            40.0,
            "Inline element should shrink-wrap its child's width"
        );
    }
}
