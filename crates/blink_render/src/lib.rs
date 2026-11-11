use blink_layout::fragment::{FragmentArena, FragmentKind, LayoutFragment, LayoutFragmentId};
use blink_layout::walker::{FragmentWalker, Renderer};
use fontdue::Font;
use tiny_skia::{Color, ColorU8, Paint, PathBuilder, Pixmap, Rect, Stroke, Transform};

pub struct Canvas {
    pub pixmap: Pixmap,
}

impl Canvas {
    pub fn new(width: u32, height: u32) -> Self {
        let mut pixmap = Pixmap::new(width, height).expect("Failed to create Pixmap");
        pixmap.fill(Color::WHITE);
        Self { pixmap }
    }

    pub fn draw_box(&mut self, x: f32, y: f32, width: f32, height: f32, color: ColorU8) {
        if width <= 0.0 || height <= 0.0 {
            return;
        }

        let Some(rect) = Rect::from_xywh(x, y, width, height) else {
            return;
        };

        let mut paint = Paint::default();
        paint.set_color_rgba8(color.red(), color.green(), color.blue(), color.alpha());

        let mut pb = PathBuilder::new();
        pb.push_rect(rect);

        if let Some(path) = pb.finish() {
            let stroke = Stroke::default();
            self.pixmap
                .stroke_path(&path, &paint, &stroke, Transform::identity(), None);
        }
    }

    pub fn draw_text(&mut self, text_content: &str, x: f32, y: f32, font: &Font, font_size: f32) {
        let mut cursor_x = x;

        let baseline_y = y + font
            .horizontal_line_metrics(font_size)
            .map_or(font_size, |metrics| metrics.ascent);

        for ch in text_content.chars() {
            let (glyph_metrics, glyph_bitmap) = font.rasterize(ch, font_size);

            if ch.is_whitespace() {
                cursor_x += glyph_metrics.advance_width;
                continue;
            }

            let x = (cursor_x + glyph_metrics.xmin as f32).round() as i32;
            let y = (baseline_y - glyph_metrics.ymin as f32 - glyph_metrics.height as f32).round()
                as i32;

            for row in 0..glyph_metrics.height {
                for col in 0..glyph_metrics.width {
                    let pixel_index = row * glyph_metrics.width + col;
                    let alpha = glyph_bitmap[pixel_index] as f32 / 255.0;

                    if alpha > 0.0 {
                        let pixel_x = x + col as i32;
                        let pixel_y = y + row as i32;

                        if pixel_x >= 0
                            && pixel_x < self.pixmap.width() as i32
                            && pixel_y >= 0
                            && pixel_y < self.pixmap.height() as i32
                        {
                            let pixmap_width = self.pixmap.width();
                            if let Some(pixel) = self.pixmap.pixels_mut().get_mut(
                                (pixel_y as u32 * pixmap_width) as usize + pixel_x as u32 as usize,
                            ) {
                                let old_alpha = pixel.alpha() as f32 / 255.0;
                                let combined_alpha = alpha + old_alpha * (1.0 - alpha);

                                let factor = 1.0 - alpha;
                                let r = (pixel.red() as f32 * factor) as u8;
                                let g = (pixel.green() as f32 * factor) as u8;
                                let b = (pixel.blue() as f32 * factor) as u8;
                                let a = (combined_alpha * 255.0) as u8;

                                *pixel = ColorU8::from_rgba(r, g, b, a).premultiply();
                            }
                        }
                    }
                }
            }

            cursor_x += glyph_metrics.advance_width;
        }
    }

    pub fn export_png(&self, output_path: &str) -> Result<(), String> {
        self.pixmap
            .save_png(output_path)
            .map_err(|err| format!("Failed to save PNG: {}", err))
    }
}

pub struct CanvasRenderer {
    pub font: Font,
    pub font_size: f32,
    pub canvas: Canvas,
}

pub struct RenderState {
    // pub offset_x: f32,
    // pub offset_y: f32,
}

impl Renderer for CanvasRenderer {
    type Context = RenderState;

    fn begin_fragment(
        &mut self,
        _fragment: &LayoutFragment,
        _ctx: &Self::Context,
    ) -> Self::Context {
        RenderState {
            // offset_x: ctx.offset_x + fragment.rect.origin.x,
            // offset_y: ctx.offset_y + fragment.rect.origin.y,
        }
    }

    fn paint_fragment(&mut self, fragment: &LayoutFragment, _ctx: &Self::Context) {
        match &fragment.kind {
            FragmentKind::Box => {
                self.canvas.draw_box(
                    fragment.rect.origin.x,
                    fragment.rect.origin.y,
                    fragment.rect.size.width,
                    fragment.rect.size.height,
                    ColorU8::from_rgba(30, 110, 210, 255),
                );
            }
            FragmentKind::Line => {}
            FragmentKind::Text(text) => {
                self.canvas.draw_text(
                    text,
                    fragment.rect.origin.x,
                    fragment.rect.origin.y,
                    &self.font,
                    self.font_size,
                );
            }
        }
    }

    fn end_fragment(&mut self, _fragment: &LayoutFragment, _ctx: &Self::Context) {
        // do nothing for now
    }
}

pub fn render_to_png(
    font: Font,
    font_size: f32,
    arena: &FragmentArena,
    width: u32,
    height: u32,
    output_path: &str,
) -> Result<(), String> {
    let mut renderer = CanvasRenderer {
        font,
        font_size,
        canvas: Canvas::new(width, height),
    };

    let Some(root_id) = LayoutFragmentId::from_usize(0) else {
        return Err("Failed to create root fragment ID".to_string());
    };
    FragmentWalker::walk(arena, root_id, &mut renderer, &RenderState {});

    renderer.canvas.export_png(output_path)
}

#[cfg(test)]
mod tests {}
