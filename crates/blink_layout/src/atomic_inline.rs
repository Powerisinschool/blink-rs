use crate::engine::{LayoutContext, layout_node};
use crate::fragment::{FragmentArena, LayoutFragment};
use crate::geom::PhysicalSize;
use crate::space::ConstraintSpace;
use blink_dom::DomNodeId;

// pub struct LayoutResult {
//     pub fragment: LayoutFragment,
//     pub width: f32,
//     pub height: f32,
// }

// impl LayoutResult {
//     pub fn new(fragment_id: LayoutFragmentId, width: f32, height: f32) -> Self {
//         Self {
//             fragment_id,
//             width,
//             height,
//         }
//     }
// }

pub fn layout_atomic_inline(
    node_id: DomNodeId,
    available_width: f32,
    ctx: &mut LayoutContext,
    arena: &mut FragmentArena,
) -> Option<LayoutFragment> {
    let child_space = ConstraintSpace {
        available_size: PhysicalSize::new(available_width, f32::MAX),
    };

    layout_node(node_id, ctx, child_space, arena)
}
