use crate::style::custom_properties::{CustomPropTable, PropInterner};
use crate::style::{ComputedStyle, Display, StyleCache, cascade, matching};
use blink_css::CSSStyleSheet;
use blink_dom::{Dom, DomNodeId, get_tag_name};

pub struct StyleContext<'a> {
    pub dom: &'a Dom,
    pub stylesheet: &'a CSSStyleSheet,
    pub custom_prop_table: &'a CustomPropTable,
    pub prop_interner: &'a PropInterner,
}

impl<'a> StyleContext<'a> {
    pub fn new(
        dom: &'a Dom,
        stylesheet: &'a CSSStyleSheet,
        custom_prop_table: &'a CustomPropTable,
        prop_interner: &'a PropInterner,
    ) -> Self {
        Self {
            dom,
            stylesheet,
            custom_prop_table,
            prop_interner,
        }
    }

    fn default_display(&self, node_id: DomNodeId) -> Display {
        if let Some(node) = self.dom.get_node(node_id)
            && let Some(tag) = node.get_tag()
        {
            match get_tag_name(tag) {
                "head" | "script" | "style" | "title" | "meta" | "link" => Display::None,
                "document" | "html" | "body" | "div" | "p" | "section" | "ul" | "li" => {
                    Display::BLOCK
                }
                _ => Display::INLINE,
            }
        } else {
            Display::INLINE
        }
    }

    fn is_stylable_element(&self, node_id: DomNodeId) -> bool {
        !self.dom.is_document_node(node_id)
            && self
                .dom
                .get_node(node_id)
                .is_some_and(|node| node.is_element())
    }

    pub fn compute_styles(&self, root: DomNodeId, initial_size: Option<usize>) -> StyleCache {
        let mut style_cache = StyleCache::new(initial_size.unwrap_or_default());

        let mut stack = vec![(root, None)];

        while let Some((node_id, parent_id)) = stack.pop() {
            let Some(node) = self.dom.get_node(node_id) else {
                continue;
            };

            if node.is_text() {
                continue;
            }

            if self.is_stylable_element(node_id) {
                let parent_style = parent_id.and_then(|pid| style_cache.get_for_node(pid));
                let computed_style = self.compute_element_style(node_id, parent_style);
                style_cache.insert(node_id, computed_style);
            }

            let mut current_child = self.dom.get_first_child(node_id);
            while let Some(child) = current_child {
                stack.push((child, Some(node_id)));
                current_child = self.dom.get_next_sibling(child);
            }
        }

        style_cache
    }

    fn compute_element_style(
        &self,
        node_id: DomNodeId,
        parent_style: Option<&ComputedStyle>,
    ) -> ComputedStyle {
        let mut rules = matching::get_matching_rules(node_id, self.dom, self.stylesheet);

        let computed_style = cascade::cascade_properties(
            &mut rules,
            parent_style,
            self,
            self.default_display(node_id),
        );

        // Compute custom properties (CSS variables) based on parent style if available
        // Resolve var() in declarations using parent_style if necessary

        computed_style
    }
}
