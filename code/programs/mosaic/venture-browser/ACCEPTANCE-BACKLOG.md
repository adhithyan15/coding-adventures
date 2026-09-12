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
- [x] **P2 CSS convergence — computed box and flow values.** Extend the same
  computed-style boundary with percentages and `em`/`rem`, `auto`, min/max
  sizing, borders, `box-sizing`, per-side longhands, text alignment and white
  space, and display-aware block/inline flow. Keep value computation reusable
  and independent from layout engines, then add compact cross-host geometry
  and paint cases before broadening into flex or grid layout. Completed with a
  containing-block percentage size in Layout IR, deterministic `em`/`rem`
  computation, horizontal auto-margin distribution, min/max constraints,
  content/border-box sizing, independent side borders, inherited alignment and
  white-space flow, plus shared real-page geometry and paint acceptance.
- [x] **P2 CSS convergence — flex formatting context.** Add a reusable flex
  container/item contract with main/cross-axis sizing, wrapping, gaps,
  alignment, order, and min-content constraints. Keep computed CSS mapping
  independent from the layout algorithm and reuse the same geometry oracle
  before adding grid tracks. Completed with a host-neutral Rust flex engine,
  typed extension mapping and diagnostics, CSS longhand/shorthand computation,
  recursive block/inline dispatch, intrinsic text minimums, reverse and wrapped
  axes, complete content distribution, and one browser fixture shared by the
  native and web host pipelines.
- [x] **P2 CSS convergence — grid formatting context.** Add a reusable grid
  container/item contract with explicit and implicit tracks, named areas,
  sparse and dense auto-placement, spans, gaps, intrinsic/`fr`/`minmax()`
  sizing, order, and two-axis item/content alignment. Completed with a
  host-neutral Rust grid engine, tolerant typed diagnostics, computed CSS
  longhand/shorthand mapping, recursive `grid`/`inline-grid` dispatch, and a
  deterministic geometry-and-paint fixture shared by every Venture host.
- [x] **P2 CSS convergence — positioned formatting and clipping.** Add a
  reusable contract for relative, absolute, fixed, and sticky boxes, computed
  insets, stable z-order, overflow clips, scroll extents, and clip-aware hit
  testing. Completed with host-neutral diagnostics and geometry, paint-group
  projection for fixed/sticky scrolling, and a deterministic fixture shared by
  the Venture browser pipeline and available native/web hosts.
- [x] **P2 CSS convergence — table formatting context.** Add a reusable table
  container/cell contract with anonymous row and cell repair, ordered
  header/body/footer groups, captions, fixed and automatic intrinsic column
  sizing, column hints, row/column spans, separate and collapsed border
  geometry, vertical alignment, and minimum-content overflow. Completed with
  host-neutral diagnostics, computed CSS and HTML attribute mapping, recursive
  shared layout/paint dispatch, and a deterministic fixture used by every
  available Venture host pipeline.
- [x] **P2 CSS convergence — generated content and marker boxes.** Add reusable
  scoped counters, `::before`/`::after` content, HTML/CSS list ordinals,
  inside/outside marker geometry, typed diagnostics, and ordinary Layout IR
  text boxes that flow through shared paint, hit testing, and every available
  host without toolkit-specific behavior.
- [x] **P2 CSS and paint convergence — reusable visual effects.** Add a typed
  affine-transform, origin, opacity, filter, shadow, blend, and isolation
  contract independent from CSS and paint backends; map computed CSS plus
  uniform border radii into shared Layout IR; wrap complete positioned
  subtrees in backend-neutral groups/layers; and compose the same transforms
  into link hit regions. Completed with bounded diagnostics, deterministic
  device-pixel scaling, and one browser fixture shared by available hosts.
- [x] **P2 CSS and paint convergence — layered backgrounds and corners.** Add a
  reusable contract for multiple image and linear/radial gradient layers,
  per-layer position/size/repeat/origin/clip, painting-box geometry, and four
  normalized elliptical corner pairs. Completed with computed CSS mapping,
  backend-neutral gradient/path/image/clip emission, bounded repeat geometry,
  diagnostics, and a deterministic Venture fixture.
- [x] **P2 CSS and paint convergence — rounded clipping and border geometry.**
  Add path clips with conservative bounds for background images and overflow
  descendants, normalized elliptical inner/outer curves, joined per-side
  solid/dashed/dotted/double borders, and clip-aware hit testing. Completed
  through shared layout metadata, paint scenes, deterministic fixtures,
  native Canvas/Cairo/Skia/SVG execution, and explicit GPU degradation
  diagnostics.
- [x] **P2 browser convergence — reusable form controls.** Add shared
  input/button/textarea/select intrinsic sizing, computed appearance and state,
  focus/disabled/checked/value semantics, keyboard and pointer interaction,
  backend-neutral paint and hit regions, and one deterministic fixture used by
  every available Venture host. Completed with dedicated layout/state crates,
  typed control metadata and diagnostics, retained pointer/keyboard reduction,
  native host activation, and cross-format deterministic fixture coverage.
- [x] **P1 browser convergence — form submission and validation.** Build on
  the shared control model with successful-control collection, radio and select
  serialization, constraint validation, submit/reset activation, GET and
  urlencoded POST navigation, history integration, bounded diagnostics, and
  deterministic acceptance shared by every available host. Completed with a
  dedicated host-neutral planner, transactional session requests, HTTP POST
  transport, shared host activation, bounded diagnostics, and deterministic
  core/Cairo fixtures covering validation, reset, serialization, and history.
- [x] **P1 browser convergence — native form editing and feedback.** Promote
  retained controls from append/backspace semantics to selection, caret,
  replacement, composition/IME, password masking, and multiline editing;
  focus the first invalid control and expose reusable validation/accessibility
  metadata. Route the same keyboard, text-input, and focus contract through
  available native/web hosts with deterministic interaction fixtures, without
  moving editing or validation policy into toolkit adapters.
- [x] **P1 browser convergence — editor presentation and clipboard.** Project
  retained caret, selection, composition ranges, and validation feedback into
  backend-neutral paint/accessibility scenes; add pointer drag selection,
  input/textarea viewport scrolling, clipboard cut/copy/paste, and deterministic
  blink timing. Route host clipboard and IME candidate-rectangle capabilities
  through explicit interfaces, preserving password secrecy and avoiding
  toolkit-owned editing state.
- [x] **P1 browser convergence — advanced editing transactions.** Add
  grapheme- and word-aware navigation, undo/redo transactions, clipboard HTML
  flavor negotiation, drag autoscroll, double/triple-click selection, and
  platform accessibility actions while keeping edit history and selection
  policy shared across every host. Completed with generated Unicode 17
  grapheme boundaries, a shared word/click policy, bounded per-control history,
  typed clipboard and accessibility contracts, native semantic-key routing,
  retained reflow acceptance, and deterministic drag/click fixtures.
- [x] **P1 browser convergence — typed input value semantics.** Extend the
  retained control reducer with live `maxlength`, email/URL syntax, numeric
  min/max/step parsing and stepping, type-appropriate selection restrictions,
  and reusable value-state diagnostics. Keep validity, keyboard increment, and
  accessibility value actions shared while native/web hosts only translate
  platform input events. Completed with Unicode-scalar live replacement
  limits, reusable email/URL/numeric diagnostics, aligned and clamped number
  stepping, public number-selection restrictions, shared submission planning,
  accessibility value actions, and retained-session acceptance.
- [x] **P1 browser convergence — advanced choice and range controls.** Extend
  the same reducer with multi-select selection, disabled option semantics,
  checkbox indeterminate state, radio-group arrow navigation, and range input
  min/max/step behavior. Publish reusable choice/range accessibility state and
  actions, preserve successful-control serialization, and keep every native
  and web host on semantic input translation rather than toolkit-owned state.
  Completed with ordered multi-selection, disabled option/optgroup filtering,
  checkbox mixed state, form-scoped wrapping radio navigation, normalized
  range stepping, reusable accessibility projections/actions, successful
  control serialization, and retained-session acceptance over the semantic
  key seams already shared by every generated host.
- [x] **P1 browser convergence — temporal and color value controls.** Add
  shared date, month, week, time, datetime-local, and color parsing,
  normalization, bounds, stepping, accessibility value text/actions, and
  successful-control serialization before any host adds native pickers.
  Completed with canonical Gregorian and ISO-week scalar conversions,
  subsecond local-time normalization, reusable bound/step diagnostics,
  semantic keyboard and accessibility mutation, canonical color values,
  retained-session acceptance, and deterministic successful serialization.
- [x] **P1 browser convergence — file values and multipart submission.** Add
  an opaque host file-selection contract, accept/multiple filtering, reusable
  file-list accessibility state, multipart/form-data planning with deterministic
  boundaries, reset behavior, bounded payload diagnostics, and native/web
  picker adapters without exposing host paths to layout or paint. Completed
  with path-free picker requests/results, normalized accept filtering,
  bounded retained file bytes, reusable accessibility projection,
  deterministic collision-safe multipart boundaries, reset semantics,
  payload diagnostics, and a shared cross-host fixture.
- [x] **P1 browser convergence — image submit coordinates and dirname.** Add
  successful-control expansion for image-button coordinates and `dirname`
  directionality fields, preserving document order and the shared submission
  planner before broadening form-associated custom element support. Completed
  with bounded control-local pointer coordinates, keyboard/accessibility
  `(0, 0)` activation, Unicode-aware live `dir=auto`, inherited direction,
  adjacent successful entries, and retained-session acceptance shared by all
  host event seams.
- [x] **P1 browser convergence — form-associated custom elements.** Add a
  host-neutral element-internals contract for form ownership, submitted values
  and state, validity anchors/messages, disabled propagation, reset/restore
  callbacks, labels, and accessibility projection. Preserve document-order
  successful-control collection and keep custom-element lifecycle policy out
  of generated host toolkits. Completed with explicit attachment and
  reassociation, bounded string/file/entry-list values and restoration state,
  typed lifecycle effects, shared custom validity, label and ARIA projection,
  accessibility value actions, and ordered native/custom serialization.
- [x] **P1 browser convergence — scripted form lifecycle dispatch.** Add
  host-neutral `requestSubmit`, cancelable submit/reset events, interactive and
  scripted validation reporting, and mutable `formdata` event entries. Route
  native and custom controls through one transactional dispatch plan before
  navigation so no generated host owns event ordering or cancellation policy.
  Completed with validated optional submitters, shared check/report/submit
  validation modes, cancelable invalid/submit/reset events, bounded mutable
  string/file form data, delayed request construction, retained lifecycle
  effects, and deterministic core/Mosaic acceptance over existing host seams.
- [x] **P1 browser convergence — form state restoration and autofill.** Add
  shared dirty-value/default-state tracking, autocomplete section and purpose
  grouping, privacy-bounded autofill transactions, input/change event order,
  history restoration, and custom-element restore callbacks. Keep persisted
  state and autofill policy out of generated native/web hosts while preserving
  one document-ordered form-control model.
  Completed with live dirty/default projection, bounded public and credential
  snapshots, section/address/contact/purpose descriptors, privacy-gated typed
  autofill, ordered input/change effects, Back/Forward restoration, and queued
  custom-element callbacks that fire after internals attach.
- [x] **P1 browser convergence — datalist suggestions and picker mediation.**
  Add shared datalist filtering, typed suggestion normalization, keyboard and
  accessibility active-option state, bounded host suggestion queries, picker
  commit/cancel transactions, deterministic fixtures, and consistent behavior
  across available native/web hosts without giving toolkits value policy.
  Completed with parser-resolved option metadata, bounded value/label/text
  matching, shared typed validation and deduplication, keyboard and semantic
  accessibility movement, transactional input/change commits, host JSON
  projections, native ABI seams, and implicit-submit suppression.
- [x] **P1 browser convergence — live output, meter, and progress semantics.**
  Add shared output dependency recalculation, meter optimum/range state,
  determinate and indeterminate progress behavior, form reset integration,
  reusable accessibility projections, deterministic fixtures, and consistent
  native/web rendering without moving value policy into generated hosts.
  Completed with live dependency snapshots and script-owned recalculation,
  form-reset baselines, normalized meter regions, determinate/indeterminate
  progress, bounded diagnostics and host JSON, shared reflow, and native ABI
  projections for every host seam.
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
