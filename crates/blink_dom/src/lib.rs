use markup5ever::{Attribute, QualName, ns};
use std::collections::HashMap;
use std::fmt;
use std::fmt::Formatter;
use blink_core::generator::GEN_DEAD;

const HTML_TAGS: &[&str] = &[
    "a",
    "abbr",
    "address",
    "area",
    "article",
    "aside",
    "audio",
    "b",
    "base",
    "bdi",
    "bdo",
    "blockquote",
    "body",
    "br",
    "button",
    "canvas",
    "caption",
    "cite",
    "code",
    "col",
    "colgroup",
    "data",
    "datalist",
    "dd",
    "del",
    "details",
    "dfn",
    "dialog",
    "div",
    "dl",
    "dt",
    "em",
    "embed",
    "fieldset",
    "figcaption",
    "figure",
    "footer",
    "form",
    "h1",
    "h2",
    "h3",
    "h4",
    "h5",
    "h6",
    "head",
    "header",
    "hgroup",
    "hr",
    "html",
    "i",
    "iframe",
    "img",
    "input",
    "ins",
    "kbd",
    "label",
    "legend",
    "li",
    "link",
    "main",
    "map",
    "mark",
    "menu",
    "meta",
    "meter",
    "nav",
    "noscript",
    "object",
    "ol",
    "optgroup",
    "option",
    "output",
    "p",
    "picture",
    "pre",
    "progress",
    "q",
    "rp",
    "rt",
    "ruby",
    "s",
    "samp",
    "script",
    "section",
    "select",
    "slot",
    "small",
    "source",
    "span",
    "strong",
    "style",
    "sub",
    "summary",
    "sup",
    "table",
    "tbody",
    "td",
    "template",
    "textarea",
    "tfoot",
    "th",
    "thead",
    "time",
    "title",
    "tr",
    "track",
    "u",
    "ul",
    "var",
    "video",
    "wbr",
];

// O(log(N))
pub fn get_tag_index(name: &str) -> u8 {
    match HTML_TAGS.binary_search(&name) {
        Ok(idx) => (idx + 1) as u8,
        Err(_) => 0, // Fallback/unknown
    }
}

// O(1)
pub fn get_tag_name(tag: u8) -> &'static str {
    if tag == 0 {
        return "document";
    }

    let array_idx = (tag - 1) as usize;
    if array_idx < HTML_TAGS.len() {
        HTML_TAGS[array_idx]
    } else {
        "div"
    }
}

// A proposed, more memory-efficient representation of the DOM
#[cfg(feature = "dense-core")]
pub struct Dom {
    nodes: Vec<DomNode>,
    free_slots: Vec<u32>,
    pub attribute_table: Vec<Option<Box<[Attribute]>>>,
    pub tag_registry: HashMap<u8, Box<QualName>>,
}

struct IndexedNode<'a> {
    index: usize,
    node: &'a DomNode,
}

impl<'a> fmt::Debug for IndexedNode<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        // Write the custom prefix, then forward formatting directly to the inner fragment
        write!(f, "DomNode <{}>: {:?}", self.index, self.node)
    }
}

impl fmt::Debug for Dom {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let indexed_list: Vec<IndexedNode> = self
            .nodes
            .iter()
            .enumerate()
            .map(|(i, node)| IndexedNode { index: i + 1, node })
            .collect();

        f.debug_struct("Dom").field("nodes", &indexed_list).finish()
    }
}

#[cfg(feature = "dense-core")]
pub struct AttrTag;
#[cfg(feature = "dense-core")]
pub type AttrId = blink_core::base::Id<AttrTag>;

#[cfg(feature = "dense-core")]
pub enum NodeKind {
    Element {
        tag: u8, // Easily fits the indices for the ~150 HTML tags
        first_child: Option<DomNodeId>,
        attrs: Option<AttrId>,
    },
    Text {
        // text_id: u32
    },
    EmptyLeaf {
        tag: u8, // Easily fits the indices for the ~150 HTML tags
        attrs: Option<AttrId>,
    },
    Free,
}

#[cfg(feature = "dense-core")]
impl fmt::Debug for NodeKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            NodeKind::Element {
                tag,
                first_child,
                attrs,
            } => {
                let mut debug = f.debug_struct("Element");
                debug.field("tag_str", &get_tag_name(*tag));

                match first_child {
                    Some(id) => debug.field("first_child", id),
                    None => debug.field("first_child", &None::<DomNodeId>),
                };

                match attrs {
                    Some(a) => debug.field("attrs", a),
                    None => debug.field("attrs", &None::<AttrId>),
                };

                debug.finish()
            }
            NodeKind::Text { .. } => f.debug_struct("Text").finish(),
            NodeKind::EmptyLeaf { tag, attrs } => {
                let mut debug = f.debug_struct("EmptyLeaf");

                match attrs {
                    Some(a) => debug.field("attrs", a),
                    None => debug.field("attrs", &None::<AttrId>),
                };

                debug.field("tag_str", &get_tag_name(*tag)).finish()
            }
            NodeKind::Free => f.debug_struct("Free").finish(),
        }
    }
}

// Memory-safe, arena-allocated DOM
#[cfg(not(feature = "dense-core"))]
#[derive(Debug)]
pub struct Dom {
    nodes: Vec<DomNode>,
    pub tag_registry: HashMap<u8, Box<QualName>>,
}

#[cfg(not(feature = "dense-core"))]
pub enum NodeKind {
    Element {
        tag: u8, // Easily fits the indices for the ~150 HTML tags
        first_child: Option<DomNodeId>,
        attrs: Option<Box<[Attribute]>>,
    },
    Text {
        // text_id: u32
    },
    EmptyLeaf {
        tag: u8, // Easily fits the indices for the ~150 HTML tags
    },
}

#[cfg(not(feature = "dense-core"))]
impl fmt::Debug for NodeKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            NodeKind::Element {
                tag,
                first_child,
                attrs,
            } => f
                .debug_struct("Element")
                .field("tag_str", &get_tag_name(*tag))
                .field("first_child", first_child)
                .field("attrs", attrs)
                .finish(),
            NodeKind::Text { .. } => f.debug_struct("Text").finish(),
            NodeKind::EmptyLeaf { tag } => f
                .debug_struct("EmptyLeaf")
                .field("tag_str", &get_tag_name(*tag))
                .finish(),
        }
    }
}

pub struct DomNodeTag;
pub type DomNodeId = blink_core::generator::GenId<DomNodeTag>;

pub struct DomNode {
    parent: Option<DomNodeId>,
    next_sibling: Option<DomNodeId>,
    generation: u16,
    pub kind: NodeKind,
}

impl fmt::Debug for DomNode {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let mut debug = f.debug_struct("DomNode");
        match &self.parent {
            Some(id) => debug.field("parent", id),
            None => debug.field("parent", &None::<DomNodeId>),
        };
        match &self.next_sibling {
            Some(id) => debug.field("next_sibling", id),
            None => debug.field("next_sibling", &None::<DomNodeId>),
        };
        debug.field("kind", &self.kind).finish()
    }
}

// ---------------------------------------------------------
// Unified API Implementation
// ---------------------------------------------------------
impl Dom {
    // pub fn new() -> Self {
    //     Self {
    //         nodes: Vec::new(),
    //         tag_registry: Default::default(),
    //     }
    // }

    // pub fn add_element(&mut self, tag: u8, attrs: Option<Box<[Attribute]>>) -> NonZeroU32 {
    //     self.ensure_tag_registered(tag);
    //     let node_idx = self.nodes.len();
    //
    //     if node_idx == usize::MAX {
    //         panic!("Maximum number of nodes reached");
    //     }
    //
    //     match get_tag_name(tag) {
    //         "br" | "hr" => self.nodes.push(DomNode {
    //             parent: None,
    //             next_sibling: None,
    //             kind: NodeKind::EmptyLeaf {
    //                 tag,
    //             },
    //         }),
    //         _ => {
    //             self.nodes.push(DomNode {
    //                 parent: None,
    //                 next_sibling: None,
    //                 kind: NodeKind::Element {
    //                     tag,
    //                     first_child: None,
    //                 },
    //             });
    //             self.add_attributes(attrs);
    //         },
    //     }
    //
    //     NonZeroU32::new((node_idx as u32) + 1).unwrap()
    // }

    // pub fn add_text(&mut self, _text_id: u32) -> DomNodeId {
    //     let node_idx = self.nodes.len();
    //     self.nodes.push(DomNode {
    //         parent: None,
    //         next_sibling: None,
    //         kind: NodeKind::Text {
    //             // text_id
    //         },
    //     });
    //     DomNodeId::from_usize(node_idx).unwrap()
    // }

    pub fn set_parent(&mut self, node: DomNodeId, parent: DomNodeId) {
        let idx = node.slot();
        self.nodes[idx].parent = Some(parent);
    }

    pub fn set_first_child(&mut self, parent: DomNodeId, child: Option<DomNodeId>) {
        if let NodeKind::Element {
            ref mut first_child,
            ..
        } = self.nodes[parent.slot()].kind
        {
            *first_child = child;
        }
    }

    pub fn set_next_sibling(&mut self, node: DomNodeId, sibling: Option<DomNodeId>) {
        self.nodes[node.slot()].next_sibling = sibling;
    }

    fn is_dead(&self, node: &DomNode) -> bool {
        node.generation >= GEN_DEAD as u16
    }

    pub fn get_parent(&self, id: DomNodeId) -> Option<DomNodeId> {
        let node = &self.nodes[id.slot()];
        if self.is_dead(&node) {
            None
        } else {
            node.parent
        }
    }

    pub fn get_first_child(&self, parent: DomNodeId) -> Option<DomNodeId> {
        let node = &self.nodes[parent.slot()];

        if !self.is_dead(&node)
        && let NodeKind::Element { first_child, .. } = node.kind {
            first_child
        } else {
            None
        }
    }

    pub fn get_next_sibling(&self, node: DomNodeId) -> Option<DomNodeId> {
        let node = &self.nodes[node.slot()];
        if self.is_dead(&node) {
            None
        } else {
            node.next_sibling
        }
    }

    pub fn get_tag(&self, id: DomNodeId) -> Option<u8> {
        let node = &self.nodes[id.slot()];
        if self.is_dead(&node) {
            None
        } else {
            match node.kind {
                NodeKind::Element { tag, .. } => Some(tag),
                NodeKind::EmptyLeaf { tag, .. } => Some(tag),
                _ => None,
            }
        }
    }

    pub fn get_nodes_len(&self) -> usize {
        self.nodes.len()
    }

    pub fn get_node(&self, id: DomNodeId) -> Option<&DomNode> {
        self.nodes.get(id.slot())
    }

    pub fn get_root_node_id(&self) -> DomNodeId {
        DomNodeId::new(0, 0).unwrap()
    }

    pub fn ensure_tag_registered(&mut self, tag: u8) {
        self.tag_registry.entry(tag).or_insert_with(|| {
            let name_str = get_tag_name(tag);

            Box::new(QualName::new(
                None,
                ns!(html),
                markup5ever::LocalName::from(name_str),
            ))
        });
    }
}

impl Default for Dom {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "dense-core")]
impl Dom {
    pub fn new() -> Self {
        Self {
            nodes: Vec::with_capacity(512),
            free_slots: Vec::with_capacity(128),
            attribute_table: Vec::with_capacity(512),
            tag_registry: HashMap::new(),
        }
    }

    pub fn add_element(&mut self, tag: u8, attrs: Option<Box<[Attribute]>>) -> DomNodeId {
        self.ensure_tag_registered(tag);
        let node_idx = self.nodes.len();

        let attrs = attrs.filter(|a| !a.is_empty()).map(|a| {
            self.attribute_table.push(Some(a));
            AttrId::from_usize(self.attribute_table.len() - 1).unwrap()
        });

        let mut kind = match get_tag_name(tag) {
            "br" | "hr" => NodeKind::EmptyLeaf { tag, attrs },
            _ => NodeKind::Element {
                tag,
                first_child: None,
                attrs,
            },
        };

        kind = match self.use_free_slot(kind) {
            Ok(id) => return id,
            Err(kind) => kind,
        };

        self.nodes.push(DomNode {
            parent: None,
            next_sibling: None,
            generation: 0,
            kind,
        });
        DomNodeId::new(node_idx, 0).unwrap()
    }

    pub fn add_text(&mut self, _text_id: u32) -> DomNodeId {
        let mut kind = NodeKind::Text {
            // text_id
        };

        kind = match self.use_free_slot(kind) {
            Ok(id) => return id,
            Err(kind) => kind,
        };

        let node_idx = self.nodes.len();

        self.nodes.push(DomNode {
            parent: None,
            next_sibling: None,
            generation: 0,
            kind,
        });
        DomNodeId::new(node_idx, 0).unwrap()
    }

    fn use_free_slot(&mut self, kind: NodeKind) -> Result<DomNodeId, NodeKind> {
        while !self.free_slots.is_empty() {
            let slot = self.free_slots.pop().unwrap() as usize;
            if let Some(node) = self.nodes.get_mut(slot)
                && let Some(id) = DomNodeId::new(slot, node.generation) {
                node.parent = None;
                node.next_sibling = None;
                node.kind = kind;
                return Ok(id);
            };
        }

        Err(kind)
    }

    pub fn set_attributes(&mut self, id: DomNodeId, new: Option<Box<[Attribute]>>) -> Result<(), String> {
        let node = self.nodes.get_mut(id.slot()).ok_or("Node not found")?;
        let slot = match &mut node.kind {
            NodeKind::Element { attrs, .. } | NodeKind::EmptyLeaf { attrs, .. } => attrs,
            NodeKind::Text {} => return Err("Text nodes have no attributes".into()),
            NodeKind::Free => return Err("Free nodes have no attributes".into()),
        };
        match (new.filter(|a| !a.is_empty()), *slot) {
            (Some(list), Some(existing)) => self.attribute_table[existing.get_index()] = Some(list),
            (Some(list), None) => {
                self.attribute_table.push(Some(list));
                *slot = Some(AttrId::from_usize(self.attribute_table.len() - 1).unwrap());
            }
            (None, Some(existing)) => {
                self.attribute_table[existing.get_index()] = None;
                *slot = None;
            }
            (None, None) => {}
        }
        Ok(())
    }

    pub fn get_attributes(&self, id: DomNodeId) -> Option<&[Attribute]> {
        let node = &self.nodes.get(id.slot())?;
        if self.is_dead(&node) {
            return None;
        }

        match node.kind {
            NodeKind::EmptyLeaf {
                attrs: Some(attrs_id),
                ..
            }
            | NodeKind::Element {
                attrs: Some(attrs_id),
                ..
            } => self.attribute_table.get(attrs_id.get_index())?.as_deref(),
            _ => None,
        }
    }

    fn free(&mut self, id: DomNodeId) -> bool {
        let Some(node) = self.nodes.get_mut(id.slot())
            .filter(|n| n.generation == id.generation()) else { return false };
        node.generation += 1;
        node.kind = NodeKind::Free;
        if node.generation < GEN_DEAD as u16 { self.free_slots.push(id.slot() as u32); }

        true
    }
}

#[cfg(not(feature = "dense-core"))]
impl Dom {
    pub fn new() -> Self {
        Self {
            nodes: Vec::with_capacity(512),
            tag_registry: HashMap::new(),
        }
    }

    pub fn add_element(&mut self, tag: u8, attrs: Option<Box<[Attribute]>>) -> DomNodeId {
        self.ensure_tag_registered(tag);
        let node_idx = self.nodes.len();

        if node_idx == usize::MAX {
            panic!("Maximum number of nodes reached");
        }

        match get_tag_name(tag) {
            "br" | "hr" => self.nodes.push(DomNode {
                parent: None,
                next_sibling: None,
                kind: NodeKind::EmptyLeaf { tag },
            }),
            _ => {
                self.nodes.push(DomNode {
                    parent: None,
                    next_sibling: None,
                    kind: NodeKind::Element {
                        tag,
                        first_child: None,
                        attrs,
                    },
                });
            }
        }

        DomNodeId::from_u32((node_idx as u32) + 1).unwrap()
    }

    pub fn set_attributes(&mut self, index: DomNodeId, _attrs: Option<Box<[Attribute]>>) {
        let node_kind = &mut self.nodes[index.get_index()].kind;

        match node_kind {
            NodeKind::Element { attrs, .. } => {
                *attrs = _attrs;
            }
            _ => {}
        }
    }

    pub fn get_attributes(&self, index: DomNodeId) -> Option<&[Attribute]> {
        let node = self.nodes.get(index.get_index())?;

        match &node.kind {
            NodeKind::Element { attrs, .. } => attrs.as_deref(),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_and_retrieve_element() {
        let mut dom = Dom::new();

        // Define a theoretical u8 mapping for a <div> tag
        let div_tag = 15;
        let node_id = dom.add_element(div_tag, None);

        assert_eq!(dom.get_tag(node_id), Some(div_tag));
        assert_eq!(dom.get_parent(node_id), None);
    }

    #[test]
    fn test_nonzero_indexing_offset() {
        let mut dom = Dom::new();

        let first_id = dom.add_element(10, None);
        let second_id = dom.add_element(20, None);

        // Ensure IDs start at 1 and increment
        assert_eq!(first_id.slot(), 0);
        assert_eq!(second_id.slot(), 1);

        // Ensure correct tag resolution despite the +1 offset logic
        assert_eq!(dom.get_tag(first_id), Some(10));
        assert_eq!(dom.get_tag(second_id), Some(20));
    }
}
