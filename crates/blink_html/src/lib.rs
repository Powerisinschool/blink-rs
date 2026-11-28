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
        let doc_root = dom.add_element(0, None);
        dom.set_virtual_root(doc_root)
            .expect("Failed to set virtual root");
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
        self.0
            .borrow()
            .root_node_id()
            .expect("Document should have a root node")
    }

    fn elem_name<'a>(&'a self, target: &'a Self::Handle) -> ExpandedName<'a> {
        let dom = self.0.borrow();
        let tag = dom.get_tag(*target).unwrap_or(0);
        let name: &QualName = dom.tag_qual_name(tag).expect("Tag should be registered");

        let ptr: *const QualName = name;

        // SAFETY: `ExpandedName<'a>` must borrow for `'a` (the borrow of `self`),
        // but `name` is only borrowed for as long as the `RefCell` guard `dom`
        // lives. Extending it is sound because `name` points into a
        // `Box<QualName>` owned by `Dom::tag_registry`, and that registry is
        // insert-only and private to `blink_dom`: an entry is never dropped,
        // replaced or moved (growing the map moves the `Box` pointers, not the
        // heap allocations they point to) while the `Dom` is alive. The `Dom`
        // lives inside `self`, so it outlives `'a`.
        //
        // This breaks if anything replaces or drops the `Dom` while an
        // `ExpandedName` is live. `HtmlDom`'s field is public, so callers must
        // not do that during a parse. The test
        // `elem_name_stays_valid_while_registry_grows` exercises the growth case.
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
            tag = dom.intern_custom_tag(name);
        }

        let attrs_payload = if attrs.is_empty() {
            None
        } else {
            Some(attrs.into_boxed_slice())
        };

        dom.add_element(tag, attrs_payload)
    }

    fn create_comment(&self, text: StrTendril) -> Self::Handle {
        self.0.borrow_mut().add_comment(text.to_string().as_str())
    }

    fn create_pi(&self, target: StrTendril, data: StrTendril) -> Self::Handle {
        self.0
            .borrow_mut()
            .add_pi(target.to_string().as_str(), data.to_string().as_str())
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
                    && dom
                        .get_node(last_child_id)
                        .is_some_and(|node| node.is_text())
                {
                    // append to the last text node instead of creating a new one
                    dom.append_text(last_child_id, text.to_string().as_str())
                        .expect("Failed to append text to last text node");
                    return;
                }
                dom.add_text(text.to_string().as_str())
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
        let (new_handle, text) = match new_node {
            NodeOrText::AppendNode(node) => {
                self.remove_from_parent(&node);
                (Some(node), None)
            }
            NodeOrText::AppendText(text) => (None, Some(text)),
        };

        let mut dom = self.0.borrow_mut();

        let parent_opt = dom.get_parent(*sibling);
        if let Some(parent) = parent_opt {
            let mut handle = None;
            if let Some(first) = dom.get_first_child(parent) {
                if first == *sibling {
                    let new_handle = if let Some(new_handle) = new_handle {
                        new_handle
                    } else {
                        dom.add_text(text.unwrap().to_string().as_str())
                    };
                    dom.set_next_sibling(new_handle, Some(*sibling))
                        .expect("Failed to set next sibling for new node");
                    dom.set_first_child(parent, Some(new_handle))
                        .expect("Failed to set first child for parent node");
                    handle = Some(new_handle);
                } else {
                    let mut curr = first;
                    while let Some(next) = dom.get_next_sibling(curr) {
                        if next == *sibling {
                            if dom.get_node(curr).is_some_and(|node| node.is_text())
                                && let Some(text) = text
                            {
                                dom.append_text(curr, text.to_string().as_str())
                                    .expect("Failed to append text to previous text node");
                                handle = None; // No new node was created, so we don't need to set the parent or next sibling
                            } else {
                                let new_handle = if let Some(new_handle) = new_handle {
                                    new_handle
                                } else {
                                    dom.add_text(text.unwrap().to_string().as_str())
                                };
                                dom.set_next_sibling(new_handle, Some(*sibling))
                                    .expect("Failed to set next sibling for new node");
                                dom.set_next_sibling(curr, Some(new_handle))
                                    .expect("Failed to set next sibling for current node");
                                handle = Some(new_handle);
                            }
                            break;
                        }
                        curr = next;
                    }
                }
            }

            if let Some(new_handle) = handle {
                dom.set_parent(new_handle, parent)
                    .expect("Failed to set parent for new node");
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
                dom.set_first_child(p, next)
                    .expect("Failed to set first child for parent node");
            } else {
                let mut curr = first;
                while let Some(next) = dom.get_next_sibling(curr) {
                    if next == *target {
                        let target_next = dom.get_next_sibling(*target);
                        dom.set_next_sibling(curr, target_next)
                            .expect("Failed to set next sibling for current node");
                        break;
                    }
                    curr = next;
                }
            }
        }
        dom.clear_parent(*target)
            .expect("Failed to clear parent for target node");
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
    fn test_merge_adjacent_text_nodes_indirect() {
        let html = r#"
        <div>
            Hello,
            <span>world!</span>
            How are you?
        </div>"#;
        let dom = parse_html(html);

        // Check that the root node has one child (the div)
        let document = dom
            .root_node_id()
            .expect("Document should have a root node");
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

    #[test]
    fn test_merge_adjacent_text_nodes_direct() {
        let html_dom = HtmlDom::new();
        let document = html_dom.get_document();

        let div = html_dom
            .0
            .borrow_mut()
            .add_element(get_tag_index("div"), None);

        html_dom.append(&document, NodeOrText::AppendNode(div));

        html_dom.append(&div, NodeOrText::AppendText("Hello, ".into()));
        html_dom.append(&div, NodeOrText::AppendText("World".into()));

        let dom = html_dom.0.borrow();

        assert_eq!(dom.get_tag(div).unwrap(), get_tag_index("div"));

        // Check that the div has three children: text, span, text
        let first_child = dom
            .get_first_child(div)
            .expect("Div should have a first child");

        assert!(dom.get_tag(first_child).is_none(), "Expected a text node"); // Text node

        assert_eq!(dom.get_text(first_child).unwrap(), "Hello, World");

        let impossible_second_child = dom.get_next_sibling(first_child);
        assert!(
            impossible_second_child.is_none(),
            "Div should not have a second child"
        );
    }

    #[test]
    fn test_append_before_sibling_moves_node_between_parents() {
        let html_dom = HtmlDom::new();
        let document = html_dom.get_document();

        let source = html_dom
            .0
            .borrow_mut()
            .add_element(get_tag_index("div"), None);
        let destination = html_dom
            .0
            .borrow_mut()
            .add_element(get_tag_index("div"), None);

        let source_first = html_dom
            .0
            .borrow_mut()
            .add_element(get_tag_index("span"), None);
        let moving = html_dom
            .0
            .borrow_mut()
            .add_element(get_tag_index("span"), None);
        let source_last = html_dom
            .0
            .borrow_mut()
            .add_element(get_tag_index("span"), None);

        let destination_first = html_dom
            .0
            .borrow_mut()
            .add_element(get_tag_index("span"), None);
        let target = html_dom
            .0
            .borrow_mut()
            .add_element(get_tag_index("span"), None);
        let destination_last = html_dom
            .0
            .borrow_mut()
            .add_element(get_tag_index("span"), None);

        html_dom.append(&document, NodeOrText::AppendNode(source));
        html_dom.append(&document, NodeOrText::AppendNode(destination));

        // Initial source child chain: source_first -> moving -> source_last.
        html_dom.append(&source, NodeOrText::AppendNode(source_first));
        html_dom.append(&source, NodeOrText::AppendNode(moving));
        html_dom.append(&source, NodeOrText::AppendNode(source_last));

        // Initial destination child chain: destination_first -> target -> destination_last.
        html_dom.append(&destination, NodeOrText::AppendNode(destination_first));
        html_dom.append(&destination, NodeOrText::AppendNode(target));
        html_dom.append(&destination, NodeOrText::AppendNode(destination_last));

        html_dom.append_before_sibling(&target, NodeOrText::AppendNode(moving));

        let dom = html_dom.0.borrow();

        // Verify the source chain no longer contains moving.
        assert_eq!(dom.get_first_child(source), Some(source_first));
        assert_eq!(dom.get_next_sibling(source_first), Some(source_last));
        assert_eq!(dom.get_next_sibling(source_last), None);

        // Verify the destination chain is destination_first -> moving -> target -> destination_last.
        assert_eq!(dom.get_first_child(destination), Some(destination_first));
        assert_eq!(dom.get_next_sibling(destination_first), Some(moving));
        assert_eq!(dom.get_next_sibling(moving), Some(target));
        assert_eq!(dom.get_next_sibling(target), Some(destination_last));
        assert_eq!(dom.get_next_sibling(destination_last), None);

        // Verify the parent pointers are consistent with the child chains.
        assert_eq!(dom.get_parent(source_first), Some(source));
        assert_eq!(dom.get_parent(source_last), Some(source));

        assert_eq!(dom.get_parent(destination_first), Some(destination));
        assert_eq!(dom.get_parent(moving), Some(destination));
        assert_eq!(dom.get_parent(target), Some(destination));
        assert_eq!(dom.get_parent(destination_last), Some(destination));
    }

    /// `<body>`'s element children, in order.
    fn body_children(dom: &Dom) -> Vec<DomNodeId> {
        let document = dom.root_node_id().expect("document root");
        let html = dom.get_first_child(document).expect("html");
        let head = dom.get_first_child(html).expect("head");
        let body = dom.get_next_sibling(head).expect("body");

        let mut out = Vec::new();
        let mut next = dom.get_first_child(body);
        while let Some(id) = next {
            out.push(id);
            next = dom.get_next_sibling(id);
        }
        out
    }

    #[test]
    fn custom_elements_keep_their_names() {
        let dom = parse_html("<x-a></x-a><x-b></x-b><x-a></x-a>");
        let kids = body_children(&dom);

        assert_eq!(kids.len(), 3);
        assert_eq!(dom.node_tag_name(kids[0]), Some("x-a"));
        assert_eq!(dom.node_tag_name(kids[1]), Some("x-b"));
        assert_eq!(dom.node_tag_name(kids[2]), Some("x-a"));
        assert_eq!(dom.get_tag(kids[0]), dom.get_tag(kids[2]));
        assert_ne!(dom.get_tag(kids[0]), dom.get_tag(kids[1]));
    }

    #[test]
    fn many_repeated_custom_elements_do_not_wrap_the_tag_index() {
        // Before interning, every custom element took a fresh index, so a few
        // hundred of them wrapped `u8` and collided with built-in tags.
        let html = format!("{}<div></div>", "<x-repeat></x-repeat>".repeat(500));
        let dom = parse_html(&html);
        let kids = body_children(&dom);

        assert_eq!(kids.len(), 501);
        let first = dom.get_tag(kids[0]);
        for &id in &kids[..500] {
            assert_eq!(dom.get_tag(id), first);
            assert_eq!(dom.node_tag_name(id), Some("x-repeat"));
        }
        assert_eq!(dom.get_tag(kids[500]), Some(get_tag_index("div")));
    }

    #[test]
    fn elem_name_stays_valid_while_registry_grows() {
        // Parsing makes html5ever call `elem_name` throughout while new custom
        // names keep being registered (forcing the registry map to grow).
        let mut html = String::new();
        for i in 0..100 {
            html.push_str(&format!("<x-n{i}><x-n{i}></x-n{i}></x-n{i}>"));
        }
        let dom = parse_html(&html);
        let kids = body_children(&dom);

        assert_eq!(kids.len(), 100);
        for (i, &id) in kids.iter().enumerate() {
            let expected = format!("x-n{i}");
            assert_eq!(dom.node_tag_name(id), Some(expected.as_str()));
        }
    }
}
