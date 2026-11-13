use crate::fragment::{FragmentArena, LayoutFragment, LayoutFragmentId};
use crate::geom::PhysicalOffset;

pub trait Renderer {
    type Context;

    fn begin_fragment(
        &mut self,
        fragment: &LayoutFragment,
        abs_origin: PhysicalOffset,
        ctx: &Self::Context,
    ) -> Self::Context;

    fn paint_fragment(
        &mut self,
        fragment: &LayoutFragment,
        abs_origin: PhysicalOffset,
        ctx: &Self::Context,
    );

    fn end_fragment(
        &mut self,
        fragment: &LayoutFragment,
        abs_origin: PhysicalOffset,
        ctx: &Self::Context,
    );
}

pub struct FragmentWalker;

impl FragmentWalker {
    pub fn walk<R: Renderer>(
        arena: &FragmentArena,
        root_id: LayoutFragmentId,
        renderer: &mut R,
        initial_ctx: &R::Context,
        parent_offset: PhysicalOffset,
    ) {
        let mut current_id_opt = Some(root_id);

        while let Some(current_id) = current_id_opt {
            let Some(fragment) = arena.get_fragment(current_id) else {
                break;
            };

            let abs_origin = parent_offset + *fragment.get_origin();
            let child_ctx = renderer.begin_fragment(fragment, abs_origin, initial_ctx);

            renderer.paint_fragment(fragment, abs_origin, &child_ctx);

            if let Some(child_id) = fragment.get_first_child() {
                Self::walk(arena, child_id, renderer, &child_ctx, abs_origin);
            }

            renderer.end_fragment(fragment, abs_origin, &child_ctx);

            current_id_opt = fragment.get_next_sibling();
        }
    }
}
