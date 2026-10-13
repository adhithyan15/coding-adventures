# Shared XAML table viewport events

Status: implemented with native Debug evidence; full acceptance remains open. Tracks #14277 and the scroll workload required by #16865; active PR #17287.

## Evidence and owning layer

On head 9beda75349 a native wheel action inside VisiCalc left rows 1-20 visible and emitted no interaction profile. RowHeaderGrid authors HostTable onViewportRows, onViewportShift, viewport-offset and total-rows. The generated event union includes the events, but the generated table has no producers. The Rust adapter already handles both events. Repair mosaic-emit-xaml, never generated output or spreadsheet-specific branches.

The native shape emitter in pipeline.rs creates a Grid with an Auto header and star body, nesting a body ScrollViewer inside an authored outer HostScroll. A working wheel callback alone does not prove that the measured viewport is finite or that headers remain fixed. Measurement and physical ownership must be solved together.

## Contract

Match the existing React contract: viewport events each require exactly one numeric `rows` parameter. onViewportShift requires onViewportRows plus numeric viewport-offset and total-rows bindings. Validate declaration and property types before emission, including renamed event cases. Invalid contracts must produce a compiler diagnostic, never silently disappear. Tables without these optional events retain existing native scrolling behavior.

Generate typed component handlers that dispatch the declared event union. Native support raises data-only row-capacity and row-shift events; it must not reference MosaicRuntimeHost or a specific app. Use current bindings when handling input, not captured old offset/total values.

## Measurement and routing

For an opted-in uniform-row table, measure finite available body height and realized row pitch after layout; subtract the header exactly once. Report at least one visible row, capped by total rows. Do not infer pitch from a fixed spreadsheet constant. Suppress unchanged capacity notifications and coalesce pending layout notifications through DispatcherQueue to avoid synchronous layout/dispatch recursion. Detached tables must not dispatch queued reports. Nonuniform or unmeasurable row geometry needs a documented fallback rather than an unbounded estimate.

There must be one vertical scroll owner for the virtual row window. Preserve horizontal header/body alignment and do not let a nested ScrollViewer consume a vertical wheel event before the virtual-window handler. Translate vertical wheel input using the platform delta convention, accumulate fractional wheel deltas, bound row shifts against the current offset and total, and do not hijack horizontal-wheel or editor input. At boundaries, avoid no-op dispatch loops. Confirm wheel routing on an actual WinUI host before claiming success.

Maintain selected-cell visibility and focus across row-window replacement. Native cell coordinates remain relative to the window while adapter coordinates include viewport-offset; header cells must not consume a data column. Stale retained candidates must continue to fall back to the current tree.

## Required verification

- Emitter tests cover renamed typed emits, malformed payloads, missing offset/total/measurement contracts, nested table scope and ordinary tables without viewport events.
- Executed native-support tests cover capacity deduplication, finite/zero dimensions, row pitch, fractional wheel deltas, boundaries, detachment and changing offset/total.
- Regenerate and compile VisiCalc; use actual wheel actions to move beyond the initial window, verify fixed column headers and numbered rows, click and navigate after scrolling, resize and remeasure, edit/commit and restart the fixture.
- Record realized native cell counts as well as projected VM counts and warm Release/Debug event timings. Merely using ItemsRepeater is not bounded-realization evidence.
- Keep #14277 and #16865 open until the corresponding evidence exists; this document is not acceptance evidence.

## Native layout prerequisite discovered during implementation

The authored shared styles use `flex: 1` on the sheet scroll port. XAML only recognized `flex-grow`, producing an Auto grid row that kept its content height after window resize. Before viewport acceptance, lower the exact `flex: 1` shorthand through the reusable Row/Column star-sizing hints and report it as consumed. Other shorthand forms remain explicitly unsupported; this does not claim general CSS flex equivalence. Add Row and Column regression coverage.

## Implementation and evidence (2026-10-12)

The shared emitter now validates callback schemas/bindings (including renamed
emits), rejects opted-in noncanonical shapes, emits typed component handlers,
and supplies a data-only native viewport producer. Capacity reporting is queued
and deduplicated; detached-generation callbacks are ignored. Virtual tables use
one outer scroll port, top alignment and bounded fractional wheel shifts. The
finite viewport comes from WinUI EffectiveViewportChanged in table coordinates:
https://learn.microsoft.com/en-us/windows/windows-app-sdk/api/winrt/microsoft.ui.xaml.frameworkelement.effectiveviewportchanged

Only the proposed visible rows are checked for uniform geometry. Native resize
testing showed that retained ItemsRepeater cells outside that window can have
stale/unarranged geometry; treating those as visible blocked valid resizing.
Capacity uses available viewport space, capped by the authored MaxHeight, rather
than the previous row window's ActualHeight, so it can grow as well as shrink.

Validation: 386 emitter unit tests plus 14 integration tests pass; an additional
nested-table scope regression also passes (one ignored
doctest). The generated C# probe executes capacity/fractional-delta/boundary
arithmetic and queue coalescing, unload/reload generation checks and queue
refusal. Generated WinUI Debug build passes with zero errors and two existing
unused PropertyChanged event warnings. Security subagent review passed after
repairing its finding that valid callbacks could disappear on fallback shapes.

Actual interaction used the isolated release-fixture-state.json and the retained
Oct 7 Rust DLL (not a matched new release runtime):

- Wheel delta 600 moved rows 1-20 to 16-35; headers stayed fixed. Clicking B22
  selected B22, then Right selected C22. Entering =20+22 and committing produced
  42. Normal close/restart restored C22, 42, formula =20+22 in both screenshot
  and accessibility text.
- After the shared flex shorthand fix, ordinary window capacity was 19. A
  shorter window measured 12; persisted cursor size confirmed 12, offset 15.
- Final growth probe restored 19 rows, then maximization measured 31, showing
  rows 16-46 with headers anchored at the top. Wheel delta 2400 reached rows
  70-100; another 600 left the boundary unchanged. Persisted cursor offset 69,
  size 31 confirms the bound.
- Final growth profile is adjacent in mosaic-xaml-table-viewport-growth-20261012.jsonl.
  Its three interaction-to-next-render samples are 201.8829, 628.3800 and
  13.9605 ms (startup capacity, maximize capacity, wheel respectively). The
  extra boundary wheel produced no fourth interaction. These are callback
  timings, not presentation timestamps or meaningful warm percentiles.

## Remaining acceptance

#14277 and #16865 remain open. Required next evidence includes matched warm
Release/Debug percentile workloads, full projected/native realization counts,
horizontal wheel/editor interaction and keyboard movement across window edges.
The accessibility tree exposed another existing shared gap under #14278: with
row header 16, cell names still say row 1 (relative window coordinate). Absolute
accessible row labels need repair before full accessibility acceptance. No native
release or epic completion is claimed.
