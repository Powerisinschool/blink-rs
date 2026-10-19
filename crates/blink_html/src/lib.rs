use blink_dom::{Dom, get_tag_index};
use html5ever::interface::{ElementFlags, NodeOrText, QuirksMode, TreeSink};
use html5ever::tendril::StrTendril;
use html5ever::{Attribute, ExpandedName, QualName};
use std::borrow::Cow;
use std::cell::RefCell;
use std::num::NonZeroU32;

pub struct HtmlDom(pub RefCell<Dom>);

impl HtmlDom {
    pub fn new() -> Self {
        let mut dom = blink_dom::Dom::new();
        dom.add_element(0, None);
        Self(RefCell::new(dom))
    }
}

impl TreeSink for HtmlDom {
    type Handle = NonZeroU32;
    type Output = Self;
    type ElemName<'a> = ExpandedName<'a> where Self: 'a;

    fn finish(self) -> Self {
        self
    }

    fn parse_error(&self, _msg: Cow<'static, str>) {
        // todo!("will do later")
    }

    fn get_document(&self) -> Self::Handle {
        NonZeroU32::new(1).unwrap()
    }

    fn elem_name<'a>(&'a self, target: &'a Self::Handle) -> ExpandedName<'a> {
        let dom = self.0.borrow();
        let tag = dom.get_tag(*target).unwrap_or(0);
        let boxed_name = dom.tag_registry.get(&tag).expect("Tag should be registered");

        let ptr = &**boxed_name as *const QualName;

        unsafe {
            (*ptr).expanded()
        }
    }

    fn create_element(&self, name: QualName, attrs: Vec<Attribute>, _flags: ElementFlags) -> Self::Handle {
        let mut dom = self.0.borrow_mut();
        let mut tag = get_tag_index(&name.local);

        if tag == 0 {
            tag = (115 + dom.tag_registry.len()) as u8;
            dom.tag_registry.insert(tag, Box::new(name));
        }

        let attrs_payload = if attrs.is_empty() { None } else { Some(attrs.into_boxed_slice()) };
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

        let child_handle = match child {
            NodeOrText::AppendNode(node) => node,
            NodeOrText::AppendText(text) => {
                let text_id = text.len() as u32; // Replace with proper text id
                dom.add_text(text_id)
            }
        };

        dom.set_parent(child_handle, *parent);

        if let Some(first) = dom.get_first_child(*parent) {
            let mut curr = first;
            while let Some(next) = dom.get_next_sibling(curr) {
                curr = next;
            }
            dom.set_next_sibling(curr, Some(child_handle));
        } else {
            dom.set_first_child(*parent, Some(child_handle));
        }
    }

    fn append_based_on_parent_node(&self, element: &Self::Handle, _prev_element: &Self::Handle, child: NodeOrText<Self::Handle>) {
        self.append(element, child)
    }

    fn append_doctype_to_document(&self, _name: StrTendril, _public_id: StrTendril, _system_id: StrTendril) {
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

    fn append_before_sibling(&self, _sibling: &Self::Handle, _new_node: NodeOrText<Self::Handle>) {
        // todo!()
    }

    fn add_attrs_if_missing(&self, _target: &Self::Handle, _attrs: Vec<Attribute>) {
        // todo!()
    }

    fn remove_from_parent(&self, target: &Self::Handle) {
        let mut dom = self.0.borrow_mut();

        if let Some(p) = dom.get_parent(*target) {
            if let Some(first) = dom.get_first_child(p) {
                if first == *target {
                    let next = dom.get_next_sibling(*target);
                    if let Some(n) = next {
                        dom.set_first_child(p, Some(n));
                    }
                } else {
                    let mut curr = first;
                    while let Some(next) = dom.get_next_sibling(curr) {
                        if next == *target {
                            dom.set_next_sibling(curr, self.0.borrow().get_next_sibling(*target));
                            break;
                        }
                        curr = next;
                    }
                }
            }
        }
        dom.set_next_sibling(*target, None);
    }

    fn reparent_children(&self, node: &Self::Handle, new_parent: &Self::Handle) {
        let mut dom = self.0.borrow_mut();

        if let Some(first) = dom.get_first_child(*node) {
            let mut curr = first;
            dom.set_parent(curr, *new_parent);
            while let Some(next) = dom.get_next_sibling(curr) {
                curr = next;
                dom.set_parent(curr, *new_parent);
            }

            // Append the whole chain to the end of the new parent's children
            if let Some(np_first) = dom.get_first_child(*new_parent) {
                let mut np_curr = np_first;
                while let Some(np_next) = dom.get_next_sibling(np_curr) {
                    np_curr = np_next;
                }
                dom.set_next_sibling(np_curr, Some(first));
            } else {
                dom.set_first_child(*new_parent, Some(first));
            }
        }
        dom.set_first_child(*node, None); // Clear out old child pointer
    }
}

pub fn add(left: u64, right: u64) -> u64 {
    left + right
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}
