# Changelog — task-mosaic-app

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
