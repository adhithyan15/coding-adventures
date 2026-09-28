## 2026-09-25 (checkbox row index)

- **`HostCheckbox` in a list reports which row changed (UI29-2 §2.1.1).** Inside a `For`, an `onToggle` that targets `( index : number )` now carries the row index, exactly as a `HostButton` click does. Before, it carried only the new checked value, so a list of checkboxes could not say which item was toggled, and `mosaic-pkg-checklist` had to draw a toggle button beside a "☐" glyph. Any other single parameter still receives the checked value.
  - `emit_host_checkbox` takes the `For` payload scope; the new `checkbox_toggle_dispatch` picks the payload for both `Checkbox` and `TriStateCheckbox`, and the lambda parameter is `_` when the dispatch does not read it.

