# matrix-ir-json

JSON wire format for `matrix-ir` `Graph` values — a sibling crate to
`matrix-ir` that adds human-readable serialization without violating
matrix-ir's MX00 zero-dependency mandate.

## Why a sibling crate?

`matrix-ir` is the canonical IR for the Rust matrix execution backbone
(ARCH02).  Per the **MX00 zero-dependency mandate**, `matrix-ir` may
depend on nothing outside the Rust standard library — a CI lint
enforces this.  That keeps the IR layer trivially auditable, embeddable
in `no_std`-adjacent contexts, and free of supply-chain risk.

`matrix-ir` already ships a compact, deterministic **binary** wire
format (`Graph::to_bytes` / `Graph::from_bytes`) implemented by hand in
~750 lines of zero-dep code.  But for browser inspection,
cross-language port reference, test-fixture commits, and schema
documentation we also want a **human-readable** JSON variant.

Hand-rolling a JSON encoder/decoder inside `matrix-ir` would either
(a) bloat the crate with ~1000 lines of mostly-orthogonal code, or
(b) duplicate work that already lives in the workspace's
`json-lexer` / `json-parser` / `json-value` / `json-serializer`
crates.  Neither is desirable.

This crate is the resolution: `matrix-ir-json` depends on both
`matrix-ir` and the workspace JSON crates, and exposes a tiny surface
that round-trips losslessly through the *same* `Graph` value as the
binary format.  Choosing binary vs. JSON becomes a deployment
decision, not a data-model decision.

## API

```rust
use matrix_ir::{DType, GraphBuilder, Shape};
use matrix_ir_json::{encode, encode_pretty, decode};

let mut g = GraphBuilder::new();
let x = g.input(DType::F32, Shape::from(&[1, 4]));
let w = g.constant(DType::F32, Shape::from(&[4, 1]), vec![0u8; 16]);
let y = g.matmul(&x, &w);
g.output(&y);
let graph = g.build();

// Encode to compact JSON.
let s: String = encode(&graph);

// Encode to pretty JSON (2-space indent).
let pretty: String = encode_pretty(&graph);

// Decode back into a Graph — round-trips byte-for-byte through the
// binary wire format.
let round_tripped = decode(&s).unwrap();
assert_eq!(graph.to_bytes(), round_tripped.to_bytes());
```

## Schema

The JSON schema is intentionally close to the in-memory `Graph`
layout, with a top-level `matrix_ir_version` integer so older
decoders can fail cleanly on newer payloads.

```jsonc
{
  "matrix_ir_version": 1,
  "tensors": [
    { "id": 0, "dtype": "f32", "shape": [1, 4] },
    { "id": 1, "dtype": "f32", "shape": [4, 1] },
    { "id": 2, "dtype": "f32", "shape": [1, 1] }
  ],
  "inputs":  [0],
  "outputs": [2],
  "ops": [
    { "kind": "MatMul", "lhs": 0, "rhs": 1, "output": 2 }
  ],
  "constants": [
    {
      "tensor_id": 1,
      "dtype": "f32",
      "shape": [4, 1],
      "bytes_hex": "00000000000000000000000000000000"
    }
  ]
}
```

* `matrix_ir_version` must equal `matrix_ir::WIRE_FORMAT_VERSION` (= 1).
* `dtype` strings: `"f32" | "f64" | "i32" | "i64" | "u8" | "u32"`.
* `kind` strings match the `Op` variant name verbatim (`"Add"`,
  `"MatMul"`, `"ReduceSum"`, `"Concat"`, …).
* Constant bytes are encoded as lowercase hex with no separator,
  no `0x` prefix.  Length is always `2 * num_bytes` characters
  (one hex digit per nibble, leading zeros preserved).

See [`specs/ARCH02-rust-native-execution-backbone.md`](../../../specs/ARCH02-rust-native-execution-backbone.md)
for the broader vision: this crate is **Phase 1** (universal wire
format) of the Rust-as-native-backbone plan.

## Round-trip guarantee

The test suite covers, for every op variant and every dtype:

```
graph -> JSON -> graph' -> binary -> graph''
```

and asserts `graph.to_bytes() == graph''.to_bytes()`.  In other
words, binary and JSON are interchangeable representations of the
*same* `Graph` value — you can encode through one and decode through
the other.

## Where this fits

```
┌──────────────────────────────────────────────────────────────┐
│  matrix-ir (zero-dep, MX00-enforced)                         │
│    ├─ Graph, Op, Tensor, DType, Constant                     │
│    └─ Graph::{to_bytes, from_bytes}  ← canonical binary wire │
└──────────────────────────────────────────────────────────────┘
                              │
                              │  (sibling, no upward dep)
                              ▼
┌──────────────────────────────────────────────────────────────┐
│  matrix-ir-json (this crate)                                 │
│    ├─ depends on coding-adventures-json-{value,serializer}   │
│    └─ encode / encode_pretty / decode  ← JSON wire           │
└──────────────────────────────────────────────────────────────┘
```

## License

MIT
