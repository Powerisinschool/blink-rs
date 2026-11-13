use crate::geom::{PhysicalOffset, PhysicalRect, PhysicalSize};
use crate::style::ComputedStyle;
use blink_dom::DomNodeId;
use std::fmt;
use std::fmt::Formatter;

pub struct LayoutFragmentTag;
pub type LayoutFragmentId = blink_core::base::Id<LayoutFragmentTag>;

pub enum FragmentKind {
    Box,
    Line,
    Text(String),
}

pub struct LayoutFragment {
    pub dom_node: Option<DomNodeId>,
    rect: PhysicalRect,
    pub style: ComputedStyle,
    pub kind: FragmentKind,

    first_child: Option<LayoutFragmentId>,
    next_sibling: Option<LayoutFragmentId>,
}

impl LayoutFragment {
    pub fn new(
        dom_node: Option<DomNodeId>,
        rect: PhysicalRect,
        style: ComputedStyle,
        kind: FragmentKind,
    ) -> Self {
        Self {
            dom_node,
            rect,
            style,
            kind,
            first_child: None,
            next_sibling: None,
        }
    }

    #[must_use]
    pub fn with_first_child(mut self, first_child_id: Option<LayoutFragmentId>) -> Self {
        if let Some(child_id) = first_child_id {
            self.first_child = Some(child_id);
        }
        self
    }

    #[must_use]
    pub fn with_next_sibling(mut self, next_sibling_id: Option<LayoutFragmentId>) -> Self {
        if let Some(sibling_id) = next_sibling_id {
            self.next_sibling = Some(sibling_id);
        }
        self
    }

    #[must_use]
    pub fn with_origin(mut self, origin: PhysicalOffset) -> Self {
        self.rect.origin = origin;
        self
    }

    #[must_use]
    pub fn get_first_child(&self) -> Option<LayoutFragmentId> {
        self.first_child
    }

    #[must_use]
    pub fn get_next_sibling(&self) -> Option<LayoutFragmentId> {
        self.next_sibling
    }

    pub fn is_box(&self) -> bool {
        matches!(self.kind, FragmentKind::Box)
    }

    pub fn is_line(&self) -> bool {
        matches!(self.kind, FragmentKind::Line)
    }

    pub fn is_text(&self) -> bool {
        matches!(self.kind, FragmentKind::Text(_))
    }

    pub fn get_text(&self) -> Option<&str> {
        if let FragmentKind::Text(ref text) = self.kind {
            Some(text)
        } else {
            None
        }
    }

    pub fn get_origin(&self) -> &PhysicalOffset {
        &self.rect.origin
    }

    pub fn get_size(&self) -> &PhysicalSize {
        &self.rect.size
    }

    pub fn get_width(&self) -> f32 {
        self.rect.size.width
    }

    pub fn get_height(&self) -> f32 {
        self.rect.size.height
    }

    pub(self) fn get_kind_text(&self) -> String {
        match &self.kind {
            FragmentKind::Box => "Box".to_string(),
            FragmentKind::Line => "Line".to_string(),
            FragmentKind::Text(text) => format!("Text({})", text),
        }
    }
}

impl fmt::Debug for LayoutFragment {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let mut debug = f.debug_struct("");

        match &self.dom_node {
            Some(id) => debug.field("dom_node_id", id),
            None => debug.field("dom_node_id", &"None"),
        };

        debug
            .field("rect", &self.rect)
            .field("style", &self.style)
            .field("kind", &self.get_kind_text());

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
    fragments: Vec<LayoutFragment>,
    root: Option<LayoutFragmentId>,
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

impl FragmentArena {
    pub fn new() -> Self {
        Self {
            fragments: Vec::new(),
            root: None,
        }
    }

    pub fn add_fragment(&mut self, fragment: LayoutFragment) -> LayoutFragmentId {
        let fragment_id = self.fragments.len();
        self.fragments.push(fragment);
        LayoutFragmentId::from_usize(fragment_id).unwrap()
    }

    pub(crate) fn set_root(&mut self, root_id: Option<LayoutFragmentId>) {
        self.root = root_id;
    }

    pub fn get_root(&self) -> Option<LayoutFragmentId> {
        self.root
    }

    pub fn get_fragment_rect(&self, fragment_id: LayoutFragmentId) -> Option<&PhysicalRect> {
        self.fragments
            .get(fragment_id.get_index())
            .map(|fragment| &fragment.rect)
    }

    pub fn get_fragment(&self, fragment_id: LayoutFragmentId) -> Option<&LayoutFragment> {
        self.fragments.get(fragment_id.get_index())
    }

    pub fn is_empty(&self) -> bool {
        self.fragments.is_empty()
    }

    #[cfg(test)]
    pub(crate) fn get_fragment_by_dom_node(
        &self,
        dom_node_id: DomNodeId,
    ) -> Option<&LayoutFragment> {
        self.fragments
            .iter()
            .find(|fragment| fragment.dom_node == Some(dom_node_id))
    }
}

impl Default for FragmentArena {
    fn default() -> Self {
        Self::new()
    }
}
