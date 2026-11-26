# vos-model-sat

The host model-SAT lane of the verified-OS program (backlog VOS-040a and VOS-040b, spec
[VOS31](../../../specs/VOS31-host-model-sat.md)): the **encoder** (040a) and the **solve, DRAT
certificate, control, premise witness, collapse rule and committed table** (040b) with the
`gen` and `check` commands. The BUILD/`vos-link` wiring of a contract crate (`sat = "model"`,
`CK11`) is VOS-040c.

## What it is

The `sched` contract (`code/packages/rust/vos-sched-contract`) states what a scheduler means as a
Warrant model, `warrant/contract.part`, and ten lemmas about it, `warrant/lemmas.part`. An
exhaustive evaluation of a lemma stops at N = 8. This crate turns the model and each lemma into
an and-inverter graph (`bit-blaster`) at **any N from 1 to 64**, so a later PR can hand the
negated lemma to a SAT solver at N = 8, 16, 32 and 64.

```text
contract.part ──(hand transcription)──> SchedModel / Encoder      src/model.rs
mutants/*.part ─────────────────────> Mutant, one per lemma      src/mutants.rs
lemmas.part ────────────────────────> lemma_circuit(id, model)  src/lemmas.rs
```

## Where it sits in the stack

```text
vos-sched-contract  (the model and lemmas: Warrant text)        read, never edited
warrant-grammar     (the evaluator)                             read, never edited
bit-blaster         (AIG, bit-vectors)                          read, never edited
vos-link            (the contract loader, evaluation document)  read, never edited
        |
        v
vos-model-sat       this crate: encoder + mutants + cross-checks
        |
        v   (040b) cdcl-sat, drat-check, sha256: solve, certify, table (src/lane.rs)
```

**VOS31 section 3 is a hard constraint.** The capacity recording of VOS30 records the tree ids of
46 inputs (`meta::TREE_ENTRIES` in `vos-capacity-probe`). This crate depends on some of them and
edits none.

## The trust problem and the three cross-checks

The encoder is hand-written (there is no Warrant-to-QF_BV translator; VOS-D072). Three checks
turn the transcription into evidence, and each has a negative control that shows it can fail:

| check | compares | domain |
|---|---|---|
| (a) lemma | encoder `holds` against the Warrant evaluator, on the base document and on each of the ten mutant documents; the failing **sets** must be equal and the counts must equal `warrant/lemmas.table` | every case, N = 8 |
| (b) function | each model function's circuit against the evaluator's value | every case, N = 8 (the two relations on a documented 4-candidate slice) |
| (c) width | the encoder against a plain Rust reference (`src/reference.rs`), and the reference against the evaluator | every case, N = 1 to 10; reference vs evaluator at N = 8 |

Only (c) relates two widths, so a result at N = 16, 32 or 64 is evidence about the encoder
*family*, not proof about `contract.part`. The labels (`src/labels.rs`, VOS31 section 11) say so.

## Usage

```text
vos-model-sat cross-check <contract-crate-dir>            the three cross-checks alone
vos-model-sat gen   <contract-crate-dir> [--root R]        writes warrant/lemmas.sat
vos-model-sat check <contract-crate-dir> [--root R] [--emit-drat DIR]
```

The crate path must be `<root>/code/packages/rust/<crate>` with no `..`; run from inside the
contract crate: `cd code/packages/rust/vos-sched-contract && cargo run -q --release
--manifest-path ../../../programs/rust/vos-model-sat/Cargo.toml -- check .`

`check` regenerates in memory, runs every gate and compares the table byte for byte; it never
reads a status from the table. Exit 0 pass, 1 findings, 2 no verdict.

| finding | meaning |
|---|---|
| MS01 | inputs: `vos.toml` lacks `sat = "model"`, a symlink or oversize file, more than 64 lemmas, N out of bounds |
| MS02 | the encoder disagrees with the evaluator or the reference (VOS31 section 6) |
| MS03 | a lemma is not UNSAT (the encoder has a counterexample) |
| MS04 | the DRAT proof is rejected under `drat_check::Limits::default()` |
| MS05 | over the route budget of VOS30 8.6.1 (50,000 conflicts, `check_work` 2^31) |
| MS06 | the control (the lemma's mutant) is not SAT, or its model does not replay |
| MS07 | an implication lemma's premise is not witnessed (the parser, not the encoder, says which are implications) |
| MS08 | a collapse that does not re-derive (constant-fold from the AIG, propagation-only by the lane's own propagator) |
| MS09 | `warrant/lemmas.sat` is absent or stale (first differing line shown) |
| MS10 | a lemma of `lemmas.part` has no row, or none ran |

`--allow-missing-sat-key` (gen and check) skips the `vos.toml` half of MS01. Until VOS-040c teaches
`vos-link` the `sat` key, its loader refuses a `vos.toml` that carries one, so the real contract
cannot opt in yet.

**Committed fields** of `lemmas.sat` are solver-independent (sizes, CNF digest, verdicts, collapse
reason, control, premise). **Gated, never committed**: conflicts, `check_work`, checker acceptance;
they are printed in the report. DRAT files are written only with `--emit-drat`.

Library use of the encoder:

```rust
use bit_blaster::aig::Aig;
use vos_model_sat::{lemma_circuit, Encoder, LemmaId};

let mut aig = Aig::new();
let c = lemma_circuit(LemmaId::TopIsTheMaximum, &Encoder::new(64).unwrap(), &mut aig);
// `c.holds` is true exactly when the lemma holds; assert `!c.holds` to ask a solver for a
// counterexample.
```

Exit codes of `cross-check`: 0 every check clean, 1 a disagreement, 2 no verdict.

```rust
use bit_blaster::aig::Aig;
use vos_model_sat::{lemma_circuit, Encoder, LemmaId};

let mut aig = Aig::new();
let c = lemma_circuit(LemmaId::TopIsTheMaximum, &Encoder::new(64).unwrap(), &mut aig);
// `c.holds` is true exactly when the lemma holds; assert `!c.holds` to ask a solver for a
// counterexample.
```

## Running the tests

The cross-checks interpret the Warrant evaluator over millions of cases, so run them in release:
`cargo test --release` (about 2 minutes: the evaluator and the solves).

## Not here

The `vos-link` `sat` key, the generated BUILD line and `CK11` (VOS-040c) and the Rust-lowering
spike (VOS-040d). This crate claims nothing about any implementation: results are labelled
"lemma", never "proved" or "verified", and a label saying "SAT-certified" is produced only after a
DRAT-checked UNSAT with a passing control.
