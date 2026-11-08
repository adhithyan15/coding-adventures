# VER19 — Symbolic Execution of WIR and Bit-Precise Proofs (VER-P024)

**Parent:** [VER00 §7.6.2](VER00-verified-shipping-program.md), [VER25](VER25-wir-core-interpreter.md), [VER25b](VER25b-rust-to-wir-frontend.md), [VER26 §3–§5](VER26-ct-compare-pilot.md).

---

## 1. Purpose & Scope

This specification establishes the Symbolic Execution engine for the Warrant Intermediate Representation (WIR), delivering **VER-P024**.

WIR functions lowered from source languages (including Rust via [VER25b](VER25b-rust-to-wir-frontend.md)) are executed over symbolic values rather than concrete runtime inputs. The engine bit-blasts integer and bitwise operations into And-Inverter Graphs (AIGs) and generates CNF formulas solved by `cdcl-sat`, proving functional correctness and memory-safety invariants across entire input spaces ($2^{64}$ states and beyond) without testing approximations.

The symbolic execution engine delivers:
1. **Bit-Precise Symbolic Arithmetic & Logic:** Exact fixed-width circuit semantics (addition, subtraction, XOR, AND, OR, shift, multiplexers) mapped to `bit-blaster::bv::BitVec`.
2. **Symbolic Memory & Bounds Checking:** Bounded provenance allocations, symbolic load/store evaluation, and verification of memory safety invariants (closing CWE-125 out-of-bounds reads and CWE-787 out-of-bounds writes).
3. **Path Exploration & Feasibility Filtering:** Worklist path traversal collecting symbolic path conditions, pruning provably unsatisfiable branches.
4. **Loop Unrolling & Honest Labelling:** Bounded unrolling up to budget $k$, emitting certified `proved` results for acyclic code and honest `bounded(k)` labels for unrolled loops per VER00 §7.6.2 and §7.6.5.
5. **Miters & Proof Generation:** Automated construction of equivalence miters and invariant verification queries, discharging them via `cdcl-sat` to produce UNSAT proofs or concrete counterexamples.

---

## 2. Symbolic Value Model (`SymVal`)

Symbolic execution tracks values in an environment `SymEnv: VarId -> SymVal`:

| Variant | Representation | Semantic Description |
|---|---|---|
| `SymVal::Bits(BitVec)` | `bit_blaster::bv::BitVec` | Bit-vector of fixed width $W \in \{1, 8, 16, 32, 64\}$. |
| `SymVal::Bool(Edge)` | `bit_blaster::aig::Edge` | Symbolic boolean condition / predicate. |
| `SymVal::Ptr(ProvenanceId, BitVec)` | Base allocation + 64-bit byte offset | Pointer with tracked provenance and symbolic offset. |

### 2.1 Symbolic Operations
All WIR operations lower to exact AIG circuits in `bit-blaster`:
- **Bitwise:** `BitAnd`, `BitOr`, `BitXor`, `Not` $\to$ `bv::and`, `bv::or`, `bv::xor`, `bv::not`.
- **Arithmetic:** `Add`, `Sub`, `Neg` with wrapping overflow $\to$ `bv::add`, `bv::sub`, `bv::neg`.
- **Shifts:** `Shl`, `Shr` $\to$ `bv::shl`, `bv::lshr`.
- **Comparisons:** `Eq`, `Ne`, `Lt`, `Gt`, `Le`, `Ge` $\to$ `bv::eq`, `bv::ult`, `bv::slt`.
- **Conditionals / Mux:** Selection expressions $\to$ `bv::ite(cond, then_val, else_val)`.
- **Zero/Sign Extension:** Cast operations $\to$ `bv::zero_extend`, `bv::extract`.

---

## 3. Symbolic Memory (`SymMemory`)

Memory in symbolic execution models allocations with tracked provenance:
1. **Allocations:** `SymAlloc { base: ProvenanceId, size: usize, bytes: Vec<BitVec> }`.
2. **Loads:** Reading $N$ bytes from `Ptr(prov, offset)`:
   - If `offset` is concrete and $0 \le \text{offset} + N \le \text{size}$, loads the corresponding byte bit-vectors and concatenates them in little-endian order.
   - If `offset + N > \text{size}`, the engine asserts an out-of-bounds error condition (flagging potential CWE-125).
3. **Stores:** Writing $N$ bytes from bit-vector `val` to `Ptr(prov, offset)`:
   - If `offset` is concrete and within bounds, updates the allocation's byte bit-vectors.
   - If out-of-bounds, asserts a bounds violation (flagging potential CWE-787).
4. **Dynamic Slice Invariants:** `WirExpr::PtrLen(ptr)` returns a concrete 64-bit `BitVec` holding the allocation size.

---

## 4. Path Exploration & Bounded Unrolling

Execution proceeds across Basic Blocks via a worklist algorithm:

```text
State = (BlockId, SymEnv, SymMemory, PathCondition, StepBudget, LoopDepths)
```

1. **Step Budget:** Every path is subject to a maximum step budget (default 100,000 instructions) to prevent infinite loops.
2. **Branching (`WirTerminator::Branch`):**
   - If condition resolves to `Edge::TRUE`, continue to `then_bb`.
   - If condition resolves to `Edge::FALSE`, continue to `else_bb`.
   - Otherwise, fork state into two paths:
     - Then-path: path condition $PC' = PC \land \text{cond}$.
     - Else-path: path condition $PC' = PC \land \neg \text{cond}$.
   - Prune any path whose path condition is proven unsatisfiable by `cdcl-sat`.
3. **Loop Unrolling Budget ($k$):**
   - Track iteration count per loop header.
   - When iteration count reaches unroll bound $k$, stop exploring the loop body and mark the outcome as `bounded(k)`.
4. **Return (`WirTerminator::Return`):**
   - Record terminal path result: `(PathCondition, Option<SymVal>, SymMemory)`.

---

## 5. Miter Construction & Property Checking

To verify a claim (e.g. $F(\vec{x}) = G(\vec{x})$ or $F(\vec{x}) = \text{expected}$):
1. **Symbolic Arguments:** Instantiate inputs $\vec{x}$ as unconstrained symbolic bit-vectors `BitVec::input(&mut aig, width)`.
2. **Symbolic Execution:** Run the WIR implementation to obtain symbolic return value $R_{impl}$ under path condition $PC_{impl}$.
3. **Specification Circuit:** Construct the reference specification circuit $R_{spec}$.
4. **Miter Assertion:**
   - Equivalence predicate: `equiv = bv::eq(&mut aig, &R_impl, &R_spec)`.
   - Miter goal: find an assignment satisfying $PC_{impl} \land \neg \text{equiv}$.
5. **SAT Solving:**
   - Convert miter into CNF via `bit_blaster::tseitin::Tseitin`.
   - Solve with `cdcl_sat::Solver`:
     - **UNSAT:** No counterexample exists $\implies$ `VerificationVerdict::Proved` (or `Bounded(k)` if any loop was unrolled to limit $k$).
     - **SAT:** Solver extracts counterexample inputs $\vec{x}_{ce}$ $\implies$ `VerificationVerdict::Falsified(Counterexample)`.

---

## 6. Verification & Quality Standards

- **Zero External Dependencies:** Only standard Rust and verified repository crates (`warrant-ir`, `bit-blaster`, `cdcl-sat`, `drat-check`, `test-bootstrap`).
- **Strict Coverage Thresholds:** ≥ 90% Line Coverage AND ≥ 90% Branch Coverage across all new symbolic execution modules.
- **Independent Security Review:** Independent subagent audit verifying fail-closed error handling, resource quota enforcement, and prevention of algorithmic denial-of-service.
