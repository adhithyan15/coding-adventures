### Security notes

- `href`, `label`, `text`, and `placeholder` slot/string values
  all flow through `escape_dart_string`, which handles `\`, `"`,
  `$` (critical: Dart interpolates `$ident` inside double-quoted
  strings), `\n`, and `\r`.
- `external`/`target` keywords are validated against allow-lists
  before splicing into block comments — a malicious keyword like
  `false*/dispatch(evil())/*` is impossible because the grammar
  layer guarantees keywords are bare identifiers and we further
  match against `same|new-tab|parent|top` / `false|true`.
- `min`/`max`/`step` are `LayoutPropValue::Number(f64)` from the
  IR, never strings — no injection possible by construction.

## [0.1.0] - 2026-05-23 — initial release

Brand-new backend bringing Flutter (Dart) as the seventh supported
target alongside React, SwiftUI, Qt, HTML, WebComponent, and XAML.

