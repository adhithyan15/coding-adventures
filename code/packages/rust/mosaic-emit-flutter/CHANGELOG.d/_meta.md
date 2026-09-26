# Changelog — mosaic-emit-flutter

All notable changes to `mosaic-emit-flutter` will be documented in
this file.

## [Unreleased]

- **`HostCheckbox` in a list reports which row changed (UI29-2 §2.1.1).** Inside a `For`, an `onToggle` that targets `( index : number )` now carries the row index, exactly as a `HostButton` click does. Before, it carried only the new checked value, so a list of checkboxes could not say which item was toggled, and `mosaic-pkg-checklist` had to draw a toggle button beside a "☐" glyph. Any other single parameter still receives the checked value.
  - The index form reuses `HostButton`'s event args, replacing the old `(v ?? false) ? 1 : 0` flag a `number` parameter used to receive. `checked` reads a loop binding or row expression through `_mosaicTruthy`; `label` takes the same forms as a `HostButton` label.

