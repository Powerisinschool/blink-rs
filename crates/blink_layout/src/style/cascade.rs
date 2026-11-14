use super::types::{BorderSide, BorderStyle, Color, ComputedStyle, Display, EdgeSizes};
use blink_css::CSSStyleRule;

pub(super) fn cascade_properties(
    matched_rules: &mut Vec<&CSSStyleRule>,
    display: Display,
) -> ComputedStyle {
    matched_rules.sort_by_key(|a| a.specificity());

    let mut style = ComputedStyle::new(display, None, None);

    for rule in matched_rules {
        let props = &rule.properties;

        if let Some(d) = props.get("display").and_then(|v| parse_display(v)) {
            style.display = d;
        }

        if let Some(w) = props.get("width") {
            style.width = parse_px_value(w);
        }

        if let Some(h) = props.get("height") {
            style.height = parse_px_value(h);
        }

        apply_edge_properties(
            props,
            "margin",
            ["margin-top", "margin-right", "margin-bottom", "margin-left"],
            true,
            &mut style.margin,
        );

        apply_edge_properties(
            props,
            "padding",
            [
                "padding-top",
                "padding-right",
                "padding-bottom",
                "padding-left",
            ],
            false,
            &mut style.padding,
        );

        apply_border_properties(props, &mut style.border)
    }

    style
}

fn apply_edge_properties(
    props: &std::collections::HashMap<String, String>,
    shorthand: &str,
    sides: [&str; 4],
    allow_negative: bool,
    target: &mut EdgeSizes<f32>,
) {
    let clamp = |v: f32| if allow_negative { v } else { v.max(0.0) };
    if let Some(all) = props
        .get(shorthand)
        .and_then(|v| parse_edge_sizes(v, allow_negative))
    {
        *target = all;
    }
    if let Some(v) = props.get(sides[0]).and_then(|v| parse_px_value(v)) {
        target.top = clamp(v);
    }
    if let Some(v) = props.get(sides[1]).and_then(|v| parse_px_value(v)) {
        target.right = clamp(v);
    }
    if let Some(v) = props.get(sides[2]).and_then(|v| parse_px_value(v)) {
        target.bottom = clamp(v);
    }
    if let Some(v) = props.get(sides[3]).and_then(|v| parse_px_value(v)) {
        target.left = clamp(v);
    }
}

fn apply_border_properties(
    props: &std::collections::HashMap<String, String>,
    border: &mut EdgeSizes<BorderSide>,
) {
    if let Some(side) = props.get("border").and_then(|v| parse_border_shorthand(v)) {
        *border = EdgeSizes::all(side);
    }
    if let Some(w) = props
        .get("border-width")
        .and_then(|v| parse_edge_sizes(v, false))
    {
        border.top.width = w.top;
        border.right.width = w.right;
        border.bottom.width = w.bottom;
        border.left.width = w.left;
    }
    if let Some(s) = props.get("border-style")
        && let Some(parsed) = s
            .split_whitespace()
            .map(parse_border_style)
            .collect::<Option<Vec<_>>>()
        && let Some(edges) = expand_four_sides(&parsed)
    {
        border.top.style = edges.top;
        border.right.style = edges.right;
        border.bottom.style = edges.bottom;
        border.left.style = edges.left;
    }
    if let Some(c) = props.get("border-color")
        && let Some(parsed) = c
            .split_whitespace()
            .map(parse_border_color)
            .collect::<Option<Vec<_>>>()
        && let Some(edges) = expand_four_sides(&parsed)
    {
        border.top.color = edges.top;
        border.right.color = edges.right;
        border.bottom.color = edges.bottom;
        border.left.color = edges.left;
    }

    let sides = [
        (
            "border-top",
            "border-top-width",
            "border-top-style",
            "border-top-color",
            &mut border.top,
        ),
        (
            "border-right",
            "border-right-width",
            "border-right-style",
            "border-right-color",
            &mut border.right,
        ),
        (
            "border-bottom",
            "border-bottom-width",
            "border-bottom-style",
            "border-bottom-color",
            &mut border.bottom,
        ),
        (
            "border-left",
            "border-left-width",
            "border-left-style",
            "border-left-color",
            &mut border.left,
        ),
    ];

    for (shorthand, w_key, s_key, c_key, target) in sides {
        if let Some(side) = props.get(shorthand).and_then(|v| parse_border_shorthand(v)) {
            *target = side;
        }
        if let Some(w) = props.get(w_key).and_then(|v| parse_px_value(v)) {
            target.width = w.max(0.0);
        }
        if let Some(s) = props.get(s_key).and_then(|v| parse_border_style(v)) {
            target.style = s;
        }
        if let Some(c) = props.get(c_key).and_then(|v| parse_border_color(v)) {
            target.color = c;
        }
    }
}

fn parse_display(value: &str) -> Option<Display> {
    match value.trim() {
        "none" => Some(Display::None),
        "block" => Some(Display::BLOCK),
        "inline" => Some(Display::INLINE),
        "inline-block" => Some(Display::INLINE_BLOCK),
        _ => None,
    }
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

fn expand_four_sides<T: Copy>(values: &[T]) -> Option<EdgeSizes<T>> {
    match *values {
        [a] => Some(EdgeSizes::all(a)),
        [v, h] => Some(EdgeSizes {
            top: v,
            right: h,
            bottom: v,
            left: h,
        }),
        [t, h, b] => Some(EdgeSizes {
            top: t,
            right: h,
            bottom: b,
            left: h,
        }),
        [t, r, b, l] => Some(EdgeSizes {
            top: t,
            right: r,
            bottom: b,
            left: l,
        }),
        _ => None,
    }
}

fn parse_edge_sizes(values: &str, allow_negative: bool) -> Option<EdgeSizes<f32>> {
    let clamp = |v: f32| if allow_negative { v } else { v.max(0.0) };
    let parsed_values: Vec<f32> = values
        .split_whitespace()
        .map(|part| parse_px_value(part).map(clamp))
        .collect::<Option<_>>()?;

    expand_four_sides(&parsed_values)
}

fn parse_border_style(value: &str) -> Option<BorderStyle> {
    match value.trim() {
        "none" | "hidden" => Some(BorderStyle::None),
        "solid" => Some(BorderStyle::Solid),
        _ => None,
    }
}

fn parse_hex_color(hex_str: &str) -> Option<Color> {
    let color = match hex_str.len() {
        3 => {
            let r = u8::from_str_radix(&hex_str[0..1], 16).ok()? * 17;
            let g = u8::from_str_radix(&hex_str[1..2], 16).ok()? * 17;
            let b = u8::from_str_radix(&hex_str[2..3], 16).ok()? * 17;
            Color { r, g, b, a: 255 }
        }
        6 => {
            let r = u8::from_str_radix(&hex_str[0..2], 16).ok()?;
            let g = u8::from_str_radix(&hex_str[2..4], 16).ok()?;
            let b = u8::from_str_radix(&hex_str[4..6], 16).ok()?;
            Color { r, g, b, a: 255 }
        }
        8 => {
            let r = u8::from_str_radix(&hex_str[0..2], 16).ok()?;
            let g = u8::from_str_radix(&hex_str[2..4], 16).ok()?;
            let b = u8::from_str_radix(&hex_str[4..6], 16).ok()?;
            let a = u8::from_str_radix(&hex_str[6..8], 16).ok()?;
            Color { r, g, b, a }
        }
        _ => return None,
    };

    Some(color)
}

fn parse_border_color(value: &str) -> Option<Option<Color>> {
    let v = value.trim();
    if v.eq_ignore_ascii_case("currentcolor") {
        return Some(None);
    }

    if v.eq_ignore_ascii_case("transparent") {
        return Some(Some(Color {
            r: 0,
            g: 0,
            b: 0,
            a: 0,
        }));
    }

    if let Some(hex_str) = v.strip_prefix('#') {
        return parse_hex_color(hex_str).map(Some);
    }

    let named = match v.to_ascii_lowercase().as_str() {
        "black" => Color {
            r: 0,
            g: 0,
            b: 0,
            a: 255,
        },
        "white" => Color {
            r: 255,
            g: 255,
            b: 255,
            a: 255,
        },
        "red" => Color {
            r: 255,
            g: 0,
            b: 0,
            a: 255,
        },
        "green" => Color {
            r: 0,
            g: 128,
            b: 0,
            a: 255,
        },
        "blue" => Color {
            r: 0,
            g: 0,
            b: 255,
            a: 255,
        },
        _ => return None,
    };

    Some(Some(named))
}

fn parse_border_shorthand(value: &str) -> Option<BorderSide> {
    let mut side = BorderSide {
        width: 3.0, // CSS "medium" default when width is omitted
        style: BorderStyle::None,
        color: None,
    };
    let mut saw_any = false;
    for part in value.split_whitespace() {
        saw_any = true;
        if let Some(s) = parse_border_style(part) {
            side.style = s;
        } else if let Some(w) = parse_px_value(part) {
            side.width = w.max(0.0);
        } else {
            let c = parse_border_color(part)?;
            side.color = c;
        }
    }
    saw_any.then_some(side)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_px_value_edge_cases() {
        assert_eq!(parse_px_value("100px"), Some(100.0));
        assert_eq!(parse_px_value("50.5px"), Some(50.5));

        assert_eq!(parse_px_value("  200  px  "), Some(200.0));
        assert_eq!(parse_px_value(" 300 "), Some(300.0));

        // Unhandled units and keywords should safely fall back to None (auto)
        assert_eq!(parse_px_value("auto"), None);
        assert_eq!(parse_px_value("100vw"), None);
        assert_eq!(parse_px_value("inherit"), None);
    }

    #[test]
    fn test_parse_display() {
        assert_eq!(parse_display("none"), Some(Display::None));
        assert_eq!(parse_display("block"), Some(Display::BLOCK));
        assert_eq!(parse_display("inline"), Some(Display::INLINE));
        assert_eq!(parse_display("inline-block"), Some(Display::INLINE_BLOCK));
        assert_eq!(parse_display("unknown"), None);
    }

    #[test]
    fn test_parse_border_color() {
        // Hex Color
        assert_eq!(
            parse_border_color("#ff00ff"),
            Some(Some(Color {
                r: 255,
                g: 0,
                b: 255,
                a: 255
            }))
        );

        // Named Color
        assert_eq!(
            parse_border_color("white"),
            Some(Some(Color {
                r: 255,
                g: 255,
                b: 255,
                a: 255
            }))
        );

        // Transparent
        assert_eq!(
            parse_border_color("transparent"),
            Some(Some(Color {
                r: 0,
                g: 0,
                b: 0,
                a: 0
            }))
        );

        // Inherited color
        assert_eq!(parse_border_color("currentcolor"), Some(None));
    }

    #[test]
    fn test_parse_edge_sizes_shorthands_and_clamping() {
        // 1, 2, 3, and 4 value expansions
        assert_eq!(parse_edge_sizes("10px", true), Some(EdgeSizes::all(10.0)));
        assert_eq!(
            parse_edge_sizes("10px 20px", true),
            Some(EdgeSizes {
                top: 10.0,
                right: 20.0,
                bottom: 10.0,
                left: 20.0,
            })
        );
        assert_eq!(
            parse_edge_sizes("10px 20px 30px", true),
            Some(EdgeSizes {
                top: 10.0,
                right: 20.0,
                bottom: 30.0,
                left: 20.0,
            })
        );
        assert_eq!(
            parse_edge_sizes("1px 2px 3px 4px", true),
            Some(EdgeSizes {
                top: 1.0,
                right: 2.0,
                bottom: 3.0,
                left: 4.0,
            })
        );

        // Invalid counts
        assert_eq!(parse_edge_sizes("", true), None);
        assert_eq!(parse_edge_sizes("1px 2px 3px 4px 5px", true), None);

        // Negative values allowed for margin, clamped to 0.0 for padding
        assert_eq!(
            parse_edge_sizes("-10px 5px -20px", true),
            Some(EdgeSizes {
                top: -10.0,
                right: 5.0,
                bottom: -20.0,
                left: 5.0,
            })
        );
        assert_eq!(
            parse_edge_sizes("-10px 5px -20px", false),
            Some(EdgeSizes {
                top: 0.0,
                right: 5.0,
                bottom: 0.0,
                left: 5.0,
            })
        );
    }

    #[test]
    fn test_parse_border_shorthand_and_colors() {
        // 3-digit and 8-digit hex colors
        assert_eq!(
            parse_border_color("#f0a"),
            Some(Some(Color {
                r: 255,
                g: 0,
                b: 170,
                a: 255,
            }))
        );
        assert_eq!(
            parse_border_color("#11223380"),
            Some(Some(Color {
                r: 0x11,
                g: 0x22,
                b: 0x33,
                a: 0x80,
            }))
        );

        // Case-insensitivity and invalid hex strings
        assert_eq!(parse_border_color("CURRENTCOLOR"), Some(None));
        assert_eq!(
            parse_border_color("ReD"),
            Some(Some(Color {
                r: 255,
                g: 0,
                b: 0,
                a: 255,
            }))
        );
        assert_eq!(parse_border_color("#12"), None);
        assert_eq!(parse_border_color("#12345"), None);
        assert_eq!(parse_border_color("#gggggg"), None);

        assert_eq!(
            parse_border_shorthand("2px solid #00ff00"),
            Some(BorderSide {
                width: 2.0,
                style: BorderStyle::Solid,
                color: Some(Color {
                    r: 0,
                    g: 255,
                    b: 0,
                    a: 255,
                }),
            })
        );
        assert_eq!(
            parse_border_shorthand("blue solid 4px"),
            Some(BorderSide {
                width: 4.0,
                style: BorderStyle::Solid,
                color: Some(Color {
                    r: 0,
                    g: 0,
                    b: 255,
                    a: 255,
                }),
            })
        );
        assert_eq!(
            parse_border_shorthand("solid"),
            Some(BorderSide {
                width: 3.0,
                style: BorderStyle::Solid,
                color: None,
            })
        );
        assert_eq!(
            parse_border_shorthand("hidden 5px"),
            Some(BorderSide {
                width: 5.0,
                style: BorderStyle::None,
                color: None,
            })
        );
        assert_eq!(parse_border_shorthand(""), None);
        assert_eq!(parse_border_shorthand("2px dotted red"), None);
    }
}
