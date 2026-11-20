# data-eval-v1

The oracle's committed proof of **`ir.data` evaluation** (VER04r 16.14 [D47] to [D54], R27 to R39, 16.17 R67, 16.19 R94,
Appendix A.2, section 8.2; slice S11c-1).

`data-forms-v1` (slice S11a) pins the *lowering* of every form of Appendix A.2 and cannot say what the forms compute.
`make_data_eval.py` generates one synthetic document, `data_eval_v1.warrant`, that evaluates records, variants and `Option`
over their enumerated domains, writes data values in examples and binds the two faults that need `ir.data`, and six dumps of
what the oracle computes from it. The Rust generator's slice S11c-2 must reproduce **every** file here byte for byte.

## Files

| File | What it is |
|---|---|
| `make_data_eval.py` | writes (or `--check`s) the eight generated files below; `BUILD` runs `--check` (about 25 s) and `dump-lowering --check` |
| `data_eval_v1.warrant` | the generated document: 9 records and 4 variants (nested records, a record with an `Option` and a tuple field, a `U16` field, a record with a `(Bytes,Bool)` field, a payload-free variant, a variant whose payload is an `Option<U16>`), 60 models, 60 operations, 68 claims, 23 examples |
| `data_eval_v1.profile` | the `corpus.profile`: `budget 1048576`, `chunk_size 16384`, and 12 `fault` lines (below) |
| `data_eval_v1.oracle-lowering.json` | `lowering_dump`, schema `warrant-lowering-parity-v1`, with the corpus `types` rows (R59) |
| `data_eval_v1.oracle-evaluation.json` | schema `warrant-evaluation-parity-v1` under the gate this slice opens: per suite the levels, the stream header, the resolved descriptors with the `demoted` paths, `cases`, `discarded`, `expected_calls`, `wraps`, `ir_cost`, every chunk line and digest, the 23 examples (JSON and TFV) and `cases.tfv`; 68 suites, 1,839,544 cases, 169 chunk lines |
| `data_eval_v1.oracle-faults.json` | schema `warrant-fault-parity-v1`: the 12 fault rows (section 8.2's JSON) and the 12 controls under two gates, `controls_shipped` with `gate_shipped` (`ir.core`, `ir.fixed`, `ir.tuple`: the gate **before** this slice, where every `ir.data` fault is `deferred`) and `controls_gate_on` with `gate_on` (those and `ir.data`: the witness search runs) |
| `data_eval_v1.oracle-values.json` | schema `warrant-data-values-parity-v1` (`eval_dumps.data_values_view`): 28 values (`Option`, records, variants, `Result` of them) each in JSON, in the stream (hex) and in TFV, and 23 JSON forms a consumer must reject, with the reader's message |
| `data_eval_v1.oracle-domains.json` | schema `warrant-data-domain-parity-v1` (`eval_dumps.data_domain_view`): 12 descriptors with their sizes and values, 6 budget demotions with their path text, 5 descriptors a consumer must accept and 11 it must reject |
| `data_eval_v1.oracle-refusals.json` | schema `warrant-data-faults-refusals-v1`: 16 signatures a data fault refuses (code `generation`, message `fault: ...`) and 9 fault rows whose type R67 and R94 compare with the declared one (5 accepted, 4 refused) |

`.gitattributes` (`* -text`) keeps the bytes exact on every platform.

## What the dumps pin

* **Evaluation.** The four tags and `match` with the four data patterns are evaluated; a record is the tuple of its fields, a
  variant `(constructor, fields)`, an `Option` `none` or `some v`; equality is structural. 52 reference claims check one model
  each (`op_<m>(args) == m(args)`), 8 more are data inline in the pure side, a subject call inside a data `match` arm and as its
  scrutinee, a computed argument and a claim that only sees `Err`.
* **A suite on each stream header.** 29 suites keep the **v1** header (every type in the v1 set, `Option<U8>` included: the
  `tri_state` shape and `Result<Option<U8>,E>`, with the v1 digests) and 39 use the **v2** header (any record, variant or tuple).
* **The demotion paths of section 5.3 [D54].** `1` (a binder), `0.0` (field 0 of a record), `0.Done.1.some` (the `Option` that is
  field 1 of `Done`, the spec's own example), and in the domain table `0.some` and `0.ok.0`.
* **The faults of section 8.2**, all 12 live under the open gate: `field-zero` bare (`Point`, `Wide` with a `U16` field) and in an
  `Ok` (`Sum`, the shape of arithmetic's `RippleCarryResult`, `k` = 1), and `missing-error` on an `Option` zero (`["none","U8"]`,
  `tri_state`), an `Option` and a variant taken from the parameter of exactly their type, a record built from a parameter, a
  literal and a literal (`ripple_carry_adder`, `..._with_carry`, `alu`), and the two R67 shapes (a record field and a `Result`
  payload of type `(Bytes,Bool)` with no `Bytes` parameter). `controls_gate_on` shows all three states: witnessed (a chunk),
  `unproven` (a computed argument) and blind (a claim that only sees `Err`).
* **The typing of R67 and R94** (the first comparison of a fault row's type with a declared type), in the refusals dump: a zero
  literal has the type it is the zero *of*, a tuple is invariant, and a fault expression needs a **common type** with the declared
  one, not "flows to". These rows are oracle-only: the generator's evaluator is untyped (section 16.9 item 3).

## The contract with the generator

Slice S11c-2 (`rust/warrant-corpus-gen`) must reproduce every file above byte for byte, refuse each signature of
`oracle-refusals.json` with the code `fault`, and open its own `ir.data` gate (`generate` enabled). The dumps carry no generator
name or version. Regenerate with `python3 make_data_eval.py`; never edit a file by hand.

## Differences from the Rust harness, recorded as [D50] and [D37] record them

`missing-error` is **not value-identical** to the harness: it returns the first element of the payload's domain on a contained
panic (`Ok(0)` for `Result<U8,E>`, where this template gives `Ok(a)`; `RippleCarryResult(b"", 0, false)`, where this template
passes the first operand through). Both turn every `Err` into an `Ok` and leave every `Ok` alone, so the claims that catch one
catch the other. The adopters' own fixture READMEs repeat this.

## The adopters' derive cost (measured here, for the two adopters' PRs)

The oracle derives both adopters whole from their committed specs (`code/specs/logic-gates.warrant`,
`code/specs/arithmetic.warrant`), with a `corpus.profile` written from their `evidence.bindings` (51 and 13 `fault` lines):

| Adopter | suites | cases | derive (one core) | largest suite | largest `ir_cost` | profile needs |
|---|---|---|---|---|---|---|
| logic-gates | 88 | 3,770,123 | about 60 s (60.1 to 61.6 s measured) | 1,048,576 cases | 2,372 (`demux_routes_data`) | `max_ir_cost` above 2,000 (4,096 proposed), one `omit_example` (`decoder_at_the_cap_has_65536_lines`, `not-op-eq-pure`) |
| arithmetic | 63 | 2,655,437 | about 36 s | 284,258 cases | **1,524** (`alu_add_4_bit_exhaustive`) | **no `max_ir_cost` line**: 1,524 is under the default of 2,000 |

No control of either adopter is all-blind, so no `accept_blind` line is needed (logic-gates: 162 `unproven`, 32 witnessed and 6 blind
candidates; arithmetic: 105, 2 and 4). Neither adopter is blocked on anything outside S11c-1.
