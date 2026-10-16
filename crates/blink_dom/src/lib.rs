use std::num::{NonZeroU32, NonZeroU8};

// A more memory-efficient representation of the DOM
#[cfg(feature = "dense-core")]
struct ElementPayload {
    tag: u8,
    first_child: Option<NonZeroU32>,
}

#[cfg(feature = "dense-core")]
struct Dom {
    pub parents: Vec<Option<NonZeroU32>>,       // 4 bytes
    pub next_siblings: Vec<Option<NonZeroU32>>, // 4 bytes
    pub payload_keys: Vec<Option<NonZeroU32>>,  // 4 bytes (Bitmasked Type + Dense Index)

    pub element_payloads: Vec<ElementPayload>,  // { tag: u8, first_child: Option<u32> }
    pub text_payloads: Vec<Option<NonZeroU32>>, // plain text_ids
    pub void_payloads: Vec<u8>,                 // plain tags for empty leaves
}

// Memory-safe, arena-allocated DOM
#[cfg(not(feature = "dense-core"))]
struct Dom {
    nodes: Vec<DomNode>
}

#[cfg(not(feature = "dense-core"))]
enum NodeKind {
    Element {
        tag: u8, // Easily fits the indices for the ~150 HTML tags
        first_child: Option<NonZeroU32>
    },
    Text { text_id: u32 },
    EmptyLeaf {
        tag: u8, // Easily fits the indices for the ~150 HTML tags
    },
}

#[cfg(not(feature = "dense-core"))]
struct DomNode {
    parent: Option<NonZeroU32>,
    next_sibling: Option<NonZeroU32>,
    kind: NodeKind,
}

// ---------------------------------------------------------
// Unified API Implementation
// ---------------------------------------------------------

#[cfg(feature = "dense-core")]
impl Dom {
    pub fn new() -> Self {
        Self {
            parents: Vec::new(),
            next_siblings: Vec::new(),
            payload_keys: Vec::new(),
            element_payloads: Vec::new(),
            text_payloads: Vec::new(),
            void_payloads: Vec::new(),
        }
    }

    pub fn add_element(&mut self, tag: u8) -> NonZeroU32 {
        let dense_idx = self.element_payloads.len();
        if dense_idx >= 1073741824 {
            panic!("Maximum number of nodes reached");
        }

        self.element_payloads.push(ElementPayload {tag, first_child: None });

        let key_val = 0_u32 | ((dense_idx as u32) + 1);
        self.payload_keys.push(NonZeroU32::new(key_val));

        let node_idx = self.parents.len();
        if node_idx == usize::MAX {
            panic!("Maximum number of nodes reached");
        }
        self.parents.push(None);
        self.next_siblings.push(None);

        NonZeroU32::new((node_idx as u32) + 1).unwrap()
    }

    pub fn parent(&self, id: NonZeroU32) -> Option<NonZeroU32> {
        let idx = (id.get() - 1) as usize;
        self.parents[idx]
    }

    pub fn get_tag(&self, id: NonZeroU32) -> Option<u8> {
        let idx = (id.get() - 1) as usize;
        let key = self.payload_keys[idx]?.get();
        let type_bits = key >> 30;

        let dense_idx = (key & 0x3F_FF_FF_FF) as usize - 1;

        match type_bits {
            0 => Some(self.element_payloads[dense_idx].tag), // Element
            2 => Some(self.void_payloads[dense_idx]),  // EmptyLeaf
            _ => None, // Text nodes have no tag
        }
    }
}

#[cfg(not(feature = "dense-core"))]
impl Dom {
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
        }
    }

    pub fn add_element(&mut self, tag: u8) -> NonZeroU32 {
        let node_idx = self.nodes.len();

        if node_idx == usize::MAX {
            panic!("Maximum number of nodes reached");
        }

        self.nodes.push(DomNode {
            parent: None,
            next_sibling: None,
            kind: NodeKind::Element {
                tag,
                first_child: None,
            },
        });

        NonZeroU32::new((node_idx as u32) + 1).unwrap()
    }

    pub fn parent(&self, id: NonZeroU32) -> Option<NonZeroU32> {
        let idx = (id.get() - 1) as usize;
        self.nodes[idx].parent
    }

    pub fn get_tag(&self, id: NonZeroU32) -> Option<u8> {
        let idx = (id.get() - 1) as usize;
        match self.nodes[idx].kind {
            NodeKind::Element { tag, .. } => Some(tag),
            NodeKind::EmptyLeaf { tag } => Some(tag),
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
        let node_id = dom.add_element(div_tag);

        assert_eq!(dom.get_tag(node_id), Some(div_tag));
        assert_eq!(dom.parent(node_id), None);
    }

    #[test]
    fn test_nonzero_indexing_offset() {
        let mut dom = Dom::new();

        let first_id = dom.add_element(10);
        let second_id = dom.add_element(20);

        // Ensure IDs start at 1 and increment
        assert_eq!(first_id.get(), 1);
        assert_eq!(second_id.get(), 2);

        // Ensure correct tag resolution despite the +1 offset logic
        assert_eq!(dom.get_tag(first_id), Some(10));
        assert_eq!(dom.get_tag(second_id), Some(20));
    }
}
