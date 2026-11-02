# VER10 — Encodings, part 1: and-inverter graphs, Tseitin and bit-blasting

**Status:** Accepted. Implemented by `code/packages/rust/bit-blaster`.

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

## 5. QF_BV through `smt-lib-format`

`bit_blaster::smt` takes a `constraint_instructions::Program`, as read by
`smt-lib-format`, and decides it:

- **Declarations:** `Bool` and `(_ BitVec w)` variables. Any other sort is
  rejected with a reason.
- **Literals:** `#b0101` and `#x1f` (read by `smt-lib-format` as symbols
  beginning with `#`), and `(_ bvN w)` (read as an application of `_`).
- **Operators:** every operation in §4 under its SMT-LIB name (`bvadd`,
  `bvult`, …), plus `=`, `distinct`, `not`, `and`, `or`, `=>`, `ite` and
  `true`/`false`.
- **Commands:** `set-logic`, `set-option`, `declare-const` / `declare-fun`
  with no parameters, `assert`, `check-sat` and `get-model`. Each
  `check-sat` gives an answer. `push`, `pop` and `reset` are also handled,
  by re-encoding from the assertions in scope.
- **Results:** one answer per `check-sat`. `Sat` carries a model, with
  bit-vectors as bit lists readable as unsigned integers up to 128 bits.
  `Unsat` carries the CNF that was solved and its DRAT proof.
- **Limits:** widths up to 512 bits and circuits up to 4 million AIG nodes
  by default (`Limits`). Predicates deeper than `constraint-core`'s
  `MAX_PREDICATE_DEPTH` are refused before lowering.

Anything outside this list (`extract`, `concat`, quantifiers, uninterpreted
functions, other theories) makes the whole program fail with an
`Unsupported` error that names it. It is never a guess.

## 6. Verification

| Test | What it shows |
|---|---|
| Every operation, every width 1–8, **all** operand pairs: AIG simulation equals concrete integer arithmetic | the circuits are right (the exhaustive tier) |
| Same, widths 9–64, random operands | width-generic code has no small-width accident |
| Tseitin: random AIGs, every input assignment; the CNF with inputs fixed is SAT with exactly the simulated output | the encoding is equisatisfiable and its outputs are exact |
| Miters proved UNSAT, with DRAT proofs accepted by `drat-check`: `a − b = a + ¬b + 1`, `a + b = b + a`, `¬(a ∧ b) = ¬a ∨ ¬b`, shift and comparison identities at 8, 16 and 32 bits | the full pipeline to a checked `proved` |
| A differential `QF_BV` corpus (`tests/qfbv/*.smt2`) read through `smt-lib-format`. Each SAT model is re-checked by a concrete evaluator. Each UNSAT answer is checked by brute force where inputs total ≤ 16 bits, and by `drat-check` everywhere | the SMT front end, end to end |
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

None yet.
