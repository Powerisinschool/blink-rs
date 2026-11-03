use blink_layout::fragment::FragmentArena;
use fontdue::Font;
use tiny_skia::{Color, ColorU8, Paint, PathBuilder, Pixmap, Rect, Stroke, Transform};

pub fn render_to_png(
    font: Font,
    font_size: f32,
    arena: &FragmentArena,
    width: u32,
    height: u32,
    output_path: &str,
) -> Result<(), String> {
    // let font_data = std::fs::read("/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf")
    //     .or_else(|_| std::fs::read("C:\\Windows\\Fonts\\arial.ttf"))
    //     .or_else(|_| std::fs::read("/Library/Fonts/Arial Unicode.ttf"))
    //     .map_err(|err| format!("Failed to read font file: {}", err))?;
    //
    // let font = fontdue::Font::from_bytes(font_data, fontdue::FontSettings::default())
    //     .map_err(|err| format!("Failed to parse font: {}", err))?;

    let mut pixmap =
        Pixmap::new(width, height).ok_or_else(|| "Failed to create pixmap".to_string())?;

    pixmap.fill(Color::WHITE);

    // let font_size = 16.0;

    for fragment in &arena.fragments {
        let rect_data = fragment.rect;

        if rect_data.size.width <= 0.0 || rect_data.size.height <= 0.0 {
            continue;
        }

        let skia_rect = match Rect::from_xywh(
            rect_data.origin.x,
            rect_data.origin.y,
            rect_data.size.width,
            rect_data.size.height,
        ) {
            Some(r) => r,
            None => continue,
        };

        if let Some(text) = fragment.get_text() {
            // println!("Rendering text fragment: {:?}", text);
            let mut cursor_x = rect_data.origin.x;

            let metrics = font.horizontal_line_metrics(font_size).unwrap();
            let baseline_y = rect_data.origin.y + metrics.ascent;

            for ch in text.chars() {
                let (glyph_metrics, glyph_bitmap) = font.rasterize(ch, font_size);

                if ch.is_whitespace() {
                    cursor_x += glyph_metrics.advance_width;
                    continue;
                }

                let x = (cursor_x + glyph_metrics.xmin as f32).round() as i32;
                let y = (baseline_y - glyph_metrics.ymin as f32 - glyph_metrics.height as f32)
                    .round() as i32;

                for row in 0..glyph_metrics.height {
                    for col in 0..glyph_metrics.width {
                        let pixel_index = row * glyph_metrics.width + col;
                        let alpha = glyph_bitmap[pixel_index] as f32 / 255.0;

                        if alpha > 0.0 {
                            let pixel_x = x + col as i32;
                            let pixel_y = y + row as i32;

                            if pixel_x >= 0
                                && pixel_x < pixmap.width() as i32
                                && pixel_y >= 0
                                && pixel_y < pixmap.height() as i32
                            {
                                let pixmap_width = pixmap.width();
                                if let Some(pixel) = pixmap.pixels_mut().get_mut(
                                    (pixel_y as u32 * pixmap_width) as usize
                                        + pixel_x as u32 as usize,
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

            continue; // TODO: Implement proper text rendering
        }

        if fragment.is_line() {
            continue;
        }

        let mut paint = Paint::default();
        paint.set_color_rgba8(30, 110, 210, 255);

        let mut pb = PathBuilder::new();
        pb.push_rect(skia_rect);

        if let Some(path) = pb.finish() {
            let stroke = Stroke::default();
            pixmap.stroke_path(&path, &paint, &stroke, Transform::identity(), None);
        }
    }

    pixmap
        .save_png(output_path)
        .map_err(|err| format!("Failed to save PNG: {}", err))
}

#[cfg(test)]
mod tests {}
