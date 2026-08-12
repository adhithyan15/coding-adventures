# task-app backlog

Working backlog for the [super-app roadmap](../../../specs/task-app-super-app.md). Ordered
by priority — top item is next up. Re-prioritized whenever a new item is discovered mid-flight.
Each item, once picked up, follows: spec-sync → tests → implementation → CHANGELOG → README →
`/security-review` → PR → `/babysit-pr` → auto-merge.

## Next up (priority order)

The design-fidelity gap (see `CHANGELOG.md`'s "re-closed the design-fidelity gap"
entry, and Resolved below for icon/SVG assets, Board, richer Gantt, and Calendar) is
now closed except for one low-priority polish item — see the Backlog section below.
Picking this session's next-highest-priority item needs a fresh pass over
`code/specs/task-app-super-app.md`'s remaining phases rather than continuing to work
off this now-mostly-resolved list.

## Backlog (lower priority — Phase 10+, spec explicitly defers these)

- **Richer task-row critical/slack chips.** Split out from the design-fidelity gap
  (see Resolved below) — labels/priority chips, the dependency list, and the notes
  paragraph all shipped. Today the detail panel's scheduling prose already says "on
  the critical path" / states slack in prose, so a dedicated chip would be a
  value-only restyle — low priority.

- **Segmented-switch icons.** Split out from the icon/SVG-assets item (see
  Resolved below) — everything else in that item shipped. The six
  view-switcher buttons (List/Board/Sheet/Calendar/Notes/Timeline) each want
  a small line icon in the mock. Same construction technique as everything
  else in that item (small Box/Stack compositions, no new primitive) — not a
  capability gap, deferred because six icons need to read as one matched
  family at a glance, which benefits from iterating on the rendered set
  side-by-side rather than shipping six independent first guesses.
- **Gantt dependency arrows.** Split out from the richer-Gantt item (see
  Resolved below) — everything else in that item shipped. Curved FS
  connectors between two bars need genuine 2D line-drawing the UI29 kernel
  has no primitive for (no SVG-path/canvas-overlay component; `HostDraggable`/
  `HostDropTarget` are drag primitives, not a drawing surface).
  `task-app-ui-design.md` §4.6 itself anticipated this needing a dedicated
  `mosaic-pkg-gantt` package (never built) rather than the simpler inline
  view that actually shipped. Needs either a new kernel primitive (an
  SVG-overlay host component) or product guidance on visual treatment
  before it's picked up — see `code/specs/task-app-richer-gantt-v1.md`'s
  "What does NOT ship" section for the full reasoning.
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
- **Calendar weekend/out-of-month cell tinting.** Deferred believing mostyle couldn't vary
  one part's background per data value, only per branch, and a 4-way branch duplicating the
  whole drop-target + event-loop wasn't judged worth it for a colour difference. That's now
  outdated: UI36's `background` binding (extended for the icon-assets progress ring, reused
  for Board's column accent bar — see both CHANGELOGs) does exactly this, no branch
  duplication needed. Re-open when picked up — today's count badge already shipped (it only
  needed a small conditional child).
- **Notes: a real attachment picker, tags, rich text, search.** Deferred from the
  Phase 8 UI ship — see `code/specs/task-app-notes-ui-v1.md`. A minimal name-matching
  attach-to-task *text field* shipped (see Resolved below); this item is what's still
  missing: a real dropdown/autocomplete/search picker, not just the write path. Tags
  are generic and reusable in `mosaic-pkg-notes` but nothing drives them; `Note.body`
  is plain text, matching every other free-text field in the engine; no search box
  (mirrors Sheet's own v1 scope cut).
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

- **Board design-fidelity: 4th column, accent bars, count badges, critical
  border.** Closes the Board line of the design-fidelity gap. A real "In
  review" 4th column, driven by wiring in task-core's previously-dormant
  `Workflow`/`Status`/`Projections::kanban()` system (nothing ever created a
  `Workflow`, so `engine.kanban()` always errored — `ensure_default_workflow`
  seeds one and backfills task status; `set_status` now cascades `completed`
  across a workflow's `done_status` boundary, matching that field's own doc
  comment). A colored top accent bar + real card-count badge on each column
  header (UI36's `background` binding, same mechanism the progress ring
  uses). Overdue cards get a colored left border instead of the old
  `card-crit` text chip — a second static part (`board-card-crit`) rather
  than a conditional style, since `HostDraggable`'s dedicated emitter doesn't
  support `state-when-` conditional styling. Verified live in both themes:
  all 4 columns/colors/counts, a card dragged through all 4 columns and back
  with the completed flag correctly following status, the critical border
  rendering and swapping cleanly, zero console errors (including a real
  React dev-mode shorthand/longhand border-property warning caught and fixed
  during verification — both card variants now declare the same style keys).
- **Icon/SVG assets.** Closes most of the design-fidelity gap's icon line —
  see `code/specs/task-app-icon-assets-v1.md`. Pill status dot
  (`currentColor`), group-count badge (a new appended `taskRows` cell),
  composer "+" icon box (dashed border, two crossed bars), the theme toggle
  moved into the topbar as a real `HostButton` with a drawn crescent moon /
  filled sun (`HostButton` can't render children, so the shape is the
  button's own background/box-shadow, with the accessible label kept but
  visually hidden), a progress ring (needed a small, disclosed
  `mosaic-emit-react` change — UI36's bindable-property list gained
  `background`, its one continuously-data-driven property), and a brand mark
  (a bridge arc — two posts + a border-only arc — user-chosen from a
  proposed shortlist). Every shape is built from primitives that already
  exist (`Stack`'s absolute-positioned children, individual-corner
  `border-radius`, individual-side `border-*`) — no new SVG-embedding
  kernel primitive, no image files. Segmented-switch icons are the one
  piece that didn't ship — split into its own Backlog item above, since
  it's a design-consistency concern (six icons need to read as one matched
  family) rather than more of the same construction work. Verified live in
  both themes via DOM/computed-style inspection (this session's browser
  pane doesn't compose screenshot frames): the ring's `conic-gradient`
  recomputing correctly as task-done state changes, the group-count badge
  tracking group membership, the theme toggle swapping shapes with the
  correct accessible label; zero console errors.
- **Richer Gantt.** Closes most of the design-fidelity gap's Timeline
  line — see `code/specs/task-app-richer-gantt-v1.md`. A day-grid ruler
  (weekday/today shading — a strip above the bars, not composited behind
  them, since the kernel has no z-index/absolute-positioning primitive),
  a percent-complete fill inside each bar, milestones as small "inked"
  diamonds (no bound width, deliberately — UI36's size-precedence rule
  would otherwise make a fixed diamond shape unreachable), hover
  tooltips (needed a small, disclosed `mosaic-emit-react` change —
  `HostTooltip`'s `text` prop now also accepts a per-row expression, not
  just a literal or a slot), and a static legend. Dependency arrows are
  the one piece that didn't ship — split into its own Backlog item above,
  since it's a different kind of gap (missing kernel capability) than
  everything else here (layout/styling work). Verified live in both
  themes via DOM inspection (tooltip `title` text, computed colors on
  the grid/milestone/fill elements), not just visual assumption; zero
  console errors; confirmed no regression to Board-tier's Timeline-hiding
  from the complexity-config work.
- **Phase 9 — per-project complexity config (Board ↔ Full CPM).** Closes
  the gap the nested-project-tree entry below disclosed — Phase 9 is now
  fully shipped. See `code/specs/task-app-complexity-config-v1.md` for
  the decision addendum: project-level (not per-task, since task-level
  granularity already exists via `Task.schedule: Option<TaskSchedule>`),
  exactly two tiers (no middle ground — the phase's own title is a
  binary), new projects start Board, pre-field snapshots load as Full
  (zero regression). A topbar toggle flips the active project; Board
  hides Timeline, the schedule window, the CPM-derived task-detail
  lines, and the Sheet's Start/Finish columns — due dates, overdue
  status, and dependencies stay in both tiers (basic todo-app concepts,
  not CPM output). The engine keeps computing CPM unconditionally; this
  is a host-side display filter. Verified live in both themes: backward
  compat on real persisted data, the toggle's effect on every listed
  surface, new-project defaulting, and per-project independence when
  switching between two differently-tiered projects.
- **Rename off "Planner".** Renamed the app's on-screen brand to **Trestle**
  (2026-08-06, user picked from a proposed shortlist of Cadence / Waypoint /
  Keel / Trestle). "Planner" collided with Microsoft Planner and others; the
  name was never baked into any spec or package/directory path — only the
  `TaskApp.mll` `brand-name` `Text` node and the `design/ui-prototype.html`
  mock displayed it, so this was a small, contained value change, not a
  structural rename. Verified live in both themes after rebuilding the web
  bundle; zero console errors.
- **Notes attach-to-task + task-detail notes paragraph.** Closes the gap the
  dependency-list entry below disclosed. `mosaic-pkg-notes` 0.2.0 gained a
  minimal "Attach to task" text field (task NAME, resolved to
  `attachedTask` on Save, unrecognised name **rejects the whole save** —
  same discipline as the Sheet Labels column). `TaskApp`'s task-detail
  panel gained `detail-notes` (`row[13]`), reading the open task's
  attached note body. Found and fixed one real bug before shipping:
  `Note` is `#[serde(rename_all = "camelCase")]`, so the JSON field is
  `attachedTask` — the first draft used the wrong snake_case key in both
  the detail-panel filter and the editor's name-display lookup, silently
  matching nothing. Caught live-testing by reading the persisted
  IndexedDB record directly (the UI alone wouldn't have shown *why* it
  was empty). Verified live end-to-end, both themes: attach by typing a
  task name case-insensitively, detail panel shows the note body,
  reopening the note shows the resolved display name, an unrecognised
  name is rejected without corrupting the existing attachment (checked
  the persisted snapshot, not just the UI). A real picker (dropdown/
  autocomplete/search) is still deferred — see the Backlog item above.
- **Task-detail dependency list.** The open task's detail panel shows its CPM
  dependencies (`→ Build the prototype (FS)` / `← Design the wireframes (FS)`),
  read from `task-core`'s existing `flowchart()` projection — zero new engine
  work. Verified live in both themes, zero console errors.
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
  follows). The remaining Phase 9 item (complexity config) needed a product
  decision this extraction didn't — see the complexity-config entry above,
  now resolved too.
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
