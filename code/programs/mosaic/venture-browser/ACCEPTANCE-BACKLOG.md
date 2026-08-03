# Venture browser acceptance backlog

This list tracks bounded follow-ups found while exercising Venture as Mosaic's
cross-platform proving application. Items are ordered by risk and dependency.

## Prioritized discoveries

- [x] **P0 architecture — shared native host controller.** Move Mosaic event
  reduction, status/chrome synchronization, scrolling, scrollbar projection,
  link activation, and hover state into one host-neutral Rust controller used
  by both the SwiftUI/Metal and WinUI/Direct2D adapters.
- [ ] **P1 — live page bridges for the remaining generated hosts.** Replace the
  recording-only content hosts in Qt, Flutter, and Compose acceptance with
  adapters backed by the shared Venture session and page renderer, starting
  with Qt on Linux and reusing the same controller rather than introducing
  backend-specific browser behavior.
  - [x] Qt bridge foundation: generated Qt shells load the shared Rust session,
    mount a Cairo-backed `QQuickPaintedItem`, and directly launch against a
    deterministic live page before reporting render acceptance.
  - [x] Qt live interaction promotion: drive the generated address and history
    controls plus native scroll/link input through the real bridge, replacing
    the recording host as the authoritative Qt interaction gate.
  - [ ] Flutter live page bridge and direct acceptance.
  - [ ] Compose Desktop live page bridge and direct acceptance.
- [ ] **P2 — Qt native-control style compatibility.** The macOS-native Qt
  Quick Controls style rejects custom `background` items emitted from Mosaic
  MSL. Define a native-compatible palette/appearance lowering (or an explicit
  per-project style contract) so generated Qt shells do not warn and silently
  drop authored button backgrounds while retaining native controls.
- [x] **P0 regression — POSIX entry-point shell compatibility.** Keep `BUILD`
  compatible with the repository build tool's `/bin/sh` executor while it
  delegates the backend matrix to the Bash-specific implementation script.
- [x] **P0 — Web Component runtime output encoding.** Encode host-controlled text
  and attribute interpolation, reject executable dynamic link schemes, and
  constrain runtime CSS widths before adding a browser interaction gate.
- [x] **P1 — HTML and Web Component interaction acceptance.** Drive the generated
  browser controls through their real DOM and Custom Element host seams,
  covering disabled controls, address editing, Return, Go, and host-driven prop
  refresh. Keep both outputs sourced from the shared Venture MIL/MLL/MSL
  package.
- [x] **P2 — CI entry-point coverage.** Route the authoritative POSIX and
  Windows package build entry points through the shared generated-shell matrix
  so backend interaction acceptance cannot be bypassed by package-level CI.
- [x] **P3 — Primary native direct-launch coverage.** Run the generated SwiftUI
  and WinUI application launch-and-interaction tests from that shared matrix,
  rather than stopping after their projects compile.

## Completed foundations

- Native SwiftUI and XAML generated-app launch and interaction acceptance.
- Direct Flutter, Qt Quick, Compose Desktop, React, and Electron interaction
  acceptance for the shared browser-chrome contract.
