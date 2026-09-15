### Venture table formatting context

- Added a reusable `layout-table` engine for anonymous table boxes,
  header/body/footer ordering, captions, fixed/auto intrinsic column sizing,
  column hints, row/column spans, border spacing/collapse, vertical alignment,
  overflow-safe minimums, and producer diagnostics.
- Integrated computed CSS and HTML span metadata through shared recursive
  layout and paint, with a deterministic browser fixture consumed by the same
  native and generated host pipeline.

