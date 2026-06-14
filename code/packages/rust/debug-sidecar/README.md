# debug-sidecar

Source-location companion for the IIR pipeline (LANG13).

`debug-sidecar` provides a compact, JSON-backed sidecar that maps IIR instruction
indices back to their original source locations and tracks variable liveness.
It is the bridge between the compiler and all downstream debugger/insight tools.

## Pipeline position

```text
Compiler (emits IIRInstr)
  │ calls DebugSidecarWriter::record()
  │
  ↓ DebugSidecarWriter::finish() → Vec<u8>   (opaque bytes)
  │
  ├──→ written to .sidecar file alongside .aot binary
  │
Debugger / native-debug-info
  │ calls DebugSidecarReader::new(bytes)
  │
  ├── lookup("fib", 7)            → SourceLocation("fib.tetrad:3:5")
  ├── find_instr("fib.tetrad", 3) → Some(7)
  └── live_variables("fib", 7)   → [Variable { name: "n", … }]
```

## Public API

| Item | Role |
|------|------|
| `DebugSidecarWriter` | Append-only builder. Call `finish()` to serialise to `Vec<u8>`. |
| `DebugSidecarReader` | Query engine: `lookup`, `find_instr`, `live_variables`. |
| `SourceLocation` | Frozen `(file, line, col)` triple with `Display` as `"file:line:col"`. |
| `Variable` | Register binding with name, type hint, and live range. |
| `LineRow` | Raw mapping of `(instr_index, file_id, line, col)` — used by native-debug-info. |

## Quick start

```rust
use debug_sidecar::{DebugSidecarWriter, DebugSidecarReader};

let mut w = DebugSidecarWriter::new();
let fid = w.add_source_file("fibonacci.tetrad", b"");
w.begin_function("fibonacci", 0, 1);
w.declare_variable("fibonacci", 0, "n", "any", 0, 12);
w.record("fibonacci", 0, fid, 3, 5);
w.end_function("fibonacci", 12);

let sidecar = w.finish();

let r = DebugSidecarReader::new(&sidecar).unwrap();
let loc = r.lookup("fibonacci", 0).unwrap();
assert_eq!(loc.to_string(), "fibonacci.tetrad:3:5");
```

## Lookup semantics

`lookup(fn_name, instr_index)` uses a **DWARF-style "last row ≤ N"** bisect
(`partition_point`), so it correctly handles instruction sequences that span
multiple source rows. The result is the source location that was in effect at
the given instruction index.

## Wire format

The sidecar serialises to JSON (via `serde_json`). The schema is internal and
subject to change; treat the `Vec<u8>` as opaque bytes between writer and reader.

## Build

```bash
cargo build -p debug-sidecar
cargo test -p debug-sidecar
```

## Dependencies

| Crate | Use |
|-------|-----|
| `serde` (derive) | Derive `Serialize`/`Deserialize` on all internal structs |
| `serde_json` | JSON wire format |

## Tests

43 tests: 38 unit tests across `types`, `writer`, `reader`, plus 5 doc-tests.
