# Venture browser acceptance backlog

This list tracks bounded follow-ups found while exercising Venture as Mosaic's
cross-platform proving application. Items are ordered by risk and dependency.

## Prioritized discoveries

- [x] **P0 — Web Component runtime output encoding.** Encode host-controlled text
  and attribute interpolation, reject executable dynamic link schemes, and
  constrain runtime CSS widths before adding a browser interaction gate.
- [ ] **P1 — HTML and Web Component interaction acceptance.** Drive the generated
  browser controls through their real DOM and Custom Element host seams,
  covering disabled controls, address editing, Return, Go, and host-driven prop
  refresh. Keep both outputs sourced from the shared Venture MIL/MLL/MSL
  package.

## Completed foundations

- Native SwiftUI and XAML generated-app launch and interaction acceptance.
- Direct Flutter, Qt Quick, Compose Desktop, React, and Electron interaction
  acceptance for the shared browser-chrome contract.
