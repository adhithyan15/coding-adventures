## 0.149.0 — 2026-08-13 — stable local while dependencies

Capped abstract execution of a `while` control may now use statically known
ordinary local scalar dependencies when the loop body does not reference them.
Globals, arrays, by-name values, unknown snapshots, and body-referenced
dependencies remain conservative.

