use std::num::NonZeroU32;
use crate::geom::PhysicalRect;
use crate::style::ComputedStyle;

pub struct LayoutFragment {
    pub dom_node: Option<NonZeroU32>,
    pub rect: PhysicalRect,
    pub style: ComputedStyle,

    pub first_child: Option<NonZeroU32>,
    pub next_sibling: Option<NonZeroU32>,
}

pub struct FragmentArena {
    pub fragments: Vec<LayoutFragment>,
}

impl FragmentArena {
    pub fn new() -> Self {
        Self {
            fragments: Vec::new(),
        }
    }

    pub fn add_fragment(_fragment: LayoutFragment) -> NonZeroU32 {
        todo!()
    }
}
