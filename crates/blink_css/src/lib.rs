use cssparser::{ParseError, Parser, Token};

#[derive(Debug, Clone)]
pub struct Declaration {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Default)]
pub struct CSSStyleSheet {
    pub rules: Vec<CSSStyleRule>,
    properties: Vec<Vec<Declaration>>,
    custom_properties: Vec<Vec<Declaration>>,
}

impl CSSStyleSheet {
    pub fn push_rules(
        &mut self,
        selectors: Vec<Vec<Selector>>,
        properties: Vec<Declaration>,
        custom_properties: Vec<Declaration>,
    ) {
        if selectors.is_empty() {
            return;
        }

        let properties_id = self.properties.len();
        self.properties.push(properties);

        let custom_properties_id = self.custom_properties.len();
        self.custom_properties.push(custom_properties);

        for selector in selectors {
            self.rules.push(CSSStyleRule {
                selector,
                declarations_store_id: properties_id,
                custom_properties_store_id: custom_properties_id,
            });
        }
    }
}

#[derive(Debug)]
pub struct CSSStyleRule {
    pub selector: Vec<Selector>,
    pub declarations_store_id: usize,
    pub custom_properties_store_id: usize,
}

impl CSSStyleRule {
    pub fn properties<'a>(&self, stylesheet: &'a CSSStyleSheet) -> Option<&'a Vec<Declaration>> {
        stylesheet.properties.get(self.declarations_store_id)
    }

    pub fn custom_properties<'a>(
        &self,
        stylesheet: &'a CSSStyleSheet,
    ) -> Option<&'a Vec<Declaration>> {
        stylesheet
            .custom_properties
            .get(self.custom_properties_store_id)
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
    ) -> Option<Vec<&'a str>> {
        let property_store = if property_name.starts_with("--") {
            self.custom_properties(stylesheet)
        } else {
            self.properties(stylesheet)
        };

        property_store.map(|properties| {
            properties
                .iter()
                .filter(|decl| decl.name == property_name)
                .map(|decl| decl.value.as_str())
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
            properties: Vec::new(),
            custom_properties: Vec::new(),
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
    let mut selectors: Vec<Vec<Selector>> = Vec::new();
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
                    selectors.push(std::mem::take(&mut selector));
                }

                stylesheet.push_rules(selectors, properties, custom_properties);

                break;
            }
            Token::Comma => {
                if let Some(Selector::Descendant) = selector.last() {
                    selector.pop();
                }
                if selector.is_empty() {
                    return skip_invalid_rule(parser);
                }
                selectors.push(std::mem::take(&mut selector));
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

        let mut value = String::new();
        serialize_value_tokens(parser, &mut value)?;

        let declaration = Declaration {
            name: name.trim().to_string(),
            value: value.trim().to_string(),
        };

        if declaration.name.starts_with("--") {
            custom_properties.push(declaration);
        } else {
            properties.push(declaration);
        }
    }

    Ok(())
}

fn serialize_value_tokens(
    parser: &mut Parser,
    out: &mut String,
) -> Result<(), ParseError<CustomCSSParseError>> {
    while !parser.is_exhausted() {
        let token = parser.next_including_whitespace()?;

        match token {
            Token::Semicolon => break,
            Token::Ident(v) => out.push_str(v),
            Token::Number { value: num, .. } => out.push_str(&num.to_string()),
            Token::Dimension {
                value: num, unit, ..
            } => {
                out.push_str(&num.to_string());
                out.push_str(unit);
            }
            Token::Percentage { unit_value, .. } => {
                out.push_str(&(unit_value * 100.0).to_string());
                out.push('%');
            }
            Token::WhiteSpace(_) => out.push(' '),
            Token::Comma => out.push(','),
            Token::Delim(c) => out.push(*c),
            Token::IDHash(name) | Token::Hash(name) => {
                out.push('#');
                out.push_str(name);
            }
            Token::Function(name) => {
                out.push_str(name);
                out.push('(');
                parser.parse_nested_block(|nested| serialize_value_tokens(nested, out))?;
                out.push(')');
            }
            _ => {}
        }
    }

    Ok(())
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
        assert!(
            stylesheet.rules[0]
                .get_property_values(&stylesheet, "color")
                .unwrap()
                .contains(&"red")
        );
    }

    #[test]
    fn test_multiple_properties() {
        let mut stylesheet = CSSStyleSheet::new();
        parse_css_into_stylesheet("div { width: 100px; height: 50px; }", &mut stylesheet);

        assert_eq!(stylesheet.rules.len(), 1);
        assert!(
            stylesheet.rules[0]
                .get_property_values(&stylesheet, "width")
                .unwrap()
                .contains(&"100px")
        );
        assert!(
            stylesheet.rules[0]
                .get_property_values(&stylesheet, "height")
                .unwrap()
                .contains(&"50px")
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
        assert!(
            stylesheet.rules[0]
                .get_property_values(&stylesheet, "--main-color")
                .unwrap()
                .contains(&"#ff0000"),
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
        assert!(
            stylesheet.rules[0]
                .get_property_values(&stylesheet, "text-decoration")
                .unwrap()
                .contains(&"underline"),
        );
    }

    #[test]
    fn test_complex_property_values() {
        let mut stylesheet = CSSStyleSheet::new();
        parse_css_into_stylesheet(
            ".button { border: 1px solid black; background-color: #f3f3f3; }",
            &mut stylesheet,
        );

        assert!(
            stylesheet.rules[0]
                .get_property_values(&stylesheet, "border")
                .unwrap()
                .contains(&"1px solid black"),
        );
        assert!(
            stylesheet.rules[0]
                .get_property_values(&stylesheet, "background-color")
                .unwrap()
                .contains(&"#f3f3f3"),
        );
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
        assert!(
            stylesheet.rules[0]
                .get_property_values(&stylesheet, "font-size")
                .unwrap()
                .contains(&"24px"),
        );

        assert_eq!(stylesheet.rules[1].selector.len(), 1);
        assert_eq!(
            stylesheet.rules[1].selector[0],
            Selector::Tag("p".to_string())
        );
        assert!(
            stylesheet.rules[1]
                .get_property_values(&stylesheet, "color")
                .unwrap()
                .contains(&"blue"),
        );
    }
}
