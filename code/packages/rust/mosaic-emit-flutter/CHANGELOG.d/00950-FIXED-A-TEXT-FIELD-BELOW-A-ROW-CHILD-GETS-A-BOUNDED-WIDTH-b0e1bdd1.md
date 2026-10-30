### Fixed — a text field below a Row child gets a bounded width

NoteEditor crashed on Flutter (backlog 6d4). Its
`Row [ editor-body ] { Column, Box [ focused-field-column ] { Text,
HostInput, Text, HostInput } }` handed the Row's unbounded child width
through the `Box` to both fields, and a `TextField` asserts on one: "An
InputDecorator, which is typically created by a TextField, cannot have an
unbounded width". Only a DIRECT Row `HostInput` had a fallback `Expanded`.

- `emit_paired_children` now wraps a non-flex child of a flex-accepting Row
  in `Expanded` when `subtree_needs_bounded_width` finds a `HostInput` /
  `Input` without a fixed `width`, or a `HostNumberInput`, that no
  fixed-`width`, `max-width` or horizontal-`HostScroll` container
  re-bounds. A direct Row `HostInput` keeps its own single wrapper;
  `flex-grow` and a percentage `width` are still decided first.
- The wrapped child is no longer a direct Row child, so it computes
  `width_bounded` and a Row inside it accepts flex again.
- The decision reads and drops no style: every native gate (NoteEditor,
  Engram, CardBrowser, DeckOptionsPanel, NoteTypeEditor) is unchanged, as is
  MosaicBook's `--check`.
- Measured with widget probes that pump the emitted widgets: NoteEditor 27
  exceptions to 0; Engram's Add, Browse and Options screens 23 / 23 / 24 to
  0, in both the default and `touch` layouts. Of the 36 emitted packages
  and apps, only Engram and the four packages it mounts with this shape
  (NoteEditor, CardBrowser, DeckOptionsPanel, NoteTypeEditor) change.
- Tests: `row_child_input_bound_tests` pins the wrapper, the single wrap of
  a direct field, the number-input case, the fixed-width and
  horizontal-scroll exclusions, an unbounded Row getting no wrapper, and a
  nested Row being re-bounded. The new `fixtures/row-child-input` fixture
  and its widget test run in the Flutter runtime lane and fail on any
  exception the pump raises. Spec: UI59 §12.
