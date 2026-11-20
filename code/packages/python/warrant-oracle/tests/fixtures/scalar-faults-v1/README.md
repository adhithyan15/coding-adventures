# scalar-faults-v1

The oracle's committed proof of the scalar faults of slice S11f-1 (VER04r 8.2 and 16.14): `saturate-add`,
`saturate-sub`, `shift-mask` and `missing-error` on a scalar payload (`Bool`, `Nat`, a fixed-width type) or on a
`Bytes` payload with no `Bytes` parameter. None needs `ir.data`; the rows are live (`requires` `["ir.fixed"]`, or
`["ir.core"]` for `missing-error`).

Section 8.2 gives their exact JSON, so the spec, not the dump, is normative: the dump here must reproduce it. The
meaning of each is the Rust harness's (`V1_FAULTS` and `scalar_adapter_v1` in
`rust/warrant-grammar/src/evidence.rs`): a saturating fault replaces the call (the export is never called, so the
`Result` form is `Ok` of the value on every input); `shift-mask` rewrites the last argument to `p % w`, `w` the width
of the first argument's type.

## Files

| File | What it is |
|---|---|
| `make_scalar_faults.py` | writes (or `--check`s) the four files below; `BUILD` runs `--check` (about 15 s) |
| `scalar_faults_v1.warrant` | generated synthetic document: `sat_<t>` for the eight fixed-width types, `satr_<t>` (a `Result` with a third, ignored parameter) for `U8`, `U64`, `I8`, `I64`, shift ops over each signedness and width combination, the scalar `missing-error` shapes (a parameter of the exact type, the zero, a `Bytes<4>` zero literal), `me_u8`'s second claim on its `Err` region (so that control has two required claims), and `sat_u8_tuple_param`, whose tuple parameter adds `ir.tuple` to the row's `requires` (R9) |
| `scalar_faults_v1.profile` | the `corpus.profile`: one `fault <op> <fault>` line per op and fault |
| `scalar_faults_v1.oracle-faults.json` | `parity_dumps.faults_view`: 41 fault rows and the 41 controls under the explicit closed gate and under the open gate (`controls_shipped` and `controls_gate_on`) |
| `scalar_faults_v1.oracle-refusals.json` | every signature each fault refuses (`Nat` parameters, mixed widths or signs, other result types, a non-`Result` for `missing-error`, an `Option` payload, a variant payload, a named-width `Bytes` payload with nothing to take), each with the code `generation` and a message starting `fault:` |

`.gitattributes` (`* -text`) keeps the bytes exact on every platform.

## The refusals schema: `warrant-faults-refusals-v2`

`off-by-one-consumed-v1` has one fault and its refusals dump (`warrant-faults-refusals-v1`) carries a top-level
`fault` string, which the generator's `off_by_one_consumed_parity.rs` reads under that id. This directory has
several faults, so the shape differs and the id is `-v2`:

```json
{"schema": "warrant-faults-refusals-v2",
 "faults": ["saturate-add", "saturate-sub", "shift-mask", "missing-error"],
 "refusals": [{"fault": "...", "name": "...", "oracle_code": "generation", "message": "fault: ...", "source": "..."}]}
```

A row has a `fault` key of its own; there is no top-level `fault`.

## Why a directory of its own

No committed fixture names these faults, so no emitted byte of any existing dump moves. A new row in an older
directory would turn the generator's parity test for it red until its mirror landed.

## The contract with the generator

Slice S11f-2 (`rust/warrant-corpus-gen`) must reproduce `oracle-faults.json` byte for byte (the fault rows and the
controls) and refuse the signatures of `oracle-refusals.json` with the code `fault`. The dumps carry no generator name
or version. Regenerate with `python3 make_scalar_faults.py`; never edit a file by hand.

A record or `Option` payload of `missing-error` needs `ir.data` and is not in this directory (slice S11c-1).
