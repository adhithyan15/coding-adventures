# HTML Renderer

**Language-agnostic HTML visualization generator** for the computing stack.

## What this package does

Reads JSON conforming to the **PipelineReport** contract and generates self-contained HTML files. Any language that can produce the JSON (Python, Ruby, TypeScript, etc.) can use this renderer — it has no dependency on other coding-adventures Python packages.

## Where it fits

```
Logic Gates → Arithmetic → CPU → ARM → Assembler → Lexer → Parser → Compiler → VM
                                                                                 │
                                                                          Pipeline Runner
                                                                                 │
                                                                          JSON Report
                                                                                 │
                                                                       ┌─────────┴─────────┐
                                                                       │                   │
                                                                  Python impl        Ruby / TS impl
                                                                       │                   │
                                                                       └─────────┬─────────┘
                                                                                 │
                                                                        [HTML Renderer]
                                                                                 │
                                                                        Static HTML file
```

The renderer is **pluggable**: any language can emit the PipelineReport JSON, and this single renderer turns it into a visual HTML page.

## Installation

```bash
uv add coding-adventures-html-renderer
```

## Usage

```python
from html_renderer import HTMLRenderer

renderer = HTMLRenderer()

# From a JSON file
renderer.render("pipeline-report.json", output="report.html")

# From a Python dict (already parsed JSON)
report = {
    "meta": {"generated_at": "2025-01-01T00:00:00Z"},
    "stages": [...]
}
renderer.render_dict(report, output="report.html")
```

## Spec

See [11-html-visualizer.md](../../../specs/11-html-visualizer.md) for the full specification.
