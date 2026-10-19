use crate::fragment::{FragmentArena, LayoutFragment, LayoutFragmentId};
use crate::geom::{PhysicalOffset, PhysicalRect, PhysicalSize};
use crate::space::ConstraintSpace;
use crate::style::{Display, compute_style};
use blink_css::CSSStyleSheet;
use blink_dom::{Dom, DomNodeId, NodeKind};

pub fn build_layout_tree(root_id: DomNodeId, dom: &Dom, stylesheet: &CSSStyleSheet, initial_space: ConstraintSpace) -> FragmentArena {
    let mut arena = FragmentArena::new();

    if let Some(root_fragment_id) = walk_node(root_id, dom, stylesheet, initial_space, &mut arena) {
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

fn walk_node(node_id: DomNodeId, dom: &Dom, stylesheet: &CSSStyleSheet, space: ConstraintSpace, arena: &mut FragmentArena) -> Option<LayoutFragmentId> {
    if matches!(dom.get_node(node_id).map(|n| &n.kind), Some(NodeKind::Text {})) {
        return None;
    }

    let computed_style = compute_style(node_id, dom, stylesheet);

    if matches!(computed_style.display, Display::None) {
        return None;
    }

    let node_width = computed_style.width;
    let node_height = computed_style.height;

    let initial_width = node_width.unwrap_or_else(|| match computed_style.display {
        Display::Inline => 0.0,
        _ => space.available_size.width,
    });

    let node_size = PhysicalSize::new(
        initial_width,
        node_height.unwrap_or_default(),
    );

    let bounding_rect = PhysicalRect::new(PhysicalOffset::default(), node_size);
    let fragment = LayoutFragment::new(Some(node_id), bounding_rect, computed_style);

    let fragment_id = arena.add_fragment(fragment);

    let mut previous_child_fragment_id: Option<LayoutFragmentId> = None;
    let mut first_child_fragment_id: Option<LayoutFragmentId> = None;

    let mut current_dom_child = dom.get_first_child(node_id);
    let mut child_x_cursor = 0.0;
    let mut child_y_cursor = 0.0;
    let mut line_height = 0.0;

    let child_space = ConstraintSpace {
        available_size: PhysicalSize::new(initial_width, space.available_size.height),
    };
    while let Some(dom_child) = current_dom_child {
        if let Some(fragment_child_id) = walk_node(dom_child, dom, stylesheet, child_space, arena) {
            let child_id = (fragment_child_id.get() - 1) as usize;

            let child_width = arena.fragments[child_id].rect.size.width;
            let child_height = arena.fragments[child_id].rect.size.height;

            match arena.fragments[child_id].style.display {
                Display::Block => {
                    if line_height > 0.0 {
                        child_y_cursor += line_height;
                        line_height = 0.0;
                    }
                    arena.fragments[child_id].rect.origin.y = child_y_cursor;
                    child_x_cursor = 0.0;
                    child_y_cursor += child_height;
                },
                Display::Inline => {
                    arena.fragments[child_id].rect.origin.x = child_x_cursor;
                    arena.fragments[child_id].rect.origin.y = child_y_cursor;
                    child_x_cursor += child_width;
                    if child_height > line_height {
                        line_height = child_height;
                    }
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

    if line_height > 0.0 {
        child_y_cursor += line_height;
    }

    let fragment_idx = (fragment_id.get() - 1) as usize;

    if node_width.is_none() && matches!(arena.fragments[fragment_idx].style.display, Display::Inline) {
        arena.fragments[fragment_idx].rect.size.width = child_x_cursor;
    }

    if node_height.is_none() {
        arena.fragments[fragment_idx].rect.size.height = child_y_cursor;
    }

    if let Some(first_id) = first_child_fragment_id {
        arena.fragments[fragment_idx].set_first_child(first_id);
    }

    Some(fragment_id)
}