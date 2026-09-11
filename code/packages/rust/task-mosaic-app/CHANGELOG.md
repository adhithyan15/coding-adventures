# Changelog — task-mosaic-app

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
