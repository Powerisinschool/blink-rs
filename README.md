# Blink-Rust: A Systems-First Web Rendering Engine

## Executive Strategy & Architectural Philosophy

Blink-Rust is a focused, structural rewrite of the Blink rendering engine architecture in Rust. The goal is to build a modern web rendering engine by cloning the single-threaded layout mental model of Chromium's LayoutNG.

* The project avoids building commodity components from scratch to maintain strict focus on the layout and DOM pipeline.


* Industry-standard technologies like Google's V8 (via rusty-v8) and Skia are integrated for execution and graphics rendering.


* Rust's memory safety and immutable pipeline paradigms are leveraged entirely for the core tree construction and layout architectures.



## Core Engineering Frontier: The Arena-Allocated DOM

Standard tree structures using traditional raw pointers yield undefined behavior in Rust, while `Rc<RefCell>` implementations result in cycle leaks and severe performance degradation. Blink-Rust solves this through a flat, arena-allocated generational index topology.

### Data-Oriented `dense-core` Architecture

* The DOM topology is split into strictly typed parallel arrays of primitives rather than bloated standard enums.


* Parent and sibling pointers utilize `NonZeroU32` structures to guarantee a strict 4-byte memory footprint per reference.


* Payload keys pack a 2-bit type identifier and a 30-bit dense index into a single 32-bit integer, allowing addressing for up to 1,073,741,823 items per array.


* This Structure of Arrays layout drastically improves CPU cache hit rates during DOM traversal by eliminating the padding overhead natively introduced by Rust enums.



## Workspace & Component Structure

The project is structured as a Cargo virtual workspace monorepo. This ensures strict architectural boundaries and prevents dependency fragmentation across the pipeline.

* **`blink_dom`**: Implements the memory-safe, arena-allocated DOM tree utilizing generational `u32` index keys.


* **`blink_html`**: Provides a strict 1:1 implementation of the HTML5 tokenization and tree-construction algorithms via `html5ever` integrations.


* **`blink_css`**: Houses the custom CSS tokenizer and style cascading engine.


* **`blink_layout`**: Contains the core immutable layout pipeline and Constraint Spaces logic.


* **`blink_render`**: Manages the graphical output and text shaping coordination.


* **`blink_shell`**: The primary executable entry point wiring the core libraries together.



## Phased Implementation Roadmap

The development pipeline is structured into three distinct milestones.

### Phase 1: The Linear Static Pipeline

Establish a unidirectional, non-reactive pipeline mapping text to a rasterized image buffer.

- [x] Implement an HTML string parser generating a basic tree hierarchy.


- [ ] Build a style cascading engine matching selector specificities.


- [ ] Write a simplified box-model layout calculator providing physical boundaries for Block and Inline text flows.


- [ ] Bind layout outputs to drawing coordinates for static frame generation.



### Phase 2: Cloning the LayoutNG Paradigm

Transition to Blink's modern immutable architecture.

- [ ] Encapsulate layout limits into immutable Constraint Spaces passed down during traversal.


- [ ] Generate an immutable physical Fragment Tree to avoid Rust mutable reference collision conflicts.


- [ ] Implement incremental display caching to bypass downstream layout computations based on invalidation boundaries.



### Phase 3: The Binding & Reactive Layer

Inject execution functionality into the static engine layout system.

- [ ] Build a robust marshalling abstraction layer between V8 JavaScript objects and the underlying Rust arena DOM nodes.


- [ ] Connect JS element modifications to an explicit invalidation manager for re-styling and re-painting.


- [ ] Standardize a single-threaded main event queue processing timers, UI inputs, and layout frames.



## Testing and Orchestration

* Unified module testing validates isolated memory layout behaviors across both dense and standard allocation feature flags.


* An `xtask` binary orchestrates complex web platform test pipelines and command-line system builds.