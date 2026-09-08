# mosaic-pkg-spice-workbench

`SpiceWorkbench` is the first renderable Mosaic UI for Berkeley SPICE. It
contains a multiline deck editor, Inspect and Run controls, runnable-analysis
selection, source-span diagnostics, selected-analysis result tables, and a raw
result pane. The matching
`spice-mosaic-app` Rust adapter owns state and invokes the parser/engine.

Waveform traces, schematic capture, and vendor-dialect controls are deliberately
later phases; this package gives each of them one real workbench surface to
extend.
