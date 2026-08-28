# Changelog

All notable changes to the `task-app` web program are documented here.

## [0.1.0] - Unreleased

### Added - `elevation: raised;` on every raised-card part (#12028 item 1, UI41)

`TaskApp.{light,dark}.msl`'s 13 raised-card-style parts (Kanban cards,
task rows, selected tab/nav buttons, the composer/label-composer input
rows, the Gantt timeline panel) now each declare `elevation: raised;`
alongside their existing `box-shadow:`. Additive, not a replacement —
`box-shadow` is unchanged and still drives correct CSS shadow rendering
on web; `elevation` is the new channel native backends will read to
render their own native shadow primitive (mosstyle-compiler's
`elevation` property, #12028 item 1, UI41). The `theme-toggle-moon`
part's `inset` `box-shadow` (a decorative crescent-moon cutout, not
elevation) is untouched. No native backend reads `elevation` yet — see
`code/specs/UI41-elevation-tokens.md` for the rollout.

### Added - progress ring's percent-complete now flows to every host as typed data (#12028 item 2)

The workspace-progress ring's percent-complete was computed by the
shared `task-mosaic-app` engine (used by every native host) only to
build a `ring-gradient: ""` placeholder that never carried any real
value — the web host redundantly recomputed the same percent itself in
`main.tsx`, and only the web host's own recomputation ever reached the
rendered CSS `conic-gradient(...)`. Native hosts had no numeric
fallback to render *anything* from.

New `slot ring-percent-value : number ;` (`TaskApp.mil`) carries the
same 0..100 percent every host already needed, sourced once from
`task-mosaic-app`'s own computation instead of being redundantly
re-derived per host. `ring-gradient`/`ring-percent` are unchanged — web
still renders its own CSS-trick donut from them, appropriate for its
platform.

Native rendering of the ring from this number (a real circular
progress indicator per backend) is a deliberately separate, still-open
follow-up — see the tracking issue linked from
`code/specs/task-app-icon-assets-v1.md`'s "the one real gap" section.
This change only closes the data-contract leak.

### Added - Board design-fidelity: 4th column, accent bars, count badges, critical border

Closes the Board line of the design-fidelity gap backlog item.

- **A 4th "In review" column.** Board's 3 columns were a hardcoded
  `completed`/`percent_complete` heuristic that never touched
  `task.status` at all. task-core already had a full `Workflow`/
  `Status`/`Projections::kanban()` system, exported end-to-end through
  `task-wasm` — but nothing ever created a `Workflow`, so
  `engine.kanban(id)` always errored. Wired it in:
  `ProjectState::ensure_default_workflow` (task-core) seeds a 4-status
  workflow (Up next/In progress/In review/Done) and backfills any task
  that's never had a status set, and `set_status` now cascades
  `completed` when a status crosses a workflow's `done_status` boundary
  — `Workflow.done_status`'s own doc comment already promised this, the
  op just never implemented it. Board now reads real `task.status`;
  dropping a card is one `setStatus` call, no more manual
  `setCompleted`/`setPercentComplete` branching. See the task-core and
  task-wasm CHANGELOGs for the full engine-side story, including the
  disclosed one-directional-cascade limitation.
- **A colored top accent bar + card-count badge** on each column
  header — a real per-column count from `kanban()`, and an accent
  color per status resolved by the host (UI36's `background` binding,
  same mechanism as the progress ring) since the four colors are
  theme-dependent and the controller that builds `board-columns`
  doesn't know which theme is active.
- **Overdue cards get a colored left border, not a text chip** — the
  old `card-crit` text chip is gone. `HostDraggable`'s dedicated
  `mosaic-emit-react` emitter doesn't support `state-when-` conditional
  styling (confirmed by reading it directly), so this is a second
  static part (`board-card-crit`) rather than a conditional spread —
  same pattern as the icon-assets slice's `pill-warn`/`pill-ok` and
  `theme-toggle-sun`/`theme-toggle-moon`. Both card variants declare
  the same `border-left-*` style keys (only the values differ) —
  otherwise a card crossing the overdue boundary while dragged between
  columns triggers a real React dev-mode warning about mixing
  shorthand and longhand border properties, caught live in the browser
  during verification.

Verified live in both themes: all 4 columns with correct accent colors
and counts, a card dragged through all 4 columns and back (the
completed flag correctly follows status crossing the done boundary,
confirmed via the List view's done glyph and the progress ring), the
critical border rendering correctly and cleanly swapping as a card's
overdue-ness changes, zero console errors.

### Added - zero-degradation native-complete XAML TaskApp

Board cards and calendar items now use native WinUI drag/drop events with the
same accepted-drop lifecycle for pointer, touch, and keyboard operation.
Component-local target scoping, authored kind filtering, disabled states, RTL
keyboard order, focus semantics, and UI Automation announcements preserve UI35's
cross-backend contract. Together with the existing native Sheet semantics and
concrete Rust runtime, Windows CI now emits and builds the complete TaskApp under
the strict `native-complete` profile with zero degradations.

### Added - native XAML Sheet table semantics

The canonical dynamic Sheet now keeps its existing WinUI visuals and editable
cell subtree while generated component-scoped automation peers expose native
UIA Table/Grid and TableItem/GridItem patterns. Narrator and other automation
clients receive table dimensions, column-header associations, row/column
coordinates, accessible cell names, and arrow-key cell navigation. The concrete
TaskApp report now contains exactly the four remaining drag/drop degradations.

### Added - concrete Rust TaskApp runtime in the XAML WinUI artifact

The Windows acceptance lane now builds `task-mosaic-app`, supplies it while
emitting the complete TaskApp WinUI project, and verifies that the DLL copied
beside `TaskApp.exe` is byte-for-byte identical. A task-specific .NET console
fixture loads that app-local DLL through the generated standard binding, checks
initial TaskApp props, dispatches `newTaskNameChange`, and checks the revised prop
without `MOSAIC_APP_LIBRARY`.

At this historical milestone the generated XAML report still contained exactly
four inert drag/drop paths; the native drag milestone above subsequently closes
them. Visible WinUI launch remains an interactive Windows-worker gate because hosted
GitHub workers cannot reliably initialize a desktop surface.

### Added - concrete Rust TaskApp runtime on SwiftUI for macOS

SwiftUI's strict acceptance lane now keeps ABI conformance on its dedicated
counter runtime, then separately bundles `task-mosaic-app` into the generated
TaskApp resource bundle. CI requires zero degradations, checks the bundled dylib
byte-for-byte, and launches the generated macOS executable from outside its build
directory without `MOSAIC_APP_LIBRARY`, rejecting runtime, required-prop, and
Swift fatal-error output. A separate permissive build continues to compile the
same generated UI for iOS 16 without treating the macOS dylib as an iOS artifact.

### Added - concrete Rust TaskApp runtime on Compose Desktop

Compose Desktop's strict acceptance lane now keeps ABI conformance on its
dedicated counter runtime, then separately bundles `task-mosaic-app` into the
generated TaskApp distributable. CI checks the installed runtime byte-for-byte
and launches the packaged Linux application under a virtual display without
`MOSAIC_APP_LIBRARY`, rejecting runtime, required-prop, JVM exception, and error
output.

### Added - concrete Rust TaskApp runtime on Flutter

Flutter's strict acceptance lane now bundles `task-mosaic-app` rather than the
counter conformance fixture, verifies that the Linux bundle contains the exact
library built from this checkout, and launches the generated desktop app under a
virtual display without `MOSAIC_APP_LIBRARY`. The counter runtime and harness stay
in a separate build, preserving an independent proof of the standard ABI while the
TaskApp launch proves that the complete MIL prop surface reaches a real native host.

### Added - concrete Rust application adapter and strict Qt app

TaskApp now has a concrete `task-mosaic-app` native runtime instead of borrowing
the unrelated counter conformance engine. The adapter owns portable UI state,
delegates domain behavior to `task-core`, covers the complete MIL slot/event
surface, preserves transactional errors, snapshots/restores state, and exports the
standard Mosaic C ABI.

The Qt acceptance lane builds and bundles that exact library, requires a
zero-degradation `native-complete` report, compiles and installs the generated app,
verifies the installed runtime byte-for-byte, and launches the installed TaskApp
without `MOSAIC_APP_LIBRARY`. Exercising its non-empty project list found a Qt
emitter defect that sample-fallback launches had hidden: Bound Repeater delegates
could not see implicit `modelData`/`index`. Generated `For` delegates now declare
those inputs explicitly, eliminating the startup `ReferenceError`/`TypeError`s.

### Added - first zero-degradation native-complete backend

The package-expanded TaskApp now passes Flutter's `native-complete` profile
with no known degradations. Its spreadsheet view is emitted with native
`DataTable` semantics, its board and calendar retain native accessible drag and
drop, the generated shell requires the standard Rust application runtime, and
CI analyzes and builds the complete output as a native Flutter desktop app.

### Added - icon/SVG assets: pill dot, progress ring, real theme-toggle icon, group-count badge, composer plus, brand mark

Closes most of the "Icon/SVG assets" line of the design-fidelity gap
backlog item — see `code/specs/task-app-icon-assets-v1.md` for the full
scope decision, including what's deliberately NOT here (the six
segmented-switch icons — not a capability gap, deferred so they can be
iterated on together as one matched family rather than shipped as six
independent first guesses).

- **Pill status dot**: a small `currentColor` circle before "On track" /
  "N overdue" in the topbar status pill.
- **Group-count badge**: a count pill next to each List-view group
  heading ("IN PROGRESS 2"), from a new appended `taskRows` cell
  (`row[14]`, present only alongside the heading cell).
- **Composer "+" icon box**: a dashed-border box with a plus mark before
  the task-name input.
- **Theme toggle, moved into the topbar with a real icon**: was a
  `position: fixed` unicode-glyph button living entirely outside the
  Mosaic-compiled component, invisible to mostyle. Now a real
  `HostButton` in the topbar, with a drawn crescent moon / filled sun
  instead of "☾"/"☀". `HostButton` can't render children
  (`host_button_label_body` in `mosaic-emit-react` reads only the flat
  `label` prop), so the shape comes entirely from the button's own
  background/box-shadow styling; the accessible label text stays real
  (screen readers) but is visually hidden via `color: transparent`.
- **Progress ring**: the active project's percent-complete as a
  circular ring in the topbar, next to the view switcher. The one shape
  here that needed real data — see the `mosaic-emit-react` CHANGELOG
  entry for UI36's `background` binding, which this ring is the reason
  for.
- **Brand mark**: a bridge arc (two upright posts joined by a border-only
  arc), replacing the previously-empty honey square. User-chosen from a
  proposed shortlist (Truss triangle / Three pillars / Bridge arc /
  A-frame chevron).

Every shape is built from primitives that already exist — no new
SVG-embedding kernel primitive, no image files. `Stack`'s
`position: relative` + absolutely-positioned children draws the
donut ring, the crescent moon, and the bridge arc; individual-corner
`border-*-radius` and individual-side `border-*` properties (already
proven safe for the Gantt day-grid) draw the arc and the dashed
composer box.

`main.tsx`: the fixed-position theme button is gone; `toggleTheme` is a
real dispatched event, intercepted in `Root` before reaching
`controller.apply` (theme is page-level React state, not part of the
engine-backed controller). `theme.ts` gained `ringGradient(theme,
percent)` — the actual `conic-gradient(...)` string needs the resolved
theme, which the (theme-agnostic, shared-by-both-themes) controller
never sees, so it's computed in `Root` and overrides the controller's
placeholder the same way `themeIsDark` does. Group sizes for the count
badge are tallied once per `getProps()` call, ahead of the row-building
loop that already walks the same list.

Verified live in a real browser session (both themes, zero console
errors, via DOM/computed-style inspection since this session's browser
pane doesn't compose screenshot frames): the ring's `conic-gradient`
recomputing correctly as a task's done state flips (0% → 100%, correct
per-theme fill/track colours), the group-count badge updating with the
group ("IN PROGRESS 1" → "DONE 1"), the theme toggle swapping between
moon and sun with the correct crescent `box-shadow` and the correct
accessible label text, and the page ground repainting on toggle.

### Added - richer Gantt: day-grid, milestones, percent-fill, tooltips, legend

Closes the "Richer Gantt" line of the design-fidelity gap backlog item —
see `code/specs/task-app-richer-gantt-v1.md` for the full scope decision,
including what's deliberately NOT here (dependency arrows — the UI29
kernel has no primitive for 2D line-drawing between two bars; left as its
own backlog item needing either a new kernel primitive or product
guidance, not forced in with a fragile workaround).

- **A day-grid ruler**: one column per calendar day in the visible span,
  weekends shaded, today's column in a stronger honey tint. Renders as a
  strip above the bars, not composited behind them — the kernel has no
  z-index/absolute-positioning primitive to do that, and the spec's own
  "Day-grid feasibility note" explains why the *shading* itself was
  cheap here even though the identical-looking limitation blocked
  Calendar's own weekend-tinting item (Gantt's grid cells carry no
  drag-target/event-overlay weight to duplicate).
- **Percent-complete fill**: a darker overlay inside each bar, sized
  from `Task.percent_complete` — data `GanttBar` already returned but
  the host never drew.
- **Milestones as diamonds**: a zero-duration `TaskKind::Milestone` task
  renders as a small "inked" diamond (the same dark/light-ink convention
  Calendar's own milestones already use) instead of the usual bar.
  Deliberately no bound width on it — UI36's "a bound size always beats
  a static one" rule would otherwise make a fixed small-diamond style
  unreachable.
- **Hover tooltips**: name, dated window, real day count, critical/
  percent status. Needed a small, disclosed kernel change alongside this
  — see the `mosaic-emit-react` CHANGELOG entry for `HostTooltip`
  accepting a per-row expression, found while wiring this in (the
  literal/slot-only `text` prop had no way to differ per `For`-loop row).
- **A legend**: static swatches for Normal / Critical / Milestone /
  Today, above the chart.

Verified live in a real browser session (both themes, zero console
errors): the day-grid's weekday/today shading, a milestone rendering as
a diamond with no percent-fill overlay (distinct from an ordinary
critical bar with the same name, confirming no rendering collision), a
60%-complete bar's fill overlay, and every tooltip's exact text content
— all checked via DOM inspection (`title` attributes, computed styles),
not just visual assumption. Confirmed no regression to the existing
Board-tier Timeline-hiding behavior from the complexity-config work.

### Added - per-project complexity config (Board ↔ Full CPM)

Phase 9's remaining half — see `code/specs/task-app-complexity-config-v1.md`
for the full decision addendum the backlog required before touching code.

- A topbar toggle next to the view switcher flips the ACTIVE project
  between two tiers: **Board** (no Timeline, no schedule window, no
  CPM-derived task-detail lines, no Sheet Start/Finish columns) and
  **Full CPM** (everything). Due dates, overdue status, and dependencies
  are unaffected in either tier — they're basic todo-app concepts, not
  CPM output (see the spec's Decision 4 for the full classification).
- New projects (the workspace's own initial one, a top-level create, or
  a nested subproject) all start **Board** — `ProjectState::empty()` is
  the single constructor every one of them routes through. This is a
  real, disclosed behavior change for anyone creating a fresh project;
  existing persisted projects are unaffected (see the next point).
- A project saved before this field existed loads as **Full** — the
  field's own `#[serde(default)]` resolves independently of `empty()`'s
  new-project default, so old data shows no behavior change on load.
- The engine keeps computing CPM unconditionally in both tiers — this is
  a display-time filter the host applies, not a computation toggle
  (`ProjectState::set_project_complexity` / task-wasm's
  `set_project_complexity` / `main.tsx`'s `activeProjectComplexity()` +
  `visibleSheetFields()`).
- Verified live in both themes: an existing persisted project (real
  IndexedDB data, not a fixture) loaded as Full CPM; toggling correctly
  shows/hides every listed surface; a newly created project started
  Board; two projects with different tiers kept their settings
  independent when switching between them; the dark-theme toggle
  renders with the correct colors.

### Changed - renamed the app's on-screen brand from "Planner" to "Trestle"

"Planner" collided with an existing trademark (Microsoft Planner et al.).
Presented the user a short list of trademark-safe alternatives (Cadence,
Waypoint, Keel, Trestle) rather than locking one in unilaterally, per
`BACKLOG.md`'s "Rename off Planner" item; the user picked **Trestle**.

The name was never baked into any spec, package name, or directory path —
only two files displayed it as a literal string: `TaskApp.mll`'s
`brand-name` `Text` node (the single source of truth the React components
are compiled from) and `design/ui-prototype.html`'s historical design
mock (`<title>` and the rail's `.name` div). Rebuilt the web bundle
(`scripts/build-web.sh`) to regenerate `host/web/src/TaskApp.{light,dark}.tsx`
from the updated source — those two files are gitignored build output, not
committed, so no other file needed touching. Verified live in both themes
after rebuild: brand text reads "Trestle", zero console errors.

### Added - minimal notes attachment + notes paragraph in the task-detail panel

Closes the gap the dependency-list entry below disclosed: `Note.attached_task`
existed since Phase 8, but no UI anywhere could ever set it, so a
task-detail "notes paragraph" cell was drafted and pulled back out as
dead plumbing. `mosaic-pkg-notes` 0.2.0 adds a single-line "Attach to
task" field to the Notes editor — a task NAME, not id, resolved to
`attachedTask` on Save. An unrecognised name **rejects the whole save**,
the same discipline the Sheet Labels column already uses (verified
live: typing a nonexistent task name and hitting Save logs a console
error and leaves the note's real attachment untouched — checked the
persisted IndexedDB record directly, not just the UI). The task-detail
panel gains `detail-notes` (`row[13]`, appended after the dependency
list's `row[12]`), reading the attached note's body for the one open
task.

Found and fixed one real bug before shipping: `Note` is
`#[serde(rename_all = "camelCase")]` in `task-core`, so the JSON field
is `attachedTask`, not `attached_task` — the first draft of both the
detail-panel filter and the editor's "show the currently attached task
name" lookup used the wrong (snake_case) key and silently matched
nothing. Caught live-testing (the notes paragraph rendered empty
despite a real attachment existing) by reading the persisted
IndexedDB snapshot directly, not by inspection.

Verified live end-to-end, both themes, zero unexpected console errors:
created a task, created a note, attached it by typing the task's name
in lowercase (case-insensitive match), confirmed the detail panel's
notes paragraph renders the body text; reopened the note and confirmed
the attach field shows the resolved display name (not the raw id);
typed an unrecognised name and confirmed Save is rejected with a
console error while the note's real attachment is left untouched in
the persisted snapshot.

See `code/specs/task-app-notes-ui-v1.md`'s addendum for the full scope
decision (why a name-matching text field, not a picker).

### Added - dependency list in the task-detail panel

The open task's detail panel now shows its CPM dependencies alongside the
existing scheduled/slack prose: `→ Build the prototype (FS)` for a
predecessor edge, `← Design the wireframes (FS)` for a successor edge,
each labelled FS/SS/FF/SF. Pure UI — `task-core`'s `flowchart()`
projection (predecessor/successor task ids + kind label, already exported
through `task-wasm`/`task-engine.mjs`) needed zero new engine work; only
`scheduling` edges are shown (real CPM dependencies), not generic links.
Follows the same "appended, not inserted" (`row[12]`) and
progressive-disclosure (computed only for the one open row) conventions
`row[6]`-`row[8]`/`row[10]`-`row[11]` already established.

Verified live: added two tasks (every new task auto-links as a
dependency successor of the previous one, so no dedicated dependency-UI
was needed to exercise this), expanded each in turn, confirmed both edge
directions and the FS label render correctly in both themes. Zero
console errors.

**Scope note**: a matching "notes paragraph" (attached `Note` entities'
body text) was drafted alongside this but pulled back out before
shipping — there is no UI anywhere yet to actually attach a note to a
task (Notes v1 deliberately deferred that as its own "attachment picker"
item), so the cell would only ever render empty. See `BACKLOG.md`: it
now needs to ship together with that picker, not before it.

### Changed - project rail extracted to mosaic-pkg-project-nav

Phase 9's first half: the nested-project tree + add/add-subproject
composer moved out of `TaskApp.mil`/`.mll`/`.msl` into a standalone
`mosaic-pkg-project-nav` package, per the roadmap's reuse map. A refactor,
not a redesign — same part names, same styling, same layout, in both
themes. The brand row and the view-switcher deliberately stayed in
TaskApp; see `code/specs/task-app-project-nav-v1.md` for why.

Verified live, behavior-identical to before: create a project, create a
nested sub-project (indent glyph renders), switch selection between
projects (the "on" raised-card styling follows). Zero console errors.

**Still open** (see `BACKLOG.md`): the per-project/task complexity config
(board-only ↔ full CPM) that the spec calls "the single most important
product rule" (§2.3) — not addressed here; it needs a product decision
the spec doesn't make (what exactly "board only" hides is undefined),
unlike this extraction, which needed none.

### Added - label management (create + assign)

Closes the gap the previous entry disclosed: labels can now actually be
created and assigned, not just displayed.

- **Creation**: a small composer ("+ Label") wrapping the Sheet tab in
  `TaskApp.mll` — deliberately TaskApp's own concern, not a
  `mosaic-pkg-sheet` slot, since Sheet is a generic grid+toolbar wrapper
  with no business knowing about labels specifically. Calls the engine's
  existing `upsertLabel({ id, name, color: "" })`; no colour picker in v1
  (a fixed empty string — nothing reads the field yet).
- **Assignment**: a new "Labels" column on the Sheet, editable like every
  other column. Accepts comma-separated *existing* label names, matched
  case-insensitively (`"urgent"` resolves to a label named "Urgent").
  An unrecognised name **rejects the whole edit** rather than creating a
  throwaway label or silently dropping it — the same "reject an
  unrecognised value rather than sending it through" discipline the
  Priority column already uses.

Verified live end-to-end: created a label named "Urgent", assigned it to
a task by typing "urgent" (lowercase) into its Sheet Labels cell — matched
case-insensitively — confirmed the chip renders on the List tab; then
confirmed typing an unknown name leaves the existing assignment untouched
rather than corrupting it. Both themes. Zero console errors.

**Still deferred**: colour picker (the `Label.color` field is set but
inert — nothing renders it yet), duplicate-name prevention (creating two
labels with the same name is allowed, mirroring how project names aren't
deduped either), and a way to *remove* a single label from a task without
retyping the whole comma-separated list.

### Added - priority and labels shown on task rows

The list view now shows a task's priority (e.g. "High") and labels (comma-
joined names) as chips, matching the existing due/schedule/overdue chip
treatment. `task-core` already had both as first-class, first-class view
dimensions (shipped earlier) — this is pure display wiring, no new engine
work: `TASK_VIEW`'s `visibleFields` gained `{ builtin: "priority" }` and
`{ builtin: "labels" }`, and `taskRows()` appends them as two new trailing
cells (`row[10]`/`row[11]`) rather than inserting between existing indices,
so no existing `row[n]` reference anywhere shifts meaning.

Verified live: set a task's priority to "High" via the Sheet tab's already-
editable Priority column (the existing, working way to set it today), then
confirmed the chip renders correctly on the List tab in both themes.

**Known gap, disclosed rather than hidden**: there is still no UI to
*assign* a label to a task (`upsertLabel`/`setTaskLabels` exist on the
engine but nothing in `main.tsx` calls `setTaskLabels`, and the Sheet's
column catalogue has no Labels column) — so the labels chip, while fully
wired and using the identical proven `If (when: (row[11]))` mechanism the
priority chip does, has no way to actually populate today. That's real
feature work (label management: create, colour-pick, assign), not a
display fix, and is tracked in `BACKLOG.md`.

### Added - notes (list + editor) view

Phase 8's UI half: a new `mosaic-pkg-notes` package (adapted from
`mosaic-pkg-note-editor`), wired into `TaskApp` as a sixth view
(List/Board/Sheet/Calendar/Notes/Timeline). The engine half — `task-core`'s
`Note` entity and `upsertNote`/`deleteNote` — shipped separately; see
`code/specs/task-app-notes-ui-v1.md` for why the two were split.

- A list of every note (title, standalone or task-attached — v1 shows
  them in one flat list, no separate browse split) with "+ New note", and
  an editor for whichever one is open (title, multiline body, Save/
  Delete/Cancel).
- Save always calls `upsertNote` — a new note's id is minted the moment
  "+ New note" is clicked (the same host-mints-ids-upfront pattern
  `addTask`/`addProject` already use), not deferred to first Save, so the
  editor stays open the whole time regardless of whether the note has
  been persisted yet.
- Deferred: an attachment picker (v1's notes are always standalone),
  tags, rich text, search — see `BACKLOG.md`.

### Added - calendar (month grid) view

Phase 7 of the roadmap: a new `mosaic-pkg-calendar` package, wired to
task-core's `calendar(range, view)` projection (no new engine work needed —
that projection shipped in #8726) plus the UI35 drag kernel for drag-to-move
rescheduling. See `code/specs/task-app-calendar-v1.md` for the full scope.

- Month grid: 42 cells (6×7, Sunday-first), today gets a filled badge,
  multi-day events render on every day they span (not just their start
  day — the engine already computes the real span).
- Drag an event onto a different day and it reschedules for real: the host
  calls `engine.setConstraint({ id, constraint: { mustStartOn } })`, which
  feeds the CPM pass — not `setDeadline`, which the calendar's own display
  precedence (computed schedule beats the deadline fallback) would have
  silently ignored for any already-scheduled task. Verified live: the
  project's own projected-finish date recomputed after a drop.
- Critical/completed/overdue events get a conditional text chip, the same
  "one draggable part, conditional child chips for state" trade-off Board's
  `card-crit` already established (not a per-state container restyle).
- Both themes, palette taken directly from `design/ui-prototype.html`'s
  `.cal-*` classes.

Deferred to a follow-up (tracked in `BACKLOG.md`): week/day views, resize
(the UI35 kernel doesn't support it), time-blocking (needs a time-of-day
field on `TaskSchedule` that doesn't exist yet — an engine gap), and
weekend/out-of-month cell tinting (would need a 4-way branch duplicating
the whole drop-target + event-loop for a colour difference — not judged
worth it, matching Board's own deferred critical-border-vs-chip gap).

Found and fixed one real bug before shipping, not after: an empty day's
`HostDropTarget` had zero intrinsic height (no events inside it, no
explicit sizing), leaving nothing for a pointer to actually land on —
fixed with `flex-grow: 1` so it fills the cell's 96px `min-height`.

### Changed - re-closed the design-fidelity gap (partial pass)

A prior pass (`### Changed - the app now looks like the design`, below) brought the app
up to `design/ui-prototype.html` once; it had drifted again. This pass closes the clear,
unambiguous value mismatches — colors that didn't match a design token, spacing/padding
that had collapsed to uniform values where the mock uses asymmetric ones, shadows that
had drifted to ad hoc single-layer values instead of the shared two-layer `--shadow`
token:

- **Active-project highlight** no longer fills solid honey — it's a quiet raised card
  (surface background + the shared shadow token), the same "on" treatment the segmented
  switch already uses. This was the single most visible color mismatch in the app.
- **Rail, topbar, and content padding** now match the mock's asymmetric values (e.g.
  rail `20px 14px` not a uniform `20px`; topbar `20px 30px 14px`; content
  `8px 30px 60px`) — mosstyle's per-property `padding-top`/`-right`/`-bottom`/`-left`
  already supported this (confirmed via the react emitter's generic kebab→camelCase CSS
  property translation), it just hadn't been used.
- **Segmented-switch "on" buttons, pills, Add-task button** all gained their mock's
  asymmetric padding and/or the shared shadow token instead of ad hoc single-layer
  shadows with drifted alpha values.
- **Task-row and task-detail padding** now match, including the mock's 47px left indent
  on the detail panel (lines it up under the task name, past the checkbox).
- **Checkbox border** and **project-off hover background** now use the mock's actual
  design tokens (`--ink-faint`, `--line-soft`) instead of invented nearby colors.
- **Font stack** gained the mock's `Helvetica Neue, Arial` fallbacks.

Verified live in both themes: zero console errors, computed styles match the mock's
values exactly (spot-checked via `getComputedStyle`).

**Deliberately NOT done in this pass** — either because they're real feature work
disguised as "drift" (the mock predates several now-shipped views) or because they need
new markup/icon assets rather than a value fix, tracked in `BACKLOG.md`:

- Calendar view (the mock has one; it's the existing Phase 7 roadmap item, not drift).
- A richer Gantt (day-grid, weekend/today shading, milestones, dependency arrows,
  legend) — the mock's timeline is considerably richer than the simple proportional-bar
  view currently shipped.
- Rich task-row data (critical/slack chips, labels, priority, dependency list, notes) —
  `task-core` already has labels/priority as first-class fields; wiring them into the
  row/detail layout is real feature work, not a style-file edit.
- Everything needing a new icon/SVG asset: the brand mark's decorative glyph, segmented-
  switch icons, the progress ring, a stroked moon icon (the theme toggle currently uses
  a floating unicode-glyph button positioned outside the topbar's flow, not the mock's
  inline icon button), the pill status dot, the group-count badge, the composer's "+"
  icon box, and the "In review" board column (board is 3 columns; mock has 4, with a
  colored top accent bar and card-count badge neither of which exist yet).
- Board's critical-card treatment (colored left border in the mock vs. a text chip live).

### Added - sheet cell editing

The sheet view's cells are editable now. The fast-follow the read-only ship below
promised: [UI37](../../../specs/UI37-generic-payload-dispatch.md) fixed the
underlying gap at the `mosaic-emit-react` level (a payload-carrying target emit on
a generic container now resolves its params from named props on the node, the same
mechanism UI35's `drag-key` already uses), and `mosaic-pkg-grid` 0.2.3 threads
`row`/`col` through `Cell` to use it. `Grid`'s `onNavigate(row, col)` contract —
declared since v0.1.0 of that package — reaches a consumer for the first time.

Click an editable cell (Done, Name, Deadline, % Complete, Priority, Status, Notes)
to edit it in place; Overdue/Start/Finish stay read-only (they're computed). Enter
commits through the matching engine op (`renameTask`, `setDeadline`,
`setPercentComplete`, `setPriority`, `setStatus`, `setCompleted`, `setNotes`);
Escape cancels. Verified live: edit → commit → persists across reload → consistent
with the list view, zero console errors.

### Added - sheet (spreadsheet) view — read-only

- **A fourth view: a sheet**, a filterable/sortable spreadsheet over 10 columns (Done,
  Name, Deadline, % Complete, Priority, Status, Notes, Overdue, Start, Finish), wired to
  the engine's `table(view)` projection — the same query the list view already uses,
  with a broader `visibleFields` and its own filter/sort toolbar state. Built on the new
  `mosaic-pkg-sheet` package (`mosaic-pkg-grid` + `mosaic-pkg-toolkit`'s `Select`).
- **v1 is READ-ONLY.** Cell editing is declared in the interface but not functional:
  `mosaic-pkg-grid`'s `Cell` is a `Box` (a generic container), and
  `mosaic-emit-react`'s connects-wiring can't synthesize an index/value payload for a
  generic container the way it can for `HostButton`/`HostInput` — so a click can't
  actually identify *which* cell was clicked, in any app built on Grid, not just this
  one. Clicking a cell is a documented no-op rather than something that looks like it
  works and silently doesn't. Tracked in `BACKLOG.md`; see `mosaic-pkg-sheet`'s and
  `mosaic-pkg-grid`'s own CHANGELOGs for the full explanation and the two ways to fix it
  properly at the emitter level.

Two bugs found building this, in packages nothing had driven through a real app before:

- **`mosaic-pkg-grid` v0.2.0/0.2.1: a click on any cell did nothing.** `Cell.mil`
  declares `emit onClick` and every call site wires a handler for it, but `Cell.mll`'s
  own `Box[cell]` never referenced `emit: onClick` — nothing for the resolver to
  substitute into. Fixed in `mosaic-pkg-grid` 0.2.2.
- **`mosaic-emit-react`: `HostInput`'s `onCommit` dispatched no payload, ever**, even
  when the target emit declared one — so a component using `Grid`'s `onEditCommit`
  failed to typecheck. Fixed: the Enter-key dispatch now includes `value` when (and
  only when) the target emit's params are non-empty.
- Also added `task-wasm`'s `set_notes` export (`ops.rs::set_notes` existed; wasn't
  wired to the ABI) — needed for the Notes column once editing lands.

### Added - board (kanban) view

- **A third view: a board**, with Up next / In progress / Done columns. Cards drag
  between them by **pointer or keyboard** — this is the first real use of the UI35 drag
  kernel, and building it is what proved the kernel end-to-end in a running app.
- A drop is a **proposal**: the card doesn't move itself. The host translates "landed in
  this column" into the engine operation that expresses it, and the engine decides.
- The columns come from the same classification the grouped list uses, so the two views
  can never disagree about a task's state.

Three bugs found while getting this working, each of which made the board look finished
while being broken:

- **"Up next" was permanently empty.** The column was derived from *"is it scheduled"*,
  but the engine schedules every task that has a duration — so everything landed in "In
  progress" and one column, plus its drop zone, was dead. It now reads **progress**,
  which is both what a person means by "I've started this" and something that can
  actually be set, which is what makes dragging between those columns work at all.
- **`percentComplete` came back `undefined` for every task**: it's on `todos()`, not
  `checklist()`. Reading the wrong projection scored every task 0. The map is now built
  once per render from the right projection instead of issuing an engine query per card.
- **`setPercentComplete` takes `percent`, not `percentComplete`** — the latter is what
  the projections *return*. The call was failing the parse and coming back as an error
  envelope that nothing read, so cards silently sprang back. Every board operation now
  checks its result and logs a failure rather than assuming success.

Also from security review: a drop's `targetKey` is validated against the known columns
(an unrecognised one previously fell through and silently un-completed the task), and
`HostDropTarget` is given a flex direction so its `gap` isn't inert — it lowers to a
bare `<div>`, unlike `Column`, so cards were stacking flush.

### Added - working backlog

`BACKLOG.md` tracks the remaining super-app roadmap phases (sheet, calendar, notes,
app-shell assembly) in priority order, plus lower-priority items the spec explicitly
defers (native drag support, recurring/reminders, automation, resource leveling,
portfolio dashboards). Kept current as phases ship and new gaps are discovered.

### Added - native pressed feedback through Mosaic

The add-task button now declares its pressed background in the shared MSL
themes. SwiftUI and WinUI project generation consume that same authored state,
so the proving app gets platform-native press feedback without parallel AppKit
or Win32 chrome.

### Changed - the app now looks like the design

A polish pass bringing the real app up to `design/ui-prototype.html`, which it had
drifted well behind.

- **Two-column shell.** A quiet left rail holds the workspace's projects (active one
  honey-filled, nested ones indented) with the new-project composer at its foot; the
  main column holds a topbar and the content.
- **Topbar** — title, the summary line, and the engine's verdict as a **status pill**
  (sage "On track", red "N overdue"), with a proper **segmented view switch**. The
  switch uses two explicit events rather than one toggle, so clicking the view you are
  already on is a no-op instead of switching away from it.
- **The list reads as sections** — In progress / Up next / Done, with the heading
  riding on the row that opens each group.
- **Card treatment throughout** — the composer and every task are warm-white cards with
  the design's radii and warm-tinted shadows; chips, the round completion toggle, and
  the disclosure panel all match the prototype.
- Both themes stay in lockstep: 57 parts with identical name *and* property sets, since
  the dark theme is generated from the light one by token substitution.
- `index.html` gained the host's only CSS — a margin reset so the shell reaches the
  viewport edge (it had been floating 8px off every side), plus a pre-hydration
  background so the first paint isn't white. The *real* ground is set by the app from
  the resolved theme, because an explicit theme choice outranks the OS and the static
  guess can therefore be wrong.

Depends on two emitter fixes in the same change — see `mosaic-emit-react`'s changelog.
Without them nothing in this layout would have been vertically centred, and the rail's
"Planner" and "Projects" labels would have rendered as empty spans.


### Added - progressive disclosure on task rows

- **Click a task's name to reveal its scheduling detail** — when it's scheduled for,
  its earliest and latest possible start, and how much slack it has (or that it's on
  the critical path, where any delay delays the whole project). Clicking again closes
  it. This is the "simple by default, complexity on request" principle from
  `code/specs/task-app-ui-design.md`: the collapsed list stays a plain to-do list, and
  the CPM detail is there the moment you ask for it.
- Every detail line is phrased from the ENGINE's own numbers — early/late dates, total
  and free slack, criticality — never recomputed in the host.
- The open row is tracked by **task id, not row index**, so it follows the right task
  when the list re-sorts or something above it is deleted; deleting the open task
  clears it.
- **The CPM recompute is skipped entirely when nothing is open.** `getProps` runs after
  every dispatch — including each keystroke in the composer — so an unconditional
  `schedule()` call would have made typing progressively more expensive as a project
  grew. (Caught in security review.)
- Also from review: pluralisation now reads the number actually shown (479 minutes
  renders as "1.0" and must not then say "1.0 days" by accident); each detail line is
  guarded on its own cell so an unscheduled task shows one explanatory line instead of
  a panel padded with blanks; and a documented-but-never-populated "notes" cell was
  removed along with its dead style part rather than left as a promise the code
  didn't keep.


### Added - timeline (Gantt) view

- **A real proportional Gantt**, switchable from the task list via a toggle beside the
  summary. Each row shows the task name, a bar positioned and sized on one shared date
  scale, and its date window; bars on the **critical path** are red, the rest honey.
  Both themes.
- This is the first consumer of **UI36 data-driven sizing**
  (`code/specs/UI36-data-driven-sizing.md`). The bar's offset and length are CSS
  percentages the host computes from the engine's own gantt output and binds with
  `width: ( t[n] )` — previously impossible, since mosstyle bakes static values and a
  slot-bound width was silently dropped. The chart is genuinely to-scale, not an
  approximation.
- Geometry lives in `src/timeline.ts` as pure functions, tested without a wasm engine or
  a DOM (12 tests; 28 total, coverage 97%).

  The rule those tests exist to protect: **task-core dates are day-granular with an
  inclusive finish**, so a task occupying one day reports `finish === start`. An earlier
  draft used the bare difference, which made every one-day task zero-width — each fell
  through to the milestone floor and the whole chart rendered as a row of slivers. A
  length is `finish - start + 1`. (Caught in security review.)
- Degrades rather than throwing on malformed engine output: bars with non-finite dates
  are skipped, an inverted bar can't produce a negative width, a date outside
  JavaScript's range renders as `—` instead of throwing from `toISOString`, and the
  min/max use `reduce` rather than spreading a large array into an argument list.
- A zero-duration milestone is floored at a sliver so it stays visible.

## [0.1.0] - Unreleased

### Added - light/dark theme switching in the web host

- **The app follows your OS theme, and you can override it.** A toggle sits top-right;
  the rule is *explicit choice > OS `prefers-color-scheme` > light*. Only an explicit
  choice is persisted, so someone who never touches the toggle keeps following their OS —
  including when it flips at sunset, which is handled live via a `matchMedia` listener
  rather than needing a reload.
- **Why a component swap rather than a CSS class:** mosstyle bakes colours into each
  emitted component's *inline* styles, so there is no variable or class to flip at
  runtime. `scripts/build-web.{sh,ps1}` now emit **both** themes
  (`TaskApp.light.tsx` / `TaskApp.dark.tsx` via `mosaic-compile --theme`), and the host
  renders whichever one the rule selects. The two share an identical props/event type,
  so they're interchangeable.
- New `src/theme.ts` isolates the selection rules from React. Notable edge cases it
  handles, each with a test: a browser reporting `false` for *both* media queries (it
  doesn't understand the feature) is treated as **no opinion** rather than a vote for
  light; a missing `matchMedia` degrades instead of throwing; a corrupted stored value
  is ignored; storage failures (private mode) fall through to the OS preference; and
  the OS listener stops steering once the user has chosen. 7 new tests (13 total).
- The theme toggle is styled by hand because it belongs to the *host*, not to the
  emitted component — mosstyle never sees it.
- **Focus and caret survive the swap.** Changing theme swaps the component *type*, so
  React unmounts the old tree and destroys the focused `<input>`. Harmless when you click
  the toggle (that already blurred the field), but the OS can flip at sunset while you
  are mid-sentence — so the host remembers which field had focus and where the caret sat
  and restores both in a layout effect, before paint. (Found in security review.)
- Also from review: `matchMedia` is wrapped in `try`/`catch`, not merely checked for
  existence — privacy-hardened builds can *throw* on a fingerprintable query, and since
  this feeds `useState`'s lazy initializer an escape would blank the whole app.
- `vitest.config.ts` now includes `src/theme.ts` in the coverage gate. It had tests but
  was absent from `coverage.include`, so the 90% threshold silently didn't measure it —
  adding it immediately failed at 84% and surfaced three untested paths (a throwing
  storage backend, a throwing `matchMedia`, and Safari's legacy `addListener`). 16 tests
  total; coverage 95.65%.

### Added - multiple projects in the UI

- **You can now create projects and switch between them.** The engine has supported
  multiple (and nested) projects since Phase 2, but nothing in the UI exposed it: the
  app was hard-wired to one implicit project. Reported by the user — "I am not able to
  create new projects."
- A project bar above the title lists the workspace's top-level projects, renders the
  active one as selected (honey fill), and carries an inline composer for creating
  another. Selecting is by row index, matching how task rows report which row was acted
  on. New parts in **both themes**: `project-bar`, `project-on`, `project-off`,
  `project-input`, `project-add`.
- `TaskApp.mil` gains `project-rows : list<list<text>>` (each row `[ name,
  active-marker ]`), `new-project-name`, and the `onNewProjectNameChange` /
  `onAddProject` / `onSelectProject` emits. The layout renders the selected/unselected
  variants with `If`/`Else` on the marker cell — both branches dispatch the same
  `onSelectProject` with the outer loop index.
- `main.tsx` derives the bar from `workspace()` + `activeProject()`, creates projects
  with a collision-probed id (rather than trusting a counter, so a snapshot restored
  from elsewhere can't collide), and **switches to a project on creation** — otherwise
  you'd have to hunt for it and an empty new project would look like nothing happened.
  Per-project task lists fall out for free: the host's task order is workspace-global
  and `rows()` keeps only the ids the active project's `table()` knows about.
- **Required an ABI change** — see `task-wasm`'s changelog. Without `set_active_project`
  a created project was unreachable.
- **The selected project survives a reload.** The ABI deliberately keeps that cursor out
  of the engine snapshot, so the host persists it alongside the row order and re-selects
  after `load`. `WorkspaceRecord` gains an optional `activeProject`; records written
  before projects existed simply lack it and fall back to the default. 2 new host tests.
- Fixed in security review before landing: the new-project id probe checked only
  *top-level* projects, but ids must be unique across the **whole** workspace — with a
  nested project present it would have proposed an id the engine rejects, and "+ Project"
  would have become a permanent silent no-op. It now probes every project, and surfaces
  a failed create instead of swallowing it.
- **Nested projects render as a hierarchy.** The bar now lists *every* project, walked
  depth-first so a sub-project immediately follows its parent, with an indent glyph
  marking the nesting. `project-rows` gains a third cell (`indent`, empty for top-level,
  so the layout's `If` hides it and the row stays flush left), and a **"+ Sub"** button
  creates a project nested under whichever one is shown — making the engine's hierarchy
  reachable from the UI for the first time.
  - The engine stores nesting as a `parent` on a flat project map (a parent doesn't list
    its children), so the host derives the child lists — bucketing every project under
    its parent once, which keeps the walk O(n) instead of re-filtering the whole map per
    node. Siblings are ordered by name (sorting by raw id would put `p10` before `p2`).
  - The walk uses an **explicit stack, not recursion**, and carries a `seen` guard: a
    deeply nested chain can't blow the JS call stack inside `getProps` and take the whole
    render down, and a malformed snapshot containing a parent cycle terminates with each
    project listed once. It also sweeps up any project the `roots` list missed — an
    orphan whose `parent` names a project that no longer exists — so nothing is
    unreachable. (Both hardened in security review; verified with a 50 000-deep chain, a
    parent cycle, and a dangling-parent orphan.)
  - Verified against a real `wasm32` build driven from Node: depth-first ordering, depths
    (0/1/2), a nested project being selectable and owning its own tasks, its tasks *not*
    leaking into the parent, and the engine's refusal to delete a project that still has
    children.

### Changed - structured task rows (UI design, step 2)

- **Each task row is now a structured set of styled elements** — a round completion
  toggle, the task name, and meta chips (due / schedule / overdue) — instead of one
  baked string in a single button. This is the richer-row step of
  [`code/specs/task-app-ui-design.md`](../../../specs/task-app-ui-design.md), and it
  needed **no compiler change**: the `task-rows` slot became `list<list<text>>` (each
  row a list of cells), and the layout places each cell with a `For` over the rows plus
  `( row[n] )` cell access. Each chip is wrapped in an `If` on its own cell, so an empty
  cell (no due date, not overdue) renders nothing at all.
  - `TaskApp.mil` — `task-rows : list<text>` → `list<list<text>>`; the cell order
    `[ done-glyph, name, due, schedule, overdue ]` is now the documented contract
    between the host and the layout's `row[n]` indices.
  - `TaskApp.mll` — the single row button is replaced by a toggle (whose `number`
    payload still carries the outer row index `i`), a name, three conditional chips,
    and Delete.
  - `TaskApp.light.msl` / `.dark.msl` — new `toggle`, `task-name`, `chip-due`,
    `chip-sched`, `chip-over` parts (both themes); the old `row-btn` part is gone.
  - `host/web/src/main.tsx` — `getProps` returns each row as a `string[]` of the
    engine's already-formatted cells rather than concatenating them into one string.
    The engine still owns every value (glyph, date formatting, "overdue"); the host
    just stops flattening them.
  - Verified without the browser: `cargo test -p task-app` compiles all sources (both
    themes); regenerating via `mosaic-compile` produces the structured rows with the
    toggle/Delete dispatching the correct row index and the chips conditionally
    rendered; and the generated `taskRows: Array<Array<string>>` matches the host's
    `string[][]`. (Pre-existing, not from this change: the emitter maps mosstyle
    `align` to an invalid CSS `align` property — already present on other parts.)

### Changed - warm visual system on the app shell (UI design, step 1)

- **`TaskApp.light.msl` restyled to the "warm & approachable" identity** from
  [`code/specs/task-app-ui-design.md`](../../../specs/task-app-ui-design.md): warm paper
  ground (`#f0ebe3`), crisp warm-white surfaces, warm-charcoal ink, and a single honey
  accent (`#e0942a`) spent only on the primary Add action and active affordances.
  Semantic red stays a separate axis, used only for the destructive Delete. Adds
  `state focused` (honey ring on inputs) and `state hover` (honey-tinted rows, deeper
  honey on the Add button) — the first hover/focus affordances the app has had.
  Verified end-to-end: the sources compile (`cargo test -p task-app`), and the palette
  and the hover/focus selectors flow through `mosaic-compile --backend react` into the
  generated component's inline styles and CSS lattice.
- **`TaskApp.dark.msl` added** — the dark half of the same identity, authored not
  inverted: a warm near-black ground (`#1a1714`), warm-ivory ink, and the honey accent
  lifted to `#eaa63f` so it stays legible on the dark ground. Resolved by
  `mosaic-compile --theme dark`; verified the dark palette + states flow through to the
  generated React (inline styles + CSS lattice) and that `--theme light` still resolves
  the light theme. The compile test now compiles both themes against the layout parts.
  Host-level theme *switching* (wiring `prefers-color-scheme` / a toggle in the web host
  to select the emitted theme) is the remaining follow-on.
- This is the first increment of the design spec's Phase-5 work (design tokens + app
  shell). Richer rows (checkbox, chips, progressive-disclosure detail) and the timeline
  and board views are the follow-on increments; the prototype at
  `design/ui-prototype.html` remains the visual target.

### Changed

- **The web host is now a committed npm package** (`host/web/`) instead of files
  overlaid onto a generated Vite project. `scripts/build-web.{sh,ps1}` now emit only the
  generated `TaskApp.tsx` component (and copy the wasm runtime) *into* the package,
  rather than emitting a whole project and overlaying `main.tsx`. This makes the host —
  and the persistence added below — reliably buildable and testable. The host wiring
  moved from `host/web/main.tsx` to `host/web/src/main.tsx`. See `host/web/CHANGELOG.md`.

### Added

- **Local persistence for the web app** — the whole workspace is serialized via the
  engine's `snapshot()` and stored through the repo's pluggable `KVStorage` contract
  (`@coding-adventures/indexeddb`'s `IndexedDBStorage`, in-memory fallback), then
  restored on reload. Persistence is host-owned; the engine stays pure. (Phase 1 of
  `code/specs/task-app-super-app.md`.) Verified end-to-end: tasks + their schedule
  survive a page reload.

- **The task-app web UI** — a to-do app with automatic scheduling, authored in Mosaic
  and wired to the pure `task-core` engine through `task-wasm` (WebAssembly).
  - `src/TaskApp.mil` — interface: slots (`app-title`, `new-task-name`, `new-task-due`,
    `summary`, `task-rows`) and events (`onNewTaskNameChange`, `onNewTaskDueChange`,
    `onAddTask`, `onToggleTask`, `onDeleteTask`).
  - `src/TaskApp.mll` — layout: header, add-row (two `HostInput`s + `HostButton`), a
    summary line, and a `For` over `task-rows` with per-row toggle/delete buttons.
  - `src/TaskApp.light.msl` — light-theme styling.
  - `host/web/src/main.tsx` — the React entry: holds the engine in React state, maps the
    emitted `dispatch(event)` to engine operations (`createTask`/`setDuration`/
    `linkDependency`/`setDeadline`/`setCompleted`/`deleteTask`), and re-derives the
    slot props from engine queries (`todos`/`gantt`). **No dispatch/props facade** —
    idiomatic React, the engine stays pure.
- **Auto-scheduling** — new tasks are chained (finish-to-start) into a work queue, so
  the CPM engine sequences them across working days; the row shows each task's computed
  start→finish, the summary shows the projected project finish, and overdue tasks
  (finishing past their due date) are flagged.
- **Build tooling** — `scripts/build-web.{sh,ps1}` builds the engine to wasm, emits the
  `TaskApp.tsx` component via `mosaic-compile`, and copies it plus the wasm runtime into
  the `host/web` package. `cargo test` (`tests/package_compiles.rs`) verifies the Mosaic
  interface/layout/style compile and the manifest exports `TaskApp`; `npm test` in
  `host/web` verifies the persistence seam.

### Verified

- End-to-end in a browser: adding tasks auto-schedules them (e.g. three tasks land on
  consecutive working days Mon→Tue→Wed), completion toggles work, and the projected
  finish updates — all driven by the Rust engine over WASM, with no console errors.
