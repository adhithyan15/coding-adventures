# Changelog — task-mosaic-app

## [Unreleased] — Compose rows share their width

- No code change here. On Compose, a control's `width: 100%` in a Row is now
  its share of the Row (`mosaic-emit-compose`). In the renders, the sidebar's
  project buttons span the sidebar, *New project* takes what `+` leaves, and
  a task row's name takes the row's slack, as on the web. `TaskAppUiTest`
  still passes.

## [Unreleased] — the screenshot harness visits every view

- `conformance/compose/TaskAppScreenshots.kt` now also renders Board, Sheet,
  Calendar and Notes with one task in them (`03a`–`03d`). It picks the view
  tab by the SegmentedControl's `segmented-option` tag plus its label,
  because "Board" is also the label of the top bar's complexity toggle,
  which would switch the tier instead of the view. The Calendar render is
  what showed Compose dropping percentage widths.

## [Unreleased] — writing questions into a checklist template (C3c, #14018)

- Selecting an outline item (`onSelectOutlineItem`; clicking it again clears
  the selection) enables four actions:
  - toggle it between a step and a question (`onToggleOutlineQuestion`). A
    step's existing sub-items become the question's Yes branch.
  - add the composer's text to the question's Yes or No branch
    (`onAddChecklistItemYes`, `onAddChecklistItemNo`);
  - delete it with everything under it, deepest first (`onDeleteOutlineItem`).
    `delete_task` alone would strand a question's branch items.
- Outline rows gain a question marker, a branch label ("Yes" or "No") and a
  selected marker. New props: `selected-outline-key`, `outline-item-selected`,
  `outline-question-selected` and `outline-toggle-label`.
- The selection is serde-defaulted, cleared with the checklist selection, and
  dropped by `repair()` when it no longer names an item.

## [Unreleased] — the Compose startup-recovery test no longer races (#15788 follow-up)

`generatedStartupFailureIsVisibleAndRetryRerunsInitialization` waited for
`attempts == 2`, then asserted that "Recovered TaskApp" was displayed. But the
counter reaches 2 when the second `loadHost` call *starts*, before the
recovered host is returned and composed. The assertion therefore raced the
recomposition and failed intermittently in CI (seen on #15947 and #15951). It
now waits for the recovered content itself, then checks the attempt count.

## [Unreleased] — the Checklists view (C3a, #14018)

Spec: `code/specs/task-app-checklists-view-v1.md`.

- A seventh view, **Checklists**, sits before Timeline in the switcher and is
  offered at both tiers. It shows a library of the active project's templates
  (by name) and runs (newest first) beside the selection:
  - a template's outline, with an *Add item* composer, Start run and Delete;
  - a run's rows, in `mosaic-pkg-checklist`'s `ChecklistRun` shape, with
    Complete (offered only once the run is complete) and Abandon.
- New slots `checklists-mode` … `checklist-abandon-label` and 13 new events.
  Answering a question with its current answer clears it.
- **Checklist items no longer appear as tasks.** List, Board, Sheet, Calendar,
  Timeline and the summary counts skip template and run items, as task-core's
  own views already do.
- `TaskMosaicApp::with_clock` injects the clock that stamps runs. The default
  is the system clock; the clock is not part of the snapshot.
- Bounds:
  - composers refuse input over 512 characters;
  - *Add item* stops at the engine's `MAX_CHECKLIST_ITEMS`;
  - new items take the next sibling order;
  - **every** minted id (task, project, label, note, checklist, run) uses a
    checked counter, so it fails rather than wraps or panics;
  - row indents are capped at 16 levels, because depth in a restored
    snapshot is unbounded;
  - library item counts come from `ChecklistSummary.items` (one pass), not
    from one outline per template on every event;
  - Board and Calendar drops refuse checklist items' keys.
- The selection and composers are `#[serde(default)]` in snapshot v1, so every
  earlier snapshot still restores. `repair()` drops a dangling selection and an
  oversized draft.

## [Unreleased] — acceptance uses TaskApp's declared window (#14789)

The 1280 x 900 Compose acceptance viewport is now pinned to TaskApp's `[app]`
manifest declaration. The generated desktop app and its fit/overflow assertions
therefore measure the same window instead of two unrelated constants.

## [Unreleased] — complete native Board column rows (#15473)

`board-columns` now supplies the count and theme-aware accent cells that the
shared TaskApp layout reads. Native hosts previously received two-cell rows for
a four-cell contract, so entering Board indexed past the row and terminated the
WinUI process inside `Microsoft.UI.Xaml.dll`.

## [Unreleased] — the view switcher sends plain labels (UI86, #15420)

`nav-options` is a `list<text>`: one label per view, instead of
`[label, accessible-name]` rows with ", selected" written into the showing
view's name. Toolkit 0.15's `SegmentedControl` reports the selection through
the kernel's selected state.

The switcher test now asserts the labels never change with the selection.

## [Unreleased] — the view switcher's rows and index (#14016)

`props()` now also returns:

- `nav-options`: `[label, accessible-name]` for List, Board, Sheet, Calendar
  and Notes, plus Timeline for a Full project. The showing view's name reads
  "<label>, selected".
- `nav-selected-index`.

`onShowView` takes an `index` into the views this project offers.

`ViewMode::SWITCHER_ORDER` is separate from the enum's declaration order,
because the enum's order is snapshot history. Timeline is last so that
omitting it never shifts another index.

**Tests.** `view_switcher_rows_track_the_view_and_the_tier` covers:

- the Board tier offers five views, and refuses index 5 without changing
  state;
- the Full tier adds Timeline last;
- rows, index and the `*-mode` slots agree for every index;
- dropping back to Board while on Timeline returns to List at index 0.

The slot-name pin list and the every-event test include the new names.

## [Unreleased] — the sheet's column widths were ratios, not pixels (#15131)

Three cells in Trestle's Sheet view measured **zero width** — the task name, the
completion toggle and the due date — at 1280, 900 and 700 alike.

`sheet-column-widths` was `[3, 1, 2, 2, 2]`, plainly intended as relative
proportions. But `Grid.mil` declares the slot as **"per-column pixel widths"**,
and every backend threads the number straight into a width, so those were 3px
and 1px columns. A 1px column has no room for its text.

Now `[240, 80, 160, 160, 160]` — the same proportions at an 80px unit, matching
the grid's own 72px minimum cell. Measured after: **0 zero-width nodes in the
Sheet view at all three viewports**, and every other Trestle view stays clean.

#### Why nothing caught it

A ratio and a pixel width are both `number`, so the slot type cannot tell them
apart and no backend can either. The viewport-independence was the tell: this
was zero at 1280 as much as at 700, whereas a starvation bug is
viewport-dependent by definition — that is what separated it from UI59.

It was also invisible to every existing gate, because presence and
accessibility assertions all pass on a `0 x h` node.

The new test asserts the **magnitude** rather than the layout: every sheet
column must be at least 40px. It is deliberately loose — it exists to catch
single digits, not to pin a design.

## [Unreleased] — the Flutter harness measured the test font, not the app (#14857)

`flutter test` ships a default font that measures **one em per glyph**, so every
width taken in this harness was roughly double the real thing. That distortion
is not a detail — it is the whole of #14857.

That issue reported Trestle's task row at **1076px on Flutter against 466 on
Compose**, a 2.3x layout defect that blocked `max-width` on Flutter (#14851).
Loading a real font and re-measuring the same row:

| widget | test font | real font | Compose |
| --- | --- | --- | --- |
| Edit | 68 | **48.0** | 64 |
| Delete | 97 | **53.1** | 77 |
| task name | 318 | **128.2** | 185 |
| due date | 200 | **96.1** | 133 |

With a real font Flutter is **narrower than Compose on every text widget**, and
the row sums to ~389px — comfortably inside the authored 760px cap. The toggle
button measured 64 on both backends all along, which already ruled out the
Material-minimum-size theory.

The harness now loads a real font when one is present and **proves it took
effect**: `Delete` is asserted under 75px, separating the real 53.1 from the
test font's 96.6. Without that check the load would be decoration — the suite
passes either way, because nothing else here asserts a width, and that is
exactly how this went unnoticed for so long.

Where no font file exists the check self-disables rather than failing, so the
suite still runs; only width claims lose their meaning there.

## [Unreleased] — declare the Compose acceptance viewport (#14771)

`TaskAppUiTest` used `createComposeRule()`, whose surface is whatever the test
framework defaults to — 1024 x 768. Nobody chose that number, yet every
`assertIsDisplayed()` in the test silently meant "fits in 1024 x 768". The
first layout change that made TaskApp a few pixels taller (#14730, directional
padding) pushed the freshly added task row to y = 768.0 and failed an assertion
that was never about height, reported only as "is not displayed".

The test now runs `runSkikoComposeUiTest` at a declared 1280 x 900 desktop
window. No assertion was weakened: that is a real window, and content still has
to be on screen in it without scrolling.

It also now asserts the invariant **directly** rather than relying on one row
happening to land above the fold. A scrollable that can scroll reports
`VerticalScrollAxisRange.maxValue > 0`, and that value *is* the overflow in
pixels, so the test asserts it is zero at three stages. Verified by falsifying
it — at the old 1024 x 768 the new assertion fails with:

```
TaskApp overflows the 1024 x 768 acceptance viewport by 244.0px at stage
'after first add' -- content below the fold is not reachable, because
performScrollTo() hangs against the emitted scroll container.
```

Width, not height, was the real constraint: at 1024 the composer and list wrap
and content grows to ~1012 px tall; at 1280 it reflows to ~538 px.

Two defects this deliberately does not paper over, both filed:

* **#14789** — the generated app's own `Window` takes Compose's 800 x 600
  default, which is smaller than this test's window and smaller than the layout
  needs, so a first launch buries the task list. Fixing that is a product
  decision (should window size be authorable in Mosaic?), not a test change.
* **#14790** — `performScrollTo()` hangs indefinitely against the emitted
  scroll container, which is why "fit the viewport" is currently the only
  option for reaching content at all.
## [Unreleased] — render TaskApp to PNGs so it can be looked at (#14798)

`TaskAppScreenshots` captures the empty inbox, one scheduled task, and that
task completed, via `captureToImage()` against the real `native-complete`
project and the real Rust runtime.

Every other gate in `conformance/compose/` asserts the **semantics tree** —
node exists, right accessible name, marked displayed — and all of them pass
while the app is unreadable. The first render showed run-together text
(`Saved locally on this deviceLocal only · ...`, `Up next1`), a `100% complete`
label clipped mid-phrase, a duplicated and overlapping segmented control, an
`On track` chip collapsed into a one-character-wide vertical column, and two
thirds of the window left blank white.

The sharpest example: the README claims the Compose gate "requires the
Rust-owned `100%` completion progress to remain displayed rather than merely
existing in an off-screen semantics tree". It passes — the node whose text is
`100%` really is displayed. The word `complete` beside it is cut off. The gate
measured exactly the thing it named and still missed the point.

This is **not** a pixel-diff gate. It asserts only that rendering succeeds and
produces a surface of the requested width; the images are for a person to look
at. A strict pixel baseline needs a pinned font stack and renderer or it fails
on every platform difference, and that is a separate decision (#14798).

Skipped unless `MOSAIC_SHOT_DIR` is set, so it never slows the normal gates.

## [Unreleased] — expose native local-data guidance (#13690)

The native adapter now publishes the shared storage status, location, and warning
slots. Windows and macOS name their exact live snapshot path; Linux points to the
backend-specific `LOCAL-DATA.txt` shipped beside every release so Qt's intentionally
different path is not misstated.

## [Unreleased] — name completion controls by action and task (#13691)

Every List row now publishes a state-aware accessible name for its compact
completion control. The name includes both the current action and task name and
switches between `Complete task: …` and `Reopen task: …` after each Rust-owned
completion mutation.

## [Unreleased] — edit List tasks atomically (#13688)

The native adapter now owns a transient one-row edit session and commits valid
name and optional due-date changes through `task-core`'s `rename_task` and
`set_deadline` operations. Blank names and impossible dates leave engine state
unchanged; save and cancel clear the draft and restore composer focus.

## [Unreleased] — expose atomic composer validation (#13689)

The native adapter now publishes plain-language task-name and due-date error
slots. Invalid submissions leave the workspace and id sequence unchanged;
correcting the relevant value clears its error, while transient validation
state remains outside durable snapshots.

## [Unreleased] — pin the first-release upgrade fixture (#13614)

The adapter now restores a committed TaskApp v0.1.0 semantic fixture in its unit
suite, including the saved task, due date, Rust schedule, and Full CPM project
setting. Release packaging materializes this same fixture into the standard host
snapshot envelope before exercising each desktop bundle's stable data path.

## [Unreleased] — share the web/native presentation contract (#13521)

The native adapter now consumes the same data-driven lifecycle fixture as the
real web/WASM controller. Each checkpoint compares canonical engine state and
user-visible core slots across navigation, task scheduling and lifecycle,
project/complexity changes, and snapshot restoration; deliberate host-only
theme and locale behavior is documented rather than silently normalized.

## [Unreleased] — require visible Compose completion progress (#13565)

The generated Compose lifecycle fixture once again requires the Rust-owned
`100%` completion value to be displayed in the default desktop viewport, not
merely present in the off-screen semantics tree.

## [Unreleased] — exercise the native simple-todo lifecycle (#13520)

Task-specific conformance fixtures now drive generated Qt, Flutter, Compose
Desktop, SwiftUI/macOS, and XAML controls through create, Rust scheduling,
complete/reopen, delete, invalid-input atomicity, and persisted restart
restoration. A shared generated-source contract rejects inert controls, sample
fallbacks, and missing standard-runtime wiring before platform execution.

## [Unreleased] — align Compose scheduling lifecycle with its selected view (#13559)

The generated Compose TaskApp UI fixture now proves Rust scheduling through the
always-present projected-finish summary instead of assuming that one complexity
toggle selects Timeline. Its two-launch lifecycle keeps invalid-input atomicity,
create, complete/reopen, delete, persistence, restoration, and restored-delete
coverage against the real Rust dynamic library.

## [Unreleased] — accept native integral index numbers (#13560)

Indexed TaskApp events now accept both JSON integer values and mathematically
integral floating-point values emitted by native Mosaic backends. Fractional,
negative, non-numeric, and out-of-range values remain invalid instead of being
silently truncated.

## [Unreleased] — prove TaskApp restart restoration (#13519)

The XAML runtime acceptance now launches twice against the same native snapshot
file and verifies that a Rust-owned composer edit survives process restart.

## [Unreleased] — expose ring-percent-value as typed data (#12028 item 2)

`"ring-gradient": ""` was an unconditional, always-empty placeholder in
the emitted props — the workspace-progress ring's percent-complete was
computed here (`percent`, already used to format the `ring-percent`
caption string) but never exposed as typed data any host besides web
could act on. Native hosts received nothing to render the ring from at
all — "a leak in the data contract," per the epic's own framing
(`code/specs/task-app-icon-assets-v1.md`'s "the one real gap" section).

Added `"ring-percent-value": percent` to the emitted props (a plain
`u64`, 0..100) and to `REQUIRED_PROPS`. `TaskApp.mil` gained the
matching `slot ring-percent-value : number ;`. `ring-gradient`/
`ring-percent` are unchanged — the web host still computes its own CSS
`conic-gradient(...)` from this same number, appropriate for its own
platform.

Native *rendering* of the ring from this number is a deliberately
separate follow-up (filed as its own issue) — this change only closes
the data leak.
