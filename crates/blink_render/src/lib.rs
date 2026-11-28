use blink_layout::geom::PhysicalOffset;
use blink_layout::layout::{FragmentArena, FragmentKind, LayoutFragment};
use blink_layout::style::types::SpecifiedColor;
use blink_layout::style::{BorderSide, EdgeSizes, StyleCache};
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

    fn draw_borders(
        &mut self,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        borders: EdgeSizes<BorderSide>,
    ) {
        let widths = borders.widths();
        let resolve_color = |side: BorderSide| match side.color {
            SpecifiedColor::Color(c) => ColorU8::from_rgba(c.r, c.g, c.b, c.a),
            SpecifiedColor::CurrentColor => ColorU8::from_rgba(0, 0, 0, 255), // default "currentcolor" = black
        };

        if widths.top > 0.0 {
            self.fill_rect(x, y, width, widths.top, resolve_color(borders.top));
        }

        if widths.right > 0.0 {
            self.fill_rect(
                x + width - widths.right,
                y + widths.top,
                widths.right,
                height - widths.top - widths.bottom,
                resolve_color(borders.right),
            );
        }

        if widths.bottom > 0.0 {
            self.fill_rect(
                x,
                y + height - widths.bottom,
                width,
                widths.bottom,
                resolve_color(borders.bottom),
            );
        }

        if widths.left > 0.0 {
            self.fill_rect(
                x,
                y + widths.top,
                widths.left,
                height - widths.top - widths.bottom,
                resolve_color(borders.left),
            );
        }
    }

    fn fill_rect(&mut self, x: f32, y: f32, width: f32, height: f32, color: ColorU8) {
        if width <= 0.0 || height <= 0.0 {
            return;
        }
        let Some(rect) = Rect::from_xywh(x, y, width, height) else {
            return;
        };
        let mut paint = Paint::default();
        paint.set_color_rgba8(color.red(), color.green(), color.blue(), color.alpha());
        self.pixmap
            .fill_rect(rect, &paint, Transform::identity(), None);
    }

    fn draw_box(&mut self, x: f32, y: f32, width: f32, height: f32, color: ColorU8) {
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

    fn draw_text(&mut self, text_content: &str, x: f32, y: f32, font: &Font, font_size: f32) {
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

pub struct CanvasRenderer<'a> {
    pub font: Font,
    pub font_size: f32,
    pub canvas: Canvas,
    pub style_cache: &'a StyleCache,
    pub debug_wireframes: bool,
}

pub struct RenderState {}

impl Renderer for CanvasRenderer<'_> {
    type Context = RenderState;

    fn begin_fragment(
        &mut self,
        _fragment: &LayoutFragment,
        _abs_origin: PhysicalOffset,
        _ctx: &Self::Context,
    ) -> Self::Context {
        RenderState {}
    }

    fn paint_fragment(
        &mut self,
        fragment: &LayoutFragment,
        abs_origin: PhysicalOffset,
        _ctx: &Self::Context,
    ) {
        let style = fragment.style_id().and_then(|id| self.style_cache.get(id));

        match &fragment.kind {
            FragmentKind::Box => {
                let borders = style.map(|s| s.border).unwrap_or_default();

                self.canvas.draw_borders(
                    abs_origin.x,
                    abs_origin.y,
                    fragment.width(),
                    fragment.height(),
                    borders,
                );

                if self.debug_wireframes {
                    self.canvas.draw_box(
                        abs_origin.x,
                        abs_origin.y,
                        fragment.width(),
                        fragment.height(),
                        ColorU8::from_rgba(30, 110, 210, 255),
                    );
                }
            }
            FragmentKind::Line => {}
            FragmentKind::Text(text) => {
                self.canvas
                    .draw_text(text, abs_origin.x, abs_origin.y, &self.font, self.font_size);
            }
        }
    }

    fn end_fragment(
        &mut self,
        _fragment: &LayoutFragment,
        _abs_origin: PhysicalOffset,
        _ctx: &Self::Context,
    ) {
        // do nothing for now
    }
}

/// Renders the fragment tree to a PNG image file.
///
/// Creates a canvas with the provided dimensions, walks the fragment tree from
/// the root fragment, renders each fragment using the supplied font and style
/// information, and writes the final image to `output_path`.
///
/// # Parameters
///
/// - `font`: Font used when rendering text.
/// - `font_size`: Base font size used for text rendering.
/// - `arena`: Fragment arena containing the layout/render tree.
/// - `style_cache`: Cache containing computed style information for fragments.
/// - `width`: Width of the output image in pixels.
/// - `height`: Height of the output image in pixels.
/// - `output_path`: Path where the PNG file should be written.
///
/// # Extended Descriptions
/// - style_cache: A reference to the StyleCache that holds computed styles for DOM nodes.
///
/// This is used to retrieve style information when rendering fragments. It renders the
/// layout tree to a PNG image file. The StyleCache is passed as an argument to ensure that the
/// rendering process has access to the computed styles for each fragment. It is assumed that
/// the StyleCache is immutable during rendering, so it can be safely borrowed. It must not be
/// computed or modified during the rendering process, as this could lead to inconsistencies
/// between the layout and the rendered output.
///
/// # Errors
///
/// Returns an error if no root fragment exists in `arena`, or if exporting the
/// rendered canvas to PNG fails.
pub fn render_to_png(
    font: Font,
    font_size: f32,
    arena: &FragmentArena,
    style_cache: &StyleCache,
    width: u32,
    height: u32,
    output_path: &str,
) -> Result<(), String> {
    let mut renderer = CanvasRenderer {
        font,
        font_size,
        canvas: Canvas::new(width, height),
        style_cache,
        debug_wireframes: false,
    };

    let Some(root_id) = arena.get_root() else {
        return Err("Failed to create root fragment ID".to_string());
    };
    FragmentWalker::walk(
        arena,
        root_id,
        &mut renderer,
        &RenderState {},
        PhysicalOffset::new(0.0, 0.0),
    );

    renderer.canvas.export_png(output_path)
}

pub struct RenderOptions {
    pub font: Font,
    pub font_size: f32,
    pub debug_wireframes: bool,
}

pub fn render_to_png_with_options(
    arena: &FragmentArena,
    style_cache: &StyleCache,
    width: u32,
    height: u32,
    output_path: &str,
    options: RenderOptions,
) -> Result<(), String> {
    let mut renderer = CanvasRenderer {
        font: options.font,
        font_size: options.font_size,
        canvas: Canvas::new(width, height),
        style_cache,
        debug_wireframes: options.debug_wireframes,
    };

    let Some(root_id) = arena.get_root() else {
        return Err("Failed to create root fragment ID".to_string());
    };
    FragmentWalker::walk(
        arena,
        root_id,
        &mut renderer,
        &RenderState {},
        PhysicalOffset::new(0.0, 0.0),
    );

    renderer.canvas.export_png(output_path)
}

#[cfg(test)]
mod tests {}
