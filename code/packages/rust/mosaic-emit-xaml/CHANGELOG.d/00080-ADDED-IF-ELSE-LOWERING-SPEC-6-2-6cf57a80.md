### Added — `If` / `Else` lowering (spec §6.2)

- `If (when: <expr>) { ... } [Else { ... }]` lowers to twin
  `<ContentControl>`s whose `Visibility` is bound to the expression
  and (for the `Else` branch) the negation via `ConverterParameter=invert`.
- A `BoolToVisibilityConverter` resource is added to
  `<UserControl.Resources>` exactly once per component when any `If`
  is emitted (the converter implementation itself is expected to ship
  with the host project or via a future PR; for now the emitter just
  references the `x:Key`).
- `Else` is paired with the preceding `If` by the new
  `emit_xaml_children` look-ahead. A standalone `Else` errors with
  `UnsupportedPrimitive("Else without preceding If")`.

