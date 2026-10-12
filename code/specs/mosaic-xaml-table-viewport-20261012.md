# Shared XAML table viewport events

Status: specified, not implemented or accepted. Tracks #14277 and the scroll workload required by #16865; active PR #17287.

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
