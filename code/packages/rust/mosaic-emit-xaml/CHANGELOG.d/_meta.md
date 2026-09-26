# Changelog — mosaic-emit-xaml


## [Unreleased]

- Repeated children of horizontal rows now use a horizontal WinUI `StackLayout`, including structural table headers and cells. Nested columns and boxes retain vertical repeater layout (#14274).

- **`HostCheckbox` in a list reports which row changed (UI29-2 §2.1.1).** Inside a `For`, an `onToggle` that targets `( index : number )` now carries the row index, exactly as a `HostButton` click does. Before, it carried only the new checked value, so a list of checkboxes could not say which item was toggled, and `mosaic-pkg-checklist` had to draw a toggle button beside a "☐" glyph. Any other single parameter still receives the checked value.
  - The index form reads the row from the element's `Tag` (`HostButton`'s payload builder) and is wired to `Click`, not `Checked`/`Unchecked`: those also fire when the app's state moves `IsChecked` through the OneWay binding, which would flip a toggle-by-index straight back. A text-marker `checked` expression (`( row[4] )`) is not yet supported here: `IsChecked` is a typed `bool?` binding and needs a truthy converter; Boolean expressions and literals work.

- `tests/task_app_hover_compiles_to_xaml.rs` expects 14 native hover bindings, not 8, and names the Checklists targets (C3c, #14018). The test reads the app by path, so #15962 never ran it and main was red.

