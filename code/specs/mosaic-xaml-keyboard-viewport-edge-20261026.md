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
pass. Keyboard input now requests the shared viewport shift and uses a bounded,
generation/request-scoped continuation. Executed emitted C# probes cover exact
offset confirmation, rejected/clamped/delayed dispatch, missing/replaced cells,
unload, newer requests, focus ownership, focus failure, editor focus retention,
and one activation. Nested-table lookup isolation also passes. All 395 emitter
library tests pass. The complete package test run also passes (395 library and
14 integration tests; one documentation example ignored).

## Native receipt, 2026-10-26

Generated VisiCalc and VisiCalcStartup component triples and row VMs using the
public `from_pipeline` API from implementation 942fe157030 (documentation head
10cf14c4219). Resolve the authored package references with LayoutPackageResolver
before emission; compile the dark style without the pre-expansion part map,
which does not contain the inlined grid parts. Other project/runtime host files
come from the retained #18294 candidate. This is a scoped emitter validation,
not a full current-main application runtime rebuild or release artifact.

Debug WinUI build: zero errors, two existing CS0067 unused-event warnings.
Managed VisiCalc.dll SHA256:
`44E572BA995DC066275227D97A307CB736E30827FFF84F4783F825D0439ED7BB`.
Unchanged Rust mosaic_app.dll SHA256:
`A18198012F2985EC5CEA6E10A2151388D2E9093F567A59F46A72E45C28BC2124`.

Actual Windows interaction, observed through screenshots and UIA:

1. Fresh isolated snapshot, scroll to visible rows 16–34, click B17.
2. Up selects B16; Up again selects B15 and reveals rows 15–33.
3. Click B33, Down selects B34 and reveals rows 16–34; another Down selects B35.
4. Right selects C35. Click the formula editor, enter `=20+22`, then Left:
   selection remains C35 and the editor retains the formula. Enter commits 42.
5. Close normally and relaunch with the same isolated snapshot. Screenshot and
   UIA report `C35, 42, formula =20+22` in the restored row window.

State isolation uses `MOSAIC_APP_STATE_PATH`. An initial launch mistakenly used
`MOSAIC_STATE_PATH`, was closed without workbook input, and is excluded from the
receipt. Subsequent interaction used `edge-state-02.json` outside the checkout.
The earlier Release Save picker remains untouched; this is snapshot restart
evidence, not actual file Save/Open acceptance.

Raw phase logs are adjacent `mosaic-xaml-keyboard-edge-debug-20261026.jsonl` and
`mosaic-xaml-keyboard-edge-restart-20261026.jsonl` (normal checkout LF conversion
may differ from captured Windows bytes). Original SHA256 respectively:
`8E5C29D3A688457D42F795423BA9D90E123DC837968F8464EB649DCBF198B664`,
`CD05A20CE5C60E1EDFCBE6555A7DF891060CDDB1DF30E603BFE737F5A65FEF5E`.
The interaction log contains 1063 phase records and 14 handler/render pairs.
It includes dispatches for both viewport movement and cell activation, not one
sample per physical key. Detailed profiling was enabled; this short correctness
probe does not establish performance percentiles. Render samples include
165.2926 ms and 108.9595 ms tails, so #16865 remains open.

An attempted full Cargo test link exhausted the available disk (LNK1318 PDB
LIMIT). Cleaning only this checkout's regenerable Cargo target recovered 1.1 GiB.
The passing package run disables debug symbols and incremental artifacts; no
source or test coverage was removed. Security review passed before publication.
Existing #16865 warm populated measurements remain valid but do not cover this
boundary. The separate PickerHost tooling limitation in #14548 does not block
implementation or independent keyboard probes; do not claim Save/Open acceptance.
