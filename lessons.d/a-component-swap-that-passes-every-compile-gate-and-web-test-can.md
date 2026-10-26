---
category: Mosaic compiler pipeline
---

# A component swap that passes every compile, gate and web test can still push a native app's rows below a fold nothing scrolls

Replacing Journal's timeline `RecordList` rows with `mosaic-pkg-card` Cards
(Mosaic backlog item 4) passed every check in the package:

- `cargo test` (`package_compiles` and the native-complete gate);
- the web Vitest suite;
- `swift build` on the SwiftUI emit;
- `flutter analyze`.

It failed only in Compose's `JournalUiTest`, which CI runs under xvfb and
which is easy to skip locally. A Card row was about 1.8× as tall as a
RecordList row (about 220px against 124px in the declared 1100×760 window),
and nothing in Journal's pane scrolls on any native backend. So the second
entry existed in the semantics tree but was not displayed. The web never
shows this, because a browser page scrolls by default (TaskApp's #14734 is
the same trap).

**Do this instead.** Before calling a layout change done:

1. Emit the Compose project (`mosaic-compile pkg ... --backend compose
   --emit-project --runtime-library <dylib>`), then run its conformance
   `*UiTest` with `MOSAIC_APP_STATE_PATH` set. This works on macOS without
   xvfb.
2. Render `*Screenshots` with `MOSAIC_SHOT_DIR`, and look at them next to a
   render of `origin/main`.

A height change in a non-scrolling native pane is a reachability change, not
just a cosmetic one.
