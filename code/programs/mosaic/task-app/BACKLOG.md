# task-app backlog

Working backlog for the [super-app roadmap](../../../specs/task-app-super-app.md). Ordered
by priority — top item is next up. Re-prioritized whenever a new item is discovered mid-flight.
Each item, once picked up, follows: spec-sync → tests → implementation → CHANGELOG → README →
`/security-review` → PR → `/babysit-pr` → auto-merge.

## Next up (priority order)

1. **Fix `Box`/generic-container payload dispatch in `mosaic-emit-react`.** (Discovered
   2026-08-06 building Sheet.) `HostButton`/`HostInput`/`HostLink` get "dedicated" wiring
   that can synthesize an index/value payload from loop context or the DOM event; a generic
   `Box`'s `onClick` (and any other generic-container emit) always dispatches void, no matter
   what the target emit declares. This is why `mosaic-pkg-grid`'s `Cell` (a `Box`) can't
   deliver `onNavigate(row, col)` to anything — blocking sheet cell-editing (currently shipped
   read-only, see `mosaic-pkg-sheet`'s CHANGELOG) and any future component that needs a
   generic container to report *which* loop iteration was interacted with. Two candidate
   fixes, either needs its own design pass: (a) narrow — give `Cell` explicit `row`/`col`
   props read via expression, the same mechanism UI35's `drag-key` already uses (payload from
   an authored expression, not synthesized loop context); (b) broad — generalize
   `mosaic-emit-react`'s loop-scope tracking from `Option<ForPayloadScope>` (innermost scope
   only) to a real stack, so index-payload synthesis works for `Box` at arbitrary nesting
   depth the way it already does for `HostButton`. Prioritized above the next roadmap phase
   because it's now a known correctness gap in a shipped, reused package, not speculative.

2. **Finish sheet cell-editing**, once the above lands. `mosaic-pkg-sheet`'s `SheetField`
   catalogue in `main.tsx` (`editable`/`write` per column, `SHEET_FIELDS`) and the
   field-kind-aware write-back dispatcher were built and then stripped back to no-ops for the
   v1 read-only ship — the design is already done, this is re-wiring once `Cell` can report
   which cell was clicked.

3. **Rename off "Planner" + close the design-fidelity gap.** (User decision, 2026-08-06.)
   "Planner" collides with an existing trademark (Microsoft Planner et al.) — needs a new,
   trademark-safe name. User wants a **short list of alternatives proposed**, not "Travail"
   locked in unilaterally. Separately, the live app has drifted from `design/ui-prototype.html`
   again — same gap a prior pass already closed once (`### Changed - the app now looks like the
   design` in `CHANGELOG.md`), so it needs re-closing, not a new visual direction; the mock is
   still the target. Two independent sub-tasks: (a) propose names, get one picked, sweep the
   rename (brand text in `TaskApp.mll`, `README.md`, any package names that embed it), (b) audit
   current app vs. `design/ui-prototype.html` and fix what's drifted. Queued deliberately behind
   Sheet — do not jump ahead of in-flight work.

4. **Phase 7 — Calendar component.** `mosaic-pkg-calendar`, wired to the engine's `calendar(range,
   view)` projection (already shipped, [#8726](https://github.com/adhithyan15/coding-adventures/pull/8726)) + the UI35 drag kernel for
   drag-to-reschedule/resize. Month/week/day views, time-blocking, auto-schedule placement around
   fixed commitments. The drag kernel is proven end-to-end now (board PR found and fixed three
   real bugs in it), so this should be smoother than the board was.

5. **Phase 8 — Notes component + entity.** The engine has **no notes entity at all** yet — this
   needs a `task-core` model addition (standalone notes + attachable to any task/project) before
   the `mosaic-pkg-notes` UI (adapted from `mosaic-pkg-note-editor`, which was built for a
   different domain — Anki notes — and needs re-pointing at generic entities).

6. **Phase 9 — App-shell assembly + progressive disclosure.** Partially done already via ad hoc
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

- **Phase 5 — Sheet component, read-only v1.** `mosaic-pkg-sheet` shipped: filter/sort
  toolbar + 10-column `Grid` view over `table(view)`. Cell editing deferred — see items
  1-2 above. Found + fixed two real bugs in packages nothing had driven through a real
  app before: `mosaic-pkg-grid` 0.2.2 (`Cell`'s `onClick` never actually fired) and
  `mosaic-emit-react` (`HostInput` `onCommit` never carried its declared payload).
- **Phase 6 — Board (kanban) view.** [#9897](https://github.com/adhithyan15/coding-adventures/pull/9897),
  merged. Drag-and-drop columns wired to the UI35 kernel.
- Dark-mode "Add task" pressed-color regression (copy-pasted from light theme) — fixed alongside
  the board PR's CI failure.
- Stale XAML hover-count acceptance-test assertion (12/9 → 16/13 after the board added 4 new
  segmented-control hover surfaces) — fixed alongside the board PR's CI failure.
