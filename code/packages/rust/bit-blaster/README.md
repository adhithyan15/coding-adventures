# `bit-blaster`

From machine integers to certified SAT proofs: **and-inverter graphs**,
**Tseitin encoding** and a **fixed-width bit-blaster** for SMT-LIB `QF_BV`.
**VER10, part 1** of the verified shipping program
([`code/specs/VER10-bit-blasting.md`](../../../specs/VER10-bit-blasting.md)).

## Why it exists

VER00 §7.6 proves claims about integer code ("for every pair of 32-bit
inputs, the implementation equals the specification") by asking a SAT
solver for a counterexample. The solver speaks Boolean clauses, so the
integers have to become clauses, exactly. This crate holds the circuits
that do it:

```text
  constraint-engine BV tactic  /  VER18 netlists  /  VER19 WIR
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

SMT-LIB `QF_BV` problems do not come here directly. They go through the
existing LANG24 stack: `smt-lib-format` → `constraint-vm` →
`constraint-engine`, whose BV tactic (`constraint-engine/src/bv.rs`) lowers
predicates onto these circuits.

## What is inside

| Module | Contents |
|---|---|
| `aig` | `Aig` and `Edge` (`2·node + complement`); `and` with constant folding and structural hashing; `or`/`xor`/`mux`; 64-pattern bit-parallel `simulate` |
| `tseitin` | Incremental Tseitin encoding of an edge's cone into a `Solver` or a `Cnf`. Stack-based, so deep circuits cannot overflow |
| `bv` | `BitVec` plus `not and or xor add sub neg mul`, `udiv urem sdiv srem` (restoring division, SMT-LIB division-by-zero), shifts by constants and by bit-vectors (barrel shifter), `eq`, unsigned and signed comparisons, `ite`, `extract concat zext sext`, all with exact SMT-LIB semantics |

## Usage

Prove an identity for **all** inputs by refuting a miter, then certify it:

```rust
use bit_blaster::{aig::Aig, bv::{self, BitVec}, tseitin::Tseitin};
use cdcl_sat::{Cnf, Solver, SolveResult};

let mut g = Aig::new();
let (a, b) = (BitVec::input(&mut g, 32), BitVec::input(&mut g, 32));
let ab = bv::add(&mut g, &a, &b);
let ba = bv::add(&mut g, &b, &a);
let same = bv::eq(&mut g, &ab, &ba);

let mut cnf = Cnf::new(0);
Tseitin::new().assert_true(&g, !same, &mut cnf);       // "they differ somewhere"
let mut s = Solver::from_cnf_with_proof(&cnf);
assert_eq!(s.solve(), SolveResult::Unsat);              // nowhere: all 2^64 pairs
// s.proof() → drat_check::check(...) certifies it.
```

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
- **Faithfulness through the solver** (`tests/faithfulness.rs`, VER10 §9):
  every operator is encoded to CNF, its inputs fixed as units and solved
  with `cdcl-sat`; the model's output bits must equal an independent Rust
  reference, and adding "the output differs" must be UNSAT (certified by
  `drat-check` on every assignment at widths 1–3 and a stride beyond).
  Widths 1–4 and 5–6 are exhaustive, 7–8 use corner values plus samples.
  Every expression tree of depth 2 (43 906 terms at widths 2 and 3, about
  3.5 million assignments) is checked by the `--ignored` test; a 1-in-25
  slice runs by default. Identities are proved with certified proofs and
  non-identities refuted with verified counterexamples.
- **End to end:** the QF_BV corpus and the random problems run through
  `constraint-vm`, in `constraint-vm/tests/qfbv.rs`.

No external solver is a dependency or was needed as an oracle. Line
coverage is 100% (measured with `cargo llvm-cov` on a standalone copy).

## Capabilities

None. It is pure computation. See `required_capabilities.json`.
