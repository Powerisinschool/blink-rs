use crate::fragment::{FragmentArena, FragmentKind, LayoutFragment, LayoutFragmentId};
use crate::geom::{PhysicalOffset, PhysicalRect, PhysicalSize};
use crate::space::ConstraintSpace;
use crate::style::{DisplayOutside, compute_style};
use blink_css::CSSStyleSheet;
use blink_dom::{Dom, DomNodeId};
use crate::text::TextMeasurer;

pub struct LayoutContext<'a> {
    pub dom: &'a Dom,
    pub stylesheet: &'a CSSStyleSheet,
    pub text_measurer: &'a Box<dyn TextMeasurer>,
}

pub fn build_layout_tree(
    root_id: DomNodeId,
    ctx: &LayoutContext,
    initial_space: ConstraintSpace,
) -> FragmentArena {
    let mut arena = FragmentArena::new();

    if let Some(root_fragment_id) = walk_node(root_id, ctx, initial_space, &mut arena) {
        let root_idx = (root_fragment_id.get() - 1) as usize;
        // Transform the relative coordinates into absolute screen coordinates for the renderer
        make_absolute(&mut arena, root_idx, 0.0, 0.0);
    }

    arena
}

fn make_absolute(arena: &mut FragmentArena, node_idx: usize, parent_x: f32, parent_y: f32) {
    let abs_x = parent_x + arena.fragments[node_idx].rect.origin.x;
    let abs_y = parent_y + arena.fragments[node_idx].rect.origin.y;

    arena.fragments[node_idx].rect.origin.x = abs_x;
    arena.fragments[node_idx].rect.origin.y = abs_y;

    let mut current_child = arena.fragments[node_idx].first_child;
    while let Some(child_id) = current_child {
        let child_idx = (child_id.get() - 1) as usize;
        make_absolute(arena, child_idx, abs_x, abs_y);
        current_child = arena.fragments[child_idx].next_sibling;
    }
}

fn walk_node(
    node_id: DomNodeId,
    ctx: &LayoutContext,
    space: ConstraintSpace,
    arena: &mut FragmentArena,
) -> Option<LayoutFragmentId> {
    if ctx.dom.get_node(node_id).is_some_and(|node| node.is_text()) {
        return None;
    }

    let computed_style = compute_style(node_id, &ctx.dom, &ctx.stylesheet);

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
    let fragment = LayoutFragment::new(Some(node_id), bounding_rect, computed_style, FragmentKind::Box);

    let fragment_id = arena.add_fragment(fragment);

    let mut previous_child_fragment_id: Option<LayoutFragmentId> = None;
    let mut first_child_fragment_id: Option<LayoutFragmentId> = None;

    let mut current_dom_child = ctx.dom.get_first_child(node_id);
    let mut child_x_cursor = 0.0;
    let mut child_y_cursor = 0.0;
    let mut line_height = 0.0;
    let mut max_line_width: f32 = 0.0;

    let available_width = node_width.unwrap_or(space.available_size.width);

    let child_space = ConstraintSpace {
        available_size: PhysicalSize::new(available_width, space.available_size.height),
    };
    while let Some(dom_child) = current_dom_child {
        if let Some(fragment_child_id) = walk_node(dom_child, ctx, child_space, arena) {
            let child_id = fragment_child_id.get_index();

            let child_width = arena.fragments[child_id].rect.size.width;
            let child_height = arena.fragments[child_id].rect.size.height;

            let child_fragment = &mut arena.fragments[child_id];

            match child_fragment.style.display.outside() {
                Some(DisplayOutside::Block) => {
                    if line_height > 0.0 {
                        child_y_cursor += line_height;
                        line_height = 0.0;
                    }
                    child_fragment.rect.origin.y = child_y_cursor;
                    child_x_cursor = 0.0;
                    child_y_cursor += child_height;
                    max_line_width = max_line_width.max(child_width);
                }
                Some(DisplayOutside::Inline) => {
                    if child_x_cursor > 0.0
                        && child_x_cursor + child_width > child_space.available_size.width
                    {
                        child_y_cursor += line_height;
                        line_height = 0.0;
                        child_x_cursor = 0.0;
                    }
                    child_fragment.rect.origin.x = child_x_cursor;
                    child_fragment.rect.origin.y = child_y_cursor;
                    child_x_cursor += child_width;
                    max_line_width = max_line_width.max(child_x_cursor);
                    line_height = line_height.max(child_height);
                }
                _ => {}
            }

            if first_child_fragment_id.is_none() {
                first_child_fragment_id = Some(fragment_child_id);
            }

            if let Some(previous_id) = previous_child_fragment_id {
                let previous_idx = previous_id.get_index();
                arena.fragments[previous_idx].set_next_sibling(fragment_child_id);
            }

            previous_child_fragment_id = Some(fragment_child_id);
        }

        current_dom_child = ctx.dom.get_next_sibling(dom_child);
    }

    if line_height > 0.0 {
        child_y_cursor += line_height;
    }

    let fragment_idx = fragment_id.get_index();

    if node_width.is_none() && is_inline_level {
        arena.fragments[fragment_idx].rect.size.width = max_line_width;
    }

    if node_height.is_none() {
        arena.fragments[fragment_idx].rect.size.height = child_y_cursor;
    }

    if let Some(first_id) = first_child_fragment_id {
        arena.fragments[fragment_idx].set_first_child(first_id);
    }

    Some(fragment_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use blink_css::parse_css_into_stylesheet;
    use blink_dom::get_tag_index;
    use crate::text::FixedWidth;

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
                div { display: inline; }
                span { width: 40px; height: 10px; }
            "#,
            &mut stylesheet,
        );

        let space = ConstraintSpace {
            available_size: PhysicalSize::new(800.0, 600.0),
        };

        let ctx = LayoutContext {
            dom: &dom,
            stylesheet: &stylesheet,
            text_measurer: &(Box::new(FixedWidth {}) as Box<dyn TextMeasurer>),
        };

        let arena = build_layout_tree(parent, &ctx, space);

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
                div { display: inline; width: 100px; }
                span { width: 40px; height: 10px; }
            "#,
            &mut stylesheet,
        );

        let space = ConstraintSpace {
            available_size: PhysicalSize::new(800.0, 600.0),
        };

        let ctx = LayoutContext {
            dom: &dom,
            stylesheet: &stylesheet,
            text_measurer: &(Box::new(FixedWidth {}) as Box<dyn TextMeasurer>),
        };

        let arena = build_layout_tree(parent, &ctx, space);

        let child1_fragment = arena
            .fragments
            .iter()
            .find(|fragment| fragment.dom_node == Some(child1))
            .expect("Expected child1 fragment");

        assert_eq!(
            child1_fragment.rect.origin.x, 0.0,
            "Inline element should shrink-wrap its child's width"
        );
        assert_eq!(
            child1_fragment.rect.origin.y, 0.0,
            "Inline element should shrink-wrap its child's width"
        );

        let child2_fragment = arena
            .fragments
            .iter()
            .find(|fragment| fragment.dom_node == Some(child2))
            .expect("Expected child2 fragment");

        assert_eq!(
            child2_fragment.rect.origin.x, 40.0,
            "Inline element should shrink-wrap its child's width"
        );
        assert_eq!(
            child2_fragment.rect.origin.y, 0.0,
            "Inline element should shrink-wrap its child's width"
        );

        let child3_fragment = arena
            .fragments
            .iter()
            .find(|fragment| fragment.dom_node == Some(child3))
            .expect("Expected child3 fragment");

        assert_eq!(
            child3_fragment.rect.origin.x, 0.0,
            "Inline element should shrink-wrap its child's width"
        );
        assert_eq!(
            child3_fragment.rect.origin.y, 10.0,
            "Inline element should shrink-wrap its child's width"
        );

        let parent_fragment = arena
            .fragments
            .iter()
            .find(|fragment| fragment.dom_node == Some(parent))
            .expect("Expected parent fragment");

        assert_eq!(
            parent_fragment.rect.size.height, 20.0,
            "Inline element should shrink-wrap its child's width"
        );
    }
}
