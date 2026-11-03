use crate::engine::{LayoutContext, walk_node};
use crate::fragment::{FragmentArena, LayoutFragmentId};
use crate::geom::PhysicalSize;
use crate::space::ConstraintSpace;
use blink_dom::DomNodeId;

pub struct LayoutResult {
    pub fragment_id: LayoutFragmentId,
    pub width: f32,
    pub height: f32,
}

impl LayoutResult {
    pub fn new(fragment_id: LayoutFragmentId, width: f32, height: f32) -> Self {
        Self {
            fragment_id,
            width,
            height,
        }
    }
}

pub fn layout_atomic_inline(
    node_id: DomNodeId,
    available_width: f32,
    ctx: &mut LayoutContext,
    arena: &mut FragmentArena,
) -> Option<LayoutResult> {
    let child_space = ConstraintSpace {
        available_size: PhysicalSize::new(available_width, f32::MAX),
    };

    if let Some(fragment_id) = walk_node(node_id, ctx, child_space, arena)
        && let Some(fragment_rect) = arena.get_fragment_rect(fragment_id)
    {
        Some(LayoutResult::new(
            fragment_id,
            fragment_rect.size.width,
            fragment_rect.size.height,
        ))
    } else {
        None
    }
}
