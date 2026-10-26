# XAML keyboard navigation across a virtual row-window edge

## Observed defect and scope

For #14278, #14277 and epic #14267. The retained generated VisiCalc Debug
candidate from #18294 reproduces this sequence: scroll down to rows 16-34;
click B17; Up selects B16; another Up leaves B16 selected and the viewport
unchanged. Settled screenshot and UIA observations agree. The shared emitter
currently calls FindCell for the adjacent local coordinate and returns when
that cell is not realized. A nonexistent local row -1 cannot activate the
adapter even though logical row 15 exists.

Repair the shared XAML HostTable capability. Do not patch generated output,
recognize VisiCalc names, or change spreadsheet selection/persistence semantics.
The current table contract already provides MosaicVirtualWindow,
MosaicViewportOffset, MosaicTotalRows and MosaicViewportShift. Use that authored
contract rather than synthesizing a new application event or guessing row data.

## Required behavior

- In-window arrow navigation keeps its existing authored MosaicNavigate path.
- At a vertical local-window edge, a virtual table with a shift subscriber may
  request one row of movement if logical rows exist beyond that edge. Up at
  global row zero and Down at the last logical row do not shift or wrap.
- Validate finite integral offsets/totals and positive window capacity. Invalid
  geometry, absent subscriptions and nonvirtual tables preserve native fallback.
- After a confirmed shift, resolve the desired logical coordinate into the new
  local window, focus its current realized cell, and invoke its authored
  activation exactly once. Preserve the column. Do not activate the old window's
  cell if the adapter rejects, delays, clamps or changes the requested shift.
- Binding/layout may realize cells later. Any continuation must be bounded,
  scoped to the same loaded table generation and request, and cancelled by newer
  navigation or pointer focus. It must not steal focus from an editor or another
  table. Do not keep detached cells alive across unbounded callbacks.
- Reacquire focus after adapter activation using current ownership/coordinates;
  row projections may be replaced. Nested tables must not satisfy outer-table
  lookups. TextBox/PasswordBox/NumberBox caret input remains untouched.
- No horizontal virtualization is introduced by this repair. Preserve existing
  left/right behavior and do not wrap columns.

## Validation plan

Commit this specification before implementation. Add executable emitted-C#
regressions for upper/lower edge success, global boundaries, malformed geometry,
rejected/delayed shifts, unload, newer-input cancellation, no double activation,
nested table ownership, detached realization and editor focus. Retain existing
emitter and native table tests. Run security review before publication.

Regenerate a native candidate from the public generator, build and interact with
it. Reproduce B17 -> B16 -> B15 with a revealed row 15, then test lower-edge
movement, repeated crossing, in-window arrows, pointer selection and formula
editing/restart. Record exact source/runtime hashes and profiler evidence.
Source assertions and stub execution alone do not establish native acceptance.

## Status

The emitted keyboard boundary arithmetic is implemented and exercised by the
compiled C# projection probe (2026-10-26): upper/lower steps, global boundaries,
short windows, fractional/nonfinite/out-of-range metadata and invalid capacity
pass. The helper is not connected to keyboard input yet. The cancellable
continuation, native regeneration and interaction acceptance remain pending;
this intermediate commit does not fix the observed native navigation defect.
Existing #16865 warm populated measurements remain valid but do not cover this
boundary. The separate PickerHost tooling limitation in #14548 does not block
implementation or independent keyboard probes; do not claim Save/Open acceptance.
