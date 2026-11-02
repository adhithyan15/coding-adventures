# `bit-blaster`

From machine integers to certified SAT proofs: **and-inverter graphs**,
**Tseitin encoding** and a **fixed-width bit-blaster** for SMT-LIB `QF_BV`.
**VER10, part 1** of the verified shipping program
([`code/specs/VER10-bit-blasting.md`](../../../specs/VER10-bit-blasting.md)).

## Why it exists

VER00 §7.6 proves claims about integer code ("for every pair of 32-bit
inputs, the implementation equals the specification") by asking a SAT
solver for a counterexample. The solver speaks Boolean clauses, so the
integers have to become clauses, exactly. This crate does that:

```text
  QF_BV problem (.smt2, via smt-lib-format)          smt.rs
        │  every w-bit value → w signals
        ▼
  bit-vector circuits: adders, shifters, comparators   bv.rs
        │  built from AND gates and inverters only
        ▼
  and-inverter graph (structurally hashed)            aig.rs
        │  one variable and three clauses per AND gate
        ▼
  CNF ──► cdcl-sat ──► SAT + model  /  UNSAT + DRAT proof ──► drat-check
                                               tseitin.rs
```

## What is inside

| Module | Contents |
|---|---|
| `aig` | `Aig` and `Edge` (`2·node + complement`); `and` with constant folding and structural hashing; `or`/`xor`/`mux`; 64-pattern bit-parallel `simulate` |
| `tseitin` | Incremental Tseitin encoding of an edge's cone into a `Solver` or a `Cnf`. Stack-based, so deep circuits cannot overflow |
| `bv` | `BitVec` plus `not and or xor add sub neg mul`, shifts by constants and by bit-vectors (barrel shifter), `eq`, unsigned and signed comparisons, `ite`, all with exact SMT-LIB semantics |
| `smt` | Decides `smt-lib-format` programs in `QF_BV`: declarations, assertions, `check-sat`, push/pop/reset. Each UNSAT carries its CNF and DRAT proof. Widths and circuit size are bounded |

## Usage

```rust
use bit_blaster::smt::{check_program, Answer};

let program = smt_lib_format::read(
    "(declare-const x (_ BitVec 8))
     (assert (= (bvadd x #x01) (_ bv5 8)))
     (check-sat)",
).unwrap();
match &check_program(&program).unwrap()[0] {
    Answer::Sat(model) => assert_eq!(model["x"].as_u128(), Some(4)),
    Answer::Unsat { .. } => unreachable!(),
}
```

To prove an identity for **all** inputs, build a miter and refute it (see
the crate docs for the full example). For instance, a 16-bit subtraction
miter is refuted and its DRAT proof certified by `drat-check`.

## How it is tested

- **Exhaustive:** all 21 operations at every width from 1 to 8, on every
  pair of operands (up to 65 536 per operation and width). The simulated
  circuit must equal Rust integer arithmetic. Widths 9–64 use random
  operands.
- **Tseitin exactness:** 200 random AIGs, every input assignment. The
  output literal in the solver's model must equal the simulated output.
- **Certified miters** at 8, 16 and 32 bits: subtraction as addition,
  commutativity, De Morgan, XOR cancellation, comparison dualities, and
  shift and multiply identities. Each is proved UNSAT, and `drat-check`
  accepts the proof. A false identity yields a real counterexample.
- **QF_BV corpus** (`tests/qfbv/*.smt2`, 15 files, 23 answers) read
  through `smt-lib-format`. Every SAT model is re-evaluated by an
  independent concrete evaluator. Every UNSAT is certified by `drat-check`,
  and is brute-forced too when its inputs total 16 bits or fewer.
- **Random QF_BV:** 400 problems over three ≤ 4-bit variables, compared
  against brute force.
- **Refusals:** 29 malformed or unsupported inputs are each refused with a
  reason.

Expected answers were derived in-house. No external solver is a dependency
or was needed as an oracle. Line coverage is above 99%.

## Capabilities

None. It is pure computation. See `required_capabilities.json`.
