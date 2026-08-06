# task-app backlog

Working backlog for the [super-app roadmap](../../../specs/task-app-super-app.md). Ordered
by priority — top item is next up. Re-prioritized whenever a new item is discovered mid-flight.
Each item, once picked up, follows: spec-sync → tests → implementation → CHANGELOG → README →
`/security-review` → PR → `/babysit-pr` → auto-merge.

## Next up (priority order)

1. **Rename off "Planner".** (User decision, 2026-08-06, still open — needs the user
   awake to review.) "Planner" collides with an existing trademark (Microsoft Planner et
   al.) — needs a new, trademark-safe name. User wants a **short list of alternatives
   proposed**, not "Travail" locked in unilaterally. Not started: this needs the user's
   input, not something to advance solo.

2. **Design-fidelity gap, remaining items.** The clear value mismatches (colors that
   didn't match a design token, collapsed-to-uniform spacing, drifted shadow alpha
   values) are closed — see `CHANGELOG.md`'s "re-closed the design-fidelity gap" entry.
   What's left needs either a product decision or real feature work, not a value fix:
   - Icon/SVG assets: brand mark glyph, segmented-switch icons, a progress ring, a
     stroked moon icon for the theme toggle (currently a floating unicode-glyph button
     outside the topbar's flow — also worth moving into the topbar while touching it),
     the pill status dot, group-count badge, composer "+" icon box.
   - Board: a 4th column ("In review" — mock has 4, live has 3), colored top accent bar
     + card-count badge on each column header, critical cards get a colored left border
     instead of a text chip.
   - Richer Gantt: day-grid columns, weekend/today shading, milestone diamonds,
     dependency arrows, hover tooltips, a legend. Current timeline is a simple
     proportional-bar-per-row view.
   - Richer task rows: labels/priority chips shipped (see Resolved below). Still
     missing: critical/slack chips, dependency list, notes paragraph in the detail
     panel — the notes-paragraph item can now pull from the real `Note` entity
     (shipped in Phase 8) via an `attached_task` lookup, not just `Task.notes`.
   - Calendar view — shipped since (Phase 7, see Resolved below); the mock's calendar
     was corroborating evidence for that roadmap phase, not a separate design-fidelity task.

3. **Phase 9 — per-project/task complexity config (board-only ↔ full CPM).** The
   nested-project tree half of Phase 9 shipped as `mosaic-pkg-project-nav` (see Resolved
   below); this is what's left. The spec calls it "the single most important product
   rule" (§2.3) but doesn't define what "board only" actually hides (Timeline tab?
   Sheet columns? which task-detail fields?), whether it's a project setting, a task
   setting, or both (the spec's own wording is "per project/task", literally
   ambiguous), or what middle tiers if any look like — `code/specs/task-app-ui-design.md`
   has no further elaboration either. The engine has no field to hang this on yet
   (`ProjectSettings` is calendar/unit conventions only, no complexity/tier field) — this
   is new `task-core` model work gated on a product decision the spec doesn't make, not
   a UI-only slice. Whoever picks this up next should write a short decision addendum
   (concrete tiers, what each hides, per-project vs. per-task) before touching code.

## Backlog (lower priority — Phase 10+, spec explicitly defers these)

- **Native drag support for HostDraggable/HostDropTarget.** Every non-web backend (SwiftUI,
  Compose, Qt, Flutter, WinUI/XAML, webcomponent) currently degrades the drag family to a plain
  static container — see `code/specs/UI35-host-drag-drop.md`. This means the board and the
  calendar are both fully interactive on web but inert on every native shell. Spec-deferred
  intentionally (native shells are Phase 10+), but tracked as a spawned task:
  [background task](task_239f7f69) "Wire HostDraggable/HostDropTarget into mosaic-emit-xaml" — do
  XAML first since it's the most-built-out native backend, then fan out the same pattern to the
  rest.
- **Calendar week/day views, resize, and time-blocking.** Deferred from the Phase 7 ship —
  see `code/specs/task-app-calendar-v1.md` for the full rationale (resize isn't supported by
  the UI35 kernel today; time-blocking needs a time-of-day field on `TaskSchedule` that
  doesn't exist yet, an engine-side gap, not a UI one).
- **Calendar weekend/out-of-month cell tinting.** Deferred alongside the critical-card border
  gap below — mosstyle can't vary one part's background per data value, only per branch, and
  a 4-way branch duplicating the whole drop-target + event-loop wasn't judged worth it for a
  colour difference. Today's badge shipped (it only needed a small conditional child, the
  same trick Board's `card-crit` chip already uses).
- **Notes: attachment picker, tags, rich text, search.** Deferred from the Phase 8 UI
  ship — see `code/specs/task-app-notes-ui-v1.md`. v1's notes are always standalone (no
  UI to set `attached_task`); tags are generic and reusable in `mosaic-pkg-notes` but
  nothing drives them; `Note.body` is plain text, matching every other free-text field
  in the engine; no search box (mirrors Sheet's own v1 scope cut).
- **Label colour picker + duplicate-name prevention + per-label removal.** Deferred
  from the label-management ship (see Resolved below) — `Label.color` is set (always
  `""` in v1) but nothing renders it; two labels can share a name (mirrors project
  names, also undeduped); removing one label from a multi-label task means retyping
  the whole comma-separated Sheet cell.
- Recurring tasks / reminders UX.
- Automation rules (Butler-style).
- Resource-leveling UI (the engine's `constraint-*` leveling exists; no UI surfaces it).
- Portfolio dashboards / cross-project rollup views.
- `IndexedDBStorage.query()`/`transaction()` are unimplemented (spec §9) — revisit if a view
  needs SQL-over-IndexedDB rather than load-all.

## Resolved (kept for traceability, not actionable)

- **Phase 9 — nested-project tree extracted to `mosaic-pkg-project-nav`.** The
  add/add-subproject composer + nested-project list, extracted verbatim from
  `TaskApp`'s own rail block — same part names, same styling (both themes), same
  layout structure. A refactor, not a redesign; `code/specs/task-app-project-nav-v1.md`
  has the full rationale. The brand row and the view-switcher deliberately stayed in
  TaskApp — the latter is a single, deeply-coupled 36-button block edited in every
  recent view-addition PR, and extracting it right after several rapid additions would
  be a large, high-blast-radius refactor with no corresponding precedent to derisk it,
  unlike the simpler, more self-contained project rail. Verified live,
  behavior-identical to before: create a project, create a nested sub-project (indent
  glyph renders), switch selection between projects (the "on" raised-card styling
  follows). The remaining Phase 9 item (complexity config) is the "Next up" item
  above — it needs a product decision this extraction didn't.
- **Label management (create + assign).** Closes the gap the task-row-chips ship
  below disclosed. A "+ Label" composer wraps the Sheet tab in `TaskApp.mll`
  (deliberately TaskApp's own concern, not a `mosaic-pkg-sheet` slot — Sheet has no
  business knowing about labels), calling the engine's existing `upsertLabel`. A new
  Sheet "Labels" column accepts comma-separated *existing* label names, matched
  case-insensitively, and **rejects the whole edit** on an unrecognised name rather
  than creating a throwaway label or silently dropping it (the same discipline the
  Priority column already uses). Verified live end-to-end: created a label named
  "Urgent", assigned it by typing "urgent" (matched case-insensitively), confirmed
  the chip renders on the List tab; confirmed an unknown name leaves the existing
  assignment untouched rather than corrupting it. Both themes, zero console errors.
  Colour picker, duplicate-name prevention, and per-label removal deferred — see the
  Backlog item above.
- **Task-row priority + labels chips.** Pure display wiring — `task-core` already had
  both fields shipped; `TASK_VIEW`'s `visibleFields` gained `priority`/`labels`,
  `taskRows()` appends them as trailing cells (`row[10]`/`row[11]`, not inserted, so no
  existing index shifts), `TaskApp.mil`/`.mll` gained `chip-priority`/`chip-labels`
  following the exact `chip-due`/`chip-sched`/`chip-over` pattern. Verified live: set a
  task's priority to "High" via the Sheet tab's already-editable Priority column,
  confirmed the chip renders on the List tab in both themes. The labels chip shipped
  with no way to populate it yet — closed immediately after by the label-management
  item above, not left hanging.
- **Phase 8 — Notes, both halves.** Engine: `task-core` gained `Note { id, title,
  body, attached_task }`, stored per-project (`ProjectState.notes`, serde-defaulted
  so already-persisted workspaces keep loading), `upsert_note`/`delete_note` ops,
  `delete_task` orphans (not deletes) a task's attached notes, and both ops are
  wired through `task-wasm`'s `export_op!` all the way to `task-engine.mjs`.
  Verified: 95 `task-core` tests (6 new) including a serde-default backward-compat
  test and a real JSON round-trip; 21 `task-wasm` tests (3 new); and a real
  end-to-end smoke test against the compiled `.wasm` binary (`js/smoke.mjs`) proving
  the delete-orphans-not-deletes behavior through the actual ABI, not just the pure
  Rust layer. Found and fixed a pre-existing gap while here: `set_notes` (a task's
  plain-text field, unrelated to this new entity) had a working WASM export but was
  never wired into `task-engine.mjs` — fixed alongside the new bindings.
  UI: `mosaic-pkg-notes` (adapted from `mosaic-pkg-note-editor` — roughly a third of
  its 25 slots were Anki-domain-specific dead weight, and the focused-field-editing
  cluster collapsed to a single multiline body field), wired into `TaskApp` as a
  sixth tab. Found and fixed one real bug live-testing it, before first ship: a slot
  referenced by its kebab-case name inside an expression (`selected-note-id` instead
  of the correct camelCase `selectedNoteId`) compiled cleanly at every static layer
  but silently miscompiled to JS (`selected - note - id`, subtraction of undefined
  identifiers) — clicking Save threw and blanked the page. Verified live end-to-end
  after the fix: create → type (single-line title + multiline body) → Save → appears
  in the list → persists across view navigation → Delete removes it → Cancel
  discards an unsaved draft without touching the engine; both themes; zero console
  errors. Deliberately split into two PRs — see `code/specs/task-app-notes-ui-v1.md`
  for why — with the UI's own deferred scope (attachment picker, tags, rich text,
  search) tracked above.
- **Phase 7 — Calendar component.** `mosaic-pkg-calendar` — month grid + drag-to-move,
  see `code/specs/task-app-calendar-v1.md` for the full scope. The engine's
  `calendar(range, view)` projection needed zero new work (shipped in #8726); this PR was
  pure UI. Verified live: month grid renders correctly (42-cell Sunday-first, both themes),
  prev/next navigation, and dragging an event onto a new day calls `setConstraint` with a
  `mustStartOn` date — confirmed the project's own projected-finish date recomputed after
  the drop, proving it's a real CPM reschedule, not a UI-only move. Found and fixed one real
  bug before shipping: an empty day's `HostDropTarget` had zero intrinsic height (no events,
  no explicit sizing), leaving nothing for a pointer to land on — fixed with `flex-grow: 1`
  so it fills the cell's `min-height`. Week/day views, resize, time-blocking, and cell
  weekend/out-of-month tinting deferred — see the two Backlog items above.
- **Phase 5 — Sheet component, now fully editable.** `mosaic-pkg-sheet` shipped
  read-only first, then editing landed as a fast-follow once the emitter gap was
  fixed properly ([UI37](../../../specs/UI37-generic-payload-dispatch.md) +
  `mosaic-pkg-grid` 0.2.3): a payload-carrying target emit on a generic container
  (`Box`) now resolves its params from named props on the node, the same mechanism
  UI35's `drag-key` uses. `Grid`'s `onNavigate(row, col)` — declared since v0.1.0 —
  reaches a consumer for the first time. Verified live: click a cell → edit → Enter
  → commits through the real engine op → persists across reload → consistent with
  the list view. Found + fixed three real bugs total in packages nothing had driven
  through a real app before: `mosaic-pkg-grid` 0.2.2 (`Cell`'s `onClick` never fired
  at all), `mosaic-emit-react` (`HostInput` `onCommit` never carried its declared
  payload), and the `Box`-payload gap itself (0.2.3/UI37).
- **Phase 6 — Board (kanban) view.** [#9897](https://github.com/adhithyan15/coding-adventures/pull/9897),
  merged. Drag-and-drop columns wired to the UI35 kernel.
- Dark-mode "Add task" pressed-color regression (copy-pasted from light theme) — fixed alongside
  the board PR's CI failure.
- Stale XAML hover-count acceptance-test assertion (12/9 → 16/13 after the board added 4 new
  segmented-control hover surfaces) — fixed alongside the board PR's CI failure.
