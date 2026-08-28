# Changelog

All notable changes to `mosaic-pkg-calendar` are documented in this file.
The format follows [Keep a Changelog](https://keepachangelog.com/) and
the package follows semantic versioning.

## [Unreleased]

### Added

- `elevation: raised;` on the `calendar-root` part in both themes,
  alongside its existing `box-shadow:` — additive, not a replacement.
  mosstyle's new `elevation` property (#12028 item 1, UI41) is the
  channel native backends will read to render their own native shadow
  primitive; no backend reads it yet.

## 0.1.0 — 2026-08-06 — initial release

### Added

- `Calendar` component: a 6×7 (42-cell) month grid, Sunday-first, built
  entirely from UI29 kernel primitives (no wrapped package). Wired to
  task-core's `calendar(range, view)` projection (host-driven — Calendar
  does no date arithmetic or filtering itself).
- Drag-to-move rescheduling via the UI35 `HostDraggable`/`HostDropTarget`
  kernel: dragging an event onto a different day fires `onEventDropped`
  as a proposal, exactly matching Board's already-shipped drop contract.
- Multi-day events render on every day they span, not just their start
  day — the engine already computes the real span; collapsing to the
  start day (as `design/ui-prototype.html`'s demo JS does) would discard
  data the engine computed.
- Today gets a filled badge; critical/completed/overdue events get a
  conditional text chip (the same "one draggable part, conditional child
  chips for state" trade-off Board's `card-crit` already established, not
  a per-state container restyle — see Calendar.mll's doc comment).
- Both themes (`Calendar.light.msl` / `Calendar.dark.msl`), palette taken
  directly from `design/ui-prototype.html`'s `.cal-*` classes.

See [task-app-calendar-v1.md](../../../specs/task-app-calendar-v1.md) for
the full scope and what's deliberately deferred (week/day views, resize,
time-blocking, weekend/out-of-month cell tinting).
