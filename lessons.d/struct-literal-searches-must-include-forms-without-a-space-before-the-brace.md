---
category: Rust
---

# Struct-literal searches must include forms without a space before the brace

The padding producer import added a required optional struct field. A search
for `SwimlaneFlowchartConfig {` found one owned literal but missed another
written `SwimlaneFlowchartConfig{`. Cargo reported E0063 before any new tests
executed. Preserve that compile-failure receipt and adapt both literals with
padding:None, retaining all earlier expected fields/assertions. Search for the
type name or optional whitespace, then rerun the executable controls; a compile
failure is not evidence of the intended work regression.
