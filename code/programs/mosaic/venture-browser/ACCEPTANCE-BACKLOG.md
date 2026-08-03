# Venture browser acceptance backlog

This list tracks bounded follow-ups found while exercising Venture as Mosaic's
cross-platform proving application. Items are ordered by risk and dependency.

## Prioritized discoveries

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
