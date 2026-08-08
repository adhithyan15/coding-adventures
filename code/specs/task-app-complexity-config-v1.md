# Trestle — per-project complexity config (v1)

The second half of Phase 9 (`task-app-super-app.md` §7), following the
nested-project tree half that shipped as `mosaic-pkg-project-nav` (see
`task-app-project-nav-v1.md`). Closes §2.3, which the roadmap calls "the
single most important product rule":

> Configurable per project/task: a project can be set "board only" and
> never reveal the scheduling machinery; another can be a full CPM plan.
> Same engine, same data, different exposed surface.

`BACKLOG.md`'s Phase 9 item flagged this as needing a decision addendum
before any code, because the spec doesn't say what "board only" hides,
whether the toggle is per-project or per-task, or what middle tiers (if
any) exist. This document is that addendum.

## Decision 1 — per-**project**, not per-task

The toggle lives on `ProjectSettings`, not on `Task`. Two reasons:

- The spec's own worked example is project-level ("a project can be set
  'board only'"). Nothing in §2.3 describes a *task* needing its own
  independent complexity flag.
- Task-level granularity **already exists**, for free, via
  `Task.schedule: Option<TaskSchedule>` — a task with no schedule already
  behaves as a plain to-do (§2.3: "an unscheduled task never appears on
  the Gantt and never forces CPM"). Layering a second, redundant
  complexity flag on `Task` would duplicate a distinction the data model
  already makes structurally.

This resolves the backlog item's literal "per project/task" ambiguity:
project-level is the *config surface*; task-level granularity is the
*existing data shape*, not a new field.

## Decision 2 — two tiers, no middle ground

`ProjectComplexity { Board, Full }`. The backlog asked "what middle tiers
if any" — the phase's own title (`board-only ↔ full CPM`) is a binary,
and nothing in §2.3 or the UI-design spec describes an intermediate
state. Adding one would be speculative scope with no spec backing it.
If a real need for a middle tier surfaces later, it's an additive enum
variant — this is not a one-way door.

## Decision 3 — defaults: safe for existing data, simple for new data

`ProjectComplexity` derives `Default -> Full`, and the field gets its own
`#[serde(default)]` (not just a struct-level `Default`, since
`ProjectState.settings` itself has no `#[serde(default)]` today — see
`model.rs`). This means:

- **Existing persisted projects** (snapshots predating this field) load
  as `Full` — identical to today's actual behavior (Timeline visible,
  full task-detail, all Sheet columns). No regression, no surprise
  disappearance of data a user was already looking at.
- **Newly created projects** get `Board` explicitly, set in
  `ProjectState::empty()` — the single constructor every new project
  goes through (top-level, nested subproject, or the workspace's own
  initial default project) — rather than relying on the struct default.
  This is where §2.3's "simple by default" actually lands: a brand-new
  project starts board-only and opts up, rather than starting fully
  CPM-exposed and never being simplified.

## Decision 4 — exactly what "Board" hides

Classified every scheduling-adjacent surface in the current UI as
**basic** (todo-app concept, independent of CPM math) or **CPM machinery**
(only meaningful once a duration-based schedule pass has run), then hid
only the latter:

| Surface | Basic (stays visible) | CPM machinery (hidden in Board tier) |
|---|---|---|
| View switcher | List, Board, Sheet, Calendar, Notes | **Timeline** — removed from the switcher entirely |
| List row cells | `due` (`row[2]`), `overdue` (`row[4]`) | `schedule` window chip (`row[3]`, "start → finish") |
| Task-detail (expanded row) | dependency list (`row[12]`), notes (`row[13]`) | scheduled/earliest/latest (`row[6]`), slack/critical prose (`row[7]`), free slack (`row[8]`) |
| Sheet columns | Done, Name, Deadline, % Complete, Priority, Status, Notes, Overdue, Labels | `start`, `finish` |

Two judgment calls worth stating explicitly:

- **`deadline`/`overdue` stay in Board tier.** A due date and "is this
  late" are todo-app concepts a board-only user still wants — they don't
  require a CPM network solve, only a date comparison. Hiding them would
  make Board tier *less* useful than a plain checklist, which isn't the
  intent (§2.3: "due date → it's a todo" is presented as the *first*,
  not the *forbidden*, rung of the capability ladder).
- **Dependencies stay in Board tier.** §2.3 lists "dependency → it
  sequences" as its own incremental capability, distinct from "duration →
  it can auto-schedule". A board-only project can still say "this card
  comes after that one" without engaging duration/CPM math — sequencing
  and scheduling are different capabilities in the spec's own ladder.

`start`/`finish` (both in the List row and the Sheet) and the three
task-detail scheduling lines are the only genuinely CPM-derived
outputs — they don't exist without a full forward/backward-pass network
solve — so those, plus the Timeline view built entirely around them, are
what Board tier suppresses.

Implementation note: the engine keeps computing CPM as it always has
(nothing about the scheduling algorithm changes); Board tier is a
**display-time filter in the host**, not an engine-side computation
toggle. This keeps the change additive and low-risk — same engine, same
data, different exposed surface, exactly as §2.3 states.

## Decision 5 — the control

A single small toggle in the topbar, next to the view-switcher, showing
the active project's current tier ("Board" / "Full CPM") and flipping it
on click. Deliberately **not** a per-project-row control in the rail:

- Settings apply to whichever project is currently active, and the
  topbar already reads as "controls for the thing you're looking at"
  (the view switcher lives there for the same reason).
- Keeping it out of `mosaic-pkg-project-nav` matches the precedent
  already set for the brand row and the view-switcher in
  `task-app-project-nav-v1.md`: things that are TaskApp's own concern,
  not generic project-list navigation, stay in `TaskApp` itself.

Switching a project from Full to Board while its Timeline is the active
view forces the view back to List (the switcher can't show an "on" state
for a button it just hid). Switching projects while on Timeline, into a
Board-tier project, does the same.

## What this does NOT include

- No middle tiers (Decision 2).
- No per-field granularity beyond the table above — e.g. no "hide labels
  but keep priority" customization. That's real feature work with no
  spec backing it; this ships the binary the spec actually describes.
- No engine-side gating of CPM computation — see the implementation note
  under Decision 4.
- No change to `Task.schedule`'s existing optionality — that mechanism
  already does its job and needed no changes here.

## Wiring summary

- `task-core`: `ProjectComplexity` enum (`Board`/`Full`, `#[serde(default)]`
  via the field), `ProjectSettings.complexity` field, a new
  `set_project_complexity(project_id, complexity)` op,
  `ProjectState::empty()` sets `Board` explicitly on the new
  `ProjectSettings` (see Decision 3).
- `task-wasm`: `set_project_complexity` exported via the existing
  `export_ws_op!` macro pattern (no settings mutation op existed before
  this — `ProjectSettings` currently only reaches JS read-only, embedded
  in the whole-project JSON `workspace()` already returns).
- `TaskApp.mil`: `slot project-complexity-label : text ;` (active
  project's tier for display), `slot allow-timeline : text ;`
  (non-empty when the active project is Full — gates the Timeline
  button), `emit onToggleProjectComplexity ;`.
- `TaskApp.mll`: a `HostButton` in the topbar next to `seg` showing
  `project-complexity-label`; each of the six view-switcher branches'
  `seg-tl-*` button wrapped in `If (when: slot: allow-timeline)`.
- `main.tsx`: reads `settings.complexity` off the active project (same
  `engine.workspace()` dump pattern every other field already uses — no
  new query needed to *read* it), dispatches
  `onToggleProjectComplexity` to the new op, forces `view` back to
  `"list"` when it would otherwise show a hidden Timeline, and filters
  the CPM-machinery cells/columns from `taskRows`/`SHEET_FIELDS` for
  Board-tier active projects.
