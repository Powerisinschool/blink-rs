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

impl Color {
    pub fn new(r: u8, g: u8, b: u8, a: u8) -> Self {
        Color { r, g, b, a }
    }

    pub const TRANSPARENT: Self = Color {
        r: 0,
        g: 0,
        b: 0,
        a: 0,
    };

    pub fn is_transparent(&self) -> bool {
        self.a == 0
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SpecifiedColor {
    CurrentColor,
    Color(Color),
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum BorderStyle {
    #[default]
    None,
    Solid,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BorderSide {
    pub color: SpecifiedColor,
    pub width: f32,
    pub style: BorderStyle,
}

impl Default for BorderSide {
    fn default() -> Self {
        BorderSide {
            color: SpecifiedColor::CurrentColor,
            width: f32::NAN,
            style: BorderStyle::None,
        }
    }
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

impl<T: std::ops::Add<Output = T>> std::ops::Add for EdgeSizes<T> {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self {
            top: self.top + rhs.top,
            right: self.right + rhs.right,
            bottom: self.bottom + rhs.bottom,
            left: self.left + rhs.left,
        }
    }
}

impl EdgeSizes<BorderSide> {
    pub fn widths(self) -> EdgeSizes<f32> {
        let effective = |s: BorderSide| {
            if s.style == BorderStyle::None || s.width.is_nan() {
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
                color: SpecifiedColor::CurrentColor,
                width: 5.0,
                style: BorderStyle::Solid,
            },
            right: BorderSide {
                color: SpecifiedColor::CurrentColor,
                width: 8.0,
                style: BorderStyle::None,
            },
            bottom: BorderSide::default(),
            left: BorderSide {
                color: SpecifiedColor::CurrentColor,
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
