# vos-model-sat

The host model-SAT lane of the verified-OS program, **encoder half** (backlog VOS-040a, spec
[VOS31](../../../specs/VOS31-host-model-sat.md)). It does **no solving**: solving, DRAT
certificates, the committed `warrant/lemmas.sat` table and the BUILD/`vos-link` wiring are
VOS-040b and VOS-040c.

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
        v   (VOS-040b) cdcl-sat, drat-check: solve, certify, table
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
cd code/packages/rust/vos-sched-contract
cargo run -q --release --manifest-path ../../../programs/rust/vos-model-sat/Cargo.toml -- cross-check .
```

Exit 0: every check clean. Exit 1: a check found a disagreement. Exit 2: no verdict.

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
`cargo test --release` (about 40 seconds).

## Not here

The solver, DRAT, the committed table, the CLI `gen`/`check`, `CK11` and the Rust-lowering spike
(VOS-040b to VOS-040d). This crate claims nothing about any implementation: results are labelled
"lemma", never "proved" or "verified".
