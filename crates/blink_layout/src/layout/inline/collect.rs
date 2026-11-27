use super::super::engine::LayoutContext;
use super::super::fragment::LayoutFragment;
use super::super::text::TextMeasurer;
use crate::style::{DisplayInside, DisplayOutside};
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
    // pub fn is_text(&self) -> bool {
    //     matches!(self.kind, InlineItemKind::Text(_))
    // }

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

    // pub fn append_text(&mut self, additional_text: &str) {
    //     if let InlineItemKind::Text(ref mut text) = self.kind {
    //         text.push_str(additional_text);
    //     }
    // }
}

pub enum PreparedInlineItemKind<T = LayoutFragment> {
    TextSegment {
        dom_node: DomNodeId,
        text: String,
        // range: Range<usize>,
    },
    AtomicInline {
        fragment: T,
    },
    ForcedBreak,
}

pub struct PreparedInlineItem<T = LayoutFragment> {
    pub kind: PreparedInlineItemKind<T>,
    pub width: f32,
    pub fit_width: f32,
    pub height: f32,
}

impl<T> PreparedInlineItem<T> {
    pub fn is_forced_break(&self) -> bool {
        matches!(self.kind, PreparedInlineItemKind::ForcedBreak)
    }
}

pub fn collect_inline_items<M: TextMeasurer + ?Sized>(
    run: &[DomNodeId],
    ctx: &mut LayoutContext<M>,
) -> Vec<InlineItem> {
    let mut items = Vec::new();
    let mut last_was_space = true;

    for &node_id in run {
        collect_recursive(node_id, ctx, &mut items, &mut last_was_space);
    }

    strip_trailing_space(&mut items);
    items
}

pub fn collect_recursive<M: TextMeasurer + ?Sized>(
    node_id: DomNodeId,
    ctx: &mut LayoutContext<M>,
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

    let Some(style) = ctx.style_cache.get_for_node(node_id) else {
        return;
    };

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
    use super::super::super::text::FixedWidth;
    use super::*;
    use crate::style::custom_properties::{CustomPropTable, PropInterner};
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

    fn make_test_ctx<'a>(
        dom: &'a Dom,
        stylesheet: &'a CSSStyleSheet,
    ) -> crate::style::context::StyleContext<'a> {
        // Lazy helper to leak the table and interner so we don't have to manage lifetimes in the test
        let table = Box::leak(Box::new(CustomPropTable::new()));
        let interner = Box::leak(Box::new(PropInterner::new()));
        crate::style::context::StyleContext::new(dom, stylesheet, table, interner)
    }

    #[test]
    fn test_collapse_basic_whitespace() {
        let mut dom = Dom::new();
        let text_id = dom.add_text("  Hello \n   world  ");

        let stylesheet = CSSStyleSheet::new();
        let ctx = make_test_ctx(&dom, &stylesheet);
        let style_cache = ctx.compute_styles(text_id, None);

        let mut ctx = LayoutContext {
            dom: &dom,
            style_cache: &style_cache,
            text_measurer: &FixedWidth {},
        };

        let items = collect_inline_items(&[text_id], &mut ctx);

        // Leading and trailing space dropped, internal whitespace flattened
        assert_eq!(get_text_strings(&items), vec!["Hello world"]);
    }

    #[test]
    fn test_adjacent_text_nodes() {
        let mut dom = Dom::new();
        let t1 = dom.add_text("Hello ");
        let t2 = dom.add_text(" world"); // Leading space here should collapse

        let stylesheet = CSSStyleSheet::new();
        let ctx = make_test_ctx(&dom, &stylesheet);
        let style_cache = ctx.compute_styles(t1, None);

        let mut ctx = LayoutContext {
            dom: &dom,
            style_cache: &style_cache,
            text_measurer: &FixedWidth {},
        };

        let items = collect_inline_items(&[t1, t2], &mut ctx);

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

        let stylesheet = CSSStyleSheet::new();
        let ctx = make_test_ctx(&dom, &stylesheet);
        let style_cache = ctx.compute_styles(parent, None);

        let mut ctx = LayoutContext {
            dom: &dom,
            style_cache: &style_cache,
            text_measurer: &FixedWidth {},
        };

        let items = collect_inline_items(&[parent], &mut ctx);

        println!("Items: {:?}", items);
        println!("Text Strings: {:?}", get_text_strings(&items));

        // "Hello " -> "big " -> "world"
        assert_eq!(get_text_strings(&items), vec!["Hello ", "big ", "world"]);
    }

    #[test]
    fn test_forced_break_strips_trailing_space() {
        let mut dom = Dom::new();

        let root = dom.add_element(0, None);
        dom.set_virtual_root(root)
            .expect("Failed to set virtual root");

        let parent = dom.add_element(get_tag_index("div"), None);
        let t1 = dom.add_text("Hello \n ");
        let br = dom.add_element(get_tag_index("br"), None);
        let t2 = dom.add_text(" world");

        dom.set_first_child(root, Some(parent))
            .expect("Failed to set first child");
        dom.set_parent(parent, root).expect("Failed to set parent");

        dom.set_first_child(parent, Some(t1))
            .expect("Failed to set first child");
        dom.set_parent(t1, parent).expect("Failed to set parent");

        dom.set_parent(br, parent).expect("Failed to set parent");
        dom.set_next_sibling(t1, Some(br))
            .expect("Failed to set next sibling");

        dom.set_parent(t2, parent).expect("Failed to set parent");
        dom.set_next_sibling(br, Some(t2))
            .expect("Failed to set next sibling");

        let stylesheet = CSSStyleSheet::new();
        let ctx = make_test_ctx(&dom, &stylesheet);
        let style_cache = ctx.compute_styles(root, None);

        let mut ctx = LayoutContext {
            dom: &dom,
            style_cache: &style_cache,
            text_measurer: &FixedWidth {},
        };

        let items = collect_inline_items(&[t1, br, t2], &mut ctx);

        // `<br>` strips the trailing space from t1, and makes `last_was_space=true` for t2
        assert_eq!(get_text_strings(&items), vec!["Hello", "<br>", "world"]);
    }

    #[test]
    fn test_whitespace_only_produces_nothing() {
        let mut dom = Dom::new();
        let t1 = dom.add_text("   \n   \t  ");

        let stylesheet = CSSStyleSheet::new();
        let ctx = make_test_ctx(&dom, &stylesheet);
        let style_cache = ctx.compute_styles(t1, None);

        let mut ctx = LayoutContext {
            dom: &dom,
            style_cache: &style_cache,
            text_measurer: &FixedWidth {},
        };

        let items = collect_inline_items(&[t1], &mut ctx);

        // Fully empty sequences yield no fragments, preventing blank ghost lines
        assert!(items.is_empty());
    }
}
