use blink_css::{CSSStyleSheet, parse_css_into_stylesheet};
use blink_html::parse_html;
use blink_layout::layout::{ConstraintSpace, FontdueMeasurer, LayoutContext};
use blink_layout::style::compute_styles;
use blink_render::{RenderOptions, render_to_png_with_options};
use blink_svg::render_to_svg;
use std::fs;

const OUTPUT_WIDTH: u32 = 900;
const OUTPUT_HEIGHT: u32 = 750;

fn main() {
    let html_input =
        fs::read_to_string("assets/test.html").expect("Failed to read assets/test.html");
    let css_input = fs::read_to_string("assets/test.css").expect("Failed to read assets/test.css");

    // let font_data = std::fs::read("/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf")
    //     .or_else(|_| std::fs::read("C:\\Windows\\Fonts\\arial.ttf"))
    //     .or_else(|_| std::fs::read("/Library/Fonts/Arial Unicode.ttf"))
    //     .expect("Failed to read font file");

    let font_data =
        std::fs::read("assets/fonts/KaushanScript-Regular.otf").expect("Failed to read font file");

    let font = fontdue::Font::from_bytes(font_data, fontdue::FontSettings::default())
        .expect("Failed to parse font");

    let font_size = 16.0;

    // Build empty DOM Tree
    let output_dom = parse_html(&html_input);
    // println!("Parsed HTML document: {:?}", output_dom);
    // println!("Total flat nodes processed: {}", output_dom.get_nodes_len());

    // Parse CSS rules
    let mut stylesheet = CSSStyleSheet::new();
    parse_css_into_stylesheet(&css_input, &mut stylesheet);
    // println!("Parsed CSS stylesheet: {:?}", stylesheet);

    // Calculate layout boundaries
    let initial_space = ConstraintSpace::from((OUTPUT_WIDTH, OUTPUT_HEIGHT));

    let Some(virtual_root_id) = output_dom.root_node_id() else {
        panic!("Document should have a root node");
    };

    let style_cache = compute_styles(
        virtual_root_id,
        &output_dom,
        &stylesheet,
        Some(output_dom.nodes_len()),
    );

    let mut ctx = LayoutContext {
        dom: &output_dom,
        style_cache: &style_cache, // Initialize StyleCache with the number of nodes in the DOM
        text_measurer: &FontdueMeasurer {
            font: font.clone(),
            font_size,
        },
    };
    let arena = ctx.build_layout_tree(initial_space);

    // println!();
    // println!("Fragment arena: {:#?}", arena);

    let debug_wireframes = std::env::var_os("DEBUG_WIREFRAMES").is_some();

    // Output the rendered image
    render_to_png_with_options(
        &arena,
        &style_cache,
        OUTPUT_WIDTH,
        OUTPUT_HEIGHT,
        "output.png",
        RenderOptions {
            font,
            font_size,
            debug_wireframes,
        },
    )
    .unwrap();
    println!("Phase 1 pipeline executed successfully! Frame written to output.png");

    if let Ok(svg_output) = render_to_svg(
        &arena,
        OUTPUT_WIDTH,
        OUTPUT_HEIGHT,
        "Kaushan Script",
        font_size,
    ) {
        fs::write("output.svg", &svg_output).expect("Failed to write output.svg");
        println!("Phase 2 pipeline executed successfully! Frame written to output.svg");
    } else {
        eprintln!("Failed to render to SVG.");
    }
}
