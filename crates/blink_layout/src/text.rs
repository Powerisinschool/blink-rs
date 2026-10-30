pub trait TextMeasurer {
    fn measure(&self, text: &str) -> f32;
    fn line_height(&self) -> f32;
}

pub struct FixedWidth {}

impl TextMeasurer for FixedWidth {
    fn measure(&self, text: &str) -> f32 {
        (text.chars().count() as f32) * 10.0
    }

    fn line_height(&self) -> f32 {
        16.0
    }
}
