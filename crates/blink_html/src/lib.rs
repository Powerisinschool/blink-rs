use blink_dom::{Dom, DomNodeId, get_tag_index};
use html5ever::interface::{ElementFlags, NodeOrText, QuirksMode, TreeSink};
use html5ever::tendril::{StrTendril, TendrilSink};
use html5ever::{Attribute, ExpandedName, ParseOpts, QualName};
use std::borrow::Cow;
use std::cell::RefCell;

pub struct HtmlDom(pub RefCell<Dom>);

impl HtmlDom {
    pub fn new() -> Self {
        let mut dom = Dom::new();
        dom.add_element(0, None);
        Self(RefCell::new(dom))
    }
}

impl Default for HtmlDom {
    fn default() -> Self {
        Self::new()
    }
}

impl TreeSink for HtmlDom {
    type Handle = DomNodeId;
    type Output = Self;
    type ElemName<'a>
        = ExpandedName<'a>
    where
        Self: 'a;

    fn finish(self) -> Self {
        self
    }

    fn parse_error(&self, _msg: Cow<'static, str>) {
        // todo!("will do later")
    }

    fn get_document(&self) -> Self::Handle {
        self.0.borrow().get_root_node_id()
    }

    fn elem_name<'a>(&'a self, target: &'a Self::Handle) -> ExpandedName<'a> {
        let dom = self.0.borrow();
        let tag = dom.get_tag(*target).unwrap_or(0);
        let boxed_name = dom
            .tag_registry
            .get(&tag)
            .expect("Tag should be registered");

        let ptr = &**boxed_name as *const QualName;

        unsafe { (*ptr).expanded() }
    }

    fn create_element(
        &self,
        name: QualName,
        attrs: Vec<Attribute>,
        _flags: ElementFlags,
    ) -> Self::Handle {
        let mut dom = self.0.borrow_mut();
        let mut tag = get_tag_index(&name.local);

        if tag == 0 {
            tag = (115 + dom.tag_registry.len()) as u8;
            dom.tag_registry.insert(tag, Box::new(name));
        }

        let attrs_payload = if attrs.is_empty() {
            None
        } else {
            Some(attrs.into_boxed_slice())
        };
        dom.add_element(tag, attrs_payload)
    }

    fn create_comment(&self, _text: StrTendril) -> Self::Handle {
        self.0.borrow_mut().add_text(0)
    }

    fn create_pi(&self, _target: StrTendril, _data: StrTendril) -> Self::Handle {
        self.0.borrow_mut().add_text(0)
    }

    fn append(&self, parent: &Self::Handle, child: NodeOrText<Self::Handle>) {
        let mut dom = self.0.borrow_mut();

        let mut last_child = None;
        if let Some(first) = dom.get_first_child(*parent) {
            let mut curr = first;
            while let Some(next) = dom.get_next_sibling(curr) {
                curr = next;
            }
            last_child = Some(curr);
        }

        let child_handle = match child {
            NodeOrText::AppendNode(node) => node,
            NodeOrText::AppendText(text) => {
                if let Some(last_child_id) = last_child
                    && let Some(last_child) = dom.get_node(last_child_id)
                    && last_child.kind.is_text()
                {
                    // append to the last text node instead of creating a new one
                    last_child_id
                } else {
                    let text_id = text.len() as u32; // Replace with proper text id
                    dom.add_text(text_id)
                }
            }
        };
        dom.set_parent(child_handle, *parent)
            .expect("Failed to set parent for child node");

        if let Some(last_child) = last_child {
            if last_child != child_handle {
                dom.set_next_sibling(last_child, Some(child_handle))
                    .expect("Failed to set next sibling for child node");
            }
        } else {
            dom.set_first_child(*parent, Some(child_handle))
                .expect("Failed to set first child for parent node");
        }
    }

    fn append_based_on_parent_node(
        &self,
        element: &Self::Handle,
        _prev_element: &Self::Handle,
        child: NodeOrText<Self::Handle>,
    ) {
        self.append(element, child)
    }

    fn append_doctype_to_document(
        &self,
        _name: StrTendril,
        _public_id: StrTendril,
        _system_id: StrTendril,
    ) {
        // todo!()
    }

    fn get_template_contents(&self, _target: &Self::Handle) -> Self::Handle {
        todo!()
    }

    fn same_node(&self, x: &Self::Handle, y: &Self::Handle) -> bool {
        // todo!()
        x == y
    }

    fn set_quirks_mode(&self, _mode: QuirksMode) {
        // todo!()
    }

    fn append_before_sibling(&self, sibling: &Self::Handle, new_node: NodeOrText<Self::Handle>) {
        let mut dom = self.0.borrow_mut();

        let new_handle = match new_node {
            NodeOrText::AppendNode(node) => node,
            NodeOrText::AppendText(text) => {
                let text_id = text.len() as u32;
                dom.add_text(text_id)
            }
        };

        let parent_opt = dom.get_parent(*sibling);
        if let Some(parent) = parent_opt {
            dom.set_parent(new_handle, parent)
                .expect("Failed to set parent for new node");

            if let Some(first) = dom.get_first_child(parent) {
                if first == *sibling {
                    dom.set_next_sibling(new_handle, Some(*sibling))
                        .expect("Failed to set next sibling for new node");
                    dom.set_first_child(parent, Some(new_handle))
                        .expect("Failed to set first child for parent node");
                } else {
                    let mut curr = first;
                    while let Some(next) = dom.get_next_sibling(curr) {
                        if next == *sibling {
                            dom.set_next_sibling(new_handle, Some(*sibling))
                                .expect("Failed to set next sibling for new node");
                            dom.set_next_sibling(curr, Some(new_handle))
                                .expect("Failed to set next sibling for current node");
                            break;
                        }
                        curr = next;
                    }
                }
            }
        }
    }

    fn add_attrs_if_missing(&self, _target: &Self::Handle, _attrs: Vec<Attribute>) {
        // todo!()
    }

    fn remove_from_parent(&self, target: &Self::Handle) {
        let mut dom = self.0.borrow_mut();

        if let Some(p) = dom.get_parent(*target)
            && let Some(first) = dom.get_first_child(p)
        {
            if first == *target {
                let next = dom.get_next_sibling(*target);
                // if let Some(n) = next {
                //     dom.set_first_child(p, Some(n));
                // }
                dom.set_first_child(p, next)
                    .expect("Failed to set first child for parent node");
            } else {
                let mut curr = first;
                while let Some(next) = dom.get_next_sibling(curr) {
                    if next == *target {
                        dom.set_next_sibling(curr, self.0.borrow().get_next_sibling(*target))
                            .expect("Failed to set next sibling for current node");
                        break;
                    }
                    curr = next;
                }
            }
        }
        dom.set_next_sibling(*target, None)
            .expect("Failed to clear next sibling for target node");
    }

    fn reparent_children(&self, node: &Self::Handle, new_parent: &Self::Handle) {
        let mut dom = self.0.borrow_mut();

        if let Some(first) = dom.get_first_child(*node) {
            let mut curr = first;
            dom.set_parent(curr, *new_parent)
                .expect("Failed to set parent for child node");
            while let Some(next) = dom.get_next_sibling(curr) {
                curr = next;
                dom.set_parent(curr, *new_parent)
                    .expect("Failed to set parent for child node");
            }

            // Append the whole chain to the end of the new parent's children
            if let Some(np_first) = dom.get_first_child(*new_parent) {
                let mut np_curr = np_first;
                while let Some(np_next) = dom.get_next_sibling(np_curr) {
                    np_curr = np_next;
                }
                dom.set_next_sibling(np_curr, Some(first))
                    .expect("Failed to set next sibling for child node");
            } else {
                dom.set_first_child(*new_parent, Some(first))
                    .expect("Failed to set first child for parent node");
            }
        }
        dom.set_first_child(*node, None)
            .expect("Failed to clear old child pointer"); // Clear out old child pointer
    }
}

pub fn parse_html(html: &str) -> Dom {
    html5ever::parse_document(HtmlDom::new(), ParseOpts::default())
        .from_utf8()
        .one(html.as_bytes())
        .0
        .into_inner()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_merge_adjacent_text_nodes() {
        let html = r#"
        <div>
            Hello,
            <span>world!</span>
            How are you?
        </div>"#;
        let dom = parse_html(html);

        // Check that the root node has one child (the div)
        let document = dom.get_root_node_id();
        let html_id = dom
            .get_first_child(document)
            .expect("Document should have a child (html)");
        let head_id = dom
            .get_first_child(html_id)
            .expect("html should have a child (head)");
        let body_id = dom
            .get_next_sibling(head_id)
            .expect("html should have a second child (body)");

        let div_id = dom
            .get_first_child(body_id)
            .expect("Body should have a child");

        assert_eq!(dom.get_tag(div_id).unwrap(), get_tag_index("div"));

        // Check that the div has three children: text, span, text
        let first_child = dom
            .get_first_child(div_id)
            .expect("Div should have a first child");
        assert!(dom.get_tag(first_child).is_none()); // Text node

        let second_child = dom
            .get_next_sibling(first_child)
            .expect("Div should have a second child");
        assert!(dom.get_tag(second_child).is_some());
        assert_eq!(dom.get_tag(second_child).unwrap(), get_tag_index("span"));

        let third_child = dom
            .get_next_sibling(second_child)
            .expect("Div should have a third child");
        assert!(dom.get_tag(third_child).is_none()); // Text node

        let impossible_fourth_child = dom.get_next_sibling(third_child);
        assert!(
            impossible_fourth_child.is_none(),
            "Div should not have a fourth child"
        );
    }
}
