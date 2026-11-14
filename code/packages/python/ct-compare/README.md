# python/ct-compare

Constant-time byte equality, byte selection, and unsigned 64-bit equality helpers.

## Accepted inputs

Each byte input must be exactly `bytes`, `bytearray`, a byte-format
`memoryview`, or a `list` or `tuple` of `int` values from 0 to 255 (`bool` is
not accepted). Anything else, including a subclass of those types, raises
`TypeError` or `ValueError` instead of being coerced: coercion once let two
different inputs compare equal, and a subclass could misreport its bytes.

`choice` in `ct_select_bytes` must be a real `bool`.

## Warrant evidence lane

`code/specs/ct-compare.warrant` is compiled by the Rust compiler into a
language-neutral corpus (`code/specs/fixtures/ct-compare-warrant-v1`). This
package runs that corpus through `evidence.bindings` (which export implements
which Warrant operation, and which exception is the declared
`CT_SELECT_LENGTH_MISMATCH`), using the shared
[`warrant-corpus`](../warrant-corpus) runner:

```sh
python tests/warrant_evidence.py run --out evidence/test-manifest.json   # regenerate
python tests/warrant_evidence.py check evidence/test-manifest.json       # CI drift check
python tests/warrant_evidence.py sabotage                                # prove it can fail
```

`evidence/test-manifest.json` is written by a real run, never by hand:
12 examples; 9 exhaustive suites (572,109 cases over the finite domains
`Bytes[347] x Bytes[347]`, `Bytes<0|1|2|4|16>`, `U64[377] x U64[377]`, each
checked against SHA-256 digests computed by the Rust compiler); and 8 negative
controls that break one operation at a time and must be caught. "Exhaustive"
means exhaustive over those domains only: `evidence/domain-labels.json` splits the total into 65,537
`full` cases (`Bytes<0>`, `Bytes<1>`) and 506,572 `bounded-set` cases, and until backlog **LP-012**
lands the suite-level "exhaustive" total conflates the two. The constant-time property itself is
**not** established by this lane (VER04i section 11): it is a statement about
CPython's execution, which the corpus does not measure.
