#[derive(Debug, Default, Clone, Copy)]
pub struct PhysicalSize {
    pub width: f32,
    pub height: f32,
}

#[derive(Debug, Default, Clone, Copy)]
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

impl PartialEq<(f32, f32)> for PhysicalOffset {
    fn eq(&self, other: &(f32, f32)) -> bool {
        self.x == other.0 && self.y == other.1
    }
}

impl PartialEq<PhysicalOffset> for (f32, f32) {
    fn eq(&self, other: &PhysicalOffset) -> bool {
        self.0 == other.x && self.1 == other.y
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct PhysicalRect {
    pub origin: PhysicalOffset,
    pub size: PhysicalSize,
}

impl PhysicalSize {
    pub fn new(width: f32, height: f32) -> Self {
        Self { width, height }
    }
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
