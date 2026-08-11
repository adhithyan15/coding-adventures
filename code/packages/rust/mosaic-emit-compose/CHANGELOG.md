# Changelog

All notable changes to `mosaic-emit-compose` are documented here.
This project follows [Semantic Versioning](https://semver.org/).

## [Unreleased]

### Added

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
