### Fixed — Path geometry binds to slots and For items (#16926)

`Path` coordinates (`cx`, `cy`, `r`, `x1` … `y2`) may now be bound to a
number slot or an expression. A bound one lowers to `(expr).toDouble()`, so
a `For` over `list<list<number>>` draws one line per element.
SpiceWorkbench's schematic grid, wires, terminals and waveform now compile on
Flutter; they were refused before with "only supports a literal number".

- `Offset` and `Size` drop `const` only when a coordinate is bound.
- A bound line's size is folded at run time from `0`, with no `dart:math`
  import.
- All-literal geometry lowers byte-identically to before.
- A slot name that is not a safe Dart identifier is refused.
- `tests/spice_workbench_compiles_to_flutter.rs` compiles the real package
  and pins the bound geometry. MosaicBook's gate checks browser backends
  only, so before this test the failure was invisible to CI.
