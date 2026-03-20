# html-renderer

Language-agnostic HTML visualization generator for computing-stack pipeline reports in Rust.

## What it does

This crate reads JSON conforming to the `PipelineReport` contract and produces a
self-contained HTML document with embedded CSS.

The Rust port supports:

- rendering from an in-memory `PipelineReport`
- loading a report from JSON on disk
- writing the rendered HTML to a file
- stage-aware rendering for lexer, parser, compiler, VM, assembler, hardware,
  ALU, and gate stages
- safe HTML escaping and a JSON fallback for unknown stages

## Why this package exists

The HTML renderer sits at the boundary between implementation code and teaching
material. Any language in the repository can produce the JSON report. This
renderer turns that report into a shareable static HTML artifact.

## Example

```rust
use html_renderer::{HTMLRenderer, PipelineReport, ReportMetadata, StageReport};

let report = PipelineReport {
    source: "x = 1 + 2".to_string(),
    language: "rust".to_string(),
    target: "vm".to_string(),
    metadata: ReportMetadata {
        generated_at: "2026-03-18T12:00:00Z".to_string(),
        generator_version: "0.1.0".to_string(),
        packages: std::collections::BTreeMap::new(),
    },
    stages: vec![StageReport::new(
        "lexer",
        "Tokenization",
        "x = 1 + 2",
        "6 tokens",
        0.12,
        serde_json::json!({
            "tokens": [
                {"type": "NAME", "value": "x", "line": 1, "column": 1}
            ]
        }),
    )],
};

let renderer = HTMLRenderer::new();
let html = renderer.render(&report).expect("render should succeed");
assert!(html.contains("Computing Stack Report"));
```

## Running tests

```bash
cargo test -p html-renderer -- --nocapture
```

## Spec

See [11-html-visualizer.md](../../../specs/11-html-visualizer.md) for the full contract and design.
