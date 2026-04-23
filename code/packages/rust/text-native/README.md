# text-native

Cross-platform facade over the device-dependent TXT03 backends. Selects
the right implementation at compile time:

| Target                    | Backend                                          |
|---------------------------|--------------------------------------------------|
| `target_vendor = "apple"` | `text-native-coretext` (TXT03a, CoreText)       |
| Windows                   | Not yet implemented (TXT03b, DirectWrite)        |
| Linux / BSD               | Not yet implemented (TXT03c, Pango)              |

Exports: `NativeResolver`, `NativeMetrics`, `NativeShaper`, `NativeHandle`
type aliases plus a re-export of the TXT00 trait vocabulary from
`text-interfaces`.

## Usage

```rust
use text_native::{text_interfaces::FontQuery, NativeMetrics, NativeResolver, NativeShaper};
use text_interfaces::{FontResolver, TextShaper, ShapeOptions};

let resolver = NativeResolver::new();
let metrics  = NativeMetrics::new();
let shaper   = NativeShaper::new();

let handle = resolver.resolve(&FontQuery::named("Helvetica"))?;
let run = shaper.shape("Hello, world!", &handle, 16.0, &ShapeOptions::default())?;
println!("{} glyphs, {} px wide", run.glyphs.len(), run.x_advance_total);
```

Spec: [code/specs/TXT03-native-shapers.md](../../../specs/TXT03-native-shapers.md).

## Non-Apple fallback

On non-Apple platforms the type aliases currently resolve to a
`UnimplementedNativeBackend` stub whose `FontResolver::resolve` returns
`FontResolutionError::LoadFailed`. This lets cross-platform binaries
compile and degrade at runtime — or, preferably, select the
device-independent path (`text-metrics-font-parser` +
`text-shaper-naive` / `text-shaper-harfbuzz`) at build time.
