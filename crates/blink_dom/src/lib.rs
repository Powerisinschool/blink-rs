mod slot_store;

use crate::slot_store::{SlotId, SlotStore};
use blink_core::generator::GEN_DEAD;
use markup5ever::{Attribute, QualName, ns};
use std::collections::HashMap;
use std::fmt;
use std::fmt::Formatter;

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

#[derive(Debug)]
pub struct CharacterDataTag;
pub type CharacterDataId = SlotId<CharacterDataTag>;

#[derive(Debug)]
pub struct AttrTag;
pub type AttrId = SlotId<AttrTag>;

pub struct Dom {
    nodes: Vec<DomNode>,
    free_slots: Vec<u32>,

    attribute_store: SlotStore<Box<[Attribute]>, AttrTag>,
    character_data: SlotStore<String, CharacterDataTag>,

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

        f.debug_struct("Dom")
            .field("nodes", &indexed_list)
            .field("attributes", &self.attribute_store)
            .field("character_data", &self.character_data)
            .finish()
    }
}

// TODO: Consider using a more compact representation for NodeKind to save memory, especially for large DOM trees. For example, we could use a single byte to represent the kind and use bitfields for additional information.
enum NodeKind {
    Element {
        tag: u8, // Easily fits the indices for the ~150 HTML tags
        first_child: Option<DomNodeId>,
        attrs: Option<AttrId>,
    },
    Text {
        text_id: CharacterDataId,
    },
    Comment {
        text_id: CharacterDataId,
    },
    ProcessingInstruction {
        target_id: CharacterDataId,
        data_id: CharacterDataId,
    },
    EmptyLeaf {
        tag: u8, // Easily fits the indices for the ~150 HTML tags
        attrs: Option<AttrId>,
    },
    Free,
}

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
            NodeKind::Text { text_id } => f.debug_struct("Text").field("text_id", text_id).finish(),
            NodeKind::EmptyLeaf { tag, attrs } => {
                let mut debug = f.debug_struct("EmptyLeaf");

                match attrs {
                    Some(a) => debug.field("attrs", a),
                    None => debug.field("attrs", &None::<AttrId>),
                };

                debug.field("tag_str", &get_tag_name(*tag)).finish()
            }
            NodeKind::Comment { text_id } => {
                f.debug_struct("Comment").field("text_id", text_id).finish()
            }
            NodeKind::ProcessingInstruction { target_id, data_id } => f
                .debug_struct("ProcessingInstruction")
                .field("target_id", target_id)
                .field("data_id", data_id)
                .finish(),
            NodeKind::Free => f.debug_struct("Free").finish(),
        }
    }
}

pub struct DomNodeTag;
pub type DomNodeId = blink_core::generator::GenId<DomNodeTag>;

pub struct DomNode {
    parent: Option<DomNodeId>,
    next_sibling: Option<DomNodeId>,
    generation: u16,
    kind: NodeKind,
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

impl DomNode {
    pub fn is_element(&self) -> bool {
        matches!(self.kind, NodeKind::Element { .. })
    }

    pub fn is_text(&self) -> bool {
        matches!(self.kind, NodeKind::Text { .. })
    }

    pub fn is_empty_leaf(&self) -> bool {
        matches!(self.kind, NodeKind::EmptyLeaf { .. })
    }

    pub fn is_alive(&self) -> bool {
        !matches!(self.kind, NodeKind::Free)
    }

    pub fn get_tag(&self) -> Option<u8> {
        match self.kind {
            NodeKind::Element { tag, .. } | NodeKind::EmptyLeaf { tag, .. } => Some(tag),
            _ => None,
        }
    }

    pub fn get_tag_name(&self) -> Option<&str> {
        Some(get_tag_name(self.get_tag()?))
    }
}

// ---------------------------------------------------------
// Unified API Implementation
// ---------------------------------------------------------
impl Default for Dom {
    fn default() -> Self {
        Self::new()
    }
}

impl Dom {
    pub fn new() -> Self {
        Self {
            nodes: Vec::with_capacity(512),
            free_slots: Vec::with_capacity(128),
            // attribute_table: Vec::with_capacity(512),
            // free_attrs: Vec::with_capacity(128),
            attribute_store: SlotStore::with_capacity(512),
            character_data: SlotStore::with_capacity(512),
            tag_registry: HashMap::new(),
        }
    }

    pub fn add_element(&mut self, tag: u8, attrs: Option<Box<[Attribute]>>) -> DomNodeId {
        self.ensure_tag_registered(tag);
        let node_idx = self.nodes.len();

        let attrs: Option<AttrId> = attrs
            .filter(|a| !a.is_empty())
            .map(|a| self.attribute_store.insert(a))
            .unwrap_or_default();

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

    pub fn add_text(&mut self, contents: &str) -> DomNodeId {
        let text_id: CharacterDataId = self
            .character_data
            .insert(contents)
            .expect("Failed to insert text into character data store");

        let mut kind = NodeKind::Text { text_id };

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

    pub fn append_text(&mut self, node: DomNodeId, contents: &str) -> Result<(), String> {
        let node = if let Some(node) = self.live(node) {
            node
        } else {
            return Err("Node is not live".to_string());
        };

        if let NodeKind::Text { text_id } = &node.kind {
            if let Some(entry) = self.character_data.get_mut(*text_id) {
                entry.push_str(contents);
                Ok(())
            } else {
                Err("Text ID not found in character data store".to_string())
            }
        } else {
            Err("Node is not a text node".to_string())
        }
    }

    pub fn add_comment(&mut self, comment: &str) -> DomNodeId {
        let text_id = self
            .character_data
            .insert(comment)
            .expect("Failed to insert comment into character data store");

        let mut kind = NodeKind::Comment { text_id };

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

    pub fn add_pi(&mut self, target: &str, data: &str) -> DomNodeId {
        let target_id = self
            .character_data
            .insert(target)
            .expect("Failed to insert processing instruction target");
        let data_id = self
            .character_data
            .insert(data)
            .expect("Failed to insert processing instruction data");

        let mut kind = NodeKind::ProcessingInstruction { target_id, data_id };

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

    pub fn set_parent(&mut self, node: DomNodeId, parent: DomNodeId) -> Result<(), String> {
        if self.live(parent).is_none() {
            return Err("parent node is not valid".into());
        }

        if let Some(n) = self.live_mut(node) {
            n.parent = Some(parent);
            Ok(())
        } else {
            Err("given node is not valid".into())
        }
    }

    pub fn set_first_child(
        &mut self,
        parent: DomNodeId,
        child: Option<DomNodeId>,
    ) -> Result<(), String> {
        if child.is_some() && self.live(child.unwrap()).is_none() {
            return Err("child node is not valid".into());
        }

        // If child is None, we can just set it to None without further checks
        if let Some(DomNode {
            kind: NodeKind::Element { first_child, .. },
            ..
        }) = self.live_mut(parent)
        {
            *first_child = child;
            Ok(())
        } else {
            Err("given node is not valid".into())
        }
    }

    pub fn set_next_sibling(
        &mut self,
        node: DomNodeId,
        sibling: Option<DomNodeId>,
    ) -> Result<(), String> {
        if sibling.is_some() && self.live(sibling.unwrap()).is_none() {
            return Err("sibling node is not valid".into());
        }

        // If sibling is None, we can just set it to None without further checks
        if let Some(n) = self.live_mut(node) {
            n.next_sibling = sibling;
            Ok(())
        } else {
            Err("given node is not valid".into())
        }
    }

    pub fn set_attributes(
        &mut self,
        id: DomNodeId,
        new: Option<Box<[Attribute]>>,
    ) -> Result<(), String> {
        let current_attr_id = {
            let node = self.live_mut(id).ok_or("Node not found")?;
            match &node.kind {
                NodeKind::Element { attrs, .. } | NodeKind::EmptyLeaf { attrs, .. } => *attrs,
                NodeKind::Comment { .. } => return Err("Comment nodes have no attributes".into()),
                NodeKind::ProcessingInstruction { .. } => {
                    return Err("Processing instruction nodes have no attributes".into());
                }
                NodeKind::Text { .. } => return Err("Text nodes have no attributes".into()),
                NodeKind::Free => return Err("Free nodes have no attributes".into()),
            }
        };

        let Some(new) = new else {
            return Ok(());
        };

        if new.is_empty() {
            if let Some(attr_id) = current_attr_id {
                self.attribute_store
                    .remove(attr_id)
                    .ok_or("Failed to remove attributes")?;

                let node = self.live_mut(id).ok_or("Node not found")?;

                match &mut node.kind {
                    NodeKind::Element { attrs, .. } | NodeKind::EmptyLeaf { attrs, .. } => {
                        *attrs = None;
                    }
                    _ => unreachable!(), // We already validated the type at the start
                }
            }

            return Ok(());
        }

        if let Some(curr) = current_attr_id {
            // There is an existing attribute declaration, so we replace the value
            self.attribute_store.replace(curr, new);
            return Ok(());
        }

        // There is no existing attribute, so we insert
        let new_attr_id = self.attribute_store.insert(new);
        let node = self.live_mut(id).ok_or("Node not found")?;
        match &mut node.kind {
            NodeKind::Element { attrs, .. } | NodeKind::EmptyLeaf { attrs, .. } => {
                *attrs = new_attr_id
            }
            _ => unreachable!(), // We already validated the type at the start
        }

        Ok(())
    }

    pub fn get_nodes_len(&self) -> usize {
        self.nodes.len()
    }

    pub fn get_node(&self, id: DomNodeId) -> Option<&DomNode> {
        let node = self.live(id)?;
        if matches!(node.kind, NodeKind::Free) {
            // Hide freed nodes
            return None;
        }

        Some(node)
    }

    pub fn get_parent(&self, id: DomNodeId) -> Option<DomNodeId> {
        self.live(id)?.parent
    }

    pub fn get_first_child(&self, id: DomNodeId) -> Option<DomNodeId> {
        match self.live(id)?.kind {
            NodeKind::Element { first_child, .. } => first_child,
            _ => None,
        }
    }

    pub fn get_next_sibling(&self, id: DomNodeId) -> Option<DomNodeId> {
        self.live(id)?.next_sibling
    }

    pub fn get_attributes(&self, id: DomNodeId) -> Option<&[Attribute]> {
        match self.live(id)?.kind {
            NodeKind::EmptyLeaf {
                attrs: Some(attr_id),
                ..
            }
            | NodeKind::Element {
                attrs: Some(attr_id),
                ..
            } => self
                .attribute_store
                .get(attr_id)
                .map(|attrs| attrs.as_ref()),
            _ => None,
        }
    }

    pub fn get_text(&self, id: DomNodeId) -> Option<String> {
        let NodeKind::Text { text_id } = &self.live(id)?.kind else {
            return None;
        };

        self.character_data.get(*text_id).map(|s| s.to_string())
    }

    pub fn get_tag(&self, id: DomNodeId) -> Option<u8> {
        match self.live(id)?.kind {
            NodeKind::Element { tag, .. } => Some(tag),
            NodeKind::EmptyLeaf { tag, .. } => Some(tag),
            _ => None,
        }
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

// Private helper functions for Dom
impl Dom {
    fn live(&self, id: DomNodeId) -> Option<&DomNode> {
        self.nodes
            .get(id.slot())
            .filter(|n| n.generation == id.generation() && !matches!(n.kind, NodeKind::Free))
    }

    fn live_mut(&mut self, id: DomNodeId) -> Option<&mut DomNode> {
        self.nodes
            .get_mut(id.slot())
            .filter(|n| n.generation == id.generation() && !matches!(n.kind, NodeKind::Free))
    }

    fn use_free_slot(&mut self, kind: NodeKind) -> Result<DomNodeId, NodeKind> {
        while !self.free_slots.is_empty() {
            let slot = self.free_slots.pop().unwrap() as usize;
            if let Some(node) = self.nodes.get_mut(slot)
                && let Some(id) = DomNodeId::new(slot, node.generation)
            {
                node.parent = None;
                node.next_sibling = None;
                node.kind = kind;
                return Ok(id);
            };
        }

        Err(kind)
    }

    // fn alloc_attrs(&mut self, old_attr_id: Option<AttrId>) -> Option<AttrId> {
    //     if let Some(a) = old_attr_id
    //         && self.attribute_table.len() > a.get_index()
    //     {
    //         return Some(a);
    //     }
    //
    //     if let Some(a) = self.free_attrs.pop() {
    //         return Some(a);
    //     }
    //
    //     let a = AttrId::from_usize(self.attribute_table.len())?;
    //     self.attribute_table.push(None);
    //     Some(a)
    // }

    #[allow(dead_code)]
    fn free(&mut self, id: DomNodeId) -> bool {
        let Some(node) = self.live_mut(id) else {
            return false;
        };
        let old = std::mem::replace(&mut node.kind, NodeKind::Free);
        node.parent = None;
        node.next_sibling = None;
        node.generation += 1;

        let retire = node.generation as u32 >= GEN_DEAD;

        if let NodeKind::Element { attrs: Some(a), .. }
        | NodeKind::EmptyLeaf { attrs: Some(a), .. } = old
        {
            self.attribute_store.remove(a);
        }

        if let NodeKind::Text { text_id } = old {
            self.character_data.remove(text_id);
        }

        if !retire {
            self.free_slots.push(id.slot() as u32);
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use markup5ever::tendril::Tendril;
    use markup5ever::{Attribute, Namespace, local_name};

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

    #[test]
    fn test_clear_attributes() {
        let mut dom = Dom::new();

        let node = dom.add_element(
            get_tag_index("div"),
            Some(Box::new([
                Attribute {
                    name: QualName::new(None, Namespace::default(), local_name!("class")),
                    value: Tendril::from("class1 class2"),
                },
                Attribute {
                    name: QualName::new(None, Namespace::default(), local_name!("id")),
                    value: Tendril::from("unique"),
                },
            ])),
        );

        let attrs = dom.get_attributes(node);
        assert!(attrs.is_some());
        assert_eq!(attrs.unwrap().len(), 2);

        let result = dom.set_attributes(node, None);
        assert!(!result.is_err());

        let attrs = dom.get_attributes(node);
        assert!(attrs.is_some());
        assert_eq!(attrs.unwrap().len(), 2);

        let result = dom.set_attributes(node, Some(Box::new([])));
        assert!(!result.is_err());

        let attrs = dom.get_attributes(node);
        assert!(attrs.is_none());
    }

    #[test]
    fn stale_id_rejected_after_reuse() {
        let mut dom = Dom::new();
        let a = dom.add_element(get_tag_index("div"), None);
        assert!(dom.free(a));
        let b = dom.add_element(get_tag_index("span"), None);
        assert_eq!(a.slot(), b.slot());
        assert_ne!(a, b);
        assert!(dom.get_node(a).is_none());
        assert_eq!(dom.get_tag(a), None);
        assert_eq!(dom.get_tag(b), Some(get_tag_index("span")));
        assert!(!dom.free(a)); // double free is a no-op
    }

    #[test]
    fn stale_id_rejected_by_setters() {
        let mut dom = Dom::new();
        let a = dom.add_element(get_tag_index("div"), None);
        let b = dom.add_element(get_tag_index("div"), None);
        assert_ne!(a.slot(), b.slot());
        assert!(dom.free(a));

        let stale_parenting = dom.set_parent(b, a);
        assert!(stale_parenting.is_err());
    }

    #[test]
    fn slot_retires_at_gen_dead() {
        let mut dom = Dom::new();
        let div = get_tag_index("div");
        let mut id = dom.add_element(div, None);
        let slot = id.slot();
        for _ in 0..(GEN_DEAD - 1) {
            // generations 1..=1022
            assert!(dom.free(id));
            id = dom.add_element(div, None);
            assert_eq!(id.slot(), slot);
        }
        assert!(dom.free(id)); // bumps to GEN_DEAD: retired
        assert_ne!(dom.add_element(div, None).slot(), slot);
    }

    #[test]
    fn stale_handles_cannot_mutate_reused_nodes() {
        let mut dom = Dom::new();
        let parent = dom.add_element(get_tag_index("div"), None);
        let stale = dom.add_element(get_tag_index("span"), None);

        assert!(dom.free(stale));

        let replacement = dom.add_element(get_tag_index("section"), None);
        assert_eq!(stale.slot(), replacement.slot());
        assert_ne!(stale, replacement);

        // A stale node ID must not modify its replacement's parent.
        assert!(dom.set_parent(stale, parent).is_err());
        assert_eq!(dom.get_parent(replacement), None);

        // Nor its next sibling.
        assert!(dom.set_next_sibling(stale, Some(parent)).is_err());
        assert_eq!(dom.get_next_sibling(replacement), None);

        // Nor the replacement's first child when used as a parent ID.
        assert!(dom.set_first_child(stale, Some(parent)).is_err());
        assert_eq!(dom.get_first_child(replacement), None);

        // Attribute mutation must reject stale IDs.
        assert!(dom.set_attributes(stale, Some(Box::new([]))).is_err());

        // The replacement itself must remain valid and unchanged.
        assert_eq!(dom.get_tag(replacement), Some(get_tag_index("section")));
    }

    #[test]
    fn stale_child_id_rejected_by_set_first_child() {
        let mut dom = Dom::new();

        let parent = dom.add_element(get_tag_index("div"), None);
        let stale_child = dom.add_element(get_tag_index("span"), None);

        // Free the child, then reuse its slot with a new generation.
        assert!(dom.free(stale_child));
        let replacement = dom.add_element(get_tag_index("section"), None);

        assert_eq!(stale_child.slot(), replacement.slot());
        assert_ne!(stale_child, replacement);

        // The old ID must not be accepted as the parent's first child.
        let result = dom.set_first_child(parent, Some(stale_child));

        assert!(result.is_err());
        assert_eq!(dom.get_first_child(parent), None);

        // Reusing the slot must not invalidate the replacement's own ID.
        assert_eq!(dom.get_tag(replacement), Some(get_tag_index("section")));
    }

    #[test]
    fn stale_sibling_id_rejected_by_set_next_sibling() {
        let mut dom = Dom::new();

        let node = dom.add_element(get_tag_index("div"), None);
        let stale_sibling = dom.add_element(get_tag_index("span"), None);

        // Free the sibling, then reuse its slot with a new generation.
        assert!(dom.free(stale_sibling));
        let replacement = dom.add_element(get_tag_index("section"), None);

        assert_eq!(stale_sibling.slot(), replacement.slot());
        assert_ne!(stale_sibling, replacement);

        // The old ID must not be stored as the node's next sibling.
        let result = dom.set_next_sibling(node, Some(stale_sibling));

        assert!(result.is_err());
        assert_eq!(dom.get_next_sibling(node), None);

        // The replacement remains valid.
        assert_eq!(dom.get_tag(replacement), Some(get_tag_index("section")));
    }

    #[test]
    fn option_id_is_four_bytes() {
        assert_eq!(size_of::<Option<DomNodeId>>(), 4);
    }
}
