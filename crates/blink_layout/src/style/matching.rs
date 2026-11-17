use blink_css::{CSSStyleRule, CSSStyleSheet, Selector};
use blink_dom::{Dom, DomNodeId};
use markup5ever::local_name;

pub(super) fn get_matching_rules<'a>(
    node_id: DomNodeId,
    dom: &Dom,
    stylesheet: &'a CSSStyleSheet,
) -> Vec<&'a CSSStyleRule> {
    let mut matching_rules: Vec<&CSSStyleRule> = vec![];
    for rule in &stylesheet.rules {
        if matches_selector(node_id, dom, rule) {
            matching_rules.push(rule);
        }
    }

    matching_rules
}

fn matches_selector(node_id: DomNodeId, dom: &Dom, rule: &CSSStyleRule) -> bool {
    match_parts(node_id, dom, &rule.selector)
}

fn match_parts(node_id: DomNodeId, dom: &Dom, parts: &[Selector]) -> bool {
    if parts.is_empty() {
        return true;
    }

    // Search for the last Descendant combinator from the right
    // e.g., parsing [Tag("div"), Descendant, Class("card")]
    if let Some(descendant_idx) = parts
        .iter()
        .rposition(|p| matches!(p, Selector::Descendant))
    {
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

fn match_compound(node_id: DomNodeId, dom: &Dom, parts: &[Selector]) -> bool {
    for part in parts {
        match part {
            Selector::Tag(target_tag) => {
                let node = match dom.get_node(node_id) {
                    Some(n) => n,
                    None => return false,
                };
                if node.get_tag_name().is_none_or(|tag| tag != *target_tag) {
                    return false;
                }
            }
            Selector::ID(target_id) => {
                let attrs = match dom.get_attributes(node_id) {
                    Some(attrs) => attrs,
                    None => return false,
                };
                if !attrs.iter().any(|attr| {
                    attr.name.local == local_name!("id") && attr.value.as_ref() == target_id
                }) {
                    return false;
                }
            }
            Selector::Class(target_class) => {
                let attrs = match dom.get_attributes(node_id) {
                    Some(attrs) => attrs,
                    None => return false,
                };
                let has_class = attrs.iter().any(|attr| {
                    attr.name.local == local_name!("class")
                        && attr
                            .value
                            .as_ref()
                            .split_whitespace()
                            .any(|c| c == target_class)
                });
                if !has_class {
                    return false;
                }
            }
            Selector::Descendant => unreachable!("Descendants are handled by match_parts"),
        }
    }
    true
}

#[cfg(test)]
pub(super) mod tests {
    use super::*;
    use blink_css::parse_css_into_stylesheet;
    use blink_dom::get_tag_index;
    use markup5ever::{Attribute, LocalName, QualName, ns};

    pub(crate) const DIV_TAG_INDEX: u8 = 29;
    const P_TAG_INDEX: u8 = 73;

    pub(crate) fn make_attr(name: &str, value: &str) -> Attribute {
        Attribute {
            name: QualName::new(None, ns!(), LocalName::from(name)),
            value: value.into(),
        }
    }

    fn get_first_rule(css: &str) -> CSSStyleRule {
        let mut stylesheet = CSSStyleSheet::new();
        parse_css_into_stylesheet(css, &mut stylesheet);
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

        dom.set_parent(p_node, div_node)
            .expect("Failed to set parent");
        dom.set_first_child(div_node, Some(p_node))
            .expect("Failed to set first child");

        let rule_p = get_first_rule("p { color: red; }");
        assert!(
            matches_selector(p_node, &dom, &rule_p),
            "Should match direct 'p' tag"
        );

        let rule_card = get_first_rule(".card { margin: 0; }");
        assert!(
            matches_selector(p_node, &dom, &rule_card),
            "Should match '.card' class"
        );

        let rule_active = get_first_rule(".active { display: block; }");
        assert!(
            matches_selector(p_node, &dom, &rule_active),
            "Should match '.active' class"
        );

        let rule_fail = get_first_rule(".button { padding: 5px; }");
        assert!(
            !matches_selector(p_node, &dom, &rule_fail),
            "Should NOT match missing '.button' class"
        );

        let rule_descendant = get_first_rule("div#main .card { border: 1px; }");
        assert!(
            matches_selector(p_node, &dom, &rule_descendant),
            "Should match descendant 'div#main .card'"
        );

        let rule_descendant_fail = get_first_rule("span#main .card { border: 1px; }");
        assert!(
            !matches_selector(p_node, &dom, &rule_descendant_fail),
            "Should NOT match if parent tag is wrong"
        );
    }

    #[test]
    fn test_deep_descendant_matching() {
        let mut dom = Dom::new();
        let div_tag = blink_dom::get_tag_index("div");
        let section_tag = blink_dom::get_tag_index("section");
        let p_tag = blink_dom::get_tag_index("p");

        // Hierarchy: <div id="main"> -> <section class="container"> -> <p class="card">
        let div_node = dom.add_element(
            div_tag,
            Some(vec![make_attr("id", "main")].into_boxed_slice()),
        );
        let section_node = dom.add_element(
            section_tag,
            Some(vec![make_attr("class", "container")].into_boxed_slice()),
        );
        let p_node = dom.add_element(
            p_tag,
            Some(vec![make_attr("class", "card")].into_boxed_slice()),
        );

        // Wire the pointers
        dom.set_parent(section_node, div_node)
            .expect("Failed to set parent for section");
        dom.set_first_child(div_node, Some(section_node))
            .expect("Failed to set section as first child for div");

        dom.set_parent(p_node, section_node)
            .expect("Failed to set parent for p");
        dom.set_first_child(section_node, Some(p_node))
            .expect("Failed to set p as first child for section");

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

        // Failing missing ancestor
        let rule_fail = get_first_rule("nav .card { color: blue; }");
        assert!(
            !matches_selector(p_node, &dom, &rule_fail),
            "Should fail if the ancestor tree exhausts without finding a 'nav'"
        );
    }

    #[test]
    fn test_attributes_survive_text_nodes() {
        let mut dom = Dom::new();
        let div = get_tag_index("div");
        let _text = dom.add_text("blah");
        let el = dom.add_element(div, Some(vec![make_attr("id", "x")].into_boxed_slice()));
        assert!(
            dom.get_attributes(el)
                .unwrap()
                .iter()
                .any(|a| a.value.as_ref() == "x")
        );
    }

    #[test]
    fn test_attributes_survive_br_and_text() {
        let mut dom = Dom::new();
        let br = get_tag_index("br");
        let div = get_tag_index("div");
        dom.add_element(br, None);
        dom.add_text("blah");
        let el = dom.add_element(div, Some(vec![make_attr("id", "x")].into_boxed_slice()));
        assert!(
            dom.get_attributes(el)
                .unwrap()
                .iter()
                .any(|a| a.value.as_ref() == "x")
        );
    }
}
