## 0.98.0 — 2026-08-11 — parenthesized real-literal output

The source-spelled real output path now unwraps exact balanced parentheses
around direct literals, including signed forms. Parenthesized arithmetic still
fails closed, so grouping cannot bypass the runtime f64 formatter boundary.

