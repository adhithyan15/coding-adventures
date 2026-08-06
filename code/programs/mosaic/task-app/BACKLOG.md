# task-app backlog

Working backlog for the [super-app roadmap](../../../specs/task-app-super-app.md). Ordered
by priority — top item is next up. Re-prioritized whenever a new item is discovered mid-flight.
Each item, once picked up, follows: spec-sync → tests → implementation → CHANGELOG → README →
`/security-review` → PR → `/babysit-pr` → auto-merge.

## Next up (priority order)

1. **Phase 5 — Sheet component.** `mosaic-pkg-sheet` on top of `mosaic-pkg-grid` (v0.2, reuse),
   wired to the engine's `table(view)` projection (already shipped, [#8626](https://github.com/adhithyan15/coding-adventures/pull/8626)/[#9110](https://github.com/adhithyan15/coding-adventures/pull/9110)-era). Needs: editable cells,
   engine-driven sort/filter/group (the view-selection pipeline already exists at the engine
   layer — this is wiring, not new engine work), column show/hide. `mosaic-pkg-grid` gap noted
   in the spec: no sort/filter/multi-select/frozen-cols/row-DnD yet — check whether the sheet
   needs any of those or can lean on the engine's own filter/sort instead of grid-native ones.
   Reordered ahead of where the roadmap originally placed it relative to the board (Phase 6
   shipped first in practice) — no dependency runs the other way, so doing it now just catches
   the roadmap up.

2. **Phase 7 — Calendar component.** `mosaic-pkg-calendar`, wired to the engine's `calendar(range,
   view)` projection (already shipped, [#8726](https://github.com/adhithyan15/coding-adventures/pull/8726)) + the UI35 drag kernel for
   drag-to-reschedule/resize. Month/week/day views, time-blocking, auto-schedule placement around
   fixed commitments. The drag kernel is proven end-to-end now (board PR found and fixed three
   real bugs in it), so this should be smoother than the board was.

3. **Phase 8 — Notes component + entity.** The engine has **no notes entity at all** yet — this
   needs a `task-core` model addition (standalone notes + attachable to any task/project) before
   the `mosaic-pkg-notes` UI (adapted from `mosaic-pkg-note-editor`, which was built for a
   different domain — Anki notes — and needs re-pointing at generic entities).

4. **Phase 9 — App-shell assembly + progressive disclosure.** Partially done already via ad hoc
   UI-design passes ([#8970](https://github.com/adhithyan15/coding-adventures/pull/8970), [#8983](https://github.com/adhithyan15/coding-adventures/pull/8983), [#9112](https://github.com/adhithyan15/coding-adventures/pull/9112), [#8994](https://github.com/adhithyan15/coding-adventures/pull/8994), [#9110](https://github.com/adhithyan15/coding-adventures/pull/9110), [#9127](https://github.com/adhithyan15/coding-adventures/pull/9127), [#9136](https://github.com/adhithyan15/coding-adventures/pull/9136))
   — theming, project switching, nested-project hierarchy, shell/groups/status/cards all landed.
   Remaining: package it as a reusable `mosaic-pkg-project-nav` (nested-project tree + view
   switcher), and the per-project/task **complexity config** (board-only ↔ full CPM) that the
   spec calls "the single most important product rule" (§2.3) — currently every project exposes
   the same surface regardless of how simple it is.

## Backlog (lower priority — Phase 10+, spec explicitly defers these)

- **Native drag support for HostDraggable/HostDropTarget.** Every non-web backend (SwiftUI,
  Compose, Qt, Flutter, WinUI/XAML, webcomponent) currently degrades the drag family to a plain
  static container — see `code/specs/UI35-host-drag-drop.md`. This means the board (and, once
  built, the calendar) is fully interactive on web but inert on every native shell. Spec-deferred
  intentionally (native shells are Phase 10+), but tracked as a spawned task:
  [background task](task_239f7f69) "Wire HostDraggable/HostDropTarget into mosaic-emit-xaml" — do
  XAML first since it's the most-built-out native backend, then fan out the same pattern to the
  rest.
- Recurring tasks / reminders UX.
- Automation rules (Butler-style).
- Resource-leveling UI (the engine's `constraint-*` leveling exists; no UI surfaces it).
- Portfolio dashboards / cross-project rollup views.
- `IndexedDBStorage.query()`/`transaction()` are unimplemented (spec §9) — revisit if a view
  needs SQL-over-IndexedDB rather than load-all.

## Resolved (kept for traceability, not actionable)

- **Phase 6 — Board (kanban) view.** [#9897](https://github.com/adhithyan15/coding-adventures/pull/9897),
  merged. Drag-and-drop columns wired to the UI35 kernel.
- Dark-mode "Add task" pressed-color regression (copy-pasted from light theme) — fixed alongside
  the board PR's CI failure.
- Stale XAML hover-count acceptance-test assertion (12/9 → 16/13 after the board added 4 new
  segmented-control hover surfaces) — fixed alongside the board PR's CI failure.
