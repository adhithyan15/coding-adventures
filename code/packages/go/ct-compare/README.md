# go/ct-compare

Constant-time byte equality, byte selection, and unsigned 64-bit equality helpers.

## Bit-returning equality

The selection bit is an integer in this language, because a boolean cannot
become one without a branch. To feed an equality result into selection, use
the bit-returning variants, which return exactly 1 or 0 with no conditional:

```
CTSelectBytes(a, b, CTEqBit(x, y))
```

`CTEqFixedBit` and `CTEqU64Bit` are the fixed-width and 64-bit forms. Never convert the
boolean `CTEq` result into a bit with an `if`.

## Warrant evidence lane

`code/specs/ct-compare.warrant` is compiled by the Rust compiler into a
language-neutral corpus (`code/specs/fixtures/ct-compare-warrant-v1`). This
package runs that corpus through `evidence.bindings` (which export implements
which Warrant operation, and which sentinel is the declared
`CT_SELECT_LENGTH_MISMATCH`), using the shared
[`warrant-corpus`](../warrant-corpus) runner:

```sh
go run ./cmd/warrant-evidence run --out evidence/test-manifest.json   # regenerate
go run ./cmd/warrant-evidence check evidence/test-manifest.json       # CI drift check
go run ./cmd/warrant-evidence sabotage                                # prove it can fail
```

`evidence/test-manifest.json` is written by a real run, never by hand: 12
examples; 9 exhaustive suites (572,109 cases over the finite domains
`Bytes[347] x Bytes[347]`, `Bytes<0|1|2|4|16>`, `U64[377] x U64[377]`, each
checked against SHA-256 digests computed by the Rust compiler); and 8 negative
controls that break one operation at a time and must be caught. The rows, counts
and digests equal the Python and TypeScript lanes'. "Exhaustive" means
exhaustive over those domains only: `evidence/domain-labels.json` splits the
total into 65,537 `full` cases (`Bytes<0>`, `Bytes<1>`) and 506,572
`bounded-set` cases, and until backlog **LP-012** lands the suite-level
"exhaustive" total conflates the two. `U64` crosses the binding as a native
`uint64`. The constant-time property itself is **not** established by this lane
(VER04i section 11): it is a statement about the compiled code's timing, which
the corpus does not measure.

`cmd/warrant-evidence` holds the two adapters (the `int` selection bit and the
panic-to-sentinel translation). They are part of the digested source and are the
one piece of the binding that is code rather than data, so they are what a
reviewer should read.
