# Changelog

All notable changes to `mosaic-emit-compose` are documented here.
This project follows [Semantic Versioning](https://semver.org/).

## [Unreleased]

### Added

- `Path` drawing primitive lowering (#12028 item 3, UI39). `Path [name]
  (kind: circle|line|curve, ...)` now lowers to real Compose vector
  geometry instead of reporting `primitive.path-unimplemented` on
  every build. `circle` reuses `Modifier.background(color,
  CircleShape)` + `Modifier.border(width, color, CircleShape)` —
  `CircleShape` plus the already-unconditionally-imported
  `.background`/`.border` modifiers match `background`/`border-color`+
  `border-width` 1:1, the same reuse Qt's `Rectangle` and Flutter's
  `BoxDecoration(shape: BoxShape.circle)` lowerings made for the same
  shape. `line`/`curve` lower to a `Canvas` drawing a Compose
  `graphics.Path` built via `moveTo`/`lineTo`/`quadraticBezierTo`,
  using absolute canvas coordinates (mirrors Qt's `ShapePath` and
  Flutter's `CustomPaint`). `arc` is a stretch goal not implemented in
  this PR; it hard-errors with a named "not yet supported" message,
  matching the XAML/Qt/Flutter lowerings' posture for the same gap.

  Positioning `circle`'s authored center is notably simpler than the
  Flutter lowering: Compose's `Modifier.offset(x, y)` shifts a
  composable's painted position relative to wherever normal layout
  would place it and is legal on ANY composable regardless of parent
  type — unlike Flutter's `Positioned`, which only type-checks (and
  only avoids a runtime panic) as a direct `Stack` child. So `circle`
  always emits `Modifier.offset(...)` unconditionally, no
  `direct_stack_child`-equivalent threading needed.

  Import gating is split in two: `CircleShape` fires whenever any
  `Path` is present, while `Canvas`/`graphics.Path`/`drawscope.Stroke`
  fire only when a `line`/`curve`/`arc` kind is actually present (new
  `tree_needs_path_canvas`, mirroring Qt's `tree_needs_shapes_import`)
  — so a circle-only tree (the common case, the crescent moon) doesn't
  pay for the Canvas imports it never uses.

  Coordinate props (`cx`/`cy`/`r`/`x1`/`y1`/`x2`/`y2`) accept a literal
  `Number` only; a `SlotRef`/`Expr`-bound coordinate is a clear compile
  error, not a silent 0 — full data-driven binding is future work, the
  same not-yet-landed gap the XAML, Qt, and Flutter lowerings all note.

  Verified against the real toolchain: a `mosaic-compile pkg --backend
  compose --profile native-complete` build of the crescent-moon shape
  (two overlapping circles, a line, and a curve) produces
  `nativeComplete: true` with zero degradations. The generated Kotlin
  was dropped into a real JetBrains Compose Multiplatform Desktop
  Gradle project (`org.jetbrains.compose` 1.6.11) and compiled cleanly
  via `gradle compileKotlin`, then launched via `gradle run` and stayed
  running with no crash or exception output.

- Native radio-group mutual exclusion (#13007). `group:` was never read
  anywhere in `emit_host_radio`. A container physically holding 2+
  `HostRadio` siblings sharing a literal `group:` value now gets
  `Modifier.selectableGroup()` on its own modifier chain (new
  `container_needs_radio_group_semantics` + `host_radio_literal_group_key`,
  wired into both `emit_container` and the root-splitting
  `emit_container_frame` path) — purely additive a11y semantics; each
  `RadioButton`'s own `selected`/`onClick` stays entirely local to its
  own `checked`/`onSelect` props, unchanged. The
  `androidx.compose.foundation.selection.selectableGroup` import is
  added conditionally via a new whole-tree `layout_has_radio_group`
  walk. New `pub fn radio_groups_with_native_semantics` lets
  `mosaic-package-artifact-builder`'s degradation analyzer stop
  reporting `property.radio-group-ignored` wherever this lowering
  actually applies. Verified against a real regenerated
  `mosaic-pkg-deck-options` project (the real multi-radio usage this
  targets): `gradle compileKotlin` — `BUILD SUCCESSFUL`.

- Native indeterminate checkbox state (#13006). `emit_host_checkbox` had
  no code path for `indeterminate:` at all. When authored as anything
  other than a literal `Keyword("false")`, the emitter now swaps the
  plain `Checkbox` for Compose's own `TriStateCheckbox(state:
  ToggleableState, onClick: () -> Unit)`, with `state` computed from
  `indeterminate`/`checked` (`_mosaicTruthy`-wrapped for `slot:`/`Expr`
  values, matching `bool_prop_expr`'s existing convention) and the
  `TriStateCheckbox.material` and `androidx.compose.ui.state.ToggleableState`
  imports added conditionally (new `layout_has_checkbox_indeterminate`
  walk). `TriStateCheckbox.onClick` takes no argument — unlike
  `Checkbox.onCheckedChange`'s `checked` lambda parameter — so the
  dispatched "new checked" value is computed inline from the same
  `ToggleableState` expression used for `state =`: clicking always
  resolves *out of* Indeterminate, toggling towards `On` unless already
  `On`. New `pub fn host_checkbox_has_native_semantics` lets
  `mosaic-package-artifact-builder`'s degradation analyzer stop
  reporting `property.checkbox-indeterminate-ignored` for Compose
  wherever this lowering actually applies. Verified against a real
  regenerated `mosaic-pkg-toolkit` project: `gradle compileKotlin` —
  `BUILD SUCCESSFUL` — on the whole Compose Desktop package, including
  the real `Checkbox.kt` this change touches.

### Security

- Validate `HostLink.href`'s URI scheme, literal and slot-bound (#13052).
  Follow-up to #12038 (the identical XAML gap). A literal href is now
  rejected at compile time when it carries an explicit, disallowed scheme
  (new `host_link_href_expr` + `has_disallowed_uri_scheme`, reusing the
  existing `UnsupportedHostLink` error variant). A slot-bound href — unknown
  until runtime — is validated inside the shared `_mosaicHostLink`
  composable via a new `_mosaicIsSafeUri` helper: when the scheme is
  disallowed, the link degrades to the same inert `Clickable` shape the
  `external == false` branch already uses (neither `onActivate` nor
  `uriHandler.openUri` fires), matching the "no navigation target" outcome
  XAML's `SafeNavigateUri` fix settled on for a null `Uri`. A relative
  reference with no scheme at all (`"#"`, a route path) is unaffected in
  both paths, since a relative href never reaches `uriHandler.openUri` as
  an external target regardless (only the `external == true` branch is
  gated).
- Two rounds of security review caught two real gaps in the scheme
  detection, both fixed before merge: a leading space or embedded
  tab/CR/LF made the first-character-alphabetic check fail and
  misclassified the string as "no scheme, therefore safe" -- but a real
  consumer strips that whitespace before parsing the scheme, so it's
  really the dangerous scheme it looks like. The first fix trimmed
  leading/trailing whitespace via Kotlin's `trim()`, but a second review
  round found `trim()`'s default `isWhitespace`-based predicate doesn't
  cover the full C0-control range a real consumer strips (control bytes
  like 0x01/0x1B bypassed it) -- `_mosaicIsSafeUri` now uses
  `raw.trim { it.code <= 0x20 }`, matching the Rust-side check exactly.

### Fixed

- MIL slots with authored defaults now emit non-null Kotlin parameters with
  matching default arguments, so reusable package components can consume their
  own defaulted text, number, and boolean values without nullable type errors.
- Preserve literal `HostInput` values and read-only state, and render its
  placeholder through `BasicTextField`'s native decoration slot.

### Added

- `HostSlider` now maps literal and slot-backed `a11y-label` values to the
  native slider semantics node without replacing its adjustable range role.
- Slot-bound or expression-backed slider steps now derive Compose's discrete
  interior-stop count at runtime, including continuous behavior when step is
  non-positive.
- `HostSlider` now lowers to Compose Material's native adjustable `Slider`,
  including controlled numeric values, range and discrete-step mapping,
  disabled state, continuous `onChange`, and release-time `onCommit` events.
  Numeric values convert at the Float-based Compose boundary and return to
  Mosaic's portable number payload as Double. CI compiles a native-complete
  slider package through the generated Compose project shell.
- `Text` now lowers literal or slot-backed accessible names, heading roles,
  and intentional hiding through Compose semantics. Replacement labels clear
  the built-in text semantics so assistive technology does not announce both
  the visible content and its authored accessible name.

- Canonical dynamic `HostTable`/UI31 Grid layouts now expose Compose's native
  collection semantics: total row/column counts on the table, heading metadata
  on header cells, and stable row/column coordinates on every body cell.
  Unsupported table shapes keep their visual fallback and remain explicit
  native-complete degradations.
- `HostDraggable` and `HostDropTarget` now lower to Compose Desktop's native
  drag source/target modifiers. Generated components add an instance-scoped
  target registry, kind filtering, disabled-state enforcement, pointer
  before/into/after hit testing, focus and Space/Enter/arrow/Escape operation,
  RTL-aware horizontal navigation, live-region state, and shared event payload
  construction for pointer and keyboard drops.
- `Icon` now lowers through a dependency-free native font-glyph vocabulary,
  including runtime glyph and accessibility-label slots, MSL color/size/test
  tags, and a visible fallback. The semantic `spinner` glyph becomes Compose's
  indeterminate `CircularProgressIndicator` with a default "Loading"
  description, allowing all 23 toolkit components to emit on this backend.
- `HostDialog` now lowers modal content to Compose's native `Dialog` and the
  contract's non-modal form to `Popup`. Generated overlays honor controlled
  visibility and interactive-dismiss policy, dispatch open/close events, render
  a semantic heading, preserve nested Mosaic content and styles, and provide
  useful Material surface chrome without application-owned dialog glue.
- `HostLink` now lowers to Compose's native annotated-text link API. External
  links open through the platform `UriHandler`; internal links retain link
  semantics while dispatching Mosaic events, including item/index payloads
  inside `For`. Generated links receive theme-aware visible styling and need no
  application-owned URL adapter.
- `Stack` now lowers to Compose's `Box` — the layering container it already
  uses for the `Box` primitive itself, since Compose's `Box` natively stacks
  its children. Found while wiring `task-app`'s icon assets (progress ring,
  crescent moon, bridge-arc brand mark — see `task-app-icon-assets-v1.md`),
  the first place a Mosaic component used `Stack` and hit this backend's
  build. Not yet lowered: a child's static `position: absolute` + `top`/
  `left` into `Modifier.offset(...)` — v1's existing "anything else
  silently skipped" posture for static props means a Stack's children all
  render at the Box's origin today rather than the pixel positions the
  web/Flutter backends place them at.
- `HostTooltip` now lowers to Compose Foundation's cross-platform
  `BasicTooltipBox`, including native overlay placement and dismissal, Material
  surface chrome, literal/slot/expression text, and assistive-technology
  semantics. This restores package-expanded TaskApp generation after its richer
  Gantt introduced per-row tooltips.
- `HostSurface ( content: slot: ... )` now accepts an
  `@Composable () -> Unit` node slot and invokes it at the shared native
  composition boundary.
- Generated Kotlin event classes now expose `mosaicName`, `mosaicPayload`, and
  `mosaicEnvelope`, giving Compose hosts the same target-neutral event map used
  by the HTML, Electron, SwiftUI, XAML, Qt, and Flutter shells.

### Fixed

- Text expressions that index a collection with an enclosing Mosaic `For`
  index now use Compose's internal Kotlin `Int` shadow. This keeps numeric loop
  comparisons type-correct while allowing toolkit patterns such as
  `bodies[i]` to compile as `Text(String)`.
- Mosaic text, number, collection, and nullable values now lower through a
  generated Kotlin truthiness helper anywhere Compose requires a Boolean,
  including `If`, state styles, checked/selected controls, disabled controls,
  and read-only inputs. Package-expanded TaskApp output now passes the Kotlin
  compiler instead of comparing dynamically typed values with `true`.
- `HostInput.onCommit` now supplies the controlled input value when the MIL
  event declares one payload parameter, while preserving data-object dispatch
  for parameterless commits.
- The legacy `Input ( multiline: true )` spelling now lowers to a native
  multiline `BasicTextField` with a useful editor-sized minimum line count.
  This preserves the multiline capability used by the shared Notes package
  without requiring app-owned Compose code.
- Generated optional boolean slot predicates now compile as nullable-safe Kotlin
  conditions, and large root containers split their direct children into private
  composables so generated Compose Desktop projects avoid JVM method-size
  limits.

## [0.1.0] - 2026-06-02

### Added

- New crate: Jetpack Compose / Compose Multiplatform backend for the Mosaic
  three-language pipeline.
- `from_pipeline(component, layout, style) -> PipelineEmitResult` emits a single
  `.kt` file containing a sealed-class event hierarchy plus a
  `@Composable fun <Component>(...)` function.
- Primitive coverage for v0.1.0:
  - `Box`, `Row`, `Column` -> `Box`, `Row`, `Column` composables
  - `Text` -> `Text(text = ...)`
  - `Spacer` -> `Spacer(modifier = Modifier.weight(1f))`
  - `HostInput` -> `BasicTextField` with strict-Flux dispatch wiring (looks up
    the referenced emit's arity to decide whether to pass `v` to the dispatched
    event)
  - `HostButton` -> `Button(onClick = { dispatch(...) }) { Text(...) }`
- Wired into `mosaic-compile` as `--backend compose`. Bare
  `mosaic-compile --backend compose --interface ... --layout ... --style ... -o
  Component.kt` works end-to-end on macOS arm64.
- 8 unit tests covering empty component, parameterless + payload-carrying emits,
  required + optional slot typing, the full FormulaBar shape end-to-end,
  parameterless button emit, and the unknown-primitive error path.

### Not Yet Implemented

- `Grid` built-in primitive (tracked as `grid-emit-compose` in the autonomous
  loop's roadmap).
- `For`, `If`/`Else` meta-primitives (UI29 sections 3.1 and 3.2).
- `HostTable`, `HostDialog`, `HostCheckbox`, `HostRadio`, `HostLink`,
  `HostNumberInput`, `HostScroll`, `HostTooltip` host primitives return
  `UnknownPrimitive` until their lowerings land.
- `mosstyle` style consumption: `.msl` input is accepted but not yet lowered
  into `Modifier` chains.
- `--emit-project` Android / Compose-Desktop app shell (analogous to the
  `mosaic-emit-flutter` and `mosaic-emit-xaml` project shells).
