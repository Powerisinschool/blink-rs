use crate::engine::LayoutContext;
use crate::style::{DisplayInside, DisplayOutside, compute_style};
use blink_dom::DomNodeId;

#[derive(Debug, PartialEq, Clone)]
pub enum InlineItemKind {
    Text(String),
    AtomicInline,
    ForcedBreak,
}

#[derive(Debug, PartialEq, Clone)]
pub struct InlineItem {
    pub dom_node: DomNodeId,
    pub kind: InlineItemKind,
}

impl InlineItem {
    pub fn is_text(&self) -> bool {
        matches!(self.kind, InlineItemKind::Text(_))
    }

    pub fn is_atomic_inline(&self) -> bool {
        matches!(self.kind, InlineItemKind::AtomicInline)
    }

    pub fn is_forced_break(&self) -> bool {
        matches!(self.kind, InlineItemKind::ForcedBreak)
    }

    pub fn get_text(&self) -> Option<&str> {
        if let InlineItemKind::Text(ref text) = self.kind {
            Some(text)
        } else {
            None
        }
    }

    pub fn get_text_mut(&mut self) -> Option<&mut String> {
        if let InlineItemKind::Text(ref mut text) = self.kind {
            Some(text)
        } else {
            None
        }
    }

    pub fn append_text(&mut self, additional_text: &str) {
        if let InlineItemKind::Text(ref mut text) = self.kind {
            text.push_str(additional_text);
        }
    }
}

pub fn collect_inline_items(run: &[DomNodeId], ctx: &LayoutContext) -> Vec<InlineItem> {
    let mut items = Vec::new();
    let mut last_was_space = true;

    for &node_id in run {
        collect_recursive(node_id, ctx, &mut items, &mut last_was_space);
    }

    strip_trailing_space(&mut items);
    items
}

pub fn collect_recursive(
    node_id: DomNodeId,
    ctx: &LayoutContext,
    items: &mut Vec<InlineItem>,
    last_was_space: &mut bool,
) {
    let node = ctx
        .dom
        .get_node(node_id)
        .expect("Node should exist in the DOM");

    if node.is_text() {
        if let Some(text) = ctx.dom.get_text(node_id) {
            process_text(&text, node_id, items, last_was_space);
        }
        return; // Text nodes are leaf nodes, so we can return early
    }

    let style = compute_style(node_id, ctx.dom, ctx.stylesheet);

    if style.display.is_none() {
        return;
    }

    if node.get_tag_name().is_some_and(|tag| tag == "br") {
        strip_trailing_space(items);
        items.push(InlineItem {
            dom_node: node_id,
            kind: InlineItemKind::ForcedBreak,
        });
        *last_was_space = true; // A line break is treated as a space for the next text node
        return;
    }

    match (style.display.outside(), style.display.inside()) {
        (Some(DisplayOutside::Inline), Some(DisplayInside::Flow)) => {
            let mut current = ctx.dom.get_first_child(node_id);
            while let Some(child_id) = current {
                collect_recursive(child_id, ctx, items, last_was_space);
                current = ctx.dom.get_next_sibling(child_id);
            }
        }
        (Some(DisplayOutside::Inline), _) => {
            items.push(InlineItem {
                dom_node: node_id,
                kind: InlineItemKind::AtomicInline,
            });
            *last_was_space = false;
        }
        _ => {
            items.push(InlineItem {
                dom_node: node_id,
                kind: InlineItemKind::AtomicInline,
            });
            *last_was_space = false;
        }
    }
}

pub fn process_text(
    text: &str,
    dom_node: DomNodeId,
    items: &mut Vec<InlineItem>,
    last_was_space: &mut bool,
) {
    let mut collapsed = String::new();

    for c in text.chars() {
        if c == ' ' || c == '\n' || c == '\t' {
            if !*last_was_space {
                collapsed.push(' ');
                *last_was_space = true;
            }
        } else {
            collapsed.push(c);
            *last_was_space = false;
        }
    }

    if !collapsed.is_empty() {
        items.push(InlineItem {
            dom_node,
            kind: InlineItemKind::Text(collapsed),
        })
    }
}

pub fn strip_trailing_space(items: &mut Vec<InlineItem>) -> bool {
    let mut removed_space = false;

    if let Some(last_item) = items.last_mut()
        && let Some(ref mut text) = last_item.get_text_mut()
        && text.ends_with(' ')
    {
        text.pop();
        removed_space = true;
    }

    if let Some(last_item) = items.last()
        && let Some(text) = last_item.get_text()
        && text.is_empty()
    {
        items.pop();
    }

    removed_space
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::text::FixedWidth;
    use blink_css::CSSStyleSheet;
    use blink_dom::{Dom, get_tag_index};

    // Helper to quickly check the output sequence
    fn get_text_strings(items: &[InlineItem]) -> Vec<String> {
        let mut result = Vec::new();
        for item in items {
            match &item.kind {
                InlineItemKind::Text(t) => result.push(t.clone()),
                InlineItemKind::ForcedBreak => result.push("<br>".to_string()),
                InlineItemKind::AtomicInline => result.push("<atomic>".to_string()),
            }
        }
        result
    }

    #[test]
    fn test_collapse_basic_whitespace() {
        let mut dom = Dom::new();
        let text_id = dom.add_text("  Hello \n   world  ");

        let ctx = LayoutContext {
            dom: &dom,
            stylesheet: &CSSStyleSheet::new(),
            text_measurer: Box::new(FixedWidth {}),
        };

        let items = collect_inline_items(&[text_id], &ctx);

        // Leading and trailing space dropped, internal whitespace flattened
        assert_eq!(get_text_strings(&items), vec!["Hello world"]);
    }

    #[test]
    fn test_adjacent_text_nodes() {
        let mut dom = Dom::new();
        let t1 = dom.add_text("Hello ");
        let t2 = dom.add_text(" world"); // Leading space here should collapse

        let ctx = LayoutContext {
            dom: &dom,
            stylesheet: &CSSStyleSheet::new(),
            text_measurer: Box::new(FixedWidth {}),
        };

        let items = collect_inline_items(&[t1, t2], &ctx);

        // State machine keeps `last_was_space=true` across node boundaries
        assert_eq!(get_text_strings(&items), vec!["Hello ", "world"]);
    }

    #[test]
    fn test_span_element_boundaries() {
        let mut dom = Dom::new();
        let parent = dom.add_element(get_tag_index("span"), None);
        let t1 = dom.add_text("Hello ");
        let span = dom.add_element(get_tag_index("span"), None);
        let t2 = dom.add_text(" big ");
        let t3 = dom.add_text(" world");

        dom.set_parent(t1, parent).unwrap();
        dom.set_parent(span, parent).unwrap();
        dom.set_parent(t3, parent).unwrap();

        dom.set_first_child(parent, Some(t1)).unwrap();
        dom.set_next_sibling(t1, Some(span)).unwrap();
        dom.set_next_sibling(span, Some(t3)).unwrap();

        dom.set_parent(t2, span).unwrap();
        dom.set_first_child(span, Some(t2)).unwrap();

        let ctx = LayoutContext {
            dom: &dom,
            stylesheet: &CSSStyleSheet::new(),
            text_measurer: Box::new(FixedWidth {}),
        };

        let items = collect_inline_items(&[parent], &ctx);

        println!("Items: {:?}", items);
        println!("Text Strings: {:?}", get_text_strings(&items));

        // "Hello " -> "big " -> "world"
        assert_eq!(get_text_strings(&items), vec!["Hello ", "big ", "world"]);
    }

    #[test]
    fn test_forced_break_strips_trailing_space() {
        let mut dom = Dom::new();
        let t1 = dom.add_text("Hello \n ");
        let br = dom.add_element(get_tag_index("br"), None);
        let t2 = dom.add_text(" world");

        let ctx = LayoutContext {
            dom: &dom,
            stylesheet: &CSSStyleSheet::new(),
            text_measurer: Box::new(FixedWidth {}),
        };

        let items = collect_inline_items(&[t1, br, t2], &ctx);

        // `<br>` strips the trailing space from t1, and makes `last_was_space=true` for t2
        assert_eq!(get_text_strings(&items), vec!["Hello", "<br>", "world"]);
    }

    #[test]
    fn test_whitespace_only_produces_nothing() {
        let mut dom = Dom::new();
        let t1 = dom.add_text("   \n   \t  ");

        let ctx = LayoutContext {
            dom: &dom,
            stylesheet: &CSSStyleSheet::new(),
            text_measurer: Box::new(FixedWidth {}),
        };

        let items = collect_inline_items(&[t1], &ctx);

        // Fully empty sequences yield no fragments, preventing blank ghost lines
        assert!(items.is_empty());
    }
}
