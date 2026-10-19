use crate::geom::PhysicalRect;
use crate::style::ComputedStyle;
use blink_dom::DomNodeId;
use std::fmt;
use std::fmt::Formatter;

pub struct LayoutFragmentTag;
pub type LayoutFragmentId = blink_core::Id<LayoutFragmentTag>;

pub struct LayoutFragment {
    pub dom_node: Option<DomNodeId>,
    pub rect: PhysicalRect,
    pub style: ComputedStyle,

    pub first_child: Option<LayoutFragmentId>,
    pub next_sibling: Option<LayoutFragmentId>,
}

impl fmt::Debug for LayoutFragment {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let mut debug = f.debug_struct("");

        match &self.dom_node {
            Some(id) => debug.field("dom_node_id", id),
            None => debug.field("dom_node_id", &"None"),
        };

        debug.field("rect", &self.rect).field("style", &self.style);

        match &self.first_child {
            Some(id) => debug.field("first_child", id),
            None => debug.field("first_child", &None::<LayoutFragmentId>),
        };
        match &self.next_sibling {
            Some(id) => debug.field("next_sibling", id),
            None => debug.field("next_sibling", &None::<LayoutFragmentId>),
        };

        debug.finish()
    }
}

pub struct FragmentArena {
    pub fragments: Vec<LayoutFragment>,
}

struct IndexedFragment<'a> {
    index: usize,
    fragment: &'a LayoutFragment,
}

impl<'a> fmt::Debug for IndexedFragment<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        // Write the custom prefix, then forward formatting directly to the inner fragment
        write!(f, "LayoutFragment <{}>: {:?}", self.index, self.fragment)
    }
}

impl fmt::Debug for FragmentArena {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let indexed_list: Vec<IndexedFragment> = self
            .fragments
            .iter()
            .enumerate()
            .map(|(i, frag)| IndexedFragment {
                index: i + 1,
                fragment: frag,
            })
            .collect();

        f.debug_struct("FragmentArena")
            .field("fragments", &indexed_list)
            .finish()
    }
}

impl LayoutFragment {
    pub fn new(dom_node: Option<DomNodeId>, rect: PhysicalRect, style: ComputedStyle) -> Self {
        Self {
            dom_node,
            rect,
            style,
            first_child: None,
            next_sibling: None,
        }
    }

    pub fn set_first_child(&mut self, child: LayoutFragmentId) {
        self.first_child = Some(child);
    }

    pub fn set_next_sibling(&mut self, sibling: LayoutFragmentId) {
        self.next_sibling = Some(sibling);
    }
}

impl FragmentArena {
    pub fn new() -> Self {
        Self {
            fragments: Vec::new(),
        }
    }

    pub fn add_fragment(&mut self, fragment: LayoutFragment) -> LayoutFragmentId {
        let fragment_id = self.fragments.len();
        self.fragments.push(fragment);
        LayoutFragmentId::from_usize(fragment_id).unwrap()
    }
}

impl Default for FragmentArena {
    fn default() -> Self {
        Self::new()
    }
}
