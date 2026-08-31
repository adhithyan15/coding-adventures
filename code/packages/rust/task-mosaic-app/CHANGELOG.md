# Changelog — task-mosaic-app

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
