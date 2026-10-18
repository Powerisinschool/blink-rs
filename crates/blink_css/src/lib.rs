use std::collections::HashMap;
use cssparser::{Parser, Token};

#[derive(Debug, Default)]
pub struct CSSStyleSheet {
    pub rules: Vec<CSSStyleRule>
}

#[derive(Debug)]
pub struct CSSStyleRule {
    pub selector: Vec<Selector>,
    pub properties: HashMap<String, String>
}

#[derive(Debug, PartialEq)]
pub enum Selector {
    Tag(String),
    ID(String),
    Class(String),
    Descendant,
    // DirectDescendant,
}

impl CSSStyleSheet {
    pub fn new() -> Self {
        Self {
            rules: Vec::new(),
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
                Selector::Class(_) => classes += 1,
                Selector::Tag(_) => tags += 1,
                _ => {}
            }
        }

        (ids, classes, tags)
    }
}

pub fn parse_css_into_stylesheet(input: &str, stylesheet: &mut CSSStyleSheet) -> Result<(), ()> {
    let mut parser = Parser::new(&input);

    while !parser.is_exhausted() {
        if let Err(_) = parse_next_rule(&mut parser, stylesheet) {
            let _ = parser.next();
        }
    }

    Ok(())
}

fn parse_next_rule(parser: &mut Parser, stylesheet: &mut CSSStyleSheet) -> Result<(), cssparser::ParseError<()>> {
    let mut selector: Vec<Selector> = Vec::new();

    loop {
        let token = parser.next_including_whitespace()?;
        match token {
            Token::CurlyBracketBlock => {
                let mut properties = HashMap::new();

                parser.parse_nested_block(|nested_parser| {
                    parse_properties(nested_parser, &mut properties)
                })?;

                match selector.last() {
                    Some(Selector::Descendant) => {
                        selector.pop();
                    },
                    _ => {}
                }

                stylesheet.rules.push(CSSStyleRule {
                    selector,
                    properties,
                });
                break;
            },
            Token::Ident(name) => selector.push(Selector::Tag(name.to_string())),
            Token::Delim(c) => {
                match c {
                    '.' => {
                        match parser.next()? {
                            Token::Ident(name) => selector.push(Selector::Class(name.to_string())),
                            _ => {}
                        }
                    },
                    _ => {}
                }
            },
            Token::WhiteSpace(_) => {
                if selector.is_empty() {
                    continue;
                }
                match selector.last() {
                    Some(Selector::Tag(_)) | Some(Selector::ID(_)) | Some(Selector::Class(_)) => {
                        selector.push(Selector::Descendant)
                    }
                    _ => {}
                }
            },
            Token::IDHash(name) => selector.push(Selector::ID(name.to_string())),
            _ => {}
        }
    }

    Ok(())
}

fn parse_properties(parser: &mut Parser, properties: &mut HashMap<String, String>) -> Result<(), cssparser::ParseError<()>> {
    while !parser.is_exhausted() {
        let name = match parser.next()? {
            Token::Ident(name) => name.to_string(),
            _ => continue,
        };

        if !matches!(parser.next()?, Token::Colon) {
            continue;
        }

        let mut value = String::new();
        while !parser.is_exhausted() {
            let token = parser.next_including_whitespace()?;
            if matches!(token, Token::Semicolon) {
                break;
            }

            match token {
                Token::Ident(v) => value.push_str(&v),
                Token::Number { value: num, .. } => value.push_str(&num.to_string()),
                Token::Dimension { value: num, unit, .. } => {
                    value.push_str(&num.to_string());
                    value.push_str(&unit);
                },
                Token::WhiteSpace(_) => value.push(' '),
                Token::IDHash(name) | Token::Hash(name) => {
                    value.push('#');
                    value.push_str(&name);
                },
                _ => {},
            }
        }

        properties.insert(name, value.trim().to_string());
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_input() {
        let mut stylesheet = CSSStyleSheet::new();
        parse_css_into_stylesheet("", &mut stylesheet).unwrap();
        assert!(stylesheet.rules.is_empty());
    }

    #[test]
    fn test_whitespace_handling() {
        let mut stylesheet = CSSStyleSheet::new();
        parse_css_into_stylesheet("   \n\t", &mut stylesheet).unwrap();
        assert!(stylesheet.rules.is_empty());
    }

    #[test]
    fn test_basic_element_rule() {
        let mut stylesheet = CSSStyleSheet::new();
        parse_css_into_stylesheet("p { color: red; }", &mut stylesheet).unwrap();

        assert_eq!(stylesheet.rules.len(), 1);
        assert_eq!(stylesheet.rules[0].selector.len(), 1);
        assert_eq!(stylesheet.rules[0].selector[0], Selector::Tag("p".to_string()));
        assert_eq!(stylesheet.rules[0].properties.get("color").unwrap(), "red");
    }

    #[test]
    fn test_multiple_properties() {
        let mut stylesheet = CSSStyleSheet::new();
        parse_css_into_stylesheet("div { width: 100px; height: 50px; }", &mut stylesheet).unwrap();

        assert_eq!(stylesheet.rules.len(), 1);
        assert_eq!(stylesheet.rules[0].properties.get("width").unwrap(), "100px");
        assert_eq!(stylesheet.rules[0].properties.get("height").unwrap(), "50px");
    }

    #[test]
    fn test_id_and_class_selectors() {
        let mut stylesheet = CSSStyleSheet::new();
        parse_css_into_stylesheet("div#main .card { margin: 10px; }", &mut stylesheet).unwrap();

        assert_eq!(stylesheet.rules.len(), 1);
        assert_eq!(stylesheet.rules[0].selector.len(), 4);
        assert_eq!(stylesheet.rules[0].selector[0], Selector::Tag("div".to_string()));
        assert_eq!(stylesheet.rules[0].selector[1], Selector::ID("main".to_string()));
        assert_eq!(stylesheet.rules[0].selector[2], Selector::Descendant);
        assert_eq!(stylesheet.rules[0].selector[3], Selector::Class("card".to_string()));
    }

    #[test]
    fn test_complex_property_values() {
        let mut stylesheet = CSSStyleSheet::new();
        parse_css_into_stylesheet(".button { border: 1px solid black; background-color: #f3f3f3; }", &mut stylesheet).unwrap();

        assert_eq!(stylesheet.rules[0].properties.get("border").unwrap(), "1px solid black");
        assert_eq!(stylesheet.rules[0].properties.get("background-color").unwrap(), "#f3f3f3");
    }

    #[test]
    fn test_multiple_rules() {
        let mut stylesheet = CSSStyleSheet::new();
        parse_css_into_stylesheet(
            "h1 { font-size: 24px; } \n p { color: blue; }",
            &mut stylesheet
        ).unwrap();

        assert_eq!(stylesheet.rules.len(), 2);
        assert_eq!(stylesheet.rules[0].selector.len(), 1);
        assert_eq!(stylesheet.rules[0].selector[0], Selector::Tag("h1".to_string()));
        assert_eq!(stylesheet.rules[0].properties.get("font-size").unwrap(), "24px");

        assert_eq!(stylesheet.rules[1].selector.len(), 1);
        assert_eq!(stylesheet.rules[1].selector[0], Selector::Tag("p".to_string()));
        assert_eq!(stylesheet.rules[1].properties.get("color").unwrap(), "blue");
    }
}
