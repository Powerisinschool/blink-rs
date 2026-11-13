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

    walk_node(root_id, ctx, initial_space, &mut arena);

    arena
}

pub(crate) fn walk_node(
    node_id: DomNodeId,
    ctx: &mut LayoutContext,
    space: ConstraintSpace,
    arena: &mut FragmentArena,
) -> Option<LayoutFragmentId> {
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

    let node_size = PhysicalSize::new(initial_width, node_height.unwrap_or_default());

    let bounding_rect = PhysicalRect::new(PhysicalOffset::default(), node_size);
    let fragment = LayoutFragment::new(
        Some(node_id),
        bounding_rect,
        computed_style,
        FragmentKind::Box,
    );

    let fragment_id = arena.add_fragment(fragment);

    let available_width = node_width.unwrap_or(space.available_size.width);
    let child_space = ConstraintSpace {
        available_size: PhysicalSize::new(available_width, space.available_size.height),
    };

    let (first_child_id, total_child_height, max_line_width) =
        layout_flow_children(node_id, ctx, &child_space, arena);

    let fragment_idx = fragment_id.get_index();

    if node_width.is_none() && is_inline_level {
        arena.fragments[fragment_idx].rect.size.width = max_line_width;
    }

    if node_height.is_none() {
        arena.fragments[fragment_idx].rect.size.height = total_child_height;
    }

    if let Some(first_id) = first_child_id {
        arena.fragments[fragment_idx].set_first_child(first_id);
    }

    Some(fragment_id)
}

fn layout_flow_children(
    node_id: DomNodeId,
    ctx: &mut LayoutContext,
    child_space: &ConstraintSpace,
    arena: &mut FragmentArena,
) -> (Option<LayoutFragmentId>, f32, f32) {
    let mut current_y_cursor = 0.0;
    let mut max_line_width: f32 = 0.0;
    let mut first_child_fragment_id: Option<LayoutFragmentId> = None;
    let mut previous_child_fragment_id: Option<LayoutFragmentId> = None;

    let mut inline_run: Vec<DomNodeId> = Vec::new();
    let mut current_dom_child = ctx.dom.get_first_child(node_id);

    while let Some(dom_child) = current_dom_child {
        let child_style = compute_style(dom_child, ctx.dom, ctx.stylesheet);

        if child_style.display.is_none() {
            current_dom_child = ctx.dom.get_next_sibling(dom_child);
            continue;
        }

        let is_inline = if ctx.dom.get_node(dom_child).unwrap().is_text() {
            true
        } else {
            let child_style = compute_style(dom_child, ctx.dom, ctx.stylesheet);
            child_style.display.outside() == Some(DisplayOutside::Inline)
        };

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
                    &mut first_child_fragment_id,
                    &mut previous_child_fragment_id,
                );
                max_line_width = max_line_width.max(run_width);
            }

            if let Some(block_fragment_id) = walk_node(dom_child, ctx, *child_space, arena) {
                let block_idx = block_fragment_id.get_index();

                arena.fragments[block_idx].rect.origin.y = current_y_cursor;

                if first_child_fragment_id.is_none() {
                    first_child_fragment_id = Some(block_fragment_id);
                }
                if let Some(prev) = previous_child_fragment_id {
                    arena.fragments[prev.get_index()].set_next_sibling(block_fragment_id);
                }
                previous_child_fragment_id = Some(block_fragment_id);

                current_y_cursor += arena.fragments[block_idx].rect.size.height;
                max_line_width = max_line_width.max(arena.fragments[block_idx].rect.size.width);
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
            &mut first_child_fragment_id,
            &mut previous_child_fragment_id,
        );

        max_line_width = max_line_width.max(run_width);
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
            // let mut current_start_index = 0;
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
                                    // range: current_start_index..i,
                                },
                                width: fit_width + space_width,
                                fit_width,
                                height: ctx.text_measurer.line_height(),
                            });
                            // current_start_index = i + 1;
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
                        // range: current_start_index..text.len(),
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
            && let Some(layout) = layout_atomic_inline(item.dom_node, max_width, ctx, arena)
        {
            prepared_items.push(PreparedInlineItem {
                kind: PreparedInlineItemKind::AtomicInline {
                    fragment_id: layout.fragment_id,
                },
                width: layout.width,
                fit_width: layout.width,
                height: layout.height,
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
    parent_first_child: &mut Option<LayoutFragmentId>,
    parent_previous_child: &mut Option<LayoutFragmentId>,
) -> f32 {
    // eprintln!("build_line_fragments received {} lines", lines.len());
    let mut run_max_width: f32 = 0.0;

    for line in lines {
        run_max_width = run_max_width.max(line.width);

        let line_rect = PhysicalRect::new(
            PhysicalOffset::new(0.0, *current_y_cursor),
            PhysicalSize::new(line.width, line.height),
        );
        let line_style = ComputedStyle::new(Display::BLOCK, Some(line.width), Some(line.height));
        let line_fragment = LayoutFragment::new(None, line_rect, line_style, FragmentKind::Line);
        let line_fragment_id = arena.add_fragment(line_fragment);

        if parent_first_child.is_none() {
            *parent_first_child = Some(line_fragment_id);
        }
        if let Some(prev) = *parent_previous_child {
            arena.fragments[prev.get_index()].set_next_sibling(line_fragment_id);
        }
        *parent_previous_child = Some(line_fragment_id);

        let mut line_x = 0.0;
        let mut line_first_child: Option<LayoutFragmentId> = None;
        let mut line_prev_child: Option<LayoutFragmentId> = None;

        for item in line.items {
            let child_fragment_id_opt = if let Some((dom_node, text)) = item.get_text_segment() {
                let text_rect = PhysicalRect::new(
                    PhysicalOffset::new(line_x, 0.0),
                    PhysicalSize::new(item.fit_width, item.height),
                );
                let text_style = compute_style(*dom_node, ctx.dom, ctx.stylesheet);
                let fragment_id = LayoutFragment::new(
                    Some(*dom_node),
                    text_rect,
                    text_style,
                    FragmentKind::Text(text.clone()),
                );
                Some(arena.add_fragment(fragment_id))
            } else if let Some(fragment_id) = item.get_atomic_inline_fragment_id() {
                Some(*fragment_id)
            } else if item.is_forced_break() {
                None
            } else {
                unreachable!("Unexpected inline item kind when building line fragments");
            };

            if let Some(child_fragment_id) = child_fragment_id_opt {
                arena.fragments[child_fragment_id.get_index()].rect.origin.x = line_x;

                if line_first_child.is_none() {
                    line_first_child = Some(child_fragment_id);
                }

                if let Some(prev) = line_prev_child {
                    arena.fragments[prev.get_index()].set_next_sibling(child_fragment_id);
                }
                line_prev_child = Some(child_fragment_id);
            }
            line_x += item.width;
        }

        if let Some(first) = line_first_child {
            arena.fragments[line_fragment_id.get_index()].set_first_child(first);
        }
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
    parent_first_child: &mut Option<LayoutFragmentId>,
    parent_previous_child: &mut Option<LayoutFragmentId>,
) -> f32 {
    let items = collect_inline_items(inline_run, ctx);
    let prepared_items = prepare_inline_items(items, max_width, ctx, arena);
    // eprintln!("Prepared items: {}", prepared_items.len());

    // for (i, item) in prepared_items.iter().enumerate() {
    //     eprintln!(
    //         "  [{i}] text={}, atomic={}, forced={}, width={}, fit_width={}, height={}",
    //         item.is_text_segment(),
    //         item.is_atomic_inline(),
    //         item.is_forced_break(),
    //         item.width,
    //         item.fit_width,
    //         item.height,
    //     );
    // }

    let min_line_height = ctx.text_measurer.line_height();
    let lines = break_lines(prepared_items, max_width, min_line_height);

    // eprintln!("Produced lines: {}", lines.len());

    // for (i, line) in lines.iter().enumerate() {
    //     eprintln!(
    //         "  Line {i}: width={}, height={}, items={}",
    //         line.width,
    //         line.height,
    //         line.items.len(),
    //     );
    // }

    let run_max_width = build_line_fragments(
        lines,
        current_y_cursor,
        ctx,
        arena,
        parent_first_child,
        parent_previous_child,
    );
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
            .fragments
            .iter()
            .find(|fragment| fragment.dom_node == Some(parent))
            .expect("Expected parent fragment");

        assert_eq!(
            parent_fragment.rect.size.width, 40.0,
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
            .fragments
            .iter()
            .find(|fragment| fragment.dom_node == Some(child1))
            .expect("Expected child1 fragment");

        assert_eq!(
            child1_fragment.rect.origin,
            (0.0, 0.0),
            "Inline element should shrink-wrap its child's width"
        );

        let child2_fragment = arena
            .fragments
            .iter()
            .find(|fragment| fragment.dom_node == Some(child2))
            .expect("Expected child2 fragment");

        assert_eq!(
            child2_fragment.rect.origin,
            (40.0, 0.0),
            "Inline element should shrink-wrap its child's width"
        );

        let child3_fragment = arena
            .fragments
            .iter()
            .find(|fragment| fragment.dom_node == Some(child3))
            .expect("Expected child3 fragment");

        assert_eq!(
            child3_fragment.rect.origin,
            (0.0, 0.0),
            "Inline element should shrink-wrap its child's width"
        );

        let parent_fragment = arena
            .fragments
            .iter()
            .find(|fragment| fragment.dom_node == Some(parent))
            .expect("Expected parent fragment");

        assert_eq!(
            parent_fragment.rect.size.height, 40.0,
            "Inline element should shrink-wrap its child's width"
        );
    }
}
