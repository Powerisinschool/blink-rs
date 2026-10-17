use html5ever::ParseOpts;
use html5ever::tendril::TendrilSink;
use blink_html::HtmlDom;

fn main() {
    let html_input = "<div><p>Hello Efficient Arena DOM</p></div>";

    // Create an empty allocation instance of your custom DOM
    let empty_dom = HtmlDom::new();

    // Drive the parsing loop directly into your flat layout architecture!
    let output_dom = html5ever::parse_document(empty_dom, ParseOpts::default())
        .from_utf8()
        .one(html_input.as_bytes());

    // Validation check on node layout tracking
    println!("Total flat nodes processed: {}", output_dom.0.borrow().get_nodes_len());
}
