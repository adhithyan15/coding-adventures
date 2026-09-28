### Fixed — at compact width the navigation pane could not be reached (#15834)

The compact branch emitted `Scaffold(drawer: …, body: …)` and nothing else.
Flutter draws the hamburger that opens a drawer **only from an `AppBar`**, and
this `Scaffold` has none, so the pane opened on an edge-drag gesture and by no
other means. Measured on the generated fixture at 500px:

```
AppBar: 0   IconButton: 0   buttons: 0   drawer button: 0
actionable semantics nodes in the whole tree: 0
```

No visible control, nothing focusable by keyboard, and nothing for a screen
reader to activate — in the primitive whose stated reason to exist (UI29-6 §3)
is that a navigation pane is a landmark a screen-reader user moves by.

The compact body now carries a real `IconButton` that calls
`Scaffold.of(context).openDrawer()` (through a `Builder`, since that needs a
context below the `Scaffold`). An `AppBar` would have supplied one for free but
also draws a title bar the layout never asked for — the same objection the XAML
lowering raised against `NavigationView`'s settings row.

The control is named with `pane-title`, and named **twice**: `semanticLabel` as
well as `tooltip`. Measured against a stock `Scaffold(appBar:, drawer:)`,
Flutter's own drawer button reports `label="Open navigation menu"` *and*
`tooltip="Open navigation menu"`; a `tooltip` alone leaves the label empty, and
an unnamed button is barely better than no button. After: one node,
`label="Projects" tooltip="Projects" actions=tap,focus`.

**The test that missed it has been rewritten.** `host-navigation-split-test`
used to open the drawer with

```dart
tester.state<ScaffoldState>(find.byType(Scaffold)).openDrawer();
```

— the test reaching into widget internals. That proves the drawer's *contents*
and nothing about whether anyone can get to them; no user can call
`ScaffoldState.openDrawer()`. It now finds the control, asserts it carries the
`tap` and `focus` semantics actions, and presses it with `tester.tap`. Verified
both ways: it passes against the fixed output and fails against the old one
with `Found 0 widgets`.

- Lower `HostNavigationSplit` to a regular-width pane/detail `Row` and a
  compact Material `Drawer`, preserving pane width and accessible pane naming.
  `collapse: never` emits a static row, while `collapse: auto` keeps Flutter's
  permanent composition limitation visible in the behavior report (#15649).

- Propagate numeric HostTable font sizes through native DataTable and structural
  table headers, text and editors (#15602). Preserve static child/container
  overrides, font families and authored fallback for invalid live sizes. The
  generated Flutter fixture exercises both table shapes, updates and new rows.

- Keep Input/HostInput controllers alive across prop updates and dispose them
  when the generated field unmounts (#15601). Presentation changes and echoed
  edits retain caret, selection and IME composition; external text changes clamp
  the selection and clear stale composition. Flutter widget tests cover controlled
  editing, legacy inputs, focus, disposal and independent table-row editors.

- Project numeric layout `font-size` on Text, HostButton and Input/HostInput
  into live Flutter text styles, retaining static/native fallback for invalid
  live sizes. Reject unsupported value forms and nonnumeric slots (#15598).
  Generated Dart widget acceptance runs in the VisiCalc CI workflow.

