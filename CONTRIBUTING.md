# Contributing to Blink(e)rs

Thanks for your interest. blink-rs is a pre-alpha engine: the architecture is still settling, so small, focused changes with tests are the easiest to review and merge. For anything large, open an issue first so we can agree on the approach.

## Scope

In scope: the DOM, style cascade, layout, fragment tree, and the glue needed to render them.

Out of scope for now: a JavaScript runtime (Phase 3), networking, and re-implementing components we reuse (such as `html5ever`).

## Getting started

```sh
git clone https://github.com/Powerisinschool/blink-rs.git
cd blink-rs # or whatever you name it
cargo build --workspace
cargo test --workspace
cargo test --workspace --features blink_dom/dense-core
# Writes output.png from assets/test.html + test.css
cargo run -p blink_shell
```

Both DOM storage layouts (default and `dense-core`) must pass the tests. CI runs both.

## Before you open a PR

```sh
cargo fmt --all
cargo clippy --workspace --all-targets
cargo test --workspace
cargo test --workspace --features blink_dom/dense-core
```

## Code layout and invariants

Please keep these in mind when changing code:

- **Crate boundaries.** `blink_layout` must not depend on `blink_render`. Data flows one way: HTML/CSS → DOM/stylesheet → fragments → pixels.
- **Typed ids.** DOM and fragment handles are 1-based, non-zero `Id<T>`. Indexing into a `Vec` goes through `get_index()` or `get() - 1`; don't mix 0-based and 1-based values.
- **Per-node data stays aligned with nodes.** Do not add parallel `Vec`s that must stay index-aligned with `nodes`. A past bug (attributes shifting after text nodes) came from exactly that. Store an index on the node instead.
- **Fragments are moving toward immutability.** Avoid new code that mutates fragments after their children have been laid out, unless it is part of the existing positioning pass.
- **No unsafe** without a comment explaining the invariant and a test that exercises it.

## Tests

- Every bug fix needs a regression test that fails without the fix.
- Layout changes should add or update an expected-rectangle test for the affected fixture in `assets/`. A page that renders "about right" is not a test.
- DOM changes should work under both feature flags.

## Pull requests

- Keep PRs focused: one behavior change per PR.
- Describe what changed and why, and link the issue.
- Include before/after output (a fragment dump or `output.png`) for layout changes.
- Avoid unrelated reformatting in the same PR.

## Good places to start

These are real gaps in the current implementation:

- **Merge adjacent text nodes** in `blink_html`. The parser emits several `Text` nodes for one run of text.
- **Unknown tag names.** `get_tag_index` returns `0` for unknown tags, which is the index for `document`. Custom elements and SVG tags should get a distinct representation.
- **Line wrapping** for inline children in `blink_layout::engine`.
- **Shrink-wrap width** for auto-width inline boxes that contain block children.
- **Golden layout test** for `assets/test.html`, asserting each fragment's rectangle.
- **Warn on unsupported `display` values** instead of silently treating them as `inline`.
- **More selectors:** child combinator (`>`), universal selector, attribute selectors.
- **Property inheritance** for inheritable properties once more properties exist.

## Provenance and licensing

- Contributions are licensed under the project's license (see `LICENSE`).
- Reading public design documents for Chromium, LayoutNG, or the CSS specifications is encouraged. Do not paste code from other engines or projects into this repository. If you think some code should be reused, raise it in an issue first so license compatibility can be checked.
- Only submit code you have the right to contribute.

## Questions and conduct

Open a GitHub issue or discussion for questions. Please be respectful and constructive. See `CODE_OF_CONDUCT.md` for the community standards.