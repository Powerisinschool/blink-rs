use crate::geom::PhysicalSize;

#[derive(Debug, Clone, Copy)]
pub struct ConstraintSpace {
    available_size: PhysicalSize,
}

impl ConstraintSpace {
    pub fn new(width: f32, height: f32) -> Self {
        Self {
            available_size: PhysicalSize { width, height },
        }
    }

    #[must_use]
    pub fn get_available_width(&self) -> f32 {
        self.available_size.width
    }

    #[must_use]
    pub fn get_available_height(&self) -> f32 {
        self.available_size.height
    }
}

impl From<(f32, f32)> for ConstraintSpace {
    fn from(value: (f32, f32)) -> Self {
        Self::new(value.0, value.1)
    }
}

impl From<(u32, u32)> for ConstraintSpace {
    fn from(value: (u32, u32)) -> Self {
        Self::new(value.0 as f32, value.1 as f32)
    }
}
