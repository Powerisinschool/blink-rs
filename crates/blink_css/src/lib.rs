use cssparser::{ParseError, Parser, Token};
use std::fmt::Formatter;

#[derive(Debug, Clone)]
pub struct Declaration {
    pub name: String,
    pub value: Box<[ValuePart]>,
}

#[derive(Debug)]
pub struct DeclarationSpan {
    pub start: usize,
    pub end: usize,
}

#[derive(Debug)]
pub struct CSSStyleRule {
    pub selector: Box<[Selector]>,
    pub properties_span: DeclarationSpan,
    pub custom_properties_span: DeclarationSpan,
}

#[derive(Debug, Default)]
pub struct CSSStyleSheet {
    pub rules: Vec<CSSStyleRule>,
    declarations: Vec<Declaration>,
}

impl CSSStyleSheet {
    pub fn push_rule(
        &mut self,
        selectors: Vec<Box<[Selector]>>,
        properties: impl IntoIterator<Item = Declaration>,
        custom_properties: impl IntoIterator<Item = Declaration>,
    ) {
        if selectors.is_empty() {
            return;
        }

        let properties_start = self.declarations.len();
        self.declarations.extend(properties);
        let properties_end = self.declarations.len();

        let custom_properties_start = self.declarations.len();
        self.declarations.extend(custom_properties);
        let custom_properties_end = self.declarations.len();

        for selector in selectors {
            self.rules.push(CSSStyleRule {
                selector,
                properties_span: DeclarationSpan {
                    start: properties_start,
                    end: properties_end,
                },
                custom_properties_span: DeclarationSpan {
                    start: custom_properties_start,
                    end: custom_properties_end,
                },
            });
        }
    }
}

impl CSSStyleRule {
    pub fn properties<'a>(&self, stylesheet: &'a CSSStyleSheet) -> Option<&'a [Declaration]> {
        let DeclarationSpan { start, end } = self.properties_span;
        stylesheet.declarations.get(start..end)
    }

    pub fn custom_properties<'a>(
        &self,
        stylesheet: &'a CSSStyleSheet,
    ) -> Option<&'a [Declaration]> {
        let DeclarationSpan { start, end } = self.custom_properties_span;
        stylesheet.declarations.get(start..end)
    }

    pub fn contains_property(&self, stylesheet: &CSSStyleSheet, property_name: &str) -> bool {
        if let Some(properties) = self.properties(stylesheet) {
            properties.iter().any(|decl| decl.name == property_name)
        } else {
            false
        }
    }

    pub fn contains_custom_property(
        &self,
        stylesheet: &CSSStyleSheet,
        property_name: &str,
    ) -> bool {
        if let Some(custom_properties) = self.custom_properties(stylesheet) {
            custom_properties
                .iter()
                .any(|decl| decl.name == property_name)
        } else {
            false
        }
    }

    pub fn get_property_values<'a>(
        &self,
        stylesheet: &'a CSSStyleSheet,
        property_name: &str,
    ) -> Option<Vec<&'a [ValuePart]>> {
        let property_store = if property_name.starts_with("--") {
            self.custom_properties(stylesheet)
        } else {
            self.properties(stylesheet)
        };

        property_store.map(|properties| {
            properties
                .iter()
                .filter(|decl| decl.name == property_name)
                .map(|decl| decl.value.as_ref())
                .collect()
        })
    }
}

#[derive(Debug, PartialEq)]
pub enum Selector {
    Tag(String),
    ID(String),
    Class(String),
    PseudoClass(String),
    PseudoElement(String),
    Function(String, Vec<String>),
    Descendant,
    // DirectDescendant,
}

impl CSSStyleSheet {
    pub fn new() -> Self {
        Self {
            rules: Vec::new(),
            declarations: Vec::new(),
        }
    }
}

impl CSSStyleRule {
    // https://developer.mozilla.org/en-US/docs/Web/CSS/Guides/Cascade/Specificity
    // returns (ID, CLASS, TYPE)
    // Rust natively sorts tuples from left to right
    pub fn specificity(&self) -> (u32, u32, u32) {
        let mut ids = 0;
        let mut classes = 0;
        let mut tags = 0;

        for part in &self.selector {
            match part {
                Selector::ID(_) => ids += 1,
                Selector::Class(_) | Selector::PseudoClass(_) => classes += 1,
                Selector::Function(name, _) => match name.as_str() {
                    "nth-child" | "nth-last-child" | "nth-of-type" | "nth-last-of-type" => {
                        classes += 1
                    }
                    _ => {}
                },
                Selector::Tag(_) | Selector::PseudoElement(_) => tags += 1,
                _ => {}
            }
        }

        (ids, classes, tags)
    }
}

pub fn parse_css_into_stylesheet(input: &str, stylesheet: &mut CSSStyleSheet) {
    let mut parser = Parser::new(input);

    while !parser.is_exhausted() {
        let _ = parse_next_rule(&mut parser, stylesheet);
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum CustomCSSParseError {
    InvalidRule,
}

fn skip_invalid_rule(parser: &mut Parser) -> Result<(), ParseError<CustomCSSParseError>> {
    while !parser.is_exhausted() {
        if matches!(parser.next()?, Token::CurlyBracketBlock) {
            break;
        }
    }
    Err(ParseError::custom(CustomCSSParseError::InvalidRule))
}

fn parse_next_rule(
    parser: &mut Parser,
    stylesheet: &mut CSSStyleSheet,
) -> Result<(), ParseError<CustomCSSParseError>> {
    let mut selectors: Vec<Box<[Selector]>> = Vec::new();
    let mut selector: Vec<Selector> = Vec::new();

    let mut seen_pseudo_element = false;

    loop {
        let token = parser.next_including_whitespace()?;

        if seen_pseudo_element
            && !matches!(
                token,
                Token::WhiteSpace(_) | Token::Comma | Token::CurlyBracketBlock
            )
        {
            return skip_invalid_rule(parser);
        }

        match token {
            Token::CurlyBracketBlock => {
                let mut properties = vec![];
                let mut custom_properties = vec![];

                parser.parse_nested_block(|nested_parser| {
                    parse_properties(nested_parser, &mut properties, &mut custom_properties)
                })?;

                if let Some(Selector::Descendant) = selector.last() {
                    selector.pop();
                }

                if !selector.is_empty() {
                    selectors.push(std::mem::take(&mut selector).into_boxed_slice());
                }

                stylesheet.push_rule(selectors, properties, custom_properties);

                break;
            }
            Token::Comma => {
                if let Some(Selector::Descendant) = selector.last() {
                    selector.pop();
                }
                if selector.is_empty() {
                    return skip_invalid_rule(parser);
                }
                selectors.push(std::mem::take(&mut selector).into_boxed_slice());
                seen_pseudo_element = false;
            }
            Token::WhiteSpace(_) => {
                if selector.is_empty() {
                    continue;
                }
                match selector.last() {
                    Some(Selector::Tag(_))
                    | Some(Selector::ID(_))
                    | Some(Selector::Class(_))
                    | Some(Selector::PseudoClass(_))
                    | Some(Selector::Function(..)) => selector.push(Selector::Descendant),
                    _ => {}
                }
            }
            Token::Colon => {
                let next_token = parser.next_including_whitespace()?.clone();
                match next_token {
                    Token::Colon => {
                        if let Token::Ident(name) = parser.next_including_whitespace()? {
                            selector.push(Selector::PseudoElement(name.to_ascii_lowercase()));
                            seen_pseudo_element = true;
                        }
                    }
                    Token::Ident(name) => {
                        let name = name.to_ascii_lowercase();
                        match name.as_str() {
                            "before" | "after" | "first-line" | "first-letter" => {
                                selector.push(Selector::PseudoElement(name.to_string()));
                                seen_pseudo_element = true;
                            }
                            _ => {
                                selector.push(Selector::PseudoClass(name.to_string()));
                            }
                        }
                    }
                    Token::Function(name) => {
                        let mut args = Vec::new();
                        parser.parse_nested_block(|nested| {
                            while !nested.is_exhausted() {
                                match nested.next()? {
                                    Token::Ident(arg) => args.push(arg.to_string()),
                                    Token::Number { value, .. } => args.push(value.to_string()),
                                    Token::Comma => continue,
                                    _ => break,
                                }
                            }
                            Ok(())
                        })?;
                        selector.push(Selector::Function(name.to_ascii_lowercase(), args));
                    }
                    _ => {
                        return skip_invalid_rule(parser);
                    }
                }
            }
            Token::Ident(name) => selector.push(Selector::Tag(name.to_string())),
            Token::Delim('.') => {
                if let Token::Ident(name) = parser.next_including_whitespace()? {
                    selector.push(Selector::Class(name.to_string()))
                }
            }
            Token::IDHash(name) => selector.push(Selector::ID(name.to_string())),
            _ => {}
        }
    }

    Ok(())
}

fn parse_properties(
    parser: &mut Parser,
    properties: &mut Vec<Declaration>,
    custom_properties: &mut Vec<Declaration>,
) -> Result<(), ParseError<CustomCSSParseError>> {
    while !parser.is_exhausted() {
        let name = match parser.next()? {
            Token::Ident(name) => name.to_string(),
            _ => continue,
        };

        if !matches!(parser.next()?, Token::Colon) {
            continue;
        }

        let mut value = vec![];
        parse_property_value(parser, &mut value)?;

        let declaration = Declaration {
            name: name.trim().to_string(),
            value: value.into_boxed_slice(),
        };

        if declaration.name.starts_with("--") {
            custom_properties.push(declaration);
        } else {
            properties.push(declaration);
        }
    }

    Ok(())
}

#[derive(Debug, Clone)]
pub enum ValuePart {
    Literal(String),
    Var {
        name: String,
        fallback: Option<Box<[ValuePart]>>,
    },
    Function {
        name: String,
        args: Option<Box<[ValuePart]>>,
    },
}

impl ValuePart {
    /// Returns the raw string for a single-literal value, or None if the value
    /// contains any `var()` references.
    pub fn as_pure_literal(value: &[ValuePart]) -> Option<&str> {
        match value {
            [ValuePart::Literal(s)] => Some(s),
            [] => Some(""),
            _ => None,
        }
    }
}

impl std::fmt::Display for ValuePart {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ValuePart::Var { name, fallback } => {
                write!(f, "var({}", name)?;
                if let Some(fallback) = fallback {
                    write!(
                        f,
                        ", {}",
                        fallback.iter().map(|p| p.to_string()).collect::<String>()
                    )?;
                }
                f.write_str(")")
            }
            ValuePart::Function { name, args } => {
                f.write_str(name)?;
                if let Some(args) = args {
                    write!(
                        f,
                        "{}",
                        args.iter().map(|p| p.to_string()).collect::<String>()
                    )?;
                }
                f.write_str(")")
            }
            ValuePart::Literal(s) => f.write_str(s),
        }
    }
}

fn parse_property_value(
    parser: &mut Parser,
    parts: &mut Vec<ValuePart>,
) -> Result<(), ParseError<CustomCSSParseError>> {
    while !parser.is_exhausted() {
        let start = parser.position();
        let token = parser.next_including_whitespace()?.clone();

        match token {
            Token::Function(ref name) if name.eq_ignore_ascii_case("var") => {
                parts.push(parser.parse_nested_block(|n| {
                    let name = n.expect_ident()?.to_string();
                    let fallback = if n.try_parse(|n| n.expect_comma()).is_ok() {
                        let mut fb = vec![];
                        parse_property_value(n, &mut fb)?;
                        Some(fb.into_boxed_slice())
                    } else {
                        n.expect_exhausted()?;
                        None
                    };
                    Ok(ValuePart::Var { name, fallback })
                })?);
            }
            Token::Function(ref name) => {
                let name = name.to_string();
                let args = parser.parse_nested_block(|n| {
                    let mut args = vec![];
                    parse_property_value(n, &mut args)?;
                    Ok(args.into_boxed_slice())
                })?;
                parts.push(ValuePart::Function {
                    name,
                    args: Some(args),
                });
            }
            Token::ParenthesisBlock | Token::SquareBracketBlock | Token::CurlyBracketBlock => {
                parser.parse_nested_block(|n| parse_property_value(n, parts))?;
            }
            Token::Semicolon => break,
            Token::Comment(_) => {}
            Token::WhiteSpace(_) if parts.is_empty() => {} // Ignore leading whitespace
            _ => push_literal(parts, parser.slice_from(start)),
        }
    }

    Ok(())
}

fn push_literal(parts: &mut Vec<ValuePart>, literal: &str) {
    if let Some(ValuePart::Literal(last)) = parts.last_mut() {
        last.push_str(literal);
    } else {
        parts.push(ValuePart::Literal(literal.to_string()));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_input() {
        let mut stylesheet = CSSStyleSheet::new();
        parse_css_into_stylesheet("", &mut stylesheet);
        assert!(stylesheet.rules.is_empty());
    }

    #[test]
    fn test_whitespace_handling() {
        let mut stylesheet = CSSStyleSheet::new();
        parse_css_into_stylesheet("   \n\t", &mut stylesheet);
        assert!(stylesheet.rules.is_empty());
    }

    fn assert_value_contains_literal(value: &[ValuePart], expected: &str, message: Option<&str>) {
        let msg = if let Some(message) = message {
            message.to_string()
        } else {
            format!(
                "Expected value to contain a literal of '{}', but got '{:?}'",
                expected, value
            )
        };
        assert!(
            ValuePart::as_pure_literal(value).is_some_and(|v| v.contains(expected)),
            "{}",
            msg
        );
    }

    #[test]
    fn test_basic_element_rule() {
        let mut stylesheet = CSSStyleSheet::new();
        parse_css_into_stylesheet("p { color: red; }", &mut stylesheet);

        assert_eq!(stylesheet.rules.len(), 1);
        assert_eq!(stylesheet.rules[0].selector.len(), 1);
        assert_eq!(
            stylesheet.rules[0].selector[0],
            Selector::Tag("p".to_string())
        );
        assert_value_contains_literal(
            stylesheet.rules[0]
                .get_property_values(&stylesheet, "color")
                .unwrap()[0],
            "red",
            None,
        );
    }

    #[test]
    fn test_multiple_properties() {
        let mut stylesheet = CSSStyleSheet::new();
        parse_css_into_stylesheet("div { width: 100px; height: 50px; }", &mut stylesheet);

        assert_eq!(stylesheet.rules.len(), 1);
        assert_value_contains_literal(
            stylesheet.rules[0]
                .get_property_values(&stylesheet, "width")
                .unwrap()[0],
            "100px",
            None,
        );
        assert_value_contains_literal(
            stylesheet.rules[0]
                .get_property_values(&stylesheet, "height")
                .unwrap()[0],
            "50px",
            None,
        );
    }

    #[test]
    fn test_id_and_class_selectors() {
        let mut stylesheet = CSSStyleSheet::new();
        parse_css_into_stylesheet("div#main .card { margin: 10px; }", &mut stylesheet);

        assert_eq!(stylesheet.rules.len(), 1);
        assert_eq!(stylesheet.rules[0].selector.len(), 4);
        assert_eq!(
            stylesheet.rules[0].selector[0],
            Selector::Tag("div".to_string())
        );
        assert_eq!(
            stylesheet.rules[0].selector[1],
            Selector::ID("main".to_string())
        );
        assert_eq!(stylesheet.rules[0].selector[2], Selector::Descendant);
        assert_eq!(
            stylesheet.rules[0].selector[3],
            Selector::Class("card".to_string())
        );
    }

    #[test]
    fn test_root_selector() {
        let mut stylesheet = CSSStyleSheet::new();
        parse_css_into_stylesheet(":root { --main-color: #ff0000; }", &mut stylesheet);

        assert_eq!(stylesheet.rules.len(), 1);
        assert_eq!(stylesheet.rules[0].selector.len(), 1);
        assert_eq!(
            stylesheet.rules[0].selector[0],
            Selector::PseudoClass("root".to_string())
        );
        assert_value_contains_literal(
            stylesheet.rules[0]
                .get_property_values(&stylesheet, "--main-color")
                .unwrap()[0],
            "#ff0000",
            None,
        );
    }

    #[test]
    fn test_standard_pseudo_class() {
        let mut stylesheet = CSSStyleSheet::new();
        parse_css_into_stylesheet("a:hover { text-decoration: underline; }", &mut stylesheet);

        assert_eq!(stylesheet.rules.len(), 1);
        assert_eq!(stylesheet.rules[0].selector.len(), 2);
        assert_eq!(
            stylesheet.rules[0].selector[0],
            Selector::Tag("a".to_string())
        );
        assert_eq!(
            stylesheet.rules[0].selector[1],
            Selector::PseudoClass("hover".to_string())
        );
        assert_value_contains_literal(
            stylesheet.rules[0]
                .get_property_values(&stylesheet, "text-decoration")
                .unwrap()[0],
            "underline",
            None,
        );
    }

    #[test]
    fn test_complex_property_values() {
        let mut stylesheet = CSSStyleSheet::new();
        parse_css_into_stylesheet(
            ".button { border: 1px solid black; background-color: #f3f3f3; }",
            &mut stylesheet,
        );

        assert_value_contains_literal(
            stylesheet.rules[0]
                .get_property_values(&stylesheet, "border")
                .unwrap()[0],
            "1px solid black",
            None,
        );
        assert_value_contains_literal(
            stylesheet.rules[0]
                .get_property_values(&stylesheet, "background-color")
                .unwrap()[0],
            "#f3f3f3",
            None,
        );
    }

    #[test]
    fn parses_non_var_functions() {
        let mut ss = CSSStyleSheet::new();
        parse_css_into_stylesheet(
            "div { color: rgb(255, 0, 0); background: url(image.png); }",
            &mut ss,
        );
        let props = ss.rules[0].properties(&ss).unwrap();
        println!("{:?}", props[0].value);
        println!("{:?}", props[1].value);
    }

    #[test]
    fn parses_non_var_function_as_function_value_part() {
        let mut ss = CSSStyleSheet::new();
        parse_css_into_stylesheet("div { color: rgb(255, 0, 0); }", &mut ss);

        assert_eq!(ss.rules.len(), 1, "rule should not be dropped");
        let props = ss.rules[0].properties(&ss).unwrap();
        assert_eq!(props.len(), 1);

        match &props[0].value[..] {
            [ValuePart::Function { name, args }] => {
                assert_eq!(name, "rgb");
                assert!(args.is_some(), "rgb should have args parsed");
            }
            other => panic!("expected Function value part, got {:?}", other),
        }
    }

    #[test]
    fn splits_normal_and_custom_properties_at_parse_time() {
        let mut ss = CSSStyleSheet::new();
        parse_css_into_stylesheet(
            "div { width: 100px; --w: 100px; height: 50px; --h: 50px }",
            &mut ss,
        );

        let rule = &ss.rules[0];
        let normal = rule.properties(&ss).unwrap();
        let custom = rule.custom_properties(&ss).unwrap();

        assert_eq!(normal.len(), 2, "normal properties must not include --*");
        assert_eq!(custom.len(), 2, "custom properties must only contain --*");
        assert!(normal.iter().all(|d| !d.name.starts_with("--")));
        assert!(custom.iter().all(|d| d.name.starts_with("--")));
    }

    #[test]
    fn test_multiple_rules() {
        let mut stylesheet = CSSStyleSheet::new();
        parse_css_into_stylesheet(
            "h1 { font-size: 24px; } \n p { color: blue; }",
            &mut stylesheet,
        );

        assert_eq!(stylesheet.rules.len(), 2);
        assert_eq!(stylesheet.rules[0].selector.len(), 1);
        assert_eq!(
            stylesheet.rules[0].selector[0],
            Selector::Tag("h1".to_string())
        );
        assert_value_contains_literal(
            stylesheet.rules[0]
                .get_property_values(&stylesheet, "font-size")
                .unwrap()[0],
            "24px",
            None,
        );

        assert_eq!(stylesheet.rules[1].selector.len(), 1);
        assert_eq!(
            stylesheet.rules[1].selector[0],
            Selector::Tag("p".to_string())
        );
        assert_value_contains_literal(
            stylesheet.rules[1]
                .get_property_values(&stylesheet, "color")
                .unwrap()[0],
            "blue",
            None,
        );
    }
}
