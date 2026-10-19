use html5ever::ParseOpts;
use html5ever::tendril::TendrilSink;
use blink_css::{parse_css_into_stylesheet, CSSStyleSheet};
use blink_html::HtmlDom;
use blink_layout::engine::build_layout_tree;
use blink_layout::geom::PhysicalSize;
use blink_layout::space::ConstraintSpace;
use blink_render::render_to_png;

const OUTPUT_WIDTH: u32 = 800;
const OUTPUT_HEIGHT: u32 = 600;

fn main() {
    let html_input = "<div><p>Hello Efficient Arena DOM</p></div>";

    // Build empty DOM Tree
    let empty_dom = HtmlDom::new();
    let output_dom = html5ever::parse_document(empty_dom, ParseOpts::default())
        .from_utf8()
        .one(html_input.as_bytes());
    // println!("Total flat nodes processed: {}", output_dom.0.borrow().get_nodes_len());

    // Parse CSS rules
    let css_input = "div { width: 500px; height: 300px; } p { width: 250px; height: 100px; }";
    let mut stylesheet = CSSStyleSheet::new();
    parse_css_into_stylesheet(css_input, &mut stylesheet).unwrap();
    // println!("Parsed CSS stylesheet: {:?}", stylesheet);

    // Calculate layout boundaries
    let initial_space = ConstraintSpace {
        available_size: PhysicalSize::new(OUTPUT_WIDTH as f32, OUTPUT_HEIGHT as f32),
    };
    let dom_ref = output_dom.0.borrow();
    let root_id = dom_ref.get_root_node_id();
    let arena = build_layout_tree(root_id, &dom_ref, &stylesheet, initial_space);

    // Output the rendered image
    render_to_png(&arena, OUTPUT_WIDTH, OUTPUT_HEIGHT, "output.png").unwrap();
    println!("Phase 1 pipeline executed successfully! Frame written to output.png");
}
