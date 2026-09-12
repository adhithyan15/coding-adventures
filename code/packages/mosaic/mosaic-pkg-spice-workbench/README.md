# mosaic-pkg-spice-workbench

`SpiceWorkbench` is the first renderable Mosaic UI for Berkeley SPICE. It
contains a multiline deck editor, Inspect and Run controls, runnable-analysis
selection, source-span diagnostics, selected-analysis result tables, selectable
waveform traces with axes on HTML and React hosts, and a raw result pane. The matching
`spice-mosaic-app` Rust adapter owns state and invokes the parser/engine.

The trace surface consumes parser-derived waveform artifacts rather than a
second result schema. The schematic host controls expose the canonical passive
and source palette plus analysis selection; their Rust document remains the
only lowering source. Data-bound Path geometry remains a native XAML follow-up;
vendor-dialect controls are later phases.
