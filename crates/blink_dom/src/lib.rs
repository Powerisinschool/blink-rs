use markup5ever::{ns, Attribute, QualName};
use std::collections::HashMap;
use std::num::NonZeroU32;

const HTML_TAGS: &[&str] = &[
    "a", "abbr", "address", "area", "article", "aside", "audio", "b", "base", "bdi",
    "bdo", "blockquote", "body", "br", "button", "canvas", "caption", "cite", "code",
    "col", "colgroup", "data", "datalist", "dd", "del", "details", "dfn", "dialog",
    "div", "dl", "dt", "em", "embed", "fieldset", "figcaption", "figure", "footer",
    "form", "h1", "h2", "h3", "h4", "h5", "h6", "head", "header", "hgroup", "hr",
    "html", "i", "iframe", "img", "input", "ins", "kbd", "label", "legend", "li",
    "link", "main", "map", "mark", "menu", "meta", "meter", "nav", "noscript",
    "object", "ol", "optgroup", "option", "output", "p", "picture", "pre",
    "progress", "q", "rp", "rt", "ruby", "s", "samp", "script", "section", "select",
    "slot", "small", "source", "span", "strong", "style", "sub", "summary", "sup",
    "table", "tbody", "td", "template", "textarea", "tfoot", "th", "thead", "time",
    "title", "tr", "track", "u", "ul", "var", "video", "wbr"
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
        return "div";
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
    pub node_attributes: Vec<Option<Box<[Attribute]>>>,
    pub tag_registry: HashMap<u8, Box<QualName>>,
}

#[cfg(feature = "dense-core")]
pub enum NodeKind {
    Element {
        tag: u8, // Easily fits the indices for the ~150 HTML tags
        first_child: Option<NonZeroU32>
    },
    Text {
        // text_id: u32
    },
    EmptyLeaf {
        tag: u8, // Easily fits the indices for the ~150 HTML tags
    },
}

// Memory-safe, arena-allocated DOM
#[cfg(not(feature = "dense-core"))]
pub struct Dom {
    nodes: Vec<DomNode>,
    pub tag_registry: HashMap<u8, Box<QualName>>,
}

#[cfg(not(feature = "dense-core"))]
pub enum NodeKind {
    Element {
        tag: u8, // Easily fits the indices for the ~150 HTML tags
        first_child: Option<NonZeroU32>,
        attrs: Option<Box<[Attribute]>>,
    },
    Text {
        // text_id: u32
    },
    EmptyLeaf {
        tag: u8, // Easily fits the indices for the ~150 HTML tags
    },
}

pub struct DomNode {
    parent: Option<NonZeroU32>,
    next_sibling: Option<NonZeroU32>,
    pub kind: NodeKind,
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

    pub fn add_text(&mut self, _text_id: u32) -> NonZeroU32 {
        let node_idx = self.nodes.len();
        self.nodes.push(DomNode {
            parent: None,
            next_sibling: None,
            kind: NodeKind::Text {
                // text_id
            },
        });
        NonZeroU32::new((node_idx as u32) + 1).unwrap()
    }

    pub fn set_parent(&mut self, node: NonZeroU32, parent: NonZeroU32) {
        let idx = (node.get() - 1) as usize;
        self.nodes[idx].parent = Some(parent);
    }

    pub fn set_first_child(&mut self, parent: NonZeroU32, child: Option<NonZeroU32>) {
        let idx = (parent.get() - 1) as usize;
        if let NodeKind::Element { ref mut first_child, .. } = self.nodes[idx].kind {
            *first_child = child;
        }
    }

    pub fn set_sibling(&mut self, node: NonZeroU32, sibling: Option<NonZeroU32>) {
        let idx = (node.get() - 1) as usize;
        self.nodes[idx].next_sibling = sibling;
    }

    pub fn get_parent(&self, id: NonZeroU32) -> Option<NonZeroU32> {
        let idx = (id.get() - 1) as usize;
        self.nodes[idx].parent
    }

    pub fn get_first_child(&self, parent: NonZeroU32) -> Option<NonZeroU32> {
        let idx = (parent.get() - 1) as usize;
        if let NodeKind::Element { first_child, .. } = self.nodes[idx].kind {
            first_child
        } else {
            None
        }
    }

    pub fn get_sibling(&self, node: NonZeroU32) -> Option<NonZeroU32> {
        let idx = (node.get() - 1) as usize;
        self.nodes[idx].next_sibling
    }

    pub fn get_tag(&self, id: NonZeroU32) -> Option<u8> {
        let idx = (id.get() - 1) as usize;
        match self.nodes[idx].kind {
            NodeKind::Element { tag, .. } => Some(tag),
            NodeKind::EmptyLeaf { tag } => Some(tag),
            _ => None,
        }
    }

    pub fn get_nodes_len(&self) -> usize {
        self.nodes.len()
    }
    
    pub fn get_node(&self, id: NonZeroU32) -> Option<&DomNode> {
        let idx = (id.get() - 1) as usize;
        self.nodes.get(idx)
    }

    pub fn ensure_tag_registered(&mut self, tag: u8) {
        self.tag_registry.entry(tag).or_insert_with(|| {
            let name_str = get_tag_name(tag);

            Box::new(QualName::new(
                None,
                ns!(html),
                markup5ever::LocalName::from(name_str)
            ))
        });
    }
}

#[cfg(feature = "dense-core")]
impl Dom {
    pub fn new() -> Self {
        Self {
            nodes: Vec::with_capacity(512),
            node_attributes: Vec::with_capacity(512),
            tag_registry: HashMap::new(),
        }
    }

    pub fn add_element(&mut self, tag: u8, attrs: Option<Box<[Attribute]>>) -> NonZeroU32 {
        self.ensure_tag_registered(tag);
        let node_idx = self.nodes.len();

        if node_idx == usize::MAX {
            panic!("Maximum number of nodes reached");
        }

        match get_tag_name(tag) {
            "br" | "hr" => self.nodes.push(DomNode {
                parent: None,
                next_sibling: None,
                kind: NodeKind::EmptyLeaf {
                    tag,
                },
            }),
            _ => {
                self.nodes.push(DomNode {
                    parent: None,
                    next_sibling: None,
                    kind: NodeKind::Element {
                        tag,
                        first_child: None,
                    },
                });
                self.node_attributes.push(attrs);
            },
        }

        NonZeroU32::new((node_idx as u32) + 1).unwrap()
    }

    pub fn set_attributes(&mut self, index: NonZeroU32, attrs: Option<Box<[Attribute]>>) {
        self.node_attributes[(index.get() - 1) as usize] = attrs;
    }

    pub fn get_attributes(&self, index: NonZeroU32) -> Option<&[Attribute]> {
        let attrs = self.node_attributes.get((index.get() - 1) as usize).map(|attrs| attrs.as_ref())??;
        Some(attrs.as_ref())
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

    pub fn add_element(&mut self, tag: u8, attrs: Option<Box<[Attribute]>>) -> NonZeroU32 {
        self.ensure_tag_registered(tag);
        let node_idx = self.nodes.len();

        if node_idx == usize::MAX {
            panic!("Maximum number of nodes reached");
        }

        match get_tag_name(tag) {
            "br" | "hr" => self.nodes.push(DomNode {
                parent: None,
                next_sibling: None,
                kind: NodeKind::EmptyLeaf {
                    tag,
                },
            }),
            _ => {
                self.nodes.push(DomNode {
                    parent: None,
                    next_sibling: None,
                    kind: NodeKind::Element {
                        tag,
                        first_child: None,
                        attrs
                    },
                });
            },
        }

        NonZeroU32::new((node_idx as u32) + 1).unwrap()
    }

    pub fn set_attributes(&mut self, index: NonZeroU32, _attrs: Option<Box<[Attribute]>>) {
        let node_kind = &mut self.nodes[(index.get() - 1) as usize].kind;

        match node_kind {
            NodeKind::Element { attrs, .. } => {
                *attrs = _attrs;
            },
            _ => {},
        }
    }

    pub fn get_attributes(&self, index: NonZeroU32) -> Option<&[Attribute]> {
        let node = self.nodes.get((index.get() - 1) as usize)?;

        match &node.kind {
            NodeKind::Element { attrs, .. } => {
                attrs.as_deref()
            },
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
        assert_eq!(first_id.get(), 1);
        assert_eq!(second_id.get(), 2);

        // Ensure correct tag resolution despite the +1 offset logic
        assert_eq!(dom.get_tag(first_id), Some(10));
        assert_eq!(dom.get_tag(second_id), Some(20));
    }
}
