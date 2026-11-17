#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct PhysicalSize {
    pub width: f32,
    pub height: f32,
}

impl PhysicalSize {
    pub fn new(width: f32, height: f32) -> Self {
        Self { width, height }
    }

    #[must_use]
    pub fn with_width(mut self, width: f32) -> Self {
        self.width = width;
        self
    }

    #[must_use]
    pub fn with_height(mut self, height: f32) -> Self {
        self.height = height;
        self
    }
}

impl From<(f32, f32)> for PhysicalSize {
    fn from(value: (f32, f32)) -> Self {
        Self {
            width: value.0,
            height: value.1,
        }
    }
}

impl From<(u32, u32)> for PhysicalSize {
    fn from(value: (u32, u32)) -> Self {
        Self {
            width: value.0 as f32,
            height: value.1 as f32,
        }
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct PhysicalOffset {
    pub x: f32,
    pub y: f32,
}

impl PhysicalOffset {
    pub fn new(x: f32, y: f32) -> PhysicalOffset {
        Self { x, y }
    }

    #[must_use]
    pub fn with_x(mut self, x: f32) -> Self {
        self.x = x;
        self
    }

    #[must_use]
    pub fn with_y(mut self, y: f32) -> Self {
        self.y = y;
        self
    }
}

impl std::ops::Add for PhysicalOffset {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self {
            x: self.x + other.x,
            y: self.y + other.y,
        }
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct PhysicalRect {
    pub origin: PhysicalOffset,
    pub size: PhysicalSize,
}

impl PhysicalRect {
    pub fn new(origin: PhysicalOffset, size: PhysicalSize) -> Self {
        Self { origin, size }
    }

    pub fn from_xywh(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            origin: PhysicalOffset::new(x, y),
            size: PhysicalSize::new(width, height),
        }
    }
}
