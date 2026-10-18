use std::num::NonZeroU32;
use markup5ever::local_name;
use blink_css::{CSSStyleRule, CSSStyleSheet, Selector};
use blink_dom::{get_tag_name, Dom, NodeKind};

#[derive(Debug, Clone)]
pub enum Display {
    Block,
    Inline,
    None,
}

#[derive(Debug, Clone)]
pub struct ComputedStyle {
    pub display: Display,
    pub width: Option<f32>,
    pub height: Option<f32>,
}

impl ComputedStyle {
    pub fn new(display: Display, width: Option<f32>, height: Option<f32>) -> Self {
        ComputedStyle {
            display,
            width,
            height,
        }
    }
}

pub fn compute_style(node_id: NonZeroU32, dom: &Dom, stylesheet: &CSSStyleSheet) -> ComputedStyle {
    let mut rules = get_matching_rules(node_id, dom, stylesheet);

    cascade_properties(&mut rules)
}

fn get_matching_rules<'a>(node_id: NonZeroU32, dom: &Dom, stylesheet: &'a CSSStyleSheet) -> Vec<&'a CSSStyleRule> {
    let mut matching_rules: Vec<&CSSStyleRule> = vec![];
    for rule in &stylesheet.rules {
        if matches_selector(node_id, dom, rule) {
            matching_rules.push(rule);
        }
    }

    matching_rules
}

fn matches_selector(node_id: NonZeroU32, dom: &Dom, rule: &CSSStyleRule) -> bool {
    match_parts(node_id, dom, &rule.selector)
}

fn match_parts(node_id: NonZeroU32, dom: &Dom, parts: &[Selector]) -> bool {
    if parts.is_empty() { return true; }

    // Search for the last Descendant combinator from the right
    // e.g., parsing [Tag("div"), Descendant, Class("card")]
    if let Some(descendant_idx) = parts.iter().rposition(|p| matches!(p, Selector::Descendant)) {
        let ancestor_parts = &parts[..descendant_idx];
        let target_parts = &parts[descendant_idx + 1..];

        if !match_compound(node_id, dom, target_parts) {
            return false;
        }

        // Walk up the DOM tree looking for ANY parent that matches the left-side
        let mut current_parent = dom.get_parent(node_id);
        while let Some(parent_id) = current_parent {
            if match_parts(parent_id, dom, ancestor_parts) {
                return true;
            }
            current_parent = dom.get_parent(parent_id);
        }

        // If we run out of parents and didn't find a match
        false
    } else {
        // No combinators present, so the node just has to match the compound selector
        match_compound(node_id, dom, parts)
    }
}

fn match_compound(node_id: NonZeroU32, dom: &Dom, parts: &[Selector]) -> bool {
    for part in parts {
        match part {
            Selector::Tag(target_tag) => {
                let node = match dom.get_node(node_id) {
                    Some(n) => n,
                    None => return false,
                };
                let is_match = match &node.kind {
                    NodeKind::Element { tag, .. } | NodeKind::EmptyLeaf { tag } => {
                        let tag_name = get_tag_name(*tag);
                        target_tag == tag_name
                    }
                    NodeKind::Text {} => false,
                };
                if !is_match { return false; }
            },
            Selector::ID(target_id) => {
                let attrs = match dom.get_attributes(node_id) {
                    Some(attrs) => attrs,
                    None => return false,
                };
                if !attrs.iter().any(|attr| attr.name.local == local_name!("id") && attr.value.as_ref() == target_id) {
                    return false;
                }
            },
            Selector::Class(target_class) => {
                let attrs = match dom.get_attributes(node_id) {
                    Some(attrs) => attrs,
                    None => return false,
                };
                let has_class = attrs.iter().any(|attr| {
                    attr.name.local == local_name!("class") &&
                        attr.value.as_ref().split_whitespace().any(|c| c == target_class)
                });
                if !has_class { return false; }
            },
            Selector::Descendant => unreachable!("Descendants are handled by match_parts"),
        }
    }
    true
}

fn cascade_properties(matched_rules: &mut Vec<&CSSStyleRule>) -> ComputedStyle {
    matched_rules.sort_by(|a, b| a.specificity().cmp(&b.specificity()));

    let mut display = Display::Inline;
    let mut width = None;
    let mut height = None;

    for rule in matched_rules {
        if let Some(d) = rule.properties.get("display") {
            display = match d.as_str() {
                "block" => Display::Block,
                "none" => Display::None,
                _ => Display::Inline,
            };
        }

        if let Some(w) = rule.properties.get("width") {
            width = parse_px_value(w);
        }

        if let Some(h) = rule.properties.get("height") {
            height = parse_px_value(h);
        }
    }

    ComputedStyle::new(display, width, height)
}

fn parse_px_value(mut val: &str) -> Option<f32> {
    val = val.trim();
    if val.ends_with("px") {
        let num_str = val.trim_end_matches("px").trim();
        num_str.parse::<f32>().ok()
    } else {
        val.parse::<f32>().ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use markup5ever::{ns, Attribute, QualName, LocalName};
    use blink_css::parse_css_into_stylesheet;

    const DIV_TAG_INDEX: u8 = 29;
    const P_TAG_INDEX: u8 = 73;

    fn make_attr(name: &str, value: &str) -> Attribute {
        Attribute {
            name: QualName::new(None, ns!(), LocalName::from(name)),
            value: value.into(),
        }
    }

    fn get_first_rule(css: &str) -> CSSStyleRule {
        let mut stylesheet = CSSStyleSheet::new();
        parse_css_into_stylesheet(css, &mut stylesheet).unwrap();
        stylesheet.rules.remove(0)
    }

    #[test]
    fn test_selector_matching() {
        let mut dom = Dom::new();

        dom.ensure_tag_registered(DIV_TAG_INDEX);
        dom.ensure_tag_registered(P_TAG_INDEX);

        let div_attrs = vec![make_attr("id", "main")].into_boxed_slice();
        let div_node = dom.add_element(DIV_TAG_INDEX, Some(div_attrs));

        let p_attrs = vec![make_attr("class", "card active")].into_boxed_slice();
        let p_node = dom.add_element(P_TAG_INDEX, Some(p_attrs));

        dom.set_parent(p_node, div_node);
        dom.set_first_child(div_node, Some(p_node));

        let rule_p = get_first_rule("p { color: red; }");
        assert!(matches_selector(p_node, &dom, &rule_p), "Should match direct 'p' tag");

        let rule_card = get_first_rule(".card { margin: 0; }");
        assert!(matches_selector(p_node, &dom, &rule_card), "Should match '.card' class");

        let rule_active = get_first_rule(".active { display: block; }");
        assert!(matches_selector(p_node, &dom, &rule_active), "Should match '.active' class");

        let rule_fail = get_first_rule(".button { padding: 5px; }");
        assert!(!matches_selector(p_node, &dom, &rule_fail), "Should NOT match missing '.button' class");

        let rule_descendant = get_first_rule("div#main .card { border: 1px; }");
        assert!(matches_selector(p_node, &dom, &rule_descendant), "Should match descendant 'div#main .card'");

        let rule_descendant_fail = get_first_rule("span#main .card { border: 1px; }");
        assert!(!matches_selector(p_node, &dom, &rule_descendant_fail), "Should NOT match if parent tag is wrong");
    }

    #[test]
    fn test_deep_descendant_matching() {
        let mut dom = Dom::new();
        let div_tag = blink_dom::get_tag_index("div");
        let section_tag = blink_dom::get_tag_index("section");
        let p_tag = blink_dom::get_tag_index("p");

        // Hierarchy: <div id="main"> -> <section class="container"> -> <p class="card">
        let div_node = dom.add_element(div_tag, Some(vec![make_attr("id", "main")].into_boxed_slice()));
        let section_node = dom.add_element(section_tag, Some(vec![make_attr("class", "container")].into_boxed_slice()));
        let p_node = dom.add_element(p_tag, Some(vec![make_attr("class", "card")].into_boxed_slice()));

        // Wire the pointers
        dom.set_parent(section_node, div_node);
        dom.set_first_child(div_node, Some(section_node));

        dom.set_parent(p_node, section_node);
        dom.set_first_child(section_node, Some(p_node));

        // Grandparent matching (The ultimate W3C descendant test)
        let rule_grandparent = get_first_rule("div#main .card { color: red; }");
        assert!(
            matches_selector(p_node, &dom, &rule_grandparent),
            "Should successfully jump over the <section> to match the <div> grandparent"
        );

        // Direct parent matching
        let rule_parent = get_first_rule(".container p { padding: 0; }");
        assert!(
            matches_selector(p_node, &dom, &rule_parent),
            "Should match the direct parent <section class='container'>"
        );

        // Multi-level descendant chain
        let rule_multi = get_first_rule("div .container .card { margin: 0; }");
        assert!(
            matches_selector(p_node, &dom, &rule_multi),
            "Should match the full multi-level ancestor chain"
        );

        // 4. Failing missing ancestor
        let rule_fail = get_first_rule("nav .card { color: blue; }");
        assert!(
            !matches_selector(p_node, &dom, &rule_fail),
            "Should fail if the ancestor tree exhausts without finding a 'nav'"
        );
    }

    #[test]
    fn test_parse_px_value_edge_cases() {
        // Standard conversions
        assert_eq!(parse_px_value("100px"), Some(100.0));
        assert_eq!(parse_px_value("50.5px"), Some(50.5));

        // Whitespace tolerance
        assert_eq!(parse_px_value("  200  px  "), Some(200.0));
        assert_eq!(parse_px_value(" 300 "), Some(300.0));

        // Unhandled units and keywords should safely fall back to None (auto)
        assert_eq!(parse_px_value("auto"), None);
        assert_eq!(parse_px_value("100vw"), None);
        assert_eq!(parse_px_value("inherit"), None);
    }

    #[test]
    fn test_compute_style() {
        let mut dom = Dom::new();

        dom.ensure_tag_registered(DIV_TAG_INDEX);

        let div_attrs = vec![make_attr("id", "main")].into_boxed_slice();
        let div_node = dom.add_element(DIV_TAG_INDEX, Some(div_attrs));

        let mut stylesheet = CSSStyleSheet::new();
        parse_css_into_stylesheet("div { width: 100px; height: 100px; }", &mut stylesheet).unwrap();

        let computed_style = compute_style(div_node, &dom, &stylesheet);
        assert_eq!(computed_style.width, Some(100.0));
        assert_eq!(computed_style.height, Some(100.0));
    }

    #[test]
    fn test_compute_style_cascade_overrides() {
        let mut dom = Dom::new();
        dom.ensure_tag_registered(DIV_TAG_INDEX);

        // Build Node: <div id="hero" class="banner">
        let attrs = vec![
            make_attr("id", "hero"),
            make_attr("class", "banner")
        ].into_boxed_slice();

        let div_node = dom.add_element(DIV_TAG_INDEX, Some(attrs));

        // Create conflicting rules with varying specificities
        let mut stylesheet = CSSStyleSheet::new();
        let css = "
            div { width: 100px; height: 50px; display: inline; }
            .banner { width: 200px; display: block; }
            div#hero { width: 300px; }
        ";
        parse_css_into_stylesheet(css, &mut stylesheet).unwrap();

        let computed = compute_style(div_node, &dom, &stylesheet);

        assert_eq!(computed.height, Some(50.0), "Height should be inherited from the base tag rule");

        assert!(matches!(computed.display, Display::Block), "Class rule should override Tag rule for display");

        assert_eq!(computed.width, Some(300.0), "ID rule should have highest specificity and win the width cascade");
    }
}
