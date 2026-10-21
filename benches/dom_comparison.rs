// use blink_html::HtmlDom;
// use criterion::{Criterion, criterion_group, criterion_main};
// use html5ever::tendril::TendrilSink;
// use html5ever::{ParseOpts, parse_document};
// use stats_alloc::{INSTRUMENTED_SYSTEM, Region, StatsAlloc};
// use std::alloc::System;
// use std::hint::black_box;
//
// #[global_allocator]
// static GLOBAL: &StatsAlloc<System> = &INSTRUMENTED_SYSTEM;
//
// // A mix of nested nodes, text, and sibling chains to stress-test your implementation
// const SMALL_HTML: &str = r#"<div><p>Hello <span>World</span></p></div>"#;
//
// const MEDIUM_HTML: &str = r#"
// <html>
//     <head><title>Test Bench</title></head>
//     <body>
//         <header><nav><ul><li>Home</li><li>About</li><li>Contact</li></ul></nav></header>
//         <main>
//             <h1>Section Title</h1>
//             <p>Lorem ipsum dolor sit amet, consectetur adipiscing elit.</p>
//             <div><img src="test.png" alt="Test"/><p>Deeply <b>nested <i>text nodes</i> here</b>.</p></div>
//         </main>
//     </body>
// </html>
// "#;
//
// const LARGE_HTML: &str = r#"
// <!DOCTYPE html>
// <html class="no-js lang-en dir-ltr" id="app-root">
// <head class="metadata-container">
//     <meta charset="utf-8" class="meta-item"/>
//     <title class="page-title-node">Attribute Heavy Production Test Engine Target</title>
// </head>
// <body class="viewport-profile full-bleed light-theme" data-route="home">
//     <header class="global-navigation-bar layered-box" id="nav-header">
//         <nav class="nav-wrapper max-width-constrain" role="navigation">
//             <ul class="nav-list flex-row space-between">
//                 <li class="nav-item active"><a href='#1' class="nav-link speed-optimized">Section 1</a></li>
//                 <li class="nav-item"><a href='#2' class="nav-link secondary-trigger">Section 2</a></li>
//                 <li class="nav-item"><a href='#3' class="nav-link tracking-analytics">Section 3</a></li>
//                 <li class="nav-item"><a href='#4' class="nav-link debug-panel-toggle">Section 4</a></li>
//             </ul>
//         </nav>
//     </header>
//     <main class="main-content-container grid-layout" id="primary-view">
//         <article class="article-body prose-optimized card-shadow" data-article-id="99281">
//             <h1 class="heading-hero core-title">Main Topic Overview String Node</h1>
//             <p class="paragraph-lead rendering-text-block">Paragraph section layout strings tracking metrics.</p>
//             <section class="content-section interactive-card" id="1" data-visible="true">
//                 <h2 class="sub-heading-node accent-border">Subheading Layout 1</h2>
//                 <div class="nested-wrapper flex-column container-padding">
//                     <p class="body-text micro-copy">Deeply <span class="highlight-text">nested <b class="bold-weight">formatting <i class="italicized-style">node stacks</i> here</b> inside</span> loops.</p>
//                     <ul class="ordered-metrics-list stylized-bullets">
//                         <li class="list-element-node active-row" data-index="0">Item A with text description block</li>
//                         <li class="list-element-node" data-index="1">Item B with text description block</li>
//                         <li class="list-element-node" data-index="2">Item C with text description block</li>
//                     </ul>
//                 </div>
//             </section>
//         </article>
//     </main>
//     <footer class="footer-bar static-bottom informational-box" id="footer-node">
//         <p class="copyright-notice small-print">Copyright 2026 Blink Render Engine Workspace Performance Module</p>
//     </footer>
// </body>
// </html>
// "#;
//
// fn parse_html_string(input: &str) -> HtmlDom {
//     parse_document(HtmlDom::new(), ParseOpts::default()).one(input)
// }
//
// fn bench_dense_core(c: &mut Criterion) {
//     let variant = if !cfg!(feature = "dense-core") {
//         "variant_a"
//     } else {
//         "variant_b"
//     };
//     let mut group = c.benchmark_group("DOM_Parsing");
//
//     group.bench_function("small_dom", |b| {
//         b.iter(|| parse_html_string(black_box(SMALL_HTML)))
//     });
//
//     group.bench_function("medium_dom", |b| {
//         b.iter(|| parse_html_string(black_box(MEDIUM_HTML)))
//     });
//
//     group.bench_function("large_dom", |b| {
//         b.iter(|| parse_html_string(black_box(LARGE_HTML)))
//     });
//
//     group.finish();
//
//     println!("\n==================================================");
//     println!("📊 HEAP METRICS FOR [{}]", variant.to_uppercase());
//
//     // Track Small HTML Memory Lifecycle
//     let reg_small = Region::new(GLOBAL);
//     let dom_small = parse_html_string(SMALL_HTML);
//     std::mem::drop(black_box(dom_small));
//     let stats_small = reg_small.change();
//     println!(
//         "   [Small HTML] Allocations: {}, Total Leaked/Retained: {} bytes",
//         stats_small.allocations, stats_small.bytes_allocated
//     );
//
//     // Track Medium HTML Memory Lifecycle
//     let reg_med = Region::new(GLOBAL);
//     let dom_med = parse_html_string(MEDIUM_HTML);
//     std::mem::drop(black_box(dom_med));
//     let stats_med = reg_med.change();
//     println!(
//         "   [Medium HTML] Allocations: {}, Total Leaked/Retained: {} bytes",
//         stats_med.allocations, stats_med.bytes_allocated
//     );
//
//     // Track Large HTML Memory Lifecycle
//     let reg_large = Region::new(GLOBAL);
//     let dom_large = parse_html_string(LARGE_HTML);
//     std::mem::drop(black_box(dom_large));
//     let stats_large = reg_large.change();
//     println!(
//         "   [Large HTML] Allocations: {}, Total Leaked/Retained: {} bytes",
//         stats_large.allocations, stats_large.bytes_allocated
//     );
//
//     println!("==================================================\n");
// }
//
// criterion_group!(benches, bench_dense_core);
// criterion_main!(benches);
