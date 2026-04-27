# text-interfaces

Rust implementation of the **TXT00** spec — the pluggable text interfaces
that every shaper / measurer / resolver in the coding-adventures stack
plugs into.

## What's in the crate

Three orthogonal traits plus one convenience function:

| Item              | What it does                                                              |
|-------------------|---------------------------------------------------------------------------|
| `FontMetrics`     | Font-global metrics (ascent, descent, line gap, x-height, cap-height, …) |
| `TextShaper`      | Codepoints → positioned glyph run (cmap, GSUB, GPOS on the full path)    |
| `measure(...)`    | Thin function that wraps a shaper + metrics to return width / bbox       |
| `FontResolver`    | `FontQuery` → concrete backend handle                                     |

Plus the data types the traits operate on: `FontQuery`, `FontWeight`,
`FontStyle`, `FontStretch`, `ShapeOptions`, `Direction`, `FeatureValue`,
`ShapedRun`, `Glyph`, `MeasureResult`, and the error enums
`FontResolutionError` / `ShapingError`.

## Design commitments

- **Orthogonal.** Shapers are pluggable independently of metrics. Both are
  pluggable independently of the measurer (which is a function, not a
  trait — there is only one correct implementation).
- **Generic over the backend's handle type** via the `Handle` associated
  type. The Rust type system enforces the font-binding invariant at
  compile time: you cannot pass a `CTFontRef` to a
  `FontMetrics<Handle = FontFile>` implementation because the types don't
  unify.
- **Infallible getters.** `FontMetrics` methods return concrete values,
  not `Result`s. Only shaping and resolution can fail.

## Usage

A consumer that wants to measure a string picks a matching pair of
implementations (both with the same `Handle` type):

```rust
use text_interfaces::{measure, ShapeOptions, FontQuery};

let resolver = my_backend::Resolver::new();
let metrics  = my_backend::Metrics::new();
let shaper   = my_backend::Shaper::new();

let handle = resolver.resolve(&FontQuery::named("Helvetica"))?;
let result = measure(&shaper, &metrics, "Hello", &handle, 16.0,
                     &ShapeOptions::default())?;
println!("{} px wide", result.width);
```

The first concrete backend shipping for this crate is
`text-native-coretext` (macOS / iOS). Others follow: `text-native-directwrite`
(Windows), `text-native-pango` (Linux), `text-metrics-font-parser` + the
naive / HarfBuzz-equivalent shapers on the device-independent side.

## Spec

See [code/specs/TXT00-text-interfaces.md](../../../specs/TXT00-text-interfaces.md).
