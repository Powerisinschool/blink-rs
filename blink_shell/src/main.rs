use blink_css::{CSSStyleSheet, parse_css_into_stylesheet};
use blink_html::parse_html;
use blink_layout::engine::build_layout_tree;
use blink_layout::geom::PhysicalSize;
use blink_layout::space::ConstraintSpace;
use blink_render::render_to_png;
use std::fs;

const OUTPUT_WIDTH: u32 = 800;
const OUTPUT_HEIGHT: u32 = 600;

fn main() {
    let html_input =
        fs::read_to_string("assets/test.html").expect("Failed to read assets/test.html");
    let css_input = fs::read_to_string("assets/test.css").expect("Failed to read assets/test.css");

    // Build empty DOM Tree
    let output_dom = parse_html(&html_input);
    // println!("Parsed HTML document: {:?}", output_dom);
    // println!("Total flat nodes processed: {}", output_dom.get_nodes_len());

    // Parse CSS rules
    let mut stylesheet = CSSStyleSheet::new();
    parse_css_into_stylesheet(&css_input, &mut stylesheet);
    // println!("Parsed CSS stylesheet: {:?}", stylesheet);

    // Calculate layout boundaries
    let initial_space = ConstraintSpace {
        available_size: PhysicalSize::new(OUTPUT_WIDTH as f32, OUTPUT_HEIGHT as f32),
    };

    let root_id = output_dom.get_root_node_id();

    let ctx = blink_layout::engine::LayoutContext {
        dom: &output_dom,
        stylesheet: &stylesheet,
        text_measurer: Box::new(blink_layout::text::FixedWidth {}),
    };
    let arena = build_layout_tree(root_id, &ctx, initial_space);

    // println!();
    // println!("Fragment arena: {:?}", arena);

    // Output the rendered image
    render_to_png(&arena, OUTPUT_WIDTH, OUTPUT_HEIGHT, "output.png").unwrap();
    println!("Phase 1 pipeline executed successfully! Frame written to output.png");
}
