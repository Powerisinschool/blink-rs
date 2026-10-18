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

#[derive(Debug, Default, Clone, Copy)]
pub struct PhysicalRect {
    pub origin: PhysicalOffset,
    pub size: PhysicalSize,
}
