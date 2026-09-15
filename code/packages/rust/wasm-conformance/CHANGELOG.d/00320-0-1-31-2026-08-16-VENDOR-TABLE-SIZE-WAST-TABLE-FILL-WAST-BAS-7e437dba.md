## 0.1.31 — 2026-08-16 — vendor table_size.wast + table_fill.wast; baseline regen (task #98)

### Changed

- Baseline regen: vendored `table_size.wast` and `table_fill.wast` --
  both 100% pass, every directive kind, on the first regen after
  implementing `table.grow`/`table.size`/`table.fill` (entirely
  unimplemented before this task). Zero regressions anywhere else in
  the now-69-file vendored corpus (per-file/per-kind aggregate deltas
  confirmed to match exactly what the 2 new files contribute, nothing
  else). Deliberately excludes their sibling `table_grow.wast`: its own
  corpus uses `(elem declare func $f)` -- declarative element segments,
  a third element-segment mode (alongside active/passive) this repo has
  no concept of yet -- and cross-module `register`/import table-growth
  propagation, both fine follow-on work once `elem declare` lands,
  most naturally alongside task #97's own Element/is_passive rework.

