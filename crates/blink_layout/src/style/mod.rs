mod cascade;
mod matching;
mod types;

use blink_core::generator::Generation;
pub use types::{
    BorderSide, BorderStyle, Color, ComputedStyle, Display, DisplayInside, DisplayOutside,
    EdgeSizes,
};

use blink_css::CSSStyleSheet;
use blink_dom::{Dom, DomNodeId, get_tag_name};

pub struct ComputedStyleTag;
pub type ComputedStyleId = blink_core::base::Id<ComputedStyleTag>;

#[derive(Debug)]
pub struct StyleCache {
    styles: Vec<ComputedStyle>,
    node_cache: Vec<Option<(Generation, ComputedStyleId)>>,
}

impl Default for StyleCache {
    fn default() -> Self {
        Self::new(0)
    }
}

impl StyleCache {
    pub fn new(size: usize) -> Self {
        Self {
            styles: Vec::with_capacity(size),
            node_cache: vec![None; size],
        }
    }

    pub fn get_entry_for_node(&self, node: DomNodeId) -> Option<(ComputedStyleId, &ComputedStyle)> {
        let idx = node.slot();
        let g = node.generation();

        self.check_and_get(idx, g)
            .and_then(|style_id| self.get(style_id).map(|style| (style_id, style)))
    }

    pub fn get_for_node(&self, node: DomNodeId) -> Option<&ComputedStyle> {
        let idx = node.slot();
        let g = node.generation();

        self.check_and_get(idx, g)
            .and_then(|style_id| self.get(style_id))
    }

    pub fn id_for_node(&self, node: DomNodeId) -> Option<ComputedStyleId> {
        let idx = node.slot();
        let g = node.generation();

        self.check_and_get(idx, g)
    }

    // /// `insert_anonymous` inserts a new anonymous style into the cache and returns its `ComputedStyleId`.
    // /// This is useful for creating styles for elements that don't have a corresponding DOM node.
    // /// Returns `None` if the insertion fails (e.g., if the cache is full).
    // pub fn insert_anonymous(&mut self, style: ComputedStyle) -> Option<ComputedStyleId> {
    //     let id = ComputedStyleId::from_u32(self.styles.len() as u32)?;
    //     self.styles.push(style);
    //     Some(id)
    // }

    #[inline]
    pub fn get(&self, id: ComputedStyleId) -> Option<&ComputedStyle> {
        self.styles.get(id.get_index())
    }
}

// Private helper function to compute the style for a given node
impl StyleCache {
    fn check_and_get(&self, index: usize, generation: Generation) -> Option<ComputedStyleId> {
        if let Some(Some((cached_gen, style_id))) = self.node_cache.get(index)
            && *cached_gen == generation
        {
            return Some(*style_id);
        }
        None
    }

    fn insert(&mut self, node: DomNodeId, style: ComputedStyle) -> ComputedStyleId {
        let id = ComputedStyleId::from_usize(self.styles.len()).expect("ComputedStyleId overflow");
        self.styles.push(style);
        let idx = node.slot();
        let g = node.generation();
        if idx >= self.node_cache.len() {
            self.node_cache.resize(idx + 1, None);
        }
        self.node_cache[idx] = Some((g, id));
        id
    }
}

fn is_stylable_element(node_id: DomNodeId, dom: &Dom) -> bool {
    !dom.is_document_node(node_id) && dom.get_node(node_id).is_some_and(|node| node.is_element())
}

pub fn compute_styles(
    root: DomNodeId,
    dom: &Dom,
    stylesheet: &CSSStyleSheet,
    initial_size: Option<usize>,
) -> StyleCache {
    let mut style_cache = StyleCache::new(initial_size.unwrap_or_default());

    let mut stack = vec![(root, None)];

    while let Some((node_id, parent_id)) = stack.pop() {
        let Some(node) = dom.get_node(node_id) else {
            continue;
        };

        if node.is_text() {
            continue;
        }

        if is_stylable_element(node_id, dom) {
            let parent_style = parent_id.and_then(|pid| style_cache.get_for_node(pid));
            let computed_style = compute_element_style(node_id, parent_style, dom, stylesheet);
            style_cache.insert(node_id, computed_style);
        }

        let mut current_child = dom.get_first_child(node_id);
        while let Some(child) = current_child {
            stack.push((child, Some(node_id)));
            current_child = dom.get_next_sibling(child);
        }
    }

    style_cache
}

fn compute_element_style(
    node_id: DomNodeId,
    #[allow(unused_variables)] // TODO: Use parent_style for computing inherited properties
    parent_style: Option<&ComputedStyle>,
    dom: &Dom,
    stylesheet: &CSSStyleSheet,
) -> ComputedStyle {
    let mut rules = matching::get_matching_rules(node_id, dom, stylesheet);

    let computed_style =
        cascade::cascade_properties(&mut rules, stylesheet, default_display(node_id, dom));

    // Compute custom properties (CSS variables) based on parent style if available
    // Resolve var() in declarations using parent_style if necessary

    computed_style
}

// fn compute_style(node_id: DomNodeId, dom: &Dom, stylesheet: &CSSStyleSheet) -> ComputedStyle {
//     let mut rules = matching::get_matching_rules(node_id, dom, stylesheet);
//
//     cascade::cascade_properties(&mut rules, stylesheet, default_display(node_id, dom))
// }

fn default_display(node_id: DomNodeId, dom: &Dom) -> Display {
    if let Some(node) = dom.get_node(node_id)
        && let Some(tag) = node.get_tag()
    {
        match get_tag_name(tag) {
            "head" | "script" | "style" | "title" | "meta" | "link" => Display::None,
            "document" | "html" | "body" | "div" | "p" | "section" | "ul" | "li" => Display::BLOCK,
            _ => Display::INLINE,
        }
    } else {
        Display::INLINE
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::style::matching::tests::{DIV_TAG_INDEX, make_attr};
    use blink_css::parse_css_into_stylesheet;
    use blink_dom::get_tag_index;

    #[test]
    fn compute_style_for_element() {
        let mut dom = Dom::new();

        dom.ensure_tag_registered(DIV_TAG_INDEX);

        let div_attrs = vec![make_attr("id", "main")].into_boxed_slice();
        let div_node = dom.add_element(DIV_TAG_INDEX, Some(div_attrs));

        let mut stylesheet = CSSStyleSheet::new();
        parse_css_into_stylesheet("div { width: 100px; height: 100px; }", &mut stylesheet);

        let style_cache = compute_styles(div_node, &dom, &stylesheet, None);

        let computed_style = style_cache.get_for_node(div_node).unwrap();
        assert_eq!(computed_style.width, Some(100.0));
        assert_eq!(computed_style.height, Some(100.0));
    }

    #[test]
    fn computes_styles_for_descendants() {
        let mut dom = Dom::new();

        let parent = dom.add_element(get_tag_index("div"), None);
        let child = dom.add_element(get_tag_index("span"), None);

        dom.set_parent(child, parent).unwrap();
        dom.set_first_child(parent, Some(child)).unwrap();

        let mut stylesheet = CSSStyleSheet::new();
        parse_css_into_stylesheet(
            "div { width: 200px; } span { width: 100px; }",
            &mut stylesheet,
        );

        let styles = compute_styles(parent, &dom, &stylesheet, None);

        assert_eq!(styles.get_for_node(parent).unwrap().width, Some(200.0));
        assert_eq!(styles.get_for_node(child).unwrap().width, Some(100.0));
    }

    #[test]
    fn computes_styles_for_descendants_of_display_none() {
        let mut dom = Dom::new();

        let parent = dom.add_element(get_tag_index("div"), None);
        let child = dom.add_element(get_tag_index("span"), None);

        dom.set_parent(child, parent).unwrap();
        dom.set_first_child(parent, Some(child)).unwrap();

        let mut stylesheet = CSSStyleSheet::new();
        parse_css_into_stylesheet(
            "div { display: none; } span { width: 100px; }",
            &mut stylesheet,
        );

        let styles = compute_styles(parent, &dom, &stylesheet, None);

        assert_eq!(styles.get_for_node(parent).unwrap().display, Display::None);
        assert_eq!(styles.get_for_node(child).unwrap().width, Some(100.0));
    }

    #[test]
    fn compute_style_cascade_overrides() {
        let mut dom = Dom::new();
        dom.ensure_tag_registered(DIV_TAG_INDEX);

        // Build Node: <div id="hero" class="banner">
        let attrs = vec![make_attr("id", "hero"), make_attr("class", "banner")].into_boxed_slice();

        let div_node = dom.add_element(DIV_TAG_INDEX, Some(attrs));

        // Create conflicting rules with varying specificities
        let mut stylesheet = CSSStyleSheet::new();
        let css = "
            div { width: 100px; height: 50px; display: inline; }
            .banner { width: 200px; display: block; }
            div#hero { width: 300px; }
        ";
        parse_css_into_stylesheet(css, &mut stylesheet);

        let style_cache = compute_styles(div_node, &dom, &stylesheet, None);

        let computed = style_cache
            .get_for_node(div_node)
            .expect("Computed style should exist");

        assert_eq!(
            computed.height,
            Some(50.0),
            "Height should be inherited from the base tag rule"
        );

        assert_eq!(
            computed.display,
            Display::BLOCK,
            "Class rule should override Tag rule for display"
        );

        assert_eq!(
            computed.width,
            Some(300.0),
            "ID rule should have highest specificity and win the width cascade"
        );
    }

    #[test]
    fn test_compute_root_overrides_direct_tag() {
        let mut dom = Dom::new();
        dom.ensure_tag_registered(DIV_TAG_INDEX);

        let vroot_node = dom.add_element(0, None); // Root node with tag index 0
        let div_node = dom.add_element(DIV_TAG_INDEX, None);

        dom.set_parent(div_node, vroot_node)
            .expect("Failed to set parent");
        dom.set_first_child(vroot_node, Some(div_node))
            .expect("Failed to set first child");

        let mut stylesheet = CSSStyleSheet::new();
        let css = "
            :root { width: 200px; }
            div { width: 100px; height: 50px; }
        ";
        parse_css_into_stylesheet(css, &mut stylesheet);

        let style_cache = compute_styles(vroot_node, &dom, &stylesheet, None);

        let computed = style_cache
            .get_for_node(div_node)
            .expect("Computed style should exist");

        assert_eq!(
            computed.width,
            Some(200.0),
            "Root rule should override the direct tag rule for width"
        );
        assert_eq!(
            computed.height,
            Some(50.0),
            "Height should be inherited from the direct tag rule"
        );
    }

    #[test]
    fn test_compute_style_margin_and_padding_cascade() {
        let mut dom = Dom::new();

        let root = dom.add_element(0, None); // Root node with tag index 0
        dom.set_virtual_root(root)
            .expect("Failed to set virtual root");

        let attrs = vec![make_attr("class", "box")].into_boxed_slice();
        dom.ensure_tag_registered(DIV_TAG_INDEX);
        let div_node = dom.add_element(DIV_TAG_INDEX, Some(attrs));

        dom.set_first_child(root, Some(div_node))
            .expect("Failed to set first child");
        dom.set_parent(div_node, root)
            .expect("Failed to set parent");

        let mut stylesheet = CSSStyleSheet::new();
        let css = "
            div {
                margin: 10px 20px;
                padding: 8px;
            }
            .box {
                margin-top: -15px;
                margin-left: 30px;
                padding: 4px 12px;
                padding-bottom: -10px;
            }
        ";
        parse_css_into_stylesheet(css, &mut stylesheet);

        let style_cache = compute_styles(root, &dom, &stylesheet, None);

        let computed = style_cache
            .get_for_node(div_node)
            .expect("Computed style should exist");

        assert_eq!(
            computed.margin,
            EdgeSizes {
                top: -15.0,
                right: 20.0,
                bottom: 10.0,
                left: 30.0,
            }
        );
        assert_eq!(
            computed.padding,
            EdgeSizes {
                top: 4.0,
                right: 12.0,
                bottom: 0.0,
                left: 12.0,
            }
        );
    }

    #[test]
    fn test_compute_style_border_cascade_and_effective_widths() {
        let mut dom = Dom::new();
        dom.ensure_tag_registered(DIV_TAG_INDEX);

        let attrs = vec![make_attr("id", "panel"), make_attr("class", "card")].into_boxed_slice();
        let div_node = dom.add_element(DIV_TAG_INDEX, Some(attrs));

        let mut stylesheet = CSSStyleSheet::new();
        let css = "
            div {
                border: 2px solid black;
            }
            .card {
                border-width: 1px 4px;
                border-style: solid none solid solid;
                border-color: red blue;
            }
            div#panel {
                border-top: 6px solid #00ff00;
                border-left-width: 10px;
                border-bottom-color: currentcolor;
            }
        ";
        parse_css_into_stylesheet(css, &mut stylesheet);

        let style_cache = compute_styles(div_node, &dom, &stylesheet, None);

        let computed = style_cache
            .get_for_node(div_node)
            .expect("Computed style should exist");

        assert_eq!(
            computed.border.top,
            BorderSide {
                width: 6.0,
                style: BorderStyle::Solid,
                color: Some(Color {
                    r: 0,
                    g: 255,
                    b: 0,
                    a: 255,
                }),
            }
        );
        assert_eq!(
            computed.border.right,
            BorderSide {
                width: 4.0,
                style: BorderStyle::None,
                color: Some(Color {
                    r: 0,
                    g: 0,
                    b: 255,
                    a: 255,
                }),
            }
        );
        assert_eq!(
            computed.border.bottom,
            BorderSide {
                width: 1.0,
                style: BorderStyle::Solid,
                color: None,
            }
        );
        assert_eq!(
            computed.border.left,
            BorderSide {
                width: 10.0,
                style: BorderStyle::Solid,
                color: Some(Color {
                    r: 0,
                    g: 0,
                    b: 255,
                    a: 255,
                }),
            }
        );

        // Right border has style `none`, so its effective layout width must be 0.0
        assert_eq!(
            computed.border.widths(),
            EdgeSizes {
                top: 6.0,
                right: 0.0,
                bottom: 1.0,
                left: 10.0,
            }
        );
    }
}
