# typescript/ct-compare

Constant-time byte equality, byte selection, and unsigned 64-bit equality helpers.

## Accepted inputs

Each byte input must be a genuine `Uint8Array` (Node's `Buffer` included), a
genuine `ArrayBuffer`, or a plain array of integers from 0 to 255. Anything
else throws a `TypeError` or `RangeError` instead of being coerced: coercion
once let two different inputs compare equal. A typed array is recognised by
its real internal slots and copied; look-alike objects and proxies over typed
arrays are rejected. A `Proxy` or subclass of a plain `Array` is accepted, but
its `length` and each element are read exactly once and validated.

`choice` in `ctSelectBytes` must be a real boolean.

**Pass a `Buffer`, not its `.buffer`.** A small `Buffer` lives inside Node's
shared allocation pool, so `buf.buffer` is the whole pool, not your bytes.

## Warrant evidence lane

`code/specs/ct-compare.warrant` is compiled by the Rust compiler into a
language-neutral corpus (`code/specs/fixtures/ct-compare-warrant-v1`). This
package runs that corpus through `evidence.bindings` (which export implements
which Warrant operation, and which thrown class is the declared
`CT_SELECT_LENGTH_MISMATCH`), using the shared
[`warrant-corpus`](../warrant-corpus) runner:

```sh
node --experimental-strip-types tests/warrant-evidence.ts run --out evidence/test-manifest.json   # regenerate
node --experimental-strip-types tests/warrant-evidence.ts check evidence/test-manifest.json       # CI drift check
node --experimental-strip-types tests/warrant-evidence.ts sabotage                                # prove it can fail
```

`evidence/test-manifest.json` is written by a real run, never by hand: 12
examples; 9 exhaustive suites (572,109 cases over the finite domains
`Bytes[347] x Bytes[347]`, `Bytes<0|1|2|4|16>`, `U64[377] x U64[377]`, each
checked against SHA-256 digests computed by the Rust compiler); and 8 negative
controls that break one operation at a time and must be caught. The rows,
counts and digests equal the Python lane's. "Exhaustive" means exhaustive over
those domains only: `evidence/domain-labels.json` splits the total into 65,537
`full` cases (`Bytes<0>`, `Bytes<1>`) and 506,572 `bounded-set` cases, and until
backlog **LP-012** lands the suite-level "exhaustive" total conflates the two.
`U64` crosses the binding as a `bigint` (a `number` cannot hold `2^64 - 1`).
The constant-time property itself is **not** established by this lane (VER04i
section 11): it is a statement about the JavaScript engine's execution, which
the corpus does not measure.

One limit worth knowing: `ctSelectBytes` throws a plain `RangeError` for the
length mismatch and also for an out-of-range byte, so the exact-class binding
cannot tell them apart. The corpus only passes `Uint8Array`s, so that cannot
happen in the run; a dedicated error class would close it (backlog, portable
Warrant queue).
