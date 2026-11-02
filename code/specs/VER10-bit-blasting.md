# VER10 — Encodings, part 1: and-inverter graphs, Tseitin and bit-blasting

**Status:** Accepted. Implemented by `code/packages/rust/bit-blaster` (circuits) and the BV
tactic in `code/packages/rust/constraint-engine` (`src/bv.rs`), run by `constraint-vm`.

**Layer:** Proof machinery, layer 2 (VER00 §7.5 step 2, §7.6.6 item 3).
This is the first slice of VER10. Equality with uninterpreted functions,
linear integer arithmetic and arrays are later slices.

**Builds on:** VER09 (`cdcl-sat`), VER11 (`drat-check`), LANG24
(`constraint-core`, `constraint-instructions`, `smt-lib-format`).

**Consumed by:** VER18 (hardware equivalence: miters over gate-level
netlists), VER19 (bit-precise proofs over WIR: "Bit-blasting. Fixed-width
integer operations become circuits"), and VER00 §7.6's first pilot target,
`ct-compare`.

---

## 1. Why this exists

A SAT solver understands clauses over Boolean variables. The claims VER00
wants to prove are about **machine integers**: `sum == (a + b) mod 2^n`,
"this comparison does not depend on where the first differing byte is".
Something has to turn 32-bit additions into clauses, exactly. That is
**bit-blasting**: every `n`-bit value becomes `n` Boolean signals, and
every operation becomes the circuit a hardware designer would build for it.

```
   ∃ a b. (a + b) ≠ (b + a)          a claim over 32-bit integers
        │  bit-blast
        ▼
   an and-inverter graph: 64 inputs, ripple-carry adders, a comparator
        │  Tseitin
        ▼
   CNF: one variable per gate, three clauses per AND
        │  cdcl-sat
        ▼
   UNSAT + DRAT proof ──► drat-check ──► proved, for all 2^64 inputs
```

## 2. And-inverter graphs (AIGs)

Every Boolean circuit can be built from two-input AND gates and inverters.
An AIG is exactly that: a DAG whose internal nodes are ANDs, whose edges
may be complemented, and whose leaves are inputs or the constant `false`.

### 2.1 Representation

- Node 0 is the constant `false`. Nodes are numbered in creation order,
  which is a topological order: a node's operands always come first.
- An **edge** is `2·node + complement`, the same packing as a literal in
  VER09. `Edge::FALSE = 0`, `Edge::TRUE = 1`, and `!e` flips the low bit.
- Inputs are leaves, numbered `0..n_inputs` in creation order.

### 2.2 Construction rules for `and(a, b)`

Applied in order, so the graph never holds an obviously redundant gate:

| Case | Result |
|---|---|
| `a` or `b` is `FALSE` | `FALSE` |
| `a` is `TRUE` | `b` (and symmetrically) |
| `a == b` | `a` |
| `a == !b` | `FALSE` |
| an AND of the same two operands exists | that node (**structural hashing**) |
| otherwise | a new node, operands in canonical order (`a ≤ b`) |

`or`, `xor`, `xnor`, `implies` and `mux` are built from `and` and `!`.

### 2.3 Evaluation

`simulate(inputs)` evaluates every node on 64 input patterns at once, one
pattern per bit of a `u64`. Exhaustive tests use it to check millions of
input combinations quickly. It shares no code with the Tseitin encoder or
the solver.

## 3. Tseitin encoding

Tseitin (1968) turns a circuit into an equisatisfiable CNF whose size is
linear in the circuit's size. Every AND node `n = a ∧ b` gets a fresh SAT
variable and three clauses that pin its value:

```
   (¬n ∨ a)       n → a
   (¬n ∨ b)       n → b
   (n ∨ ¬a ∨ ¬b)  a ∧ b → n
```

Inputs get a variable each. The constant node gets a variable and a unit
clause making it false. Only the nodes in the **cone** of the edges being
asserted are encoded. The encoder is incremental: it remembers which nodes
already have variables, so one AIG can be asserted piece by piece into one
solver, as BMC (VER18) needs.

The distributive `to_cnf` of `constraint-core` can be exponential. Tseitin
never is, which is why it replaces `to_cnf` wherever formulas can be large.

## 4. Bit-vectors

A `BitVec` is a vector of edges, least significant bit first. Widths are
arbitrary (≥ 1). Operations follow SMT-LIB `QF_BV` semantics exactly,
including the edge cases:

| Operation | Circuit | Edge cases |
|---|---|---|
| `not`, `and`, `or`, `xor` | bitwise | — |
| `add` | ripple-carry: full adder per bit | wraps mod 2^w |
| `neg`, `sub` | `a + ¬b + 1` (carry-in 1) | wraps |
| `mul` | shift-and-add, low `w` bits | wraps |
| `shl`, `lshr`, `ashr` by a constant | rewiring | amount ≥ w: all zeros (all sign bits for `ashr`) |
| `shl`, `lshr`, `ashr` by a bit-vector | barrel shifter: one mux layer per amount bit | amount ≥ w, from any bit at or above `⌈log₂ w⌉`: as above |
| `eq` | AND of bitwise XNORs | — |
| `ult`, `ule`, `ugt`, `uge` | borrow out of `a − b` | — |
| `slt`, `sle`, `sgt`, `sge` | unsigned comparison with both sign bits flipped | — |
| `ite` | per-bit mux | — |
| constants | edges `TRUE`/`FALSE` per bit | any width |

Both operands of a binary operation must have the same width. A mismatch
is an error, never an implicit extension.

## 5. QF_BV in the existing constraint stack

The repository already has a constraint stack (LANG24). `smt-lib-format`
reads `.smt2` into a `constraint_instructions::Program`. `constraint-vm`
executes it: declarations, assertions, push/pop snapshots, `check-sat`,
models, and its own resource limits. `constraint-engine` picks a tactic for
each `check-sat`. QF_BV joins that stack as a **third tactic** beside SAT
(CDCL, VER09) and LIA (Cooper). It is not a second program interpreter.

```
   .smt2 ─► smt-lib-format ─► Program ─► constraint-vm ─► Engine::check_sat
                                                              │
                         Bool only → SatTactic   Int → LiaTactic   BitVec → BvTactic
                                                                      │
                                       bit-blaster (AIG, Tseitin, bv) ┘
```

`bit-blaster` holds only the circuits, because VER18 (netlists) and VER19
(WIR) build circuits without going through `Predicate`s. The lowering from
`Predicate` to circuit lives in `constraint-engine::bv`.

**Engine dispatch.** The BV tactic handles a check when any declared
variable is a `BitVec`, when any assertion contains bit-vector syntax
(`#b`/`#x` literals, `(_ …)` terms or `bv…` operators), or when the logic
is `QF_BV`. It also handles Bool
variables declared alongside. A `BitVec` mixed with `Int` answers
`Unknown`, because there is no BV–LIA combination yet. `QF_BV` joins the
supported logics, and `BitVec(w)` joins the supported sorts for
`1 ≤ w ≤ 512`.

**Model values.** `constraint_engine::Value` gains `BitVec(Vec<bool>)`,
least significant bit first. It displays as an SMT-LIB `#b…` literal and
reads as an integer with `as_bv_u128`. Every declared variable gets a
value, and bits outside every asserted cone are `false`.

**What the tactic accepts:**

- **Literals:** `#b0101` and `#x1f`, which `smt-lib-format` reads as
  symbols beginning with `#`, and `(_ bvN w)`, read as an application of
  `_`.
- **Operators:** every operation in §4 under its SMT-LIB name (`bvadd`,
  `bvult`, …), plus `=`, `distinct`, `not`, `and`, `or`, `=>`, `iff`,
  `ite` and `true`/`false`.

Anything else answers `Unknown` with a reason that names it: `extract`,
`concat`, division, quantifiers, uninterpreted functions, other theories.
The tactic never guesses.

**Every UNSAT is certified before it is reported.** The tactic solves with
proof logging and runs `drat-check` on the proof. Only an accepted proof
becomes `SolverResult::Unsat`. A rejected one becomes `Unknown`. A solver
bug can therefore surface as `Unknown`, never as a false `Unsat`.

**Declared names may not begin with `#`.** SMT-LIB symbols cannot, and the
tactic reads `#`-names as literals. If a declared `#b1` were allowed, it
would silently mean the constant 1, so it is refused instead.

**Limits:**

- widths up to 512 bits;
- circuits up to 250 000 AIG nodes (about 150 MB once encoded and solved),
  checked after every operation;
- 2 000 000 gate constructions (`Aig::and_calls`), because structural
  hashing makes a repeated subterm free in size but not in time;
- 1 000 000 conflicts and 2³⁰ propagation steps (`Solver::set_work_budget`);
- the kernel's default limits, with a CNF that exceeds them refused before
  solving;
- predicates no deeper than `MAX_PREDICATE_DEPTH`.

On top of these, `constraint-vm`'s own instruction, scope and assertion
limits apply. A declared-but-unused width above the limit allocates
nothing: the engine's trivial-model path skips it.

## 6. Verification

| Test | What it shows |
|---|---|
| Every operation, every width 1–8, **all** operand pairs: AIG simulation equals concrete integer arithmetic | the circuits are right (the exhaustive tier) |
| Same, widths 9–64, random operands | width-generic code has no small-width accident |
| Tseitin: random AIGs, every input assignment; the CNF with inputs fixed is SAT with exactly the simulated output | the encoding is equisatisfiable and its outputs are exact |
| Miters proved UNSAT, with DRAT proofs accepted by `drat-check`: `a − b = a + ¬b + 1`, `a + b = b + a`, `¬(a ∧ b) = ¬a ∨ ¬b`, shift and comparison identities at 8, 16 and 32 bits | the full pipeline to a checked `proved` |
| A differential `QF_BV` corpus (`constraint-vm/tests/qfbv/*.smt2`) read through `smt-lib-format` and executed by `constraint-vm`. Each SAT model is re-checked by a concrete evaluator. Each UNSAT answer has passed `drat-check` inside the tactic, and is also brute-forced where inputs total ≤ 16 bits | the whole stack, end to end |
| Random `QF_BV` terms with ≤ 12 total input bits: solver answer vs brute-force concrete evaluation | the front end and blaster against an independent oracle |

The expected answers in the corpus are derived in-house: from brute force,
or from identities that hold by construction. VER00 §7.5 permits an
external solver as an offline oracle during development. None was needed,
and none is a dependency, a dev-dependency or a CI step.

Line coverage is held at 95% or more.

## 7. Capabilities

Pure computation over in-memory structures. `required_capabilities.json`
lists no capabilities.

## 8. Divergences

1. **The QF_BV front end is a `constraint-engine` tactic, not a
   `bit-blaster` module.** The first draft of this spec gave `bit-blaster`
   its own `check_program`, which re-implemented `constraint-vm`'s program
   walker: scopes, `check-sat` and models. That duplicated the existing
   LANG24 stack, so §5 now plugs into it. A side effect: UNSAT answers no
   longer carry their certificate out of the engine. They are checked by
   `drat-check` inside the tactic instead, before `Unsat` is reported,
   which is a stronger guarantee for every caller of `Engine` and
   `constraint-vm`.
2. **Budgets on every kind of work** (security review). A conflict budget
   alone did not bound time: a 155-byte 512-bit multiplication miter ran
   for more than 300 s. `cdcl-sat` gained a work budget, `bit-blaster` a
   gate-construction counter, and the node cap fell from 4 million to
   250 000.
3. **The LIA tactic is guarded** (security review). Bit-vector literals
   next to `Int` variables reached the LIA tactic, whose evaluator reads
   anything it cannot evaluate as `false`, and answered `Unsat` for true
   formulas. Dispatch now detects bit-vector syntax as well as sorts.
   Separately, every LIA assertion must sort-check and stay inside the
   evaluator's fragment, which also closes the older case of an
   undeclared name.
