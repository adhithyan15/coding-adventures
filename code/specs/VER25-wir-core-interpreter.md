# VER25 — Warrant Intermediate Representation (WIR) Core & Reference Interpreter (VER-P021)

**Parent:** [VER00 §7.5.5, §7.4.1, §8.3, §8.6.1, §8.6.6](VER00-verified-shipping-program.md), [VER26 §4–§5](VER26-ct-compare-pilot.md).

This specification establishes the core Intermediate Representation (WIR) and deterministic reference interpreter for the Verified Shipping Program. WIR is designed **for proofs and verification**, providing a mathematically sound intermediate representation with mandatory types, bounded execution, explicit memory provenance, an operational cost model, security taint labels, and an observable trace for constant-time verification.

It delivers **VER-P021**: the WIR core abstract syntax tree (AST), static type and well-formedness checker, provenance-tracking memory model, operational cost semantics, observable event trace recorder, and bounded reference interpreter.

---

## 1. Architectural Principles

```text
Source Code (Rust ct-compare, etc.)
               │
               ▼  [Front End: VER-P023]
       WIR Module & Functions
               │
               ├──► [Static Validator / Type Checker]
               │         - Strict typing (no invented types)
               │         - Provenance and label propagation
               │         - CFG well-formedness
               │
               ├──► [Reference Interpreter: VER-P021]
               │         - Bounded execution (step fuel, stack depth)
               │         - Deterministic memory model with bounds & provenance
               │         - Step cost accounting
               │         - Observable trace recording (branches, memory addresses)
               │         - Explicit undefined-behavior (Stuck) detection
               │
               └──► [Prover & Constant-Time Engine: VER-P024/P025]
                         - Self-composition over observable traces
                         - Symbolic execution & SAT bit-blasting
```

### 1.1 Core Tenets (VER00 §7.5.5)
1. **Exact Semantics:** Nothing in WIR is implementation-defined. Every instruction has explicit operational behavior, precise bit widths, and deterministic outcomes.
2. **Mandatory Types, No Invented Ones:** Every value carries an exact type. Untrusted or dynamic types are represented via explicit tagged unions (`Dynamic`) with runtime tests.
3. **Explicit Memory Model & Provenance:** Pointers carry allocation provenance and bounds `[base, limit)`. Out-of-bounds accesses or use-after-free evaluate to `Stuck` (explicit undefined behavior), never silent wrapping or unconstrained host pointer arithmetic.
4. **Effects Only Through Capabilities:** External interactions (I/O, environment, randomness) occur solely via explicit capability calls (`CallCapability`), verifiable against effect contracts (VER00 §8.6.1).
5. **Operational Cost Accounting:** Every executed instruction increments an abstract step counter according to a fixed cost schedule, backing formal complexity and execution-bound proofs (`steps <= f(n)`).
6. **Observable Execution Trace:** Memory addresses and branching choices are logged to an observable trace, enabling constant-time validation through self-composition (§8.1 of VER26).

---

## 2. Type System and Information Flow

### 2.1 Types (`WirType`)
WIR supports primitive scalar types, compound structures, pointers with provenance, and dynamic values:
- **Integer Types:** `U8`, `U16`, `U32`, `U64`, `I8`, `I16`, `I32`, `I64`.
- **Overflow Modes:** Each arithmetic operation explicitly declares overflow semantics:
  - `Wrap`: 2's complement modulo wrap.
  - `Trap`: Evaluates to a runtime trap if out of range.
  - `Saturate`: Clamps to minimum or maximum representable value.
- **Boolean:** `Bool` (`true` or `false`).
- **Unit:** `Unit` (`()`).
- **Byte Slices / Buffers:** `ByteSlice` (contiguous array of bytes).
- **Pointers:** `Ptr { pointee: Box<WirType> }` referencing memory allocations tagged with provenance.
- **Tuples:** `Tuple(Vec<WirType>)`.
- **Dynamic:** `Dynamic` representing dynamic language values with a runtime type discriminant tag (`Tag`).

### 2.2 Security & Taint Labels (`Label`)
WIR tracks information flow and secrecy across variables and expressions:
- `Public`: Freely observable data (e.g. lengths, nonces, public keys).
- `Secret`: Cryptographic keys, plaintexts, or private credentials. Secret values must not influence observable trace events (branches or memory addresses).
- `Hostile`: External untrusted input requiring verified validation and bounds checking.

Label lattice: `Public ⊑ Secret`.
Combining values propagates labels via least upper bound: `join(Public, Secret) = Secret`.

---

## 3. Memory Model and Provenance

### 3.1 Allocation and Provenance Tags
Every memory allocation receives a globally unique provenance identifier `ProvenanceId(u64)`.
A memory buffer is represented as:
```text
Allocation {
    id: ProvenanceId,
    base: u64,
    size: usize,
    bytes: Vec<u8>,
    alive: bool,
}
```

A pointer value is the pair `(ProvenanceId, offset: u64)`.

### 3.2 Memory Safety and Undefined Behavior (`Stuck`)
An access `Load(ptr, size)` or `Store(ptr, bytes)` requires:
1. `ptr.provenance` corresponds to an active allocation in the heap (`alive == true`).
2. `ptr.offset + size <= allocation.size` (spatial bounds check).
3. Alignment matches the target type requirement.

If any check fails, the interpreter immediately halts with `Outcome::Stuck(UndefinedBehavior::OutOfBounds | InvalidProvenance | UseAfterFree)`. This provides a rigorous formal model for memory safety verification (CWE-119, CWE-125, CWE-787, CWE-416).

---

## 4. Abstract Syntax and Control Flow

### 4.1 Expressions (`WirExpr`)
- **Constants:** `ConstInt(i128, WirType)`, `ConstBool(bool)`, `ConstBytes(Vec<u8>)`, `ConstUnit`.
- **Variables:** `Var(VarId)`.
- **Unary ALU:** `Not(Box<WirExpr>)`, `Neg(Box<WirExpr>, OverflowMode)`.
- **Binary ALU:** `Add`, `Sub`, `Mul`, `Div`, `Rem`, `BitAnd`, `BitOr`, `BitXor`, `Shl`, `Shr`, `Eq`, `Ne`, `Lt`, `Le`, `Gt`, `Ge`.
- **Type Conversions:** `Trunc(Box<WirExpr>, WirType)`, `ZExt(Box<WirExpr>, WirType)`, `SExt(Box<WirExpr>, WirType)`.
- **Tuple Operations:** `TupleMake(Vec<WirExpr>)`, `TupleGet(Box<WirExpr>, usize)`.

### 4.2 Instructions (`WirInst`)
- `Let { var: VarId, ty: WirType, label: Label, expr: WirExpr }`: Binds a local variable.
- `Alloc { var: VarId, size_expr: WirExpr }`: Allocates heap memory, returning a pointer.
- `Load { var: VarId, ptr_expr: WirExpr, ty: WirType }`: Loads typed value from memory.
- `Store { ptr_expr: WirExpr, val_expr: WirExpr }`: Stores typed value to memory.
- `Free { ptr_expr: WirExpr }`: Deallocates heap memory.
- `CallCapability { var: Option<VarId>, name: String, args: Vec<WirExpr> }`: Explicit external capability invocation.
- `CallFunction { var: Option<VarId>, name: String, args: Vec<WirExpr> }`: Invokes another WIR function.

### 4.3 Control Flow & Terminators (`WirTerminator`)
A function consists of a set of `BasicBlock`s. Each block contains sequential `WirInst`s and ends with a `WirTerminator`:
- `Return(Option<WirExpr>)`: Returns from function.
- `Jump(BlockId)`: Unconditional transfer of control.
- `Branch { cond: WirExpr, then_block: BlockId, else_block: BlockId }`: Conditional transfer.
- `Trap(String)`: Controlled runtime error or assertion failure.
- `Unreachable`: Invariant assertion that this point cannot be reached. Reaching it evaluates to `Stuck`.

---

## 5. Cost Model and Observable Trace

### 5.1 Step Cost Schedule
Every operational step consumes fuel:
- ALU arithmetic and bitwise: 1 step.
- Memory load / store: 2 steps.
- Function call / Return: 2 steps.
- Capability call: 5 steps.
- Control flow Jump / Branch: 1 step.

Total steps are accumulated into `ExecutionCost { steps: u64, allocations: u64, allocated_bytes: u64 }`.

### 5.2 Observable Event Trace (`ObservableTrace`)
For constant-time verification (CWE-208), the interpreter records every potentially observable microarchitectural side channel:
```rust
pub enum TraceEvent {
    Branch { from_block: BlockId, target_block: BlockId },
    MemoryAccess { provenance: ProvenanceId, offset: u64, is_write: bool, size: usize },
    CapabilityInvocation { capability: String },
}
```
If two program executions with identical public inputs but different secret inputs yield identical `ObservableTrace`s and identical step counts, timing and cache side channels are formally eliminated at the WIR level.

---

## 6. Reference Interpreter Specifications

### 6.1 State and Configuration
```rust
pub struct InterpreterConfig {
    pub max_steps: u64,
    pub max_call_depth: usize,
    pub max_memory_bytes: usize,
}
```

### 6.2 Execution Outcomes (`ExecutionOutcome`)
- `Success { value: WirValue, cost: ExecutionCost, trace: ObservableTrace }`: Successful termination.
- `Trap { reason: String, cost: ExecutionCost }`: Explicit runtime trap.
- `Stuck { reason: UndefinedBehavior, cost: ExecutionCost }`: Undefined behavior encountered (proof failure).
- `BudgetExceeded(BudgetLimit)`: Step fuel or call depth limit exceeded.

---

## 7. Verification and Test Acceptance

1. **Static Validation:** Type checker rejects type mismatches, variable use before definition, out-of-bounds tuple indexing, unknown block jumps, and invalid capability references.
2. **Deterministic Evaluation:** Reference interpreter produces byte-exact, identical results across platforms for all valid programs.
3. **Bounds and Undefined Behavior Enforcement:** Out-of-bounds pointer accesses, use-after-free, division by zero, and invalid conversions reliably halt with `Outcome::Stuck`.
4. **Constant-Time Trace Recording:** Proves constant-time property by executing constant-time and non-constant-time algorithms under identical public inputs and varying secrets, confirming trace equality for constant-time implementations and trace divergence for leaking implementations.
5. **Coverage Requirement:** Unit and integration tests must meet or exceed **≥ 90% Line Coverage AND ≥ 90% Branch Coverage**.
