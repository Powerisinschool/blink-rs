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

pub struct FontdueMeasurer {
    pub font: fontdue::Font,
    pub font_size: f32,
}

impl TextMeasurer for FontdueMeasurer {
    fn measure(&self, text: &str) -> f32 {
        let mut width = 0.0;
        for ch in text.chars() {
            // Fetch the exact layout width advance defined by font due properties
            width += self.font.metrics(ch, self.font_size).advance_width;
        }
        width
    }

    fn line_height(&self) -> f32 {
        if let Some(metrics) = self.font.horizontal_line_metrics(self.font_size) {
            // Combine ascent, descent, and gap line heights cleanly
            metrics.ascent + (-metrics.descent) + metrics.line_gap
        } else {
            16.0 // Safe fallback
        }
    }
}
