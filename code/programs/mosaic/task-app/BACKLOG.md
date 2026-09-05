# task-app backlog

> **PAUSED — 2026-09-02.** TaskApp feature work is on hold behind the Mosaic
> component program ([#14011](https://github.com/adhithyan15/coding-adventures/issues/14011),
> `code/specs/mosaic-component-program-v1.md`).
>
> This app was built app-first, on raw kernel primitives: 166 styled parts, 47
> raw `HostButton`s, 48 `If`/22 `Else`, and none of the 23 composed components
> `mosaic-pkg-toolkit` already ships. The cost showed up as platform gaps
> discovered too late — #13692 needed a runtime host environment that exists in
> none of the nine backends (#14003), and specifying that surfaced a missing
> adaptive container primitive.
>
> The native release P0 (#14249) is now closed and the product-scoped SemVer
> lane (#13543) has published `task-app-v0.1.0`. The UI49 model/compiler bridge
> is complete through #14300, and the React reference lowering landed in
> #14306, the WebComponent lowering in #14314, and the Compose lowering in
> #14322, followed by SwiftUI in #14331. The current dependency is #14336,
> the Qt runtime lowering within #14036. The remaining backends and toolkit
> retrofits still precede stories or trustworthy TaskApp composition coverage.
>
> The queue below is still accurate and is not abandoned. TaskApp is rebuilt at
> Phase 5, on components that are already proven in isolation. Until then the
> app may be an empty screen, which is the intended trade.

Working backlog for the [super-app roadmap](../../../specs/task-app-super-app.md). Ordered
by priority — top item is next up. Re-prioritized whenever a new item is discovered mid-flight.
Each item, once picked up, follows: spec-sync → tests → implementation → CHANGELOG → README →
`/security-review` → PR → `/babysit-pr` → auto-merge.

## Product completion queue

The durable product backlog is [GitHub issue #13517](https://github.com/adhithyan15/coding-adventures/issues/13517).
It tracks the local-first, Rust-scheduled, native TaskApp completion loop and
links existing Mosaic work instead of duplicating it.

1. **P0 — [#13518](https://github.com/adhithyan15/coding-adventures/issues/13518):**
   make the web release build reproducible and CI-gated. **Done in #13527.**
2. **P0 — [#13519](https://github.com/adhithyan15/coding-adventures/issues/13519):**
   persist TaskApp locally in every strict generated native host. **Done in #13542.**
3. **P0 — [#13573](https://github.com/adhithyan15/coding-adventures/issues/13573):**
   retain typed `For` row payloads in emitted XAML button handlers.
   **Discovered and fixed while validating #13520.**
4. **P0 — [#13520](https://github.com/adhithyan15/coding-adventures/issues/13520):**
   prove the native simple-todo scheduling flow end to end. **Done in #13575.**
5. **P0 — [#13651](https://github.com/adhithyan15/coding-adventures/issues/13651):**
   run the exact TaskApp release packaging matrix on release-relevant pull
   requests without granting publication authority. Discovered while packaging
   #13613 and resolved in the same validation-critical slice.
6. **P0 — [#13658](https://github.com/adhithyan15/coding-adventures/issues/13658):**
   preserve the application PRI and generated XBF files in emitted WinUI
   `dotnet publish` payloads. Discovered by the exact #13613 release gate and
   resolved in the same Windows packaging slice.
7. **P1 — [#13543](https://github.com/adhithyan15/coding-adventures/issues/13543):**
   publish incremental SemVer TaskApp GitHub releases, beginning at
   `task-app-v0.1.0` with only the artifacts that are actually verified. The
   release lane builds a tested web bundle and strict generated native projects;
   installer packaging remains #13522. **Done in #13587, with follow-up clean-
   runner fixes in #13590, #13593, and #13596; `task-app-v0.1.0` published.**
8. **P1 — [#13565](https://github.com/adhithyan15/coding-adventures/issues/13565):**
   keep native completion progress visible in the generated Compose viewport.
   **Done in #13602.**
9. **P1 — [#13521](https://github.com/adhithyan15/coding-adventures/issues/13521):**
   enforce web/native presentation-contract parity. **Done in #13607.**
10. **P1 — [#13522](https://github.com/adhithyan15/coding-adventures/issues/13522):**
   package installable local apps on supported platforms. Decomposed into the
   independently verifiable platform and operations items below. **Done — all
   four children (#13616, #13627, #13652, #13679) shipped; closed 2026-08-31.**
11. **P1 — [#13611](https://github.com/adhithyan15/coding-adventures/issues/13611):**
   ship verified portable Linux bundles for Qt, Flutter, and Compose Desktop.
   **Done in #13616.**
12. **P1 — [#13612](https://github.com/adhithyan15/coding-adventures/issues/13612):**
   package TaskApp as a macOS application bundle. **Done in #13627.**
13. **P1 — [#13613](https://github.com/adhithyan15/coding-adventures/issues/13613):**
   package TaskApp for Windows installation and launch. **Done in #13652.**
14. **P1 — [#13614](https://github.com/adhithyan15/coding-adventures/issues/13614):**
   document and verify upgrade, backup, restore, and manual recovery. **Done in
   #13679.**
15. **P1 — [#13523](https://github.com/adhithyan15/coding-adventures/issues/13523):**
   audit and refine the first-run simple-todo experience. The audit is recorded
   in `code/specs/task-app-first-run-usability-audit-v1.md`; focused findings are
   queued below.
16. **P1 — [#13687](https://github.com/adhithyan15/coding-adventures/issues/13687):**
   give an empty Inbox useful guidance and deterministic first-task focus.
   **Done in #13706.**
17. **P1 — [#13689](https://github.com/adhithyan15/coding-adventures/issues/13689):**
   surface composer validation errors with useful focus behavior. **Done in
   #13719.**
18. **P1 — [#13688](https://github.com/adhithyan15/coding-adventures/issues/13688):**
   edit task names and due dates directly from List. **Done in #13740.**
19. **P1 — [#13691](https://github.com/adhithyan15/coding-adventures/issues/13691):**
   give completion controls descriptive accessible names. **Done in #13755.**
20. **P1 — [#13717](https://github.com/adhithyan15/coding-adventures/issues/13717):**
   lower authored `HostInput.a11y-label` values across every Mosaic backend.
   Discovered while validating #13689; adjacent semantic error text keeps the
   current correction accessible without overstating input-label coverage.
   **Done in #13769.**
21. **P1 — [#13754](https://github.com/adhithyan15/coding-adventures/issues/13754):**
   preserve dynamic `HostButton.a11y-label` values in Qt, Flutter, SwiftUI,
   and XAML. Discovered while validating #13691; React and Compose are covered
   by that product fix, while the other native emitters need focused repairs.
   **Done in #13782.**
22. **P1 — [#13725](https://github.com/adhithyan15/coding-adventures/issues/13725):**
   make Flutter preserve the correct flex context through nested Mosaic
   `If`/`Else` branches. Discovered by the #13689 native widget lifecycle; the
   TaskApp layout now avoids the invalid lowering, while the emitter needs a
   focused regression so another app cannot recreate it. **Done in #13803.**
23. **P1 — [#13690](https://github.com/adhithyan15/coding-adventures/issues/13690):**
   expose durable-storage fallback, recovery, and backup information in the UI.
   **Done in #13837.**
24. **P1 — [#13695](https://github.com/adhithyan15/coding-adventures/issues/13695):**
   replace blank startup with loading and failure states.
25. **P1 — [#13692](https://github.com/adhithyan15/coding-adventures/issues/13692):**
   make the List-first shell usable in compact windows.
26. **P2 — [#13526](https://github.com/adhithyan15/coding-adventures/issues/13526):**
   move the Vitest config to Vite's native ESM loading contract. **Done in
   [#14242](https://github.com/adhithyan15/coding-adventures/pull/14242).**
27. **P2 — [#13625](https://github.com/adhithyan15/coding-adventures/issues/13625):**
   roll the TaskApp changelog forward after each published product release and
   gate against already-published versions remaining marked Unreleased.

Newly discovered work is filed as an issue and the queue is reprioritized before
the next item is selected. Only one TaskApp completion-loop PR is active at a time.

## Next up (priority order)

The design-fidelity gap (see `CHANGELOG.md`'s "re-closed the design-fidelity gap"
entry, and Resolved below for icon/SVG assets, Board, richer Gantt, and Calendar) is
now closed except for one low-priority polish item — see the Backlog section below.

The fresh pass this section used to defer to is now written down:
[`code/specs/task-app-platform-completion-v1.md`](../../../specs/task-app-platform-completion-v1.md).
It measures TaskApp against all nine Mosaic backends rather than against the
super-app feature roadmap, and it found that three backends — `html`,
`webcomponent`, and `paint` — contain **zero** TaskApp references: no test, no CI
step, no artifact. The ordered queue below comes from that spec.

**Tier A — finish the platforms TaskApp already claims.**

1. **P0 [#14249](https://github.com/adhithyan15/coding-adventures/issues/14249):**
   restore Flutter's bundled Rust runtime after the current Linux runner image
   exposed a broken 3.44.0 native-asset bundle. The repair stages the runtime
   through the hook-owned output directory while keeping the pinned 3.44.0
   toolchain, with native assets enabled explicitly on every fresh runner. The
   validation lanes now select Flutter's install-generated bundle explicitly,
   rather than whichever same-named executable filesystem traversal returns
   first. **Done in #14252.**
   This blocks validated Flutter/Linux release payloads, so it precedes the
   remaining native-host and release-polish work.
2. **P1 [#13695](https://github.com/adhithyan15/coding-adventures/issues/13695):**
   replace blank startup with loading and failure states. **Done for the web
   host.** Split out while implementing it:
   [#13984](https://github.com/adhithyan15/coding-adventures/issues/13984) —
   generated native hosts still surface startup failure only through process and
   log evidence, which needs a distinct surface in five backends and its own
   emitted-control coverage. The host-neutral contract both share is
   `code/specs/task-app-startup-states-v1.md`.
3. **P1 [#13692](https://github.com/adhithyan15/coding-adventures/issues/13692):**
   make the List-first shell usable in compact windows. **Blocked on
   [#14003](https://github.com/adhithyan15/coding-adventures/issues/14003)
   (UI48).** Picking this up revealed it was mis-scoped as a TaskApp change.
   Mosaic has no way for *any* app to respond to its runtime environment:
   mosstyle has no media queries, `--variant` selects a layout file at compile
   time, none of the nine emitters observes viewport or pointer, and
   `mosaic-app-runtime` has no environment concept. UI30 §6 deferred runtime
   selection to an ML4 that was never built. Doing it TaskApp-side would mean a
   bespoke `compact` slot plus per-backend host code — exactly the userland
   conditional UI30 explicitly rejected, and it still could not vary a *style*,
   so touch-sized tap targets would stay unexpressible. Specified generically as
   `code/specs/UI48-host-environment.md`; TaskApp then becomes
   `TaskApp.compact.mll` and nothing else.
   UI48's ENV slices continue as separate kernel work.
4. **P2 [#13526](https://github.com/adhithyan15/coding-adventures/issues/13526):**
   move the Vitest config to Vite's native ESM loading contract. **Done in
   [#14242](https://github.com/adhithyan15/coding-adventures/pull/14242).**
5. **P2 [#13625](https://github.com/adhithyan15/coding-adventures/issues/13625):**
   roll the changelog forward after each published release and gate against a
   published version still marked Unreleased. `0.1.0` is in exactly that state
   today.

**Discovered while working Tier A, awaiting prioritization.**

- **P1 [#13984](https://github.com/adhithyan15/coding-adventures/issues/13984):**
  native startup failure states (above). Ranks with Tier A, since it is the same
  defect on five platforms that ship release artifacts.
- **P2 [#13982](https://github.com/adhithyan15/coding-adventures/issues/13982):**
  a `packages.microsoft.com` 403 hard-fails required jobs through 10 unguarded
  `apt-get update` calls across 5 workflows. Red-flagged a four-file docs PR.
  Not taken yet: it edits three other products' release lanes, which is wider
  blast radius than this loop should take unprompted. Workaround is a rerun.
- **P2 [#13977](https://github.com/adhithyan15/coding-adventures/issues/13977):**
  signing/notarization/installers, filed when #13522 closed while the README
  still pointed at it.

**Tier B — close the three unexercised backends.** Filed as work is picked up;
see the spec for the completion bar each one has to clear.

6. Static HTML snapshot gate (cheapest — no runtime, no interaction claim).
7. Web Components host — the last *interactive* backend with no TaskApp presence.
8. Paint visual-regression gate — the only mechanism that would catch a purely
   visual regression.

**Tier C — reach, stated rather than silently missing.** iOS compiles but does
not run; Android has no Mosaic backend and belongs to #12017; signing,
notarization, and installers are tracked under #13977 — filed because #13522
closed on 2026-08-31 while the README still pointed readers at it, leaving the
limitation recorded against a closed issue.

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
- **Native XAML drag semantics.** WinUI now lowers UI35 to component-scoped
  native drag/drop controls with pointer/touch events, equivalent keyboard
  traversal, authored acceptance/disabled rules, RTL order, lifecycle dispatch,
  and UI Automation announcements. This removes the board/calendar's final four
  degradation reports and promotes complete TaskApp XAML generation to the strict
  `native-complete` profile.

- **Native XAML table semantics for the Sheet.** The canonical indexed UI31/Grid
  shape now emits component-scoped WinUI table, header, and cell controls whose
  peers implement UIA Table/Grid and TableItem/GridItem patterns. The authored
  editable Cell subtree remains intact, while automation clients receive stable
  dimensions, header associations, row/column coordinates, names, and arrow-key
  movement. Ambiguous HostTable trees retain the visual fallback and explicit
  degradation.

- **Concrete Rust TaskApp engine in the XAML WinUI artifact.** Windows CI now
  builds `task-mosaic-app`, emits the complete TaskApp with that selected DLL,
  verifies `mosaic_app.dll` beside `TaskApp.exe` byte-for-byte, and drives initial
  props plus a real `newTaskNameChange` event through the generated standard .NET
  binding without `MOSAIC_APP_LIBRARY`. The app remains permissive for the four
  explicitly asserted XAML drag/drop gaps, and visible launch remains an interactive
  Windows-worker gate rather than a claim made from GitHub's hosted worker.

- **Concrete Rust TaskApp engine on strict SwiftUI for macOS.** SwiftUI CI keeps
  the counter ABI conformance package and harness independent, then emits TaskApp
  with `task-mosaic-app`, requires zero degradations, compares the SwiftPM resource
  `libmosaic_app.dylib` byte-for-byte with the adapter artifact, and launches the
  generated macOS executable from `/` without an injected runtime path. The same
  generated UI also compiles separately for iOS 16 without bundling the macOS dylib.

- **Native SwiftUI table semantics for the Sheet.** The canonical dynamic
  UI31/Grid shape now uses native `Table` and runtime-sized
  `TableColumnForEach` definitions with stable row identity, safe width/cell
  bounds, and the existing interactive Cell body. macOS 13 and iOS 16 use a
  native `List`/`Section` compatibility path until the dynamic-column API is
  available. The full generated TaskApp compiles on macOS and for the iOS 16
  deployment target; SwiftUI's permissive report now retains only the sample
  runtime fallback, so the concrete Rust adapter promotion is next.

- **Native SwiftUI drag support for HostDraggable/HostDropTarget.** The UI35
  family now uses native SwiftUI pointer/touch transfer and drop delegates,
  with component-scoped payloads, accepts/disabled filtering,
  before/into/after proposals, keyboard and assistive actions, RTL movement,
  and platform accessibility announcements. Both the macOS and iOS 16
  generated TaskApp targets compile; board and calendar drag interaction no
  longer blocks strict TaskApp emission.

- **Concrete Rust TaskApp engine on strict Compose Desktop.** Compose CI keeps
  the counter ABI conformance distributable and harness independent, then emits
  TaskApp with `task-mosaic-app`, requires zero degradations, compares the installed
  `libmosaic_app.so` with the adapter artifact, and launches the packaged Linux app
  under a virtual display without an injected runtime path.

- **Concrete Rust TaskApp engine on strict Flutter.** Flutter CI keeps the small
  counter ABI conformance build as an independent binding proof, then separately
  emits TaskApp with `task-mosaic-app`, requires zero degradations, builds the Linux
  desktop bundle, compares its `libmosaic_app.so` byte-for-byte with the adapter
  artifact, and launches the packaged app under a virtual display without an
  injected runtime path. This turns the earlier compile-only TaskApp shell into the
  second concrete native application gate after Qt.

- **Concrete Rust TaskApp engine adapter + strict Qt application.**
  `task-mosaic-app` composes the pure `task-core` engine with portable presentation
  state, emits every required TaskApp MIL slot, accepts every declared semantic
  event, snapshots/restores the combined state, and exports the standard Mosaic C
  ABI. Qt CI now builds this library, emits TaskApp with `native-complete`, requires
  zero degradations, installs the app/runtime together, compares the bundled library
  to the exact build artifact, and launches the installed app offscreen without an
  injected path. The launch exposed and closed a real Qt `For` bug: under
  `pragma ComponentBehavior: Bound`, non-empty Repeater delegates must declare
  `modelData` and `index` as required properties. Empty sample data had hidden it.

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
