use blink_layout::fragment::FragmentArena;
use tiny_skia::{Color, Paint, PathBuilder, Pixmap, Rect, Stroke, Transform};

pub fn render_to_png(
    arena: &FragmentArena,
    width: u32,
    height: u32,
    output_path: &str,
) -> Result<(), String> {
    let mut pixmap =
        Pixmap::new(width, height).ok_or_else(|| "Failed to create pixmap".to_string())?;

    pixmap.fill(Color::WHITE);

    for fragment in &arena.fragments {
        let rect_data = fragment.rect;

        if rect_data.size.width <= 0.0 || rect_data.size.height <= 0.0 {
            continue;
        }

        if let Some(skia_rect) = Rect::from_xywh(
            rect_data.origin.x,
            rect_data.origin.y,
            rect_data.size.width,
            rect_data.size.height,
        ) {
            let mut paint = Paint::default();
            paint.set_color_rgba8(30, 110, 210, 255);

            let mut pb = PathBuilder::new();
            pb.push_rect(skia_rect);

            if let Some(path) = pb.finish() {
                let stroke = Stroke::default();
                pixmap.stroke_path(&path, &paint, &stroke, Transform::identity(), None);
            }
        }
    }

    pixmap
        .save_png(output_path)
        .map_err(|err| format!("Failed to save PNG: {}", err))
}

#[cfg(test)]
mod tests {}
