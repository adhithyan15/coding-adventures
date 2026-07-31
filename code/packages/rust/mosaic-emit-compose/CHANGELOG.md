# Changelog

All notable changes to `mosaic-emit-compose` are documented here.
This project follows [Semantic Versioning](https://semver.org/).

## [Unreleased]

### Added

- `HostSurface ( content: slot: ... )` now accepts an
  `@Composable () -> Unit` node slot and invokes it at the shared native
  composition boundary.
- Generated Kotlin event classes now expose `mosaicName`, `mosaicPayload`, and
  `mosaicEnvelope`, giving Compose hosts the same target-neutral event map used
  by the HTML, Electron, SwiftUI, XAML, Qt, and Flutter shells.

### Fixed

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
