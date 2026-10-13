## 2026-10-13 (Path geometry binds to slots and For items)

- `Path` coordinates may now be bound to a number slot or an expression
  (#16926). A bound one lowers to `(expr).toFloat()`, so a `For` over
  `list<list<number>>` draws one line per element. SpiceWorkbench now
  compiles on Compose; it was refused before with "only supports a literal
  number".
- In bound contexts, literals are written as `Float` (`188f`), so
  `maxOf(0f, …)` and the circle's `cx - r` type-check.
- An expression goes through `compose_collection_index_expr`, as `Text`
  content does, so `xs[i]` inside a `For` uses the loop's `Int` index.
- A bound value that is not finite becomes `0f`, and a bound radius is
  floored at `0f`, so a diverged value never reaches `.dp` layout.
- All-literal geometry lowers byte-identically to before. An unsafe slot name
  is refused.
- `tests/spice_workbench_compiles_to_compose.rs` compiles the real package
  and pins the bound geometry.
