use blink_layout::fragment::{FragmentArena, FragmentKind, LayoutFragment, LayoutFragmentId};
use blink_layout::walker::{FragmentWalker, Renderer};

pub struct SvgRenderer {
    pub svg_content: String,
    pub font_family: String,
    pub font_size: f32,
}

pub struct RenderState {}

impl SvgRenderer {
    pub fn new(width: u32, height: u32, font_family: &str, font_size: f32) -> Self {
        // Initialize the SVG wrapper with the viewport dimensions
        let header = format!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {} {}" width="{}" height="{}">"#,
            width, height, width, height
        );

        Self {
            svg_content: header,
            font_family: font_family.to_string(),
            font_size,
        }
    }

    pub fn finish(mut self) -> String {
        self.svg_content.push_str("\n</svg>");
        self.svg_content
    }
}

impl Renderer for SvgRenderer {
    type Context = RenderState;

    fn begin_fragment(
        &mut self,
        _fragment: &LayoutFragment,
        _ctx: &Self::Context,
    ) -> Self::Context {
        RenderState {}
    }

    fn paint_fragment(&mut self, fragment: &LayoutFragment, _ctx: &Self::Context) {
        let x = fragment.rect.origin.x;
        let y = fragment.rect.origin.y;
        let width = fragment.rect.size.width;
        let height = fragment.rect.size.height;

        match &fragment.kind {
            FragmentKind::Box => {
                if width <= 0.0 || height <= 0.0 {
                    return;
                }

                self.svg_content.push_str(&format!(
                    "\n  <rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"none\" stroke=\"rgb(30, 110, 210)\" stroke-width=\"1\" />",
                    x, y, width, height
                ));
            }
            FragmentKind::Line => {}
            FragmentKind::Text(text) => {
                let escaped_text = text
                    .replace("&", "&amp;")
                    .replace("<", "&lt;")
                    .replace(">", "&gt;");

                self.svg_content.push_str(&format!(
                    "\n  <text x=\"{}\" y=\"{}\" font-family=\"{}\" font-size=\"{}px\" fill=\"rgb(0, 0, 0)\" dominant-baseline=\"hanging\">{}</text>",
                    x, y, self.font_family, self.font_size, escaped_text
                ));
            }
        }
    }

    fn end_fragment(&mut self, _fragment: &LayoutFragment, _ctx: &Self::Context) {}
}

pub fn render_to_svg(
    arena: &FragmentArena,
    width: u32,
    height: u32,
    font_family: &str,
    font_size: f32,
) -> Result<String, String> {
    let mut renderer = SvgRenderer::new(width, height, font_family, font_size);

    let Some(root_id) = LayoutFragmentId::from_usize(0) else {
        return Err("Failed to create root fragment ID".to_string());
    };

    FragmentWalker::walk(arena, root_id, &mut renderer, &RenderState {});

    Ok(renderer.finish())
}

#[cfg(test)]
mod tests {}
