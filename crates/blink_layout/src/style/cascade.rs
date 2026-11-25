use super::types::{BorderSide, BorderStyle, Color, ComputedStyle, Display, EdgeSizes};
use blink_css::{CSSStyleRule, CSSStyleSheet};

pub(super) fn cascade_properties(
    matched_rules: &mut Vec<&CSSStyleRule>,
    stylesheet: &CSSStyleSheet,
    display: Display,
) -> ComputedStyle {
    matched_rules.sort_by_key(|a| a.specificity());

    let mut style = ComputedStyle::new(display, None, None);

    for rule in matched_rules {
        let Some(props) = rule.properties(stylesheet) else {
            continue; // Skip rules without properties
        };

        for declaration in props {
            let name = declaration.name.as_str();
            let value = declaration.value.as_str();

            match name {
                "display" => {
                    if let Some(d) = parse_display(value) {
                        style.display = d;
                    }
                }
                "width" => {
                    if let Some(w) = parse_px_value(value) {
                        style.width = Some(w);
                    }
                }
                "height" => {
                    if let Some(h) = parse_px_value(value) {
                        style.height = Some(h);
                    }
                }
                name if name.starts_with("margin") => {
                    apply_edge_declaration(name, "margin", value, true, &mut style.margin);
                }
                name if name.starts_with("padding") => {
                    apply_edge_declaration(name, "padding", value, false, &mut style.padding);
                }
                name if name.starts_with("border") => {
                    apply_border_properties(name, value, &mut style.border);
                }
                _ => {}
            }
        }
    }

    style
}

fn apply_edge_declaration(
    name: &str,
    prefix: &str,
    value: &str,
    allow_negative: bool,
    target: &mut EdgeSizes<f32>,
) {
    let clamp = |v: f32| if allow_negative { v } else { v.max(0.0) };

    match name.strip_prefix(prefix) {
        Some("") => {
            if let Some(all) = parse_edge_sizes(value, allow_negative) {
                *target = all;
            }
        }
        Some("-top") => {
            if let Some(v) = parse_px_value(value) {
                target.top = clamp(v);
            }
        }
        Some("-right") => {
            if let Some(v) = parse_px_value(value) {
                target.right = clamp(v);
            }
        }
        Some("-bottom") => {
            if let Some(v) = parse_px_value(value) {
                target.bottom = clamp(v);
            }
        }
        Some("-left") => {
            if let Some(v) = parse_px_value(value) {
                target.left = clamp(v);
            }
        }
        _ => (), // Not a relevant property for this edge
    }
}

fn apply_border_properties(name: &str, value: &str, border: &mut EdgeSizes<BorderSide>) {
    let Some(rest) = name.strip_prefix("border") else {
        return; // Not a border property
    };

    match rest {
        "" => {
            if let Some(side) = parse_border_shorthand(value) {
                *border = EdgeSizes::all(side);
            }
        }
        "-width" => {
            if let Some(w) = parse_edge_sizes(value, false) {
                border.top.width = w.top;
                border.right.width = w.right;
                border.bottom.width = w.bottom;
                border.left.width = w.left;
            }
        }
        "-style" => {
            if let Some(parsed) = value
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
        }
        "-color" => {
            if let Some(parsed) = value
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
        }
        _ => {
            let (side_rest, side_target) = if let Some(r) = rest.strip_prefix("-top") {
                (r, &mut border.top)
            } else if let Some(r) = rest.strip_prefix("-right") {
                (r, &mut border.right)
            } else if let Some(r) = rest.strip_prefix("-bottom") {
                (r, &mut border.bottom)
            } else if let Some(r) = rest.strip_prefix("-left") {
                (r, &mut border.left)
            } else {
                return; // Not a relevant property for border
            };

            match side_rest {
                "" => {
                    if let Some(side) = parse_border_shorthand(value) {
                        *side_target = side;
                    }
                }
                "-width" => {
                    if let Some(w) = parse_px_value(value) {
                        side_target.width = w;
                    }
                }
                "-style" => {
                    if let Some(s) = parse_border_style(value) {
                        side_target.style = s;
                    }
                }
                "-color" => {
                    if let Some(c) = parse_border_color(value) {
                        side_target.color = c;
                    }
                }
                _ => (), // Not a relevant property for this border edge
            }
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
        "silver" => Color {
            r: 192,
            g: 192,
            b: 192,
            a: 255,
        },
        "gray" | "grey" => Color {
            r: 128,
            g: 128,
            b: 128,
            a: 255,
        },
        "white" => Color {
            r: 255,
            g: 255,
            b: 255,
            a: 255,
        },
        "maroon" => Color {
            r: 128,
            g: 0,
            b: 0,
            a: 255,
        },
        "red" => Color {
            r: 255,
            g: 0,
            b: 0,
            a: 255,
        },
        "purple" => Color {
            r: 128,
            g: 0,
            b: 128,
            a: 255,
        },
        "fuchsia" | "magenta" => Color {
            r: 255,
            g: 0,
            b: 255,
            a: 255,
        },
        "green" => Color {
            r: 0,
            g: 128,
            b: 0,
            a: 255,
        },
        "lime" => Color {
            r: 0,
            g: 255,
            b: 0,
            a: 255,
        },
        "olive" => Color {
            r: 128,
            g: 128,
            b: 0,
            a: 255,
        },
        "yellow" => Color {
            r: 255,
            g: 255,
            b: 0,
            a: 255,
        },
        "navy" => Color {
            r: 0,
            g: 0,
            b: 128,
            a: 255,
        },
        "blue" => Color {
            r: 0,
            g: 0,
            b: 255,
            a: 255,
        },
        "teal" => Color {
            r: 0,
            g: 128,
            b: 128,
            a: 255,
        },
        "aqua" | "cyan" => Color {
            r: 0,
            g: 255,
            b: 255,
            a: 255,
        },
        "orange" => Color {
            r: 255,
            g: 165,
            b: 0,
            a: 255,
        }, // CSS 2.1 addition
        "transparent" => Color {
            r: 0,
            g: 0,
            b: 0,
            a: 0,
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
