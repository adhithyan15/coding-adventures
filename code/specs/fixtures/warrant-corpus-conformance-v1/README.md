# warrant-corpus-conformance-v1

The shared conformance cases every Warrant corpus lane runner must pass
(VER04i section 9.1, backlog WD-100). One JSON file per category; `index.json`
lists them with case counts and SHA-256.

| File | Cases | What a lane runs |
|---|---|---|
| `json.json` | strict reader + canonical writer | `input_hex` bytes through the reader then the renderer |
| `tagged.json` | tagged-value decoder + stream encoder | `input_hex` through reader, decoder, encoder |
| `containment.json` | containment helper | `root` and `child` as text, with the `flavor`'s path rules |
| `label-digest.json` | manifest digest | `manifest` object to its SHA-256 |

Seven more categories, the **v2 kit** (VER04r section 11, slices S5b, S5d and S10d), sit beside them in
the same layout: `v2-value.json` (177 cases), `v2-arith.json` (129), `v2-ir.json` (717),
`v2-domain.json` (219), `v2-chunk.json` (97), `v2-fault.json` (113) and `v2-binding.json` (176), 1,628 cases (S5b committed
734 in six files; S5e added 48; S5d added the 176 of `v2-binding`; S10d added 670 at `ir.fixed` and `ir.tuple`), 2,089 with the
four v1 categories. Every
case carries `id`, `summary`, `requires` and `expect`; a lane runs a case exactly when it
implements the levels in `requires`. The levels
`ir.data` and `ir.list` add vectors in S11 (S10d added those of `ir.fixed` and `ir.tuple`). A runner
that does not know a `v2-*` category digest-checks its file and does not run it.

| File | What a lane runs |
|---|---|
| `v2-value.json` | `type` (and `types`) text, `input_hex` through the strict reader, the section 2.3 decoder and the section 2.4 stream |
| `v2-arith.json` | one node over `lit` operands, at the case's `type` |
| `v2-ir.json` | an IR tree with `levels`, `env`, `models`, `ops` and `subjects` (the pure stand-in per subject op); value, `calls` or an error code |
| `v2-domain.json` | a descriptor and a binder type, enumerated by `index` or `range` |
| `v2-chunk.json` | a self-contained corpus, suite row and `chunk_file_hex`; the chunk's validation or its error |
| `v2-fault.json` | a fault row instantiated over an op row, the args and the result |
| `v2-binding.json` | `binding_hex` (the binding file's bytes), the case's `corpus` (`types`, `ops`) and `host` (four closed lists); the normalized binding, or one `reason` of the closed set of 39 (VER04r section 16.19 R85 (d)) |

Where each mandatory vector is discharged (VER04r section 11, R17); "S5b" means `ir.core` kit data:

| Mandatory vector | Discharged by |
|---|---|
| Arithmetic: the ten cases of section 3.4 | S5b `v2-arith` |
| Values: signed, `Bytes<N>`, decimal text, an `Err` case not in `E`, the depth-17 type | S5b `v2-value` |
| Values: tuple values (stream tag `64`) | S10d `v2-value` (`ir.tuple`; 49 cases) |
| Values: nested `Option`, records, variants, an undeclared constructor | S11d `v2-value` (`ir.data`) |
| IR: short-circuit, nested calls, unknown tag, undeclared level, impure guard | S5b `v2-ir` |
| Domains: every `ir.core` leaf, `fx_edge`, the `result` composite | S5b `v2-domain` |
| Domains: `tuple`, `nat_shift` | S10d `v2-domain` (57 cases; the 12 `nat_shift` cases and the one `nat_0_64` case name `ir.core`, because a `Nat` leaf over a `Nat` binder needs no level) |
| Domains: `variant`, `option` | S11d `v2-domain` |
| Domains and values: lists, the length-2/length-3 boundary | the fixture PR of the reserved level `ir.list` |
| Chunks: zero admitted, first/middle/last of a 2^24 suite, a boundary inside a discarded run | S5b `v2-chunk` |
| Chunks: a v2-stream chunk with a tuple binder | S10d `v2-chunk` (23 cases over `kit-tuple.warrant`) |
| Chunks: alone equals sequential | S6a, S7a (each lane's test, over the S5b cases marked `also_sequential`) |
| Negative: a wrong subject on a `reference` suite; a reordered enumeration | S6, S7 runner tests |
| Negative: `first_admitted` off by one; `slim_chunks` omitting the first or last chunk | S5b `v2-chunk` |
| Negative: `full` claimed after a slim run | S6, S7 (the lane's own `check`); S9 |
| Negative: a shared discriminator; `err_any` on a two-case error type | S5d `v2-binding` (`discriminator-shared`, `err-any-multi`) |
| Aliasing (four fixtures) | S6, S7 runner fixtures, per lane; `v2-binding` carries only the directive-level cases |
| Bounds: the `Nat` overflow in both classes; the doubling `concat` | S5b `v2-ir`; "before allocation" is asserted by each lane's test (S6a, S7a) |
| Bounds: a v0 fixed-width node over a subject result | existing generator and oracle tests (section 3.3 rule 3); not lane data |
| Accounting | S6, S7 (the lane's own `check`); S9 |
| Paths: claim fields (generation error) | existing generator and oracle tests |
| Paths: a symbolic link or an undeclared file in `chunks/` | S6, S7 runner tests |
| Static widths: an op with `width_params` in `v2-ir`, `v2-chunk` and `v2-fault`; the four refused width lists | S5e (VER04r 16.18 R70) |
| Overflow class under an impure guard, with its pure-guard twin, and in a model body entered with a subject argument | S5e `v2-ir` (R71) |
| `errs`: empty, 65 names, a duplicate, a bad name; a list that differs from the declared cases | S5e `v2-domain` and `v2-chunk` (R72) |
| Bytes `lit`: one accepted, two refused | S5e `v2-ir` (R73) |
| A fixed-width `take` or `drop` count | S5e `v2-ir` (R74) |
| Binders: a ground suite with none (accepted); nine, and a product over `2^28` (refused) | S5e `v2-chunk` (R75) |
| `level-missing` by a type; a model of a missing level that is not reached | S5e `v2-ir` (R69) |
| The level of `match`; `len` on a list operand | S10d and S11d `v2-ir` (R76; S10d holds the `Result` and bare-tuple vectors) |
| A model chain of 65 that nothing calls | S5e `v2-ir` (R77) |
| Every `ir.fixed` and `ir.tuple` tag of Appendix A.1, with its edges and refusals; `match` on a `Result` with every pattern | S10d `v2-ir` (469 cases) |
| A stand-in whose body needs `ir.fixed` under an `ir.core` tree | S10d `v2-ir` (R100) |
| A model with an `Int` parameter; an `env` type the reader refuses | S10d `v2-ir` (R111 (d), [D75]; R120 (d)) |
| `off-by-one-consumed` on a `Result<(Bytes,Nat),E>`; `missing-error` with a tuple payload; the `ir.fixed` and `ir.tuple` templates | S10d `v2-fault` (R96, R67; 72 cases) |
| `ill-typed` in `v2-fault`: R96's second vector, the refusals of R98, an op row that names `Int`, an error type or an undeclared type | S10d `v2-fault` ([D74], R110, R121) |
| `unknown-tag`, `depth` and `model-order` in `v2-fault` | S10d `v2-fault` ([D77], R119 (e)) |
| The ops and models of a chunk corpus part, `corpus` | S10d `v2-chunk` (R125) |

The two generation-time refusals of section 3.4 stay pinned by the generator's and the oracle's own tests
(`v2-arith/nat-bound-generation-error` and `v2-arith/v0-u8-wrap-guard` are their lane-observable forms, [D45]).

Input bytes are lowercase hex, never JSON strings (a JSON string cannot carry
invalid UTF-8, a lone surrogate or a BOM). `.gitattributes` (`* -text`) keeps
every byte exact on Windows checkouts.

Do not edit the JSON by hand. The `v2-*.json` files come from the oracle: the case tables
live in `code/packages/python/warrant-oracle/tests/fixtures/conformance-v2/`, and the oracle
derives every outcome (`coding_adventures_warrant_oracle.conformance`). **Regeneration is two
steps**, in this order:

```text
python3 code/packages/python/warrant-oracle/tests/fixtures/conformance-v2/make_conformance.py --write   # the v2-*.json files
python3 code/scripts/warrant_corpus_conformance.py --write                                              # index.json
```

`make_conformance.py --check` requires the committed v2 bytes and the v2 rows of `index.json` to
match its derivation. The v1 script indexes the `v2-*` files on disk (after the four v1 rows, in
the order `v2-value`, `v2-arith`, `v2-ir`, `v2-domain`, `v2-chunk`, `v2-fault`, `v2-binding`),
checks them structurally (every case id of every category is printable ASCII, U+0021 to U+007E; a `v2-binding`
`reason` is one of its 39; the closed set of `expect.reason` of the four v1 categories applies to them only), and never
imports the oracle or a lane. The four v1 category files
come from that script's own case tables and independent reference implementation:

```text
python3 code/scripts/warrant_corpus_conformance.py          # check
python3 code/scripts/warrant_corpus_conformance.py --write  # regenerate
```

Cases are only ever added within `-v1`; changing an expectation means `-v2`.
Containment includes the WD-100b cases (Windows reserved device names refused,
win32 case folding ASCII-only; 141 cases, 461 in the four v1 categories, 2,089 with the v2 kit).
Lanes: Python, TypeScript and Go run all four v1 categories and digest-check the `v2-*` rows (the runners
read v2 in S6 and S7; `v2-binding` is run by S6b and S7b) (Go: `conformance_test.go` in
`code/packages/go/warrant-corpus`).
