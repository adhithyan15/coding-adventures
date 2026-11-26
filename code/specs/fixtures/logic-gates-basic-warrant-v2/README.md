# logic-gates-basic-warrant-v2

The committed `warrant-corpus-v2` corpus of `code/specs/logic-gates-basic.warrant` (module
`logic_gates_basic`): the eleven scalar gates only, the cross-lane contract of
`code/specs/LG02-basic-gates-cross-lane-contract.md` (LGE-S1c). No lane reads this directory yet;
the checks below are its only consumers.

| File | Written by | What it is |
|---|---|---|
| `corpus.json` | `warrant-corpus-gen` | the canonical corpus: types, ops, models, 26 suites, 39 examples, 31 faults and controls, digests |
| `chunks/<claim>.txt` | `warrant-corpus-gen` | one file per suite (one chunk each) |
| `cases.tfv` | `warrant-corpus-gen` | the document's 39 `example` rows as a TFV file |
| `corpus.profile` | **a person** | the 31 `fault` lines (LG02 5.1); the only hand-written file here |
| `.gitattributes` | a person | `* -text`: every file is hashed or compared byte for byte |

26 suites, 590,350 cases (the 9 two-input `*_rejects_non_bit` suites have 65,532 each, the rest are
tiny). Default knobs, so the profile holds only fault lines. Generator `warrant-corpus-gen` 0.6.0.
The label is "exhaustive over U8"; wider integers are not in the corpus (LG02 5.2, 9).

## Controls

31 faults: `missing-error`, `saturate-add`, `saturate-sub` on the nine two-input gates and
`missing-error`, `shift-mask` on `not_gate` and `nand_not`: exactly the basic-op `faults=` of
`code/packages/rust/logic-gates/evidence.bindings`. The profile is generated **without
`--bindings`** (Rust binds the full document); `code/scripts/logic_gates_basic_drift.py` checks the
profile against the bindings (check 10), the BUILD pins against `required_claims` (11) and the
document digest (12).

Every control is `unproven` (the claims pass computed arguments, `b2u(a)`). **Two controls name a
blind claim**: `not_gate/shift-mask` and `nand_not/shift-mask` list `*_rejects_non_bit` in
`blind_claims`. The generator's stand-in replaces the subject by the claim's expected value, a
constant `Err`, so the pre-rewrite `p mod 8` cannot reach it; in the Rust harness it is the only
claim that kills the fault (`p mod 8` is the identity on 0 and 1, so `*_truth_table` and
`nand_not_equals_not`, the `required_claims`, do not fail). The Rust BUILD pins the rejection claims,
and the guard accepts exactly those two pins (`KNOWN_STAND_IN_BLIND`). A lane that replays
`shift-mask` literally will see its required claims pass: a stand-in limitation to resolve before
lanes bind (LG02 errata). Accepted by the owner; tracked as **LGE-S1d** (a real witness for rejection
claims in the generator, then `KNOWN_STAND_IN_BLIND` is removed), which must land before any lane
binds `shift-mask`.

## Regenerate

From the repository root. Never edit `corpus.json`, `cases.tfv` or `chunks/` by hand.

```bash
cargo run --release --manifest-path code/packages/rust/Cargo.toml -p coding_adventures_warrant_corpus_gen --bin warrant-corpus-gen -- \
  --warrant code/specs/logic-gates-basic.warrant \
  --profile code/specs/fixtures/logic-gates-basic-warrant-v2/corpus.profile \
  --out code/specs/fixtures/logic-gates-basic-warrant-v2
```

## What checks it

* `rust/warrant-corpus-gen` BUILD: the same command with `--check`.
* `python/warrant-oracle` BUILD: `verify-v2 ... --expect-generator warrant-corpus-gen@0.6.0` (590,350 cases).
* `rust/logic-gates` BUILD: `code/scripts/logic_gates_basic_drift.py` (checks 10 to 12).
* The build tool selects all three for an edit here (`warrantAdopters` row, no bindings).
