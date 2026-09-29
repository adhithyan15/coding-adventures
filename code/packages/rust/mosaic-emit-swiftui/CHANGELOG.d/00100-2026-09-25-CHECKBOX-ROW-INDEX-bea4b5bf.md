## 2026-09-25 (checkbox row index)

- **`HostCheckbox` in a list reports which row changed (UI29-2 §2.1.1).** Inside a `For`, an `onToggle` that targets `( index : number )` now carries the row index, exactly as a `HostButton` click does. Before, it carried only the new checked value, so a list of checkboxes could not say which item was toggled, and `mosaic-pkg-checklist` had to draw a toggle button beside a "☐" glyph. Any other single parameter still receives the checked value.
  - `checkbox_toggle_dispatch` serves both the `Toggle` binding (its setter parameter is `_` when unused) and the mixed-state `Button`. `checked` reads a loop binding or row expression through `_mosaicTruthy` with the loop index rewritten; `label` accepts a binding or row expression.

