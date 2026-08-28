# Venture browser acceptance backlog

This list tracks bounded follow-ups found while exercising Venture as Mosaic's
cross-platform proving application. Items are ordered by risk and dependency.

## Prioritized discoveries

- [x] **P0 browser convergence — reusable visited-link state and decoration.**
  Canonicalize document URL identity in `browser-navigation`, commit only
  successful final response URLs in `BrowserSession`, and project blue/purple
  underlined link styling through Layout IR and backend-neutral paint across
  navigation, history, reload, failure, and reflow.
- [x] **P0 browser convergence — reusable durable bookmarks.** Define a
  storage-neutral canonical catalog and transactional repository, implement a
  bounded versioned native-profile file adapter with atomic replacement, route
  one shared Mosaic bookmark command through every host, and cover rollback,
  restart, generated DOM, and direct SwiftUI toolbar behavior.
- [x] **P1 browser convergence — host-neutral View Source.** Project the
  already-retained response source into a synthetic preformatted browser page
  through a reusable core command before adding toolkit-specific windows or
  menus. Completed with escaped synthetic `<pre>` documents, a typed auxiliary
  window effect, shared generated chrome, native host forwarding, and live
  Flutter/Compose plus DOM/Qt acceptance without navigation or refetch.
- [x] **P1 browser convergence — deterministic real-page visuals.** Ratchet
  representative Mosaic-era pages with screenshot and geometry fixtures for
  mixed inline content, preformatted text, images, wrapped links, and scrolling.
  Completed with a reusable page/resource router, deterministic layout and
  structural screenshot oracle, PNG diagnostics, package-contract coverage,
  and production Cairo, Metal, and Direct2D adapter sweeps. The sweep also
  closed decoded-image rendering in `paint-metal`.
- [x] **P2 paint convergence — fully ordered Metal image composition.** Promote
  decoded images from the isolated post-readback compositor into ordered Metal
  draw commands. Completed by moving Metal onto the shared GPU command plan,
  adding a host-owned URI resolver, texture-backed CoreText ordering, gradient
  textures, nested scissor clips, and real-page acceptance while preserving
  affine, opacity, scaling, and source-over behavior.
- [x] **P2 paint convergence — isolated GPU layers.** Extend the shared GPU
  command plan with explicit offscreen layer boundaries, then implement ordered
  filter chains and non-normal blend modes in Metal without flattening layer
  opacity into child draws. Reuse the command contract in WGPU and future GPU
  backends instead of introducing backend-specific scene traversal. Completed
  with balanced shared layer commands, capability profiles, validated filters,
  Metal ping-pong render/compute surfaces, all shared blend modes, native error
  diagnostics, and a reusable pixel oracle.
- [ ] **P2 paint convergence — portable isolated GPU executor.** Execute the
  shared layer commands in WGPU with offscreen render/compute passes and the
  existing cross-backend pixel oracle. Preserve the current explicit rejection
  until opacity, filter ordering, and destination-aware blends all converge;
  then reuse that executor shape for Vulkan, OpenGL, and Mesa profiles.
- [x] **P2 browser convergence — international inline content.** Expand the
  representative real-page corpus through bidi text, script fallback,
  grapheme-aware selection geometry, and UAX #14 line-breaking without moving
  language behavior into platform shells. Completed with the reusable
  `text-flow` analyzer, shared layout/measurement/paint integration, UTF-8
  grapheme selection spans, uniform bidi shaping runs, preserved native font
  fallback, and a deterministic international Venture page.
- [ ] **P2 text convergence — generated Unicode conformance tables.** Extend
  the browser-oriented text-flow profile with generated UAX #9/#14/#29 data,
  isolate/embedding controls, the full line-break pair table, and dictionary
  segmentation for Thai, Lao, and Khmer. Keep table generation independent of
  layout and preserve the current analyzer API for all consumers.
- [x] **P0 CI regression — required gate event isolation.** Keep the protected
  `CI gate` context exclusive to pull-request workflows. Branch and main push
  workflows publish `CI push gate` so a fast push build cannot auto-complete a
  PR while required macOS or Windows acceptance is still running.
- [x] **P0 architecture — shared native host controller.** Move Mosaic event
  reduction, status/chrome synchronization, scrolling, scrollbar projection,
  link activation, and hover state into one host-neutral Rust controller used
  by both the SwiftUI/Metal and WinUI/Direct2D adapters.
- [x] **P1 — live page bridges for the remaining generated hosts.** Replace the
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
  - [x] Flutter live page bridge and direct acceptance.
  - [x] Compose Desktop live page bridge and direct acceptance.
- [x] **P2 architecture — backend-neutral Cairo bridge ownership.** Qt,
  Flutter, and Compose reuse one controller, renderer, and C ABI implementation
  owned by `venture-browser-cairo`; their stable backend-named libraries and
  symbols remain thin compatibility surfaces over that session.
- [x] **P2 — Qt native-control style compatibility.** Generated Qt shells select
  the customization-capable Basic Quick Controls style unless the host explicitly
  sets `QT_QUICK_CONTROLS_STYLE`, so Mosaic MSL backgrounds render without the
  macOS-native style's warnings or silently dropped paint.
- [ ] **P3 — Qt Basic-style font fallback diagnostic.** Remove the one-time
  macOS `Sans Serif` alias-population warning without baking a platform-specific
  font family into generated QML or overriding an explicit host font policy.
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
