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

impl LayoutFragment {
    pub fn new(
        dom_node: Option<NonZeroU32>,
        rect: PhysicalRect,
        style: ComputedStyle
    ) -> Self {
        Self {
            dom_node,
            rect,
            style,
            first_child: None,
            next_sibling: None,
        }
    }

    pub fn set_first_child(&mut self, child: NonZeroU32) {
        self.first_child = Some(child);
    }

    pub fn set_next_sibling(&mut self, sibling: NonZeroU32) {
        self.next_sibling = Some(sibling);
    }
}

impl FragmentArena {
    pub fn new() -> Self {
        Self {
            fragments: Vec::new(),
        }
    }

    pub fn add_fragment(&mut self, fragment: LayoutFragment) -> NonZeroU32 {
        let fragment_id = self.fragments.len();
        self.fragments.push(fragment);
        NonZeroU32::new((fragment_id as u32) + 1).unwrap()
    }
}
