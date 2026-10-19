use std::num::NonZeroU32;
use blink_css::CSSStyleSheet;
use blink_dom::Dom;
use crate::fragment::{FragmentArena, LayoutFragment};
use crate::geom::{PhysicalOffset, PhysicalRect, PhysicalSize};
use crate::space::ConstraintSpace;
use crate::style::{compute_style, ComputedStyle, Display};

pub fn build_layout_tree(root_id: NonZeroU32, dom: &Dom, stylesheet: &CSSStyleSheet, initial_space: ConstraintSpace) -> FragmentArena {
    let mut arena = FragmentArena::new();
    walk_node(root_id, dom, stylesheet, initial_space, &mut arena);
    arena
}

fn walk_node(node_id: NonZeroU32, dom: &Dom, stylesheet: &CSSStyleSheet, space: ConstraintSpace, arena: &mut FragmentArena) -> Option<NonZeroU32> {
    let computed_style = compute_style(node_id, dom, stylesheet);

    if matches!(computed_style.display, Display::None) {
        return None;
    }

    let node_width = computed_style.width;
    let node_height = computed_style.height;
    let node_size = PhysicalSize::new(
        node_width.unwrap_or(space.available_size.width),
        node_height.unwrap_or_default(),
    );
    let bounding_rect = PhysicalRect::new(PhysicalOffset::default(), node_size);
    let fragment = LayoutFragment::new(Some(node_id), bounding_rect, computed_style);

    let fragment_id = arena.add_fragment(fragment);

    let mut previous_child_fragment_id: Option<NonZeroU32> = None;
    let mut first_child_fragment_id = None;

    let mut current_dom_child = dom.get_first_child(node_id);
    let mut child_x_cursor = 0.0;
    let mut child_y_cursor = 0.0;
    while let Some(dom_child) = current_dom_child {
        if let Some(fragment_child_id) = walk_node(dom_child, dom, stylesheet, space, arena) {
            let child_id = (fragment_child_id.get() - 1) as usize;

            let child_width = arena.fragments[child_id].rect.size.width;
            let child_height = arena.fragments[child_id].rect.size.height;

            match arena.fragments[child_id].style.display {
                Display::Block => {
                    arena.fragments[child_id].rect.origin.y = child_y_cursor;
                    child_x_cursor = 0.0;
                    child_y_cursor += child_height;
                },
                Display::Inline => {
                    arena.fragments[child_id].rect.origin.x = child_x_cursor;
                    arena.fragments[child_id].rect.origin.y = child_y_cursor;
                    child_x_cursor += child_width;
                },
                _ => {},
            }

            if first_child_fragment_id.is_none() {
                first_child_fragment_id = Some(fragment_child_id);
            }

            if let Some(previous_id) = previous_child_fragment_id {
                let previous_idx = (previous_id.get() - 1) as usize;
                arena.fragments[previous_idx].set_next_sibling(fragment_child_id);
            }

            previous_child_fragment_id = Some(fragment_child_id);
        }

        current_dom_child = dom.get_next_sibling(dom_child);
    }

    let fragment_idx = (fragment_id.get() - 1) as usize;

    if node_height.is_none() {
        arena.fragments[fragment_idx].rect.size.height = child_y_cursor;
    }

    if let Some(first_id) = first_child_fragment_id {
        arena.fragments[fragment_idx].set_first_child(first_id);
    }

    Some(fragment_id)
}
