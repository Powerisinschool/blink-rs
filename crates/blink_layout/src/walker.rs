use crate::fragment::{FragmentArena, LayoutFragment, LayoutFragmentId};

pub trait Renderer {
    type Context;

    fn begin_fragment(&mut self, fragment: &LayoutFragment, ctx: &Self::Context) -> Self::Context;

    fn paint_fragment(&mut self, fragment: &LayoutFragment, ctx: &Self::Context);

    fn end_fragment(&mut self, fragment: &LayoutFragment, ctx: &Self::Context);
}

pub struct FragmentWalker;

impl FragmentWalker {
    pub fn walk<R: Renderer>(
        arena: &FragmentArena,
        root_id: LayoutFragmentId,
        renderer: &mut R,
        initial_ctx: &R::Context,
    ) {
        let mut current_id_opt = Some(root_id);

        while let Some(current_id) = current_id_opt {
            let Some(fragment) = arena.get_fragment(current_id) else {
                break;
            };

            let child_ctx = renderer.begin_fragment(fragment, initial_ctx);

            renderer.paint_fragment(fragment, &child_ctx);

            if let Some(child_id) = fragment.first_child {
                Self::walk(arena, child_id, renderer, &child_ctx);
            }

            renderer.end_fragment(fragment, &child_ctx);

            current_id_opt = fragment.next_sibling;
        }
    }
}
