mod cascade;
mod matching;
mod types;

pub use types::{
    BorderSide, BorderStyle, Color, ComputedStyle, Display, DisplayInside, DisplayOutside,
    EdgeSizes,
};

use blink_css::CSSStyleSheet;
use blink_dom::{Dom, DomNodeId, get_tag_name};

pub fn compute_style(node_id: DomNodeId, dom: &Dom, stylesheet: &CSSStyleSheet) -> ComputedStyle {
    let mut rules = matching::get_matching_rules(node_id, dom, stylesheet);

    cascade::cascade_properties(&mut rules, default_display(node_id, dom))
}

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
    use blink_css::parse_css_into_stylesheet;
    use matching::tests::{DIV_TAG_INDEX, make_attr};

    #[test]
    fn test_compute_style() {
        let mut dom = Dom::new();

        dom.ensure_tag_registered(DIV_TAG_INDEX);

        let div_attrs = vec![make_attr("id", "main")].into_boxed_slice();
        let div_node = dom.add_element(DIV_TAG_INDEX, Some(div_attrs));

        let mut stylesheet = CSSStyleSheet::new();
        parse_css_into_stylesheet("div { width: 100px; height: 100px; }", &mut stylesheet);

        let computed_style = compute_style(div_node, &dom, &stylesheet);
        assert_eq!(computed_style.width, Some(100.0));
        assert_eq!(computed_style.height, Some(100.0));
    }

    #[test]
    fn test_compute_style_cascade_overrides() {
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

        let computed = compute_style(div_node, &dom, &stylesheet);

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
    fn test_compute_style_margin_and_padding_cascade() {
        let mut dom = Dom::new();
        dom.ensure_tag_registered(DIV_TAG_INDEX);

        let attrs = vec![make_attr("class", "box")].into_boxed_slice();
        let div_node = dom.add_element(DIV_TAG_INDEX, Some(attrs));

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

        let computed = compute_style(div_node, &dom, &stylesheet);

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

        let computed = compute_style(div_node, &dom, &stylesheet);

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
