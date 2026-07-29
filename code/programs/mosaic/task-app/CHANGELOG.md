# Changelog

All notable changes to the `task-app` web program are documented here.

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
