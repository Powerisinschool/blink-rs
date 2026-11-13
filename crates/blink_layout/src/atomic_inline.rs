use crate::engine::{LayoutContext, layout_node};
use crate::fragment::{FragmentArena, LayoutFragment};
use crate::space::ConstraintSpace;
use blink_dom::DomNodeId;

pub fn layout_atomic_inline(
    node_id: DomNodeId,
    available_width: f32,
    ctx: &mut LayoutContext,
    arena: &mut FragmentArena,
) -> Option<LayoutFragment> {
    let child_space = ConstraintSpace::new(available_width, f32::MAX);

    layout_node(node_id, ctx, child_space, arena)
}
