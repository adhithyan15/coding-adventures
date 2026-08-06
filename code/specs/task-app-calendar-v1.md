# task-app Calendar — v1 scope

Phase 7 of [task-app-super-app.md](task-app-super-app.md) calls for `mosaic-pkg-calendar`:
"month/week/day, events, drag-to-reschedule, resize, time-blocking, auto-schedule
placement." This spec narrows that to a first shippable slice, and says explicitly what's
deferred and why — the same "ship narrower, iterate" sequencing Sheet and Board already
used in this codebase (Sheet shipped read-only before cell editing; Board shipped three
columns with no accent bar before richer treatment).

## What v1 ships

- **Month view only.** The engine's `calendar(range, view)` projection
  ([task-core/src/view.rs](../packages/rust/task-core/src/view.rs)) and its WASM/JS
  binding are already shipped (#8726) — this PR is pure UI, no engine work. A 6×7
  (42-cell) grid, Sunday-first, covering the shown month plus the leading/trailing days
  needed to fill whole weeks — matching `design/ui-prototype.html`'s `renderCalendar()`.
- **Drag-to-move**, using the UI35 `HostDraggable`/`HostDropTarget` kernel, the same
  kernel Board already proved end-to-end. Dragging an event onto a different day cell
  reschedules that task to start on the dropped day.
- **Prev/next month navigation.**
- **Event styling**: critical (red-tinted, left border), completed (struck through,
  dimmed), overdue (amber-tinted, left border — the mock has no overdue treatment for
  the calendar specifically; this reuses the list view's overdue vocabulary rather than
  inventing a new one), plain (honey-tinted, left border) — mirrors
  `design/ui-prototype.html`'s `.cal-ev` classes.
- **Today** gets a filled date-number badge; **weekend** and **out-of-month** days get a
  quiet background tint — matching the mock's `.cal-cell.today`/`.wknd`/`.out`.
- **Multi-day events render on every day they span**, not just their start day. The mock
  only renders a single cell per event (it has no span logic in its demo JS), but the
  engine's `CalendarEvent` already carries a real `start`/`finish` span — rendering only
  the start day would silently discard data the engine computed. Fat engine, dumb UI: if
  the engine says a task spans five days, the UI shows it on all five.

## What's explicitly deferred (and why)

- **Week/day views.** The mock only implements a month grid (no rendered week/day
  toggle). Month view alone already exercises the projection, the grid layout, and the
  drag kernel end-to-end; week/day are a straightforward follow-up once this lands, not
  a blocker to shipping *a* calendar.
- **Resize** (dragging an event's edge to change its duration). UI35's own spec says
  this explicitly: "Drag-to-resize (calendar event edges, Gantt bar ends). That is a
  distinct gesture — worth its own primitive later; the calendar ships drag-to-move
  first." ([UI35-host-drag-drop.md](UI35-host-drag-drop.md), §7 Out of scope). The
  kernel does not support it today; building it would mean inventing a second drag
  primitive as a side effect of a calendar PR, which is out of scope here.
- **Time-blocking / intraday placement.** `CalendarEvent.all_day` is hardcoded `true` —
  "reserved for timed events (time-blocking) in a later phase" per the engine's own
  doc comment. There is no time-of-day field on `TaskSchedule` at all yet. This is an
  engine gap, not a UI gap, and belongs to a future engine-side phase.
- **Auto-schedule placement around fixed commitments.** Depends on time-blocking
  existing first (auto-placement needs a notion of "busy" intervals within a day, which
  requires the time-of-day model above).

## How drag-to-move reaches the engine

A day cell is a `HostDropTarget` with `drop-key` set to that cell's ISO date string
(`"2026-08-14"`, not an index — an index would break the moment the visible month
changes, and it wouldn't survive a re-render reordering). An event is a `HostDraggable`
inside its day cell(s), with `drag-key` set to the task's id (mirroring Board's card
`drag-key`).

On drop, the host validates the dragged task id still exists and the target key parses
as a valid date, computes it, then calls `engine.setConstraint({ id, constraint: {
mustStartOn: <day> } })` — **not** `setDeadline`. The calendar's own precedence rule
(`schedule.dates.get(&id)` wins over the deadline fallback) means a CPM-scheduled task's
displayed position comes from the computed schedule, not its deadline; changing the
deadline alone would silently fail to move it on screen. `MustStartOn` is a real,
inflexible date constraint the CPM pass honours, so the dropped position is what the
engine actually schedules going forward — not a fake optimistic move the next
recompute would revert.

## Data shape (mosmodel slots)

Two flat `list<list<text>>` slots, matching the shape Board's `board-columns`/
`board-cards` already established for this kind of loop-rendered grid data — no nested
list-of-list type is needed or (per the existing type grammar) supported:

```
slot calendar-cells  : list<list<text>> ;
// [ dayNumber, dayKey(ISO date), isOutOfMonth, isWeekend, isToday ]  — one per grid cell, 42 rows

slot calendar-events : list<list<text>> ;
// [ taskId, label, dayKey, critical, completed, overdue ]  — one row per (event, spanned day) pair;
// a 3-day event contributes 3 rows, one per day it covers, each with a different dayKey
```

Placing an event in its cell is a nested `For`+`If` exactly like Board's card placement
(`If (when: (event[2] == cell[1]))` inside the cell loop) — comparing the event's day
key against the cell's day key, not indices.

## Layout: a flex-wrap grid, not CSS grid

Mosaic's layout primitives are `Box`/`Row`/`Column`/`Stack` — no grid primitive. Rather
than introduce one, `calendar-grid` is a `Row` with `flex-wrap: "wrap"` (already proven:
mosstyle's per-property system passes through any kebab-case CSS property generically —
`padding-top`/`-right`/`-bottom`/`-left` already exercised this for the design-fidelity
pass) and each `calendar-cell` is `width: "14.2857%"` (100 ÷ 7). Seven same-width cells
per row, wrapping every 7, produces the 6-row month grid without a new primitive.

## Package structure

`code/packages/mosaic/mosaic-pkg-calendar/`, mirroring `mosaic-pkg-sheet`'s file trio
exactly (the sanctioned pattern for a real reusable package — unlike Board, which
shipped inline in `TaskApp.mll` and was flagged in `BACKLOG.md` as a spec deviation):
`Cargo.toml`, `mosaic-package.toml`, `src/lib.rs` (doc-only), `src/Calendar.mil`,
`src/Calendar.mll`, `src/Calendar.light.msl`, `src/Calendar.dark.msl`,
`tests/package_compiles.rs`. No dependency on another package — built from kernel
primitives only, same as `mosaic-pkg-grid`.

## TaskApp wiring

Same shape as Sheet's integration: a `slot calendar-mode : text ;` view flag, an
`emit onShowCalendar ;` added to TaskApp's "choose a view" group, a fifth branch in the
segmented switcher (five uniquely-named buttons per branch, following the existing
`seg-*-on`/`seg-*-off{2,3,4}` pattern so mosstyle can scope each button's hover/pressed
state), a fifth `If`/`Else` content branch inserted before the final `Else` (list), and
`pkg::mosaic-pkg-calendar::Calendar ( ... )` embedded with every slot/emit forwarded
explicitly. `main.tsx` gains a `"calendar"` view state, a `CALENDAR_VIEW` builder
calling `engine.calendar({...})`, a `calendarCells()`/`calendarEvents()` derivation
(computed only when `view === "calendar"`, matching the existing "don't compute what
the current view doesn't need" discipline), and a `calendarEventDropped` dispatch case
mirroring `cardDropped`'s validate-then-op-then-persist shape.
