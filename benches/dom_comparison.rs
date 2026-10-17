use blink_html::HtmlDom;
use criterion::{criterion_main, Criterion, criterion_group};
use html5ever::tendril::TendrilSink;
use html5ever::{ParseOpts, parse_document};
use stats_alloc::{Region, StatsAlloc, INSTRUMENTED_SYSTEM};
use std::alloc::System;
use std::hint::black_box;

#[global_allocator]
static GLOBAL: &StatsAlloc<System> = &INSTRUMENTED_SYSTEM;

// A mix of nested nodes, text, and sibling chains to stress-test your implementation
const SMALL_HTML: &str = r#"<div><p>Hello <span>World</span></p></div>"#;

const MEDIUM_HTML: &str = r#"
<html>
    <head><title>Test Bench</title></head>
    <body>
        <header><nav><ul><li>Home</li><li>About</li><li>Contact</li></ul></nav></header>
        <main>
            <h1>Section Title</h1>
            <p>Lorem ipsum dolor sit amet, consectetur adipiscing elit.</p>
            <div><img src="test.png" alt="Test"/><p>Deeply <b>nested <i>text nodes</i> here</b>.</p></div>
        </main>
    </body>
</html>
"#;

const LARGE_HTML: &str = r#"
<!DOCTYPE html>
<html>
<head>
    <meta charset="utf-8">
    <title>Large Benchmark Core Document Target</title>
</head>
<body>
    <header>
        <nav role="navigation">
            <ul>
                <li><a href='#1'>Section 1</a></li><li><a href='#2'>Section 2</a></li>
                <li><a href='#3'>Section 3</a></li><li><a href='#4'>Section 4</a></li>
            </ul>
        </nav>
    </header>
    <main>
        <article>
            <h1>Main Topic Overview</h1>
            <p>Paragraph section containing layout strings for baseline validation tracking matrices.</p>
            <section id="1">
                <h2>Subheading Layout 1</h2>
                <div>
                    <p>Deeply <span>nested <b>formatting <i>node stacks</i> here</b> inside</span> loops.</p>
                    <ul>
                        <li>Item A with text description block</li>
                        <li>Item B with text description block</li>
                        <li>Item C with text description block</li>
                    </ul>
                </div>
            </section>
            <section id="2">
                <h2>Subheading Layout 2</h2>
                <table>
                    <thead><tr><th>Col 1</th><th>Col 2</th></tr></thead>
                    <tbody>
                        <tr><td>Cell 1A</td><td>Cell 1B</td></tr>
                        <tr><td>Cell 2A</td><td>Cell 2B</td></tr>
                    </tbody>
                </table>
            </section>
        </article>
    </main>
    <footer><p>Copyright 2026 Blink Render Engine Workspace Performance Module</p></footer>
</body>
</html>
"#;

fn parse_html_string(input: &str) -> HtmlDom {
    parse_document(HtmlDom::new(), ParseOpts::default())
        .one(input)
}

fn bench_dense_core(c: &mut Criterion) {
    let variant = if !cfg!(feature = "dense-core") { "variant_a" } else { "variant_b" };
    let mut group = c.benchmark_group("DOM_Parsing");

    group.bench_function("small_dom", |b| {
        b.iter(|| parse_html_string(black_box(SMALL_HTML)))
    });

    group.bench_function("medium_dom", |b| {
        b.iter(|| parse_html_string(black_box(MEDIUM_HTML)))
    });

    group.bench_function("large_dom", |b| {
        b.iter(|| parse_html_string(black_box(LARGE_HTML)))
    });

    group.finish();

    println!("\n==================================================");
    println!("📊 HEAP METRICS FOR [{}]", variant.to_uppercase());

    // Track Small HTML Memory Lifecycle
    let reg_small = Region::new(&GLOBAL);
    let dom_small = parse_html_string(SMALL_HTML);
    std::mem::drop(black_box(dom_small));
    let stats_small = reg_small.change();
    println!("   [Small HTML] Allocations: {}, Total Leaked/Retained: {} bytes",
             stats_small.allocations, stats_small.bytes_allocated);

    // Track Medium HTML Memory Lifecycle
    let reg_med = Region::new(&GLOBAL);
    let dom_med = parse_html_string(MEDIUM_HTML);
    std::mem::drop(black_box(dom_med));
    let stats_med = reg_med.change();
    println!("   [Medium HTML] Allocations: {}, Total Leaked/Retained: {} bytes",
             stats_med.allocations, stats_med.bytes_allocated);

    // Track Large HTML Memory Lifecycle
    let reg_large = Region::new(&GLOBAL);
    let dom_large = parse_html_string(LARGE_HTML);
    std::mem::drop(black_box(dom_large));
    let stats_large = reg_large.change();
    println!("   [Large HTML] Allocations: {}, Total Leaked/Retained: {} bytes",
             stats_large.allocations, stats_large.bytes_allocated);

    println!("==================================================\n");
}

criterion_group!(benches, bench_dense_core);
criterion_main!(benches);
