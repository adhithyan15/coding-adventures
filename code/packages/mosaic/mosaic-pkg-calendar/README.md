# mosaic-pkg-calendar

> Month-grid calendar view wired to task-core's `calendar(range, view)`
> projection, with drag-to-move rescheduling via the UI35 drag kernel.

A 6×7 month grid built entirely from UI29 kernel primitives — no wrapped
package, the same shape as `mosaic-pkg-grid`. See
[task-app-calendar-v1.md](../../../specs/task-app-calendar-v1.md) for the
full scope and what's deliberately deferred.

## What this package exports

One component, per `mosaic-package.toml`'s `[components].exports`:

| Component  | Role                                   | File trio |
|---|---|---|
| `Calendar` | month grid + nav + drag-to-move events | `Calendar.mil` / `Calendar.mll` / `Calendar.{dark,light}.msl` |

## How it fits in the stack

```
          ┌──────────────────────────────────────────┐
          │  Host application (task-app's Calendar view) │
          └─────────────────────┬──────────────────────┘
                                │ component reference
                                ▼
          ┌──────────────────────────────────────────┐
          │  mosaic-pkg-calendar (this package)       │
          │  Calendar → Column/Row/Text/HostButton/   │
          │             HostDropTarget/HostDraggable  │
          └────────────────────┬─────────────────────┘
                                │ kernel primitives only
                                ▼
                     UI29 kernel + UI35 drag kernel
```

## Fat engine, dumb UI

Calendar does no date arithmetic, filtering, or scheduling of its own. The
host computes the visible month's grid range, calls task-core's
`calendar(range, view)` projection, and hands this component pre-shaped
cell/event rows — see `calendarData()` in
`code/programs/mosaic/task-app/host/web/src/main.tsx` for the reference host
consumer.

## Drag-to-move, not drag-to-resize

Dropping an event on a different day fires `onEventDropped` — a PROPOSAL,
exactly like `mosaic-pkg-grid`/task-app's Board section: the UI35 contract
(`key`, `kind`, `targetKey`, `position`). The host decides what it means; for
task-app that's `engine.setConstraint({ id, constraint: { mustStartOn } })`,
**not** a deadline change (see task-app-calendar-v1.md for why the
calendar's own display precedence makes that distinction load-bearing, not
cosmetic). Resize is out of scope — the UI35 kernel doesn't support it
today (see [UI35-host-drag-drop.md](../../../specs/UI35-host-drag-drop.md)
§7).

## Usage

```moslayout
// In a host component's .mll:
pkg::mosaic-pkg-calendar::Calendar (
  calendar-title:  slot: calendar-title ,
  calendar-cells:  slot: calendar-cells ,
  calendar-events: slot: calendar-events ,
  onPrev:          emit: onCalendarPrev ,
  onNext:          emit: onCalendarNext ,
  onEventDropped:  emit: onCalendarEventDropped
)
```

The host builds `calendar-cells` (42 rows: `[day-number, day-key, is-today]`)
and `calendar-events` (one row per event **per day it spans**: `[task-id,
label, day-key, critical, completed, overdue]`) from a `calendar(range,
view)` call whose range covers the visible 6×7 grid — see `calendarData()`
in `main.tsx` for a worked example.

## Smoke test

```bash
cd code/packages/mosaic/mosaic-pkg-calendar
cargo test
```

Mirrors `mosaic-pkg-sheet`'s own smoke test: manifest parses and declares
the expected export; `Calendar.mil` compiles via `mosmodel-compiler`;
`Calendar.mll` compiles against that interface via `moslayout-compiler`
(with an explicit pin that it actually uses `HostDropTarget`/
`HostDraggable`, not a degraded static container); both themes' `.msl`
compile against the resulting part map.

## License

MIT OR Apache-2.0.
