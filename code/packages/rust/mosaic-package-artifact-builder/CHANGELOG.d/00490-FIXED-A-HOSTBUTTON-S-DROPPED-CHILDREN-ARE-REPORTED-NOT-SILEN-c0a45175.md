### Fixed — a HostButton's dropped children are reported, not silent (#15921)

A `HostButton` with a child block compiled cleanly, and the native-complete
report stayed clean, but on most backends the button came out blank.
`collect_native_degradations` now reports
`composition.button-children-unimplemented` (blocking) at the button's
layout path.

| Backend | Reported |
| --- | --- |
| React, Electron, WebComponent, SwiftUI, Compose, Flutter, Qt | always; they lower the button from `label` alone |
| XAML, HTML | only when a `label` is also set; they render the children as content, but the label wins |

The toolkit's `RecordList` (J3a of #14416) was designed as one button per row
with its fields inside it, and that design is where this was found. No `.mll`
in the repo nests children in a `HostButton`, so no package gate changes.

**Also:** five packages lose a stale `(Flutter, "font-size")` pin. This change
rebuilds them, so their "silently fixed" gates fail on it. The same removal is
in #15926, and the identical hunks merge cleanly.

