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
- [x] **P2 paint convergence — portable isolated GPU executor.** Execute the
  shared layer commands in WGPU with offscreen render/compute passes and the
  existing cross-backend pixel oracle. Completed with owned texture arenas,
  nested render scopes, ordered filter passes, post-filter opacity, clip-aware
  destination blends, stable shader parameter layouts, and adapter-conditional
  acceptance shared with Metal. Reuse this executor shape for explicit Vulkan,
  OpenGL, and Mesa profiles.
- [x] **P2 browser convergence — international inline content.** Expand the
  representative real-page corpus through bidi text, script fallback,
  grapheme-aware selection geometry, and UAX #14 line-breaking without moving
  language behavior into platform shells. Completed with the reusable
  `text-flow` analyzer, shared layout/measurement/paint integration, UTF-8
  grapheme selection spans, uniform bidi shaping runs, preserved native font
  fallback, and a deterministic international Venture page.
- [x] **P2 text convergence — generated Unicode conformance tables.** Extend
  the browser-oriented text-flow profile with generated UAX #9/#14/#29 data,
  isolate/embedding controls, the full line-break pair table, and dictionary
  segmentation for Thai, Lao, and Khmer. Keep table generation independent of
  layout and preserve the current analyzer API for all consumers. Completed
  with ICU4X's generated Unicode 17 grapheme and full line-break state
  machines, ICU bidi properties feeding the UAX #9 resolver, complex-script
  dictionaries, profile diagnostics, focused conformance tests, and a shared
  international real-page fixture.
- [x] **P2 browser convergence — asynchronous subresource lifecycle.** Replace
  blocking inline-image fetch/decode with a host-neutral request, cancellation,
  completion, and incremental-repaint contract owned by the browser pipeline.
  Preserve retained document/layout state, deterministic ordering, failure
  fallback, navigation cancellation, and one reusable scheduler seam across
  native and web hosts rather than introducing toolkit-specific loaders.
  Completed with document-first page commits, ordered/deduplicated scheduler
  effects, navigation cancellation generations, retained ready/failed image
  state, incremental repaint outcomes, compatibility draining, native bridge
  entry points, and deterministic visual lifecycle acceptance.
- [x] **P2 browser convergence — external stylesheets and computed cascade.**
  Consume the parser's stylesheet plans through the shared subresource
  scheduler, then replace `html-to-layout`'s theme-only visual defaults with a
  reusable author/UA cascade and computed-style boundary. Preserve ordered
  stylesheet blocking, media/failure fallback, navigation cancellation,
  retained-document restyle, and backend-neutral layout/paint integration;
  avoid CSS parsing or property policy in host toolkits. Completed with a
  grammar-validated author/UA style context, specificity and source-order
  resolution, typed shared CSS/image scheduler effects, ordered blocking,
  screen-media filtering, failure fallback, navigation-safe completions,
  retained-page restyle, and a real-page fixture consumed by available hosts.
- [x] **P2 CSS convergence — imported and element-authored cascade.** Extend
  the new computed-style boundary with element `style` declarations,
  attribute/structural selector matching, inherited custom properties,
  shorthand/value resolution, viewport media evaluation, and ordered
  `@import` graph scheduling. Keep parser data generation and fetch/cycle
  policy independent from layout, then ratchet the profile with a compact CSS
  conformance corpus shared by every Venture host. Completed with retained
  style attributes, inline specificity, inherited custom properties and
  `var()` fallback, four-edge shorthands, attribute/first/last/nth-child
  selectors, viewport media, append-only import requests, depth-first cascade,
  ancestor-cycle diagnostics, and shared real-page host acceptance.
- [ ] **P2 CSS convergence — computed box and flow values.** Extend the same
  computed-style boundary with percentages and `em`/`rem`, `auto`, min/max
  sizing, borders, `box-sizing`, per-side longhands, text alignment and white
  space, and display-aware block/inline flow. Keep value computation reusable
  and independent from layout engines, then add compact cross-host geometry
  and paint cases before broadening into flex or grid layout.
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
