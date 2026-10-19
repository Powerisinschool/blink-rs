use std::num::NonZeroU32;
use blink_css::CSSStyleSheet;
use blink_dom::Dom;
use crate::fragment::{FragmentArena, LayoutFragment};
use crate::geom::PhysicalRect;
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

    let bounding_rect = PhysicalRect::default();
    let fragment = LayoutFragment::new(Some(node_id), bounding_rect, computed_style);

    let fragment_id = arena.add_fragment(fragment);

    let mut previous_child_fragment_id: Option<NonZeroU32> = None;
    let mut first_child_fragment_id = None;

    let mut current_dom_child = dom.get_first_child(node_id);
    while let Some(dom_child) = current_dom_child {
        if let Some(fragment_child_id) = walk_node(dom_child, dom, stylesheet, space, arena) {
            if first_child_fragment_id.is_none() {
                first_child_fragment_id = Some(fragment_child_id);
            }

            if let Some(previous_id) = previous_child_fragment_id {
                let previous_idx = (previous_id.get() - 1) as usize;
                arena.fragments[previous_idx].set_first_child(fragment_child_id);
            }

            previous_child_fragment_id = Some(fragment_child_id);
        }

        current_dom_child = dom.get_next_sibling(dom_child);
    }

    if let Some(first_id) = first_child_fragment_id {
        let fragment_idx = (fragment_id.get() - 1) as usize;
        arena.fragments[fragment_idx].set_first_child(first_id);
    }

    Some(fragment_id)
}
