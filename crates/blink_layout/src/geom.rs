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
    pub(crate) fn new(x: f32, y: f32) -> PhysicalOffset {
        Self { x, y }
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
}
