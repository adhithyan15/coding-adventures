# mosaic-emit-compose

Jetpack Compose / Compose Multiplatform backend for the Mosaic
three-language pipeline.

## What it does

Consumes the three-file IR produced by `mosmodel-compiler`,
`moslayout-compiler`, and `mosstyle-compiler` and emits a single
`.kt` source file containing:

1. A `sealed class <Component>Event` with one `data object` per
   parameterless emit and one `data class` per emit with a payload. Each event
   exposes `mosaicName`, `mosaicPayload`, and `mosaicEnvelope` so hosts can
   forward the same target-neutral event map as the other Mosaic backends.
2. A `@Composable fun <Component>(...)` function whose parameters
   mirror the component's slots plus a trailing
   `dispatch: (<Component>Event) -> Unit` closure
3. The composable body walks the moslayout tree, emitting one
   Composable call per node

Mosaic conditions have value truthiness rather than Kotlin's strict Boolean
typing. Generated files include a private `_mosaicTruthy` conversion for
conditionals, state styles, native control flags, and read-only/disabled state.
Native text inputs dispatch either parameterless commit events or a declared
single payload containing the current controlled value.

The same output ships unmodified to:

- **Android** (Jetpack Compose for Android)
- **Desktop** (Compose for Desktop on macOS / Linux / Windows)
- **iOS** (Compose Multiplatform for iOS — EAP)
- **Web** (Compose Multiplatform for Web — Wasm target)

That's the whole point: one `.kt` file, every Compose-supported
platform, no per-target codegen fork.

## Integration

Wired into `mosaic-compile` as `--backend compose`.  Run:

```
mosaic-compile --backend compose \
  --interface  Component.mil \
  --layout     Component.desktop.mll \
  --style      Component.dark.msl \
  -o           Component.kt
```

Drop the resulting `.kt` into a project that already has
`androidx.compose.runtime.Composable` on its classpath — Jetpack
Compose for Android, Compose for Desktop, or Compose Multiplatform.
Generated `HostTooltip` output requires Compose Foundation 1.11 or newer so it
can use the common `BasicTooltipBox` API on every Compose target.
The runtime layer (`mosaic-flux-compose`) is a separate package and
not required for the codegen output; it's only needed if the host
wants the strict-Flux store/dispatcher contract.

## Primitive coverage in v0.1.0

| mosmodel tag | Compose lowering                          |
|--------------|-------------------------------------------|
| Box          | `Box { ... }`                             |
| Stack        | `Box { ... }` (Compose's `Box` already layers children; static `position`/`top`/`left` on a child aren't lowered to `Modifier.offset` yet — see the code comment at the match arm) |
| Row          | `Row { ... }`                             |
| Column       | `Column { ... }`                          |
| Text         | `Text(text = ...)`                        |
| Icon         | native font glyph / progress indicator    |
| Spacer       | `Spacer(modifier = Modifier.weight(1f))`  |
| HostInput    | `BasicTextField(value, onValueChange...)` |
| Input        | multiline-capable `BasicTextField`        |
| HostButton   | `Button(onClick) { Text(label) }`         |
| HostLink     | native annotated text link                |
| HostDialog   | native `Dialog` / non-modal `Popup`       |

The emitter also lowers `For`, `If`/`Else`, table structure, buttons, checkbox
and radio controls, number inputs, links, accessible dialog and tooltip
overlays, and the current drag/drop degradation path. Unsupported primitives
still return a clear `UnknownPrimitive` error instead of silently disappearing.

## Style handling

MSL part styles lower into native `Modifier` chains and inherited Compose text
styles, including state-dependent backgrounds, borders, dimensions, spacing,
alignment, colors, fonts, and test tags. Hosts can still wrap the result in
their own `MaterialTheme` for platform-level theming.

## Tests

`cargo test -p mosaic-emit-compose` runs 51 focused emitter tests. The
package-expanded TaskApp is also exercised as a real Compose Desktop project:
Kotlin compilation, native macOS distribution packaging, and process launch.
