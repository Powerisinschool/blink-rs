use super::super::engine::LayoutContext;
use super::super::fragment::{FragmentArena, LayoutFragment};
use super::super::space::ConstraintSpace;
use super::super::text::TextMeasurer;
use blink_dom::DomNodeId;

pub fn layout_atomic_inline<M: TextMeasurer + ?Sized>(
    node_id: DomNodeId,
    available_width: f32,
    ctx: &mut LayoutContext<M>,
    arena: &mut FragmentArena,
) -> Option<LayoutFragment> {
    let child_space = ConstraintSpace::new(available_width, f32::MAX);

    ctx.layout_node(node_id, child_space, arena)
}
