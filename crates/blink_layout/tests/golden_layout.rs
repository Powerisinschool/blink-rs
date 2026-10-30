//! Golden layout tests.
//!
//! Each fixture `tests/fixtures/<name>.{html,css}` is laid out and dumped as an
//! indented text tree (one line per fragment: label, absolute origin, size).
//! The dump is compared with `tests/fixtures/<name>.expected.txt`.
//!
//! To accept an intentional change, regenerate the expected files and review the diff:
//!
//!     BLESS=1 cargo test -p blink_layout --test golden_layout

use blink_css::{CSSStyleSheet, parse_css_into_stylesheet};
use blink_dom::{Dom, get_tag_name};
use blink_html::parse_html;
use blink_layout::engine::{LayoutContext, build_layout_tree};
use blink_layout::fragment::FragmentArena;
use blink_layout::geom::PhysicalSize;
use blink_layout::space::ConstraintSpace;
use std::env;
use std::fmt::Write;
use std::fs;
use std::path::PathBuf;

/// Viewport used for every golden test.
const VIEWPORT: (f32, f32) = (800.0, 600.0);

fn layout_fixture(html: &str, css: &str) -> (Dom, FragmentArena) {
    let dom = parse_html(html);

    let mut stylesheet = CSSStyleSheet::new();
    parse_css_into_stylesheet(css, &mut stylesheet);

    let space = ConstraintSpace {
        available_size: PhysicalSize::new(VIEWPORT.0, VIEWPORT.1),
    };
    let ctx = LayoutContext {
        dom: &dom,
        stylesheet: &stylesheet,
        text_measurer: Box::new(blink_layout::text::FixedWidth {}),
    };

    let arena = build_layout_tree(dom.get_root_node_id(), &ctx, space);

    (dom, arena)
}

/// `div#app-root`, `span.nav-item`, `div.menu-link.active`, `p`, ...
fn label(dom: &Dom, id: blink_dom::DomNodeId) -> String {
    let mut s = dom
        .get_tag(id)
        .map(get_tag_name)
        .unwrap_or("#text")
        .to_string();

    if let Some(attrs) = dom.get_attributes(id) {
        for attr in attrs {
            match &*attr.name.local {
                "id" => {
                    s.push('#');
                    s.push_str(&attr.value);
                }
                "class" => {
                    for class in attr.value.split_whitespace() {
                        s.push('.');
                        s.push_str(class);
                    }
                }
                _ => {}
            }
        }
    }
    s
}

fn dump(dom: &Dom, arena: &FragmentArena) -> String {
    let mut out = String::new();
    if !arena.fragments.is_empty() {
        dump_node(dom, arena, 0, 0, &mut out);
    }
    out
}

fn dump_node(dom: &Dom, arena: &FragmentArena, idx: usize, depth: usize, out: &mut String) {
    let fragment = &arena.fragments[idx];
    let name = fragment
        .dom_node
        .map(|id| label(dom, id))
        .unwrap_or_else(|| "<anonymous>".to_string());

    writeln!(
        out,
        "{}{} @ ({:.1}, {:.1}) {:.1}x{:.1}",
        "  ".repeat(depth),
        name,
        fragment.rect.origin.x,
        fragment.rect.origin.y,
        fragment.rect.size.width,
        fragment.rect.size.height,
    )
    .unwrap();

    let mut child = fragment.first_child;
    while let Some(child_id) = child {
        let child_idx = (child_id.get() - 1) as usize;
        dump_node(dom, arena, child_idx, depth + 1, out);
        child = arena.fragments[child_idx].next_sibling;
    }
}

fn assert_golden(name: &str) {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let read = |ext: &str| {
        let path = dir.join(format!("{name}.{ext}"));
        fs::read_to_string(&path).unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()))
    };

    let html = read("html");
    let css = read("css");
    let (dom, arena) = layout_fixture(&html, &css);
    let actual = dump(&dom, &arena);

    let expected_path = dir.join(format!("{name}.expected.txt"));

    if env::var_os("BLESS").is_some() {
        fs::write(&expected_path, &actual).unwrap();
        return;
    }

    let expected = fs::read_to_string(&expected_path).unwrap_or_else(|_| {
        panic!(
            "missing {}; create it with BLESS=1 and review it",
            expected_path.display()
        )
    });

    assert_eq!(
        expected.replace("\r\n", "\n"),
        actual,
        "layout for fixture `{name}` changed. If intentional, re-run with BLESS=1 and review the diff."
    );
}

#[test]
fn basic_page() {
    assert_golden("basic");
}

#[test]
fn inline_wrapping() {
    assert_golden("inline_wrapping");
}

#[test]
fn mixed_block_inline() {
    assert_golden("mixed_block_inline");
}

#[test]
fn nested_inline() {
    assert_golden("nested_inline");
}

#[test]
fn display_none() {
    assert_golden("display_none");
}
