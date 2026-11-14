// https://drafts.csswg.org/css-display/#the-display-properties
// https://developer.mozilla.org/en-US/docs/Web/CSS/Reference/Properties/display
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DisplayOutside {
    Block,
    Inline,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DisplayInside {
    Flow,
    FlowRoot,
    Table,
    Flex,
    Grid,
    GridLanes,
    Ruby,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Display {
    None,
    Box {
        outside: DisplayOutside,
        inside: DisplayInside,
        list_item: bool,
    },
}

impl Default for Display {
    fn default() -> Self {
        Display::INLINE
    }
}

impl Display {
    pub const BLOCK: Self = Display::Box {
        outside: DisplayOutside::Block,
        inside: DisplayInside::Flow,
        list_item: false,
    };
    pub const INLINE: Self = Display::Box {
        outside: DisplayOutside::Inline,
        inside: DisplayInside::Flow,
        list_item: false,
    };
    pub const INLINE_BLOCK: Self = Display::Box {
        outside: DisplayOutside::Inline,
        inside: DisplayInside::FlowRoot,
        list_item: false,
    };

    pub fn outside(self) -> Option<DisplayOutside> {
        match self {
            Display::Box { outside, .. } => Some(outside),
            _ => None,
        }
    }

    pub fn inside(self) -> Option<DisplayInside> {
        match self {
            Display::Box { inside, .. } => Some(inside),
            _ => None,
        }
    }

    pub fn is_none(self) -> bool {
        self == Display::None
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum BorderStyle {
    #[default]
    None,
    Solid,
}

#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct BorderSide {
    pub color: Option<Color>,
    pub width: f32,
    pub style: BorderStyle,
}

#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct EdgeSizes<T = f32> {
    pub top: T,
    pub right: T,
    pub bottom: T,
    pub left: T,
}

impl<T: Copy> EdgeSizes<T> {
    pub fn all(value: T) -> Self {
        Self {
            top: value,
            right: value,
            bottom: value,
            left: value,
        }
    }
}

impl EdgeSizes<f32> {
    pub fn horizontal(self) -> f32 {
        self.left + self.right
    }

    pub fn vertical(self) -> f32 {
        self.top + self.bottom
    }
}

impl EdgeSizes<BorderSide> {
    pub fn widths(self) -> EdgeSizes<f32> {
        let effective = |s: BorderSide| {
            if s.style == BorderStyle::None {
                0.0
            } else {
                s.width
            }
        };

        EdgeSizes {
            top: effective(self.top),
            right: effective(self.right),
            bottom: effective(self.bottom),
            left: effective(self.left),
        }
    }
}

#[derive(Default, Debug, Clone)]
pub struct ComputedStyle {
    pub display: Display,
    pub width: Option<f32>,
    pub height: Option<f32>,
    pub margin: EdgeSizes,
    pub padding: EdgeSizes,
    pub border: EdgeSizes<BorderSide>,
}

impl ComputedStyle {
    pub fn new(display: Display, width: Option<f32>, height: Option<f32>) -> Self {
        ComputedStyle {
            display,
            width,
            height,
            ..Default::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_edge_sizes_helpers() {
        let uniform = EdgeSizes::all(12.0);
        assert_eq!(uniform.horizontal(), 24.0);
        assert_eq!(uniform.vertical(), 24.0);

        let asymmetric = EdgeSizes {
            top: 1.0,
            right: 2.0,
            bottom: 3.0,
            left: 4.0,
        };
        assert_eq!(asymmetric.horizontal(), 6.0);
        assert_eq!(asymmetric.vertical(), 4.0);

        let borders = EdgeSizes {
            top: BorderSide {
                color: None,
                width: 5.0,
                style: BorderStyle::Solid,
            },
            right: BorderSide {
                color: None,
                width: 8.0,
                style: BorderStyle::None,
            },
            bottom: BorderSide::default(),
            left: BorderSide {
                color: None,
                width: 2.5,
                style: BorderStyle::Solid,
            },
        };

        assert_eq!(
            borders.widths(),
            EdgeSizes {
                top: 5.0,
                right: 0.0,
                bottom: 0.0,
                left: 2.5,
            }
        );
    }
}
