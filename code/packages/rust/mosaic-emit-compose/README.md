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
The runtime layer (`mosaic-flux-compose`) is a separate package and
not required for the codegen output; it's only needed if the host
wants the strict-Flux store/dispatcher contract.

## Primitive coverage in v0.1.0

| mosmodel tag | Compose lowering                          |
|--------------|-------------------------------------------|
| Box          | `Box { ... }`                             |
| Row          | `Row { ... }`                             |
| Column       | `Column { ... }`                          |
| Text         | `Text(text = ...)`                        |
| Spacer       | `Spacer(modifier = Modifier.weight(1f))`  |
| HostInput    | `BasicTextField(value, onValueChange...)` |
| HostButton   | `Button(onClick) { Text(label) }`         |

`For`, `If`/`Else`, `Grid`, `HostTable`, `HostDialog`,
`HostCheckbox`, `HostRadio`, `HostLink`, `HostNumberInput`,
`HostScroll`, `HostTooltip` are not yet lowered — they return
`UnknownPrimitive`.  Those land in follow-up PRs (the
`grid-emit-compose` cycle is already on the autonomous loop's
roadmap; the other host primitives follow as each is exercised by
a real component).

## Style handling

v0.1.0 accepts the `.msl` input but does not yet consume it —
Compose styling lands on `Modifier` chains (`Modifier.padding(...)`,
`Modifier.background(...)`, etc.) which a future version will
lower.  The result is unstyled Composables; the host application is
expected to wrap them in a `MaterialTheme { ... }` or apply its own
modifiers at the call site for now.

## Tests

`cargo test -p mosaic-emit-compose` — 8 tests covering: empty
component, parameterless / one-param emits, slot typing (required +
optional), the full FormulaBar shape end-to-end, parameterless
button emit, and the unknown-primitive error path.
