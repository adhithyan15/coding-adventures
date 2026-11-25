# VER25 — Warrant Intermediate Representation (WIR) Core & Reference Interpreter (VER-P021)

**Parent:** [VER00 §7.5.5, §7.4.1, §8.3, §8.6.1, §8.6.6](VER00-verified-shipping-program.md), [VER26 §4–§5](VER26-ct-compare-pilot.md).

**Amended by KRN-006 (2026-11-23):** [section 8](#8-normative-semantics-krn-006) makes the Lean
semantics of WIR normative and settles the cost schedule. Sections 3, 4, 5 and 6 were
edited to agree with it; each edit is marked "(KRN-006)". Companion:
[VER11a §6](VER11a-lean4-proof-kernel.md). Evidence: the KRN-S0 spike report,
`code/learning/lean/krn-s0-spike/REPORT.md`, findings F1 to F3.

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
Every memory allocation receives a provenance identifier `ProvenanceId(u64)`.
The identifier is **deterministic** (KRN-006): it is the value of a per-run
allocation counter, and the counter only goes up. Ids are never reused, even after
`Free`. (`Memory::new` in `interp.rs` starts the counter at 1.) Section 8.6 gives the
rule precisely.

A memory buffer is represented as:
```text
Allocation {
    id: ProvenanceId,
    size: usize,
    bytes: Vec<u8>,
    alive: bool,
    kind: AllocKind,   // Stack | Heap          (KRN-006, new)
    secret: bool,      // the tag               (KRN-006, new)
}
```

The old draft listed a `base: u64` field. No code has one: a pointer is
`(provenance, offset)` and the offset is relative to the allocation, so there is no
host base address to store. The field is removed (KRN-006).

A pointer value is the pair `(ProvenanceId, offset: u64)`.

### 3.2 Memory Safety and Undefined Behavior (`Stuck`)
An access `Load(ptr, size)` or `Store(ptr, bytes)` requires:
1. `ptr.provenance` names an allocation that exists and is alive (`alive == true`).
2. `ptr.offset + size <= allocation.size` (spatial bounds check).

If either check fails, the interpreter immediately halts with `Outcome::Stuck(UndefinedBehavior::OutOfBounds | InvalidProvenance | UseAfterFree)`. This provides a rigorous formal model for memory safety verification (CWE-119, CWE-125, CWE-787, CWE-416).

**Out of bounds is `stuck`, not a branch (KRN-006).** There is no bounds-check
branch in WIR. `lower.rs` emits `PtrOffset` and then `Load` or `Store`, and the
access itself is the check. A failed check ends the run with outcome `stuck`; it
does not pick a successor block, and it adds nothing to the trace. Section 8.7
states it as a rule. (VER11a §8.4 says the body has "bounds checks"; that wording is
wrong, see 8.14.)

**Alignment is removed (KRN-006).** The old item 3 required alignment. WIR memory is
byte-addressed, `interp.rs` checks no alignment anywhere (`Memory::load_bytes` and
`store_bytes` test only liveness and range), and the Lean semantics has none. A
requirement the interpreter does not enforce is not part of WIR.

---

## 4. Abstract Syntax and Control Flow

### 4.1 Expressions (`WirExpr`)
- **Constants:** `ConstInt(i128, WirType)`, `ConstBool(bool)`, `ConstBytes(Vec<u8>)`, `ConstUnit`.
- **Variables:** `Var(VarId)`.
- **Unary ALU:** `Not(Box<WirExpr>)`, `Neg(Box<WirExpr>, OverflowMode)`.
- **Binary ALU:** `Add`, `Sub`, `Mul`, `Div`, `Rem`, `BitAnd`, `BitOr`, `BitXor`, `Shl`, `Shr`, `Eq`, `Ne`, `Lt`, `Le`, `Gt`, `Ge`.
- **Pointers (KRN-006: listed here because the lowering uses them):** `PtrOffset(ptr, offset_bytes)` and `PtrLen(ptr)`.
- **Dynamic values:** `MakeDynamic(expr, tag)` and `CheckDynamic(expr, expected_tag)`.
- **Type Conversions:** `Trunc(Box<WirExpr>, WirType)`, `ZExt(Box<WirExpr>, WirType)`, `SExt(Box<WirExpr>, WirType)`.
- **Tuple Operations:** `TupleMake(Vec<WirExpr>)`, `TupleGet(Box<WirExpr>, usize)`.

### 4.2 Instructions (`WirInst`)
- `Let { var: VarId, ty: WirType, label: Label, expr: WirExpr }`: Binds a local variable.
- `Alloc { var: VarId, size_expr: WirExpr, kind: AllocKind, secret: bool }`: Allocates a zero-filled buffer and returns a pointer to its first byte. `kind` is `Stack` or `Heap`. Only `Heap` counts toward the `heap` bound. `secret` is the allocation's tag. (KRN-006; today's `ast.rs` has only `var` and `size_expr`, KRN-006a adds the two fields.)
- `Load { var: VarId, ptr_expr: WirExpr, ty: WirType }`: Loads typed value from memory.
- `Store { ptr_expr: WirExpr, val_expr: WirExpr }`: Stores typed value to memory.
- `Free { ptr_expr: WirExpr }`: Marks an allocation dead. It applies to both kinds.
- `CallCapability { var: Option<VarId>, capability: String, args: Vec<WirExpr> }`: Explicit external capability invocation. (The field is `capability` in `ast.rs`; the old text said `name`.)
- `CallFunction { var: Option<VarId>, func_name: String, args: Vec<WirExpr> }`: Invokes another WIR function. (The field is `func_name` in `ast.rs`.)

### 4.3 Control Flow & Terminators (`WirTerminator`)
A function consists of a set of `BasicBlock`s. Each block contains sequential `WirInst`s and ends with a `WirTerminator`:
- `Return(Option<WirExpr>)`: Returns from function.
- `Jump(BlockId)`: Unconditional transfer of control.
- `Branch { cond: WirExpr, then_block: BlockId, else_block: BlockId }`: Conditional transfer.
- `Trap(String)`: Controlled runtime error or assertion failure.
- `Unreachable`: Invariant assertion that this point cannot be reached. Reaching it evaluates to `Stuck`.

---

## 5. Cost Model and Observable Trace

### 5.1 Step Cost Schedule (settled by KRN-006)
The machine of section 8 does one thing per step: one instruction or one terminator.
Each completed step adds exactly one cost to `steps`, from this table:

| Step | Cost |
|---|---|
| `Let` (any expression) | 1 |
| `Load` | 2 |
| `Store` | 2 |
| `Alloc` | 3 |
| `Free` | 2 |
| `CallFunction` (pushing the callee's frame) | 2 |
| `CallCapability` | 5 |
| `Jump` | 1 |
| `Branch` | 1 |
| `Return` | 1 |
| `Trap` | 1 |
| `Unreachable`, and any step that ends `stuck` | 0 |

The entry frame of a run is created by the caller of the interpreter, not by a
`CallFunction` step, so it costs nothing.

The old text said "Function call / Return: 2 steps" and did not mention `Alloc` or
`Free`, while `cost.rs` said return 1, alloc 3, free 2. Section 8.9 gives the reasons
for the table above and lists every place that changes.

Total steps are accumulated into `ExecutionCost`. Today it is
`{ steps, allocations, allocated_bytes, memory_reads, memory_writes, capability_calls }`
(`cost.rs`). KRN-006a adds `heap_bytes` (bytes of `Heap` allocations only, never
decremented) and `machine_steps` (the number of steps taken, which is what the fuel of
section 8 counts), and `allocated_bytes` keeps its meaning of all bytes.

### 5.2 Observable Event Trace (`ObservableTrace`)
For constant-time verification (CWE-208), the interpreter records every potentially observable microarchitectural side channel:
```rust
pub enum TraceEvent {
    BranchDecision { from_block: BlockId, target_block: BlockId },
    MemoryAccess { provenance: ProvenanceId, offset: u64, kind: AccessKind, size: usize },
    CapabilityInvocation { capability: String },
}
```
(The old text called the first variant `Branch` and gave the access a `is_write`
flag. `trace.rs` has `BranchDecision` and `kind: AccessKind`; the text now agrees
with the code.) Only a `Branch` terminator records a branch event. `Jump` does not:
its target does not depend on any value, so it leaks nothing. Section 8.8 lists
exactly when each event is emitted.
If two program executions with identical public inputs but different secret inputs yield identical `ObservableTrace`s and identical step counts, timing and cache side channels are formally eliminated at the WIR level.

---

## 6. Reference Interpreter Specifications

### 6.1 State and Configuration
```rust
pub struct InterpreterConfig {
    pub max_steps: u64,
    pub max_call_depth: usize,
    pub max_memory_bytes: usize,
    pub schedule: CostSchedule,   // present in interp.rs; the old text omitted it
}
```

### 6.2 Execution Outcomes (`ExecutionOutcome`)
Every outcome carries the cost and the trace (KRN-006). Today only `Success` does;
`Trap` and `Stuck` carry a cost but no trace, and `BudgetExceeded` carries neither
(`interp.rs`, `ExecutionOutcome`). A differential test against the Lean semantics
(section 8.10) compares the trace and the steps on every outcome, so all four must
report them.

- `Success { value: WirValue, cost: ExecutionCost, trace: ObservableTrace }`: Successful termination. Lean outcome kind `ret`.
- `Trap { reason: String, cost: ExecutionCost, trace: ObservableTrace }`: Explicit runtime trap. Kind `trap`.
- `Stuck { reason: UndefinedBehavior, cost: ExecutionCost, trace: ObservableTrace }`: Undefined behavior encountered (proof failure). Kind `stuck`.
- `BudgetExceeded { limit: BudgetLimit, cost: ExecutionCost, trace: ObservableTrace }`: A resource limit stopped the run before it finished. Kind `timeout`. (Today a tuple variant `BudgetExceeded(BudgetLimit)`.)

---

## 7. Verification and Test Acceptance

1. **Static Validation:** Type checker rejects type mismatches, variable use before definition, out-of-bounds tuple indexing, unknown block jumps, and invalid capability references.
2. **Deterministic Evaluation:** Reference interpreter produces byte-exact, identical results across platforms for all valid programs.
3. **Bounds and Undefined Behavior Enforcement:** Out-of-bounds pointer accesses, use-after-free, division by zero, and invalid conversions reliably halt with `Outcome::Stuck`.
4. **Constant-Time Trace Recording:** Proves constant-time property by executing constant-time and non-constant-time algorithms under identical public inputs and varying secrets, confirming trace equality for constant-time implementations and trace divergence for leaking implementations.
5. **Totality guarantee (KRN-006):** the type checker accepts only programs for which section 8.5 holds. Section 8.5 says which rules exist today and which are missing.
6. **Differential test (KRN-006):** the reference interpreter and the Lean semantics agree on obs, value and final memory, over the corpus of section 8.10, up to the listed differences.
7. **Coverage Requirement:** Unit and integration tests must meet or exceed **≥ 90% Line Coverage AND ≥ 90% Branch Coverage**.

---

## 8. Normative semantics (KRN-006)

### 8.0 Status, form, and what this section decides

This section makes the Lean semantics of WIR **normative**, as VER11a §6.1 asks. Where
the Lean definition and the reference interpreter of section 6 disagree, the Lean
definition is what WIR means and the interpreter has a bug. The one exception is
the list of allowed differences in 8.10.

**Form: a new section in VER25, not a child spec.** The reason is that this section
is not a second document about WIR. It *is* the semantics of the instructions that
sections 3 to 6 introduce. The cost schedule of 5.1, the trace of 5.2 and the
outcomes of 6.2 all had to change with it, and one document means one place where
each fact lives. A child spec would have restated them. The name `VER25c` is
reserved by VER11a §14.3 for the Rust-subset semantics (KRN-015), so a child spec would
have been `VER25d`. None is needed.

The spec states the semantics in prose and in Lean signatures. The Lean text itself is
written in KRN-007 (`warrant-lib`), ported from the KRN-S0 spike
(`code/learning/lean/krn-s0-spike/`, `KrnS0/Semantics.lean`). 8.14 lists the places
where the spike differs from this section. The spike is a hand transcription of a
subset, and where it differs, this section wins.

What it decides:

| Item | Where |
|---|---|
| The machine: frames, `step`, `run`, `obs` | 8.1 to 8.4 |
| Totality, and the type-checker guarantee | 8.5 |
| Allocations, deterministic ids, stack and heap | 8.6 |
| Out of bounds is `stuck` | 8.7 |
| Events and outcomes | 8.8 |
| The cost schedule, and its effect on the ct-compare bounds | 8.9 |
| The differential test | 8.10 |
| The label table | 8.11 |

### 8.1 Numbering: how a `Program` becomes a Lean term

Strings and sparse ids never appear while the kernel reduces. The WIR-to-Lean
embedding numbers everything, and prints the numbering with the goal (VER11a §7.4).

- **`FnId`** is the index of the function in `Program.functions`.
- **`BlockId`** is the existing `BlockId(usize)`. The embedding requires the ids of one
  function to be exactly `0 .. n-1`, with block `0` the entry. `lower.rs` produces
  that (`fresh_block_id` counts up from the entry), and the interpreter starts at
  `BlockId(0)` (`interp.rs`, `call_fn`). The checker does not enforce it today; 8.5
  lists it as a missing rule. Branch events carry `BlockId`s, so they must be the ids
  the interpreter reports, not positions.
- **`VarId`** is a number, per function. The parameters are `0 .. k-1` in declared
  order. After that, every instruction that binds a variable (`Let`, `Alloc`, `Load`,
  and a `CallFunction` or `CallCapability` that has a `var`) takes the next number,
  walking blocks by id and instructions by index. The numbering is one-to-one,
  because `check.rs` keeps one environment for the whole function and rejects a
  second definition (`VariableRedefined`).
- **Program point.** A point is `(f, b, i)`: instruction `i` of block `b` of function
  `f`. The point `i = length of the block's instruction list` is the block's
  terminator. A point with `b` or `i` beyond that does not exist.

### 8.2 Values and memory

Values mirror `WirValue` (`interp.rs`): unit, bool, integer (with its `WirType`),
bytes, pointer `(provenance, offset)`, tuple, and dynamic (tag and inner value).

Memory is **byte-addressed**. An allocation has the shape of 8.6, and its contents are
a function from offset to byte. Scalars are stored little-endian at their
`byte_size` (`encode_value` and `decode_value` in `interp.rs`).

- **Loads** read `byte_size ty` bytes (1 when the type has no size) and decode them
  as `ty`. Decoding to `bool`, `u8`..`u64` and `i8`..`i64` always succeeds. A load of
  type `ByteSlice` yields the bytes. A load whose type has no decoding (tuple, dynamic,
  unit) is `stuck`.
- **Stores** write the encoding of the value. A value with no encoding (tuple,
  dynamic, unit) is `stuck`.
- A pointer is **not storable**. `encode_value` writes only the offset of a pointer,
  and the `Ptr` arm of `decode_value` reads it back as a pointer with the **invented
  provenance 1**. That is not a pointer that was stored: provenance is lost and a
  made-up one appears. The semantics forbids the round trip: a store of a pointer
  value is `stuck`, and so is a load of a `Ptr` type. (Today the interpreter accepts
  both. See 8.13.)
- `PtrOffset` adds a signed number of bytes to the offset and saturates at 0 and at
  `u64::MAX` (`interp.rs`, `eval_expr`). It checks nothing. The check is made by the
  access that uses the pointer (8.7).
- `PtrLen(p)` returns `size` of the allocation `p` names, whether or not it is alive.
  A provenance that names no allocation is `stuck` (`InvalidProvenance`).

The KRN-S0 spike used one `Val` cell per offset and a `len` inside the pointer. Both
were simplifications for the spike. KRN-007 uses bytes and `PtrLen` as above.

### 8.3 The machine

```lean
structure Frame where
  fn : FnId; block : BlockId; ip : Nat       -- the program point, as in 8.1
  vars : VarId → Val                          -- total (8.5)
  retDst : Option VarId                       -- where this frame's result goes in its caller

structure State where
  stack  : List Frame        -- top first
  status : Status            -- running | ret v | trap | stuck
  mem    : Mem
  trace  : List Event
  steps  : Nat               -- accumulated cost, 5.1
  allocs : Nat               -- next provenance id
  heap   : Nat               -- bytes allocated with kind Heap

def step : Program → State → State
def run  : Nat → Program → State → State
  | 0,     _, s => s
  | n + 1, p, s => match s.status with
                    | .running => run n p (step p s)
                    | _        => s
```

- **`step` is total.** It maps every state to a state. It never fails and never
  needs a proof that the program is well formed. A state whose top frame points at
  no instruction (no such function, block or index), or an empty stack, goes to
  `stuck`. `step` on a state that is not `running` is not specified: `run` never calls
  it.
- **`run` recurses structurally on the step count.** It reduces in the kernel on
  concrete inputs (VER11a §6.2, §7.5). The fuel `n` counts steps. It is not cost.
  Cost is added to `steps` by `step`.
- **Where a run starts.** `entry p f args mem allocs` is the state with one frame
  `{fn := f, block := 0, ip := 0, retDst := none}`, parameters `0 .. k-1` bound to
  `args` and every other variable `unit`, status `running`, empty trace, `steps := 0`,
  `heap := 0`, and the given `mem` and `allocs`. The argument allocations were made
  before the run, by the caller; they are in `mem` and are **not** counted in `heap`.
- **Resource limits are not part of the machine.** A call depth limit, a memory limit
  and a step limit exist only in the interpreter (8.10), where each ends the run as
  `timeout`.

#### 8.3.1 One step, kind by kind

Let `fr` be the top frame, `e` an expression, and `eval` as in 8.5. **A step that ends
`stuck` adds nothing to `steps`, appends no event, and changes nothing else except the
status.** The costs below are those of 5.1.

| Step | What it does | Events | Cost |
|---|---|---|---|
| `Let x e` | `fr.vars x := eval e`, `ip + 1`. | none | 1 |
| `Alloc x sz kind secret` | `n := eval sz` as a natural number (negative: `stuck`). New allocation, see 8.6. `fr.vars x := ptr(id, 0)`, `ip + 1`. | none | 3 |
| `Load x pe ty` | Evaluate `pe` to a pointer. Check access of `byte_size ty` bytes (8.7). `fr.vars x := decode ty (bytes at offset)`, `ip + 1`. | `memoryAccess(prov, off, read, size)` | 2 |
| `Store pe ve` | Evaluate both. Check access of `size (encode v)` bytes. Write them. `ip + 1`. | `memoryAccess(prov, off, write, size)` | 2 |
| `Free pe` | The pointer must name an alive allocation, with offset 0. Set `alive := false`. `ip + 1`. | none | 2 |
| `CallCapability v c args` | Evaluate `args`. Look up `c` in the capability table (below). | `capabilityInvocation c`, emitted even when the call then traps | 5 |
| `CallFunction v f args` | Evaluate `args` in the caller. Push a callee frame (below). | none | 2 |
| `Return eo` | Evaluate `eo` (none gives `unit`). Pop the frame (below). | none | 1 |
| `Jump b` | `block := b`, `ip := 0`. | none | 1 |
| `Branch c t e` | `b := if eval c is true then t else e`; `block := b`, `ip := 0`. | `branch(fr.block, b)`, from-block first | 1 |
| `Trap msg` | `status := trap`. | none | 1 |
| `Unreachable` | `status := stuck`. | none | 0 |

**Capabilities.** The semantics takes a capability table `caps : String → List Val →
Option Val`, supplied with the program, and the same in both runs of a relational
claim. `some r` binds `r` to `v` (when there is one) and goes to `ip + 1`. `none`
ends the step with status `trap`, which is what the interpreter does with an unknown
capability (`interp.rs`, `CapabilityDispatcher::call` returning `Err`). VER11a §6.4
leaves programs that call capabilities without a kernel proof; the rule is here so
the differential test can run them.

**When a frame is pushed.** `CallFunction v f args` builds a callee frame

```text
{ fn := f, block := 0, ip := 0, retDst := v,
  vars := (param j) ↦ (eval args[j] in the caller's vars), all else unit }
```

and the new stack is `callee :: fr{ip := ip + 1} :: rest`. The caller's frame stays
under it, already pointing at the next instruction. `f` that does not exist, or an
argument count that differs from the parameter count, is `stuck`. If the new stack
would be longer than the interpreter's `max_call_depth`, the interpreter reports
`timeout` instead (8.10). The Lean machine has no limit.

**When a frame is popped, and how the result reaches the caller.** `Return eo`
computes the value `v`.

- If the stack has one frame: `stack := []`, `status := ret v`.
- Otherwise the top frame `callee` is removed, and the frame `g` below it becomes
  `g{vars := g.vars[d := v]}` if `callee.retDst = some d`, else `g` unchanged. `g.ip`
  already points after the call. Execution resumes in `g`.

The destination `retDst` is stored **in the callee's frame**, naming a variable of the
caller. A callee that returns `unit` to a caller that asked for a value delivers
`unit`; the checker rules this out through the return type (8.5).

**Branch and jump.** `Jump` has no event: its target is a constant. `Branch` has an
event, which records which successor was chosen, and so reveals the condition. A
`Branch` whose condition has the wrong shape is covered by 8.5.

### 8.4 What an observer sees

```lean
def Status.kind : Status → Outcome       -- ret | trap | stuck, or timeout for running
def obs (s : State) : Outcome × List Event × Nat := (s.status.kind, s.trace, s.steps)
```

A state that is still `running` when the fuel is spent has outcome kind `timeout`.
The kind forgets the returned value, the trap message, and the stuck reason. The
returned value is deliberately **not** observed: a constant-time comparison returns a
secret-dependent boolean by design. `obs` is a pure function of the state; it reads
nothing the semantics does not define.

### 8.5 Totality, and the type-checker guarantee

The semantics is total in two places, and a third thing is left to the checker.

1. **Variables are a total map.** `Frame.vars : VarId → Val`. A variable that was never
   written reads as `unit`. The interpreter instead stops with
   `Stuck(TypeMismatch("Undefined var"))` (`eval_expr`, `WirExpr::Var`).
2. **An operation whose operands have the wrong shape returns a default.** For example
   an integer operator applied to a pointer reads the operand as `0`, and a branch on
   a non-boolean reads it as `false`. The interpreter instead stops with
   `Stuck(TypeMismatch(...))` (`as_int`, `as_bool`, `as_ptr`).
3. **Real undefined behaviour is not defaulted.** It ends the step as stated. The
   cases are: division or remainder by zero; a shift count outside `0..128`
   (`interp.rs` rejects `!(0..128).contains(&r)`); `CheckDynamic` with a different
   tag; `PtrLen` of an unknown provenance; and every case in 8.7. They are stuck in
   Lean and in the interpreter alike.
   **One decided change.** An integer operation in overflow mode `Trap` whose result
   is out of range ends the step with status **`trap`**, not `stuck`, as 2.1 of this
   spec says ("evaluates to a runtime trap"). `interp.rs` today returns `Stuck`
   (`apply_overflow`, and `Neg` in `eval_expr`). That is an interpreter bug under 8.0,
   fixed in KRN-006a.

   A `Let` that traps this way charges the cost of a `Let`, 1, since the step
   completed with outcome `trap`; only a step that ends `stuck` is free.

   The Lean `eval` therefore returns one of: a value, `trap`, or `stuck`. Each is a
   defined result, which the lockstep proof treats like a load: a secret-dependent
   divisor or shift count makes the two runs differ in outcome, and the obligation
   fails, which is correct.

**Why total.** The relational lockstep proof of VER11a §8 must show that both runs do
the same thing. With total `vars` and total operators it never has to prove that a
variable is bound or that an operand has the right shape, only that the public
parts agree. The KRN-S0 spike measured this (REPORT.md, F2, last item): the
alternative is a "same value shape" fact carried for secrets in the relation.

**The guarantee that makes it sound.** Totality is faithful to the interpreter only on
programs where neither case can happen. So the semantics adds a **precondition on
every kernel goal about a function**: the program is accepted by the type checker,
and the checker guarantees all of the following.

| # | Guarantee | Provided by | Status |
|---|---|---|---|
| T1 | Every expression operand has the shape its operator needs. | `check_expr`: `ExpectedInteger`, `TypeMismatch`, `ExpectedPointer`, `ExpectedTuple`, `TupleIndexOutOfBounds`, `InvalidTruncation`, `InvalidExtension`; and the `Branch` arm of `check_block`: `BranchConditionNotBool` | **present** |
| T2 | A `Let`'s declared type is the expression's type; a call has the callee's arity and argument types; a `Return` has the function's return type. | `check_block` (`Let`, `CallFunction`, `Return` arms): `TypeMismatch`, `ArgumentCountMismatch` | **present** |
| T3 | Every jump target and branch target exists. | `check_block`: `BlockNotFound` | **present** |
| T4 | A variable is bound before every read **on every path from the entry**. | `UndefinedVariable` exists, but it is not this rule. | **missing** |
| T5 | The block ids of a function are `0 .. n-1`, and block `0` exists (the entry). | `check_function` rejects only `DuplicateBlock` and `EmptyFunction`. | **missing** |
| T6 | Every callee name resolves. | `check_block`, `CallFunction`: `FunctionNotFound` | **present** |

**T4 is the gap that matters.** `check_function` (`check.rs`) threads *one* `env`
through `func.blocks` **in vector order**, and `check_block` only adds to it. A read is
accepted if some *earlier instruction in that order* bound the variable. Vector order is
not control-flow order. A counter-example the checker accepts: block 0 jumps to
block 2; block 1 binds `x` and jumps to block 2; block 2 reads `x`. Block 1 comes
first in the vector, so the read passes; the path `0 → 2` reads an unbound variable.
The interpreter stops there; Lean reads `unit`. The fix, for KRN-006a: reject a read
unless the binding instruction **dominates** the read (same block and earlier, or in
a block that dominates the reading block). The error is a new `UseBeforeDefinition`.

**T5 is missing too.** `interp.rs` looks blocks up by `id` and starts at `BlockId(0)`;
a function with no block `0` stops with `Stuck`. Lean indexes blocks by position
and would read a missing entry as `stuck` too, but the *numbering* of 8.1 needs the
ids to be dense. New errors: `NoEntryBlock` and `BlockIdsNotDense`.

**Two things that are not gaps.** First, `Load` and `Store` accept any scalar type
through a `Ptr<U8>`, and `Alloc` always yields `Ptr<U8>` (`check.rs`, `Alloc` and
`Load` arms). That is loose typing, but with byte memory (8.2) every scalar load is
defined, so it breaks neither totality nor lockstep. Second, a load or store of a type
with no encoding is `stuck` (8.2); `stuck` is a defined outcome that a `no_panic`
claim will refuse. It is not a totality failure.

**What is not proved.** The checker does not come with a preservation theorem (a
well-typed program never produces a value of the wrong shape at run time). The
semantics does not need one for its own definition, because every shape error is
defaulted. It does need it to say the interpreter and Lean agree. That is exactly
what the differential test of 8.10 samples, and it is an **open assumption**,
proposed id `wir.typing.checker-guarantees` in bucket (c) of VER11a §13, until the
checker is stated and proved in Lean. The id is for VER11a's table, which this change
does not edit.

### 8.6 Allocations

Each allocation has this shape. Its fields are exactly those of `Alloc` in 3.1:

```lean
structure Alloc where
  size   : Nat
  alive  : Bool
  kind   : AllocKind        -- stack | heap
  secret : Bool             -- the tag
  bytes  : Nat → UInt8
```

**The tag.** The tag says whether the allocation holds a secret. It is the `secret`
field of the `Alloc` instruction that made it, or, for an argument allocation, the
claim's label for that argument. VER11a §6.3 also names the allocation *site* as part
of the tag. The site is the program point of the `Alloc`, which the program already
fixes, so the machine does not store it. The tag has **no effect on `obs`**: `step`
copies it into the new allocation and reads it nowhere. It matters only to the
relation of VER11a §8.3, which allows memory to differ between the two runs only
inside secret allocations. A tag that is too "secret" weakens the relation and
proves nothing false. A tag that is too "public" makes a later obligation fail. So the
tag is untrusted metadata, and the label inference of KRN-012 writes it into the
program before the program is embedded and hashed (VER11a §7.3).

**The allocation id rule.** `Alloc` creates the allocation with id `s.allocs`, then
sets `s.allocs := s.allocs + 1`. Nothing else changes `allocs`. `Free` does not
recycle ids. Two runs that allocate in the same order therefore get the same ids. The
interpreter agrees: `Memory::allocate` uses `next_id` and then increments it, and
starts at 1.

**The entry premise and the invariant.** Every kernel goal about a function assumes

```lean
∀ q, allocs ≤ q → mem q = none        -- no argument id collides with a fresh id
```

for the `allocs` it starts from. The invariant is part of the relation `R` of VER11a
§8.3, not only the premise: `Alloc` must show at each step that the new id was unused
(it is, because id `= allocs ≥ allocs`), and re-establish the invariant for
`allocs + 1`. The spike found it needs to be a field of `R` (REPORT.md, F2).

**Which allocations count toward the `heap` bound.** `Alloc` with `kind = heap` adds `n`
to `s.heap`; `kind = stack` adds nothing. `heap` is **cumulative**: `Free` does not
subtract. A bound on cumulative bytes is stronger than a bound on live bytes, so it is
sound for either reading. The unit is **bytes**. The `heap` row of a `bound` block
(for example `heap <= 2 * len(a)`) is a statement about `s.heap`.
Argument buffers made by the caller are not counted.

`kind` does not change an allocation's lifetime. A stack slot lives until it is freed
or the run ends.

**How the lowering marks them.** The rule: an allocation the lowering invents for
a local is `Stack`; an allocation that models a program's own buffer is `Heap`. Today's
sites, all in `lower.rs`:

| Site | Variable prefix | Kind | Why |
|---|---|---|---|
| `bool as int` cast (`Expr::Cast`) | `bool_slot` | **stack** | a one-to-eight byte local |
| `for` loop counter (`Stmt::For`) | `<i>_ptr`, 8 bytes | **stack** | a local the SSA form mutates |
| `let acc` (`Stmt::Let` named `acc`) | `acc_ptr`, 1 byte | **stack** | a local the SSA form mutates |
| `vec![v; n]` (`Expr::MacroCall`) | `vec_buf` | **heap** | `Vec` is heap in Rust |
| `[v; n]` (`Expr::ArrayRepeat`) | `arr_rep` | **stack** | a Rust array is a stack value |
| `[a, b, ...]` (`Expr::Array`) | `arr_lit` | **stack** | same |

The marking must come from the construct, not from a name. Today `lower_block`
makes a variable a one-byte slot **because it is called `acc`** (`if name == "acc"`),
which is an accident of the one function it was written for. KRN-006a replaces that
test; this spec only requires that stack-ness never depends on an identifier.

The lowering emits `secret: false` everywhere; the label inference sets it later.

**Effect on the claim `heap <= 2 * len(a)`.** With kinds, `ct_select_bytes` allocates
`len` heap bytes (`vec_buf`), plus a 1-byte `bool_slot` and an 8-byte loop slot on the
stack. Counting every allocation, the figure would be `len + 9`, which exceeds
`2 * len` whenever `len < 9`, so the claim would be false for short inputs (VER11a §6.2
says "false at `len = 0`"). The same count for `ct_eq_equal_length`, which has only
stack slots (9 bytes), would exceed `2 * len` for `len <= 4`. With kinds, the figures
are `len` and `0`, and both hold. The `heap` row is unchecked today
(`tests/warrant_proofs.rs` records it as `unchecked`: "the pilot prover does not track
this resource"), so no test notices.

### 8.7 Out of bounds is `stuck`

> **Rule.** For `Load`, `Store` and `Free`, let the pointer value be `ptr(prov, off)`
> and the access width `w` (for `Free`, 0). The step is `stuck`, with no event, no
> cost and no other change, unless **all** of these hold:
> (1) `mem prov = some a`;
> (2) `a.alive`;
> (3) `off + w ≤ a.size` (for `Free`: `off = 0`).
> `PtrOffset` never checks. There is no branch, no bounds-check block, and no
> third outcome.

Why this is a rule and not an implementation detail. A bounds failure that were a
*branch* would put a secret-dependent event in the trace. As a `stuck` outcome it is
an outcome kind, which `obs` includes, so two runs that differ in whether they get stuck
are distinguished. And the relation of VER11a §8.3 needs no `WfState` fact for it
(REPORT.md, F2): when address, width and liveness agree in both runs, both runs are
stuck together. `WfState` is needed for the **unary** claims `no_panic` and memory
safety, which say the step is never stuck.

Two changes in `interp.rs` follow (8.13): the access event is recorded **after** the
check, and `Free` with a non-zero offset is `stuck` (today the offset is ignored;
`interp.rs`, `Memory::free` takes only the provenance).

### 8.8 Events and outcomes

An event is one of the three of 5.2. The machine emits them in exactly these
places, and nowhere else:

| Event | Emitted by | Fields |
|---|---|---|
| `branch` | a completed `Branch` | from block, chosen block |
| `memoryAccess` | a completed `Load` or `Store` | provenance, offset, read or write, byte width |
| `capabilityInvocation` | a `CallCapability` (also when it then traps) | capability name |

The outcome of a run is one of four kinds:

| Kind | Status when the run stops | Interpreter variant |
|---|---|---|
| `ret` | `ret v` | `Success` |
| `trap` | `trap` | `Trap` |
| `stuck` | `stuck` | `Stuck` |
| `timeout` | still `running` | `BudgetExceeded` |

**The interpreter must report the trace and the steps on every outcome.** The
reason is the differential test (8.10): the Lean `obs` is `(kind, trace, steps)` for
all four kinds, so the interpreter's four outcomes must carry the same three things.
Today `Trap` and `Stuck` carry `cost` and no trace, and `BudgetExceeded(BudgetLimit)`
carries neither (`interp.rs`, `ExecutionOutcome`). The new shapes are in 6.2. For a
run that never started (entry function not found, wrong argument count), the trace
is empty and `steps` is 0.

### 8.9 The cost schedule, settled

**Three documents give three different schedules**, and a fourth number is what the
code really charges.

| Step | VER25 §5.1 (old) | `cost.rs` default | `interp.rs`, as run | `sym.rs`, as run | KRN-S0 spike |
|---|---|---|---|---|---|
| `Let` | 1 | 1 | 1 | 1 | 1 |
| `Load` / `Store` | 2 | 2 | **3** | 1 | 2 |
| `Alloc` | none | 3 | **4** | 1 | 3 |
| `Free` | none | 2 | **3** | 1 | 2 |
| `CallCapability` | 5 | 5 | **6** | n/a | none |
| `CallFunction` | 2 | 2 | **3** (+0 at the callee) | refused | 2 |
| entry call | none | none | **2** | n/a | 0 |
| `Return` | **2** | **1** | 1 | **0** | **2** |
| `Jump` / `Branch` | 1 | 1 | 1 | **0** | 1 |
| `Trap` | none | none | **0** | error | 1 |
| `Unreachable` | none | none | 0 | 0 | 0 |

The interpreter's "3, 4, 3, 6" come from one line: `call_fn` adds
`alu_step_cost` for **every** instruction before it matches on the instruction
kind, and then the arm adds its own cost on top (`interp.rs`, the `for inst in
&block.instructions` loop). The symbolic executor charges one per instruction and
nothing for a terminator (`sym.rs`, `step_count += 1` in the instruction loop and no
charge in the terminator `match`).

**The decision.** The schedule is the table of 5.1: `Let` 1, `Load` 2, `Store` 2,
`Alloc` 3, `Free` 2, `CallFunction` 2, `CallCapability` 5, `Jump` 1, `Branch` 1,
`Return` 1, `Trap` 1, `Unreachable` and any stuck step 0, **one cost per completed
step**, and no cost for the entry frame.

**Why this one.**

1. **It is `cost.rs`'s default schedule, unchanged.** Every number already stands
   in the code, in `CostSchedule::default()`. The only differences from that default
   are one new field, `trap_cost`, and a rule for when each field is charged. The
   spec text was the odd one out.
2. **Return costs 1, not 2.** The old line, "Function call / Return: 2 steps", reads
   either way: each costs 2, or the pair costs 2. The code resolved it to call 2,
   return 1, and the interpreter, the only executable reading, already does this. A
   return is a control transfer like a `Jump`, which costs 1. The call costs more
   because it binds arguments and builds a frame. Choosing 2 would change `cost.rs`
   and every recorded figure for no benefit.
3. **`Alloc` and `Free` have costs.** An allocation does real work. A schedule that
   made it free would let a program loop on `Alloc` and `Free` at no cost and still
   meet a `steps` bound. 3 and 2 are what `cost.rs` says.
4. **One cost per step is what the Lean machine can do.** `step` adds one number to
   `steps`, chosen by the instruction kind. The interpreter's extra per-instruction
   charge cannot be written as such a function without a second rule, so it is
   removed, not specified.
5. **A stuck step costs nothing.** Charging it would make the cost of a stuck step
   depend on how far evaluation got before it failed. Zero is the simplest rule, and
   a stuck run's `steps` then counts only completed steps. The interpreter charged
   first and checked later.
6. **`Trap` costs 1.** It is a completed terminator, like `Jump`. The interpreter
   charged 0.

**Every place that changes** (all in KRN-006a, except this spec):

| Place | Change |
|---|---|
| This spec, 5.1 | Done here. |
| `warrant-ir/src/cost.rs` | Add `trap_cost: 1` to `CostSchedule` and its `Default`. Every other value is unchanged. Add the counters `heap_bytes` and `machine_steps` (5.1) to `ExecutionCost`. |
| `warrant-ir/src/interp.rs` | (a) Charge `alu_step_cost` in the `Let` arm only, not for every instruction. (b) Charge `call_cost` in the `CallFunction` arm, and not at callee entry; charge nothing for the entry frame. (c) Charge `trap_cost` for a `Trap` terminator. (d) Charge each step **after** it is known not to be stuck, and record a memory event after the access check. (e) Check the step budget **before every step, including terminators.** Today the check is inside the instruction loop and in `call_fn` only, so a loop whose blocks hold no instructions never reaches it and spins forever. (f) Return the trace and the cost on every outcome (6.2). |
| `warrant-ir/src/sym.rs` | Charge by kind, as 5.1, in the instruction loop and in the terminator `match` (today 1 per instruction, 0 per terminator). `SymbolicConfig::max_steps` (default 100,000) is then in cost units, not instructions. The step counter of a path is the schedule's `steps`. `Free` is a no-op that charges nothing today; it must charge 2. `CallFunction` is refused today and stays refused until a call is inlined, in which case the inlined `Jump`s and `Return` replace it (below). |
| `warrant-ir/src/ct_proof.rs`, `tests/` | `BoundProver::prove_step_bound` compares `outcome.steps` with the bound and needs no code change, but every number it is tested against changes: `tests/test_ct_proofs.rs` (`max_path_steps == 3`, `actual_steps == 3`) and `tests/test_coverage_ultra.rs` (`ConstantTime { steps: 4, ... }`) must be recomputed, and the recorded `max_steps` in `ct-compare/evidence/proof-manifest.json` regenerates. |
| `code/specs/ct-compare.warrant`, `ct-compare` source | **No change.** The bounds are not edited (below). |
| The Lean spike (becomes KRN-007) | `costRet` 2 becomes 1; the entry frame costs 0 (already the case); `Jump` emits no event (8.3.1); a stuck step costs 0 (already the case). |

**Effect on the ct-compare bounds.** The Warrant rows are
`steps <= 64 * len(a) + 512` for `eq`, `eq_fixed` and `select_bytes`, and
`steps <= 64` for `eq_u64` (`code/specs/ct-compare.warrant`, `bound` blocks). The
counts below were measured on the **real lowering** of
`code/packages/rust/ct-compare/src/lib.rs` by running the reference interpreter once
per cost kind, with that kind's cost set to 1 and every other cost set to 0, at
`len` 0, 1, 2 and 5. Every count is linear in `len` (the 1, 2, 5 differences agree),
and the totals of kinds match the static instruction counts of the lowered blocks
(`Alloc` 2, 3, 0; `Free` 1, 2, 0; `Load` 5, 5, 0; and 19 `Let`s over the three
functions). The functions have no `CallFunction`: the volatile helpers are
lowered inline.

*`ct_eq_equal_length`* (header, body, exit; `len = n` iterations of the body):

| Step | Cost | Per iteration (count) | Fixed (count) |
|---|---|---|---|
| `Let` | 1 | 5 | 1 |
| `Load` | 2 | 4 (counter, `acc`, `a[i]`, `b[i]`) | 2 (last header test, final `acc`) |
| `Store` | 2 | 2 (`acc`, counter) | 2 (`acc` init, counter init) |
| `Alloc` | 3 | 0 | 2 (`acc_ptr`, counter) |
| `Free` | 2 | 0 | 1 (counter) |
| `Jump` | 1 | 1 | 1 |
| `Branch` | 1 | 1 | 1 |
| `Return` | 1 | 0 | 1 |

Per iteration: `5·1 + 4·2 + 2·2 + 1 + 1 = 5 + 8 + 4 + 1 + 1 = 19`.
Fixed: `1·1 + 2·2 + 2·2 + 2·3 + 1·2 + 1 + 1 + 1 = 1 + 4 + 4 + 6 + 2 + 1 + 1 + 1 = 20`.
**`steps = 19n + 20`.**

*`ct_select_bytes`* (the same loop; the fixed part adds the `assert_eq!` branch, which
`lower_block` lowers to a `Branch` and a `Trap` block, and the `bool_slot`):

| Step | Cost | Per iteration (count) | Fixed (count) |
|---|---|---|---|
| `Let` | 1 | 5 | 6 |
| `Load` | 2 | 4 (counter, `b[i]`, `a[i]`, `b[i]`) | 2 (last header test, `bool_slot`) |
| `Store` | 2 | 2 (`out[i]`, counter) | 2 (`bool_slot`, counter init) |
| `Alloc` | 3 | 0 | 3 (`bool_slot`, `vec_buf`, counter) |
| `Free` | 2 | 0 | 2 (`bool_slot`, counter) |
| `Jump` | 1 | 1 | 1 |
| `Branch` | 1 | 1 | 2 (the assert, last header test) |
| `Return` | 1 | 0 | 1 |

Per iteration: 19 as above. Fixed: `6 + 4 + 4 + 9 + 4 + 1 + 2 + 1 = 31`.
**`steps = 19n + 31`.**

*`ct_eq_u64`* (no loop, no memory): two `Let`s and a `Return`: `2·1 + 1 = 3`.
**`steps = 3`.**

| Operation | `steps` under 5.1 | Bound | Holds? |
|---|---|---|---|
| `ct_eq_equal_length` | `19n + 20` | `64n + 512` | **yes**: `(64n + 512) − (19n + 20) = 45n + 492 > 0` for every `n ≥ 0` |
| `ct_select_bytes` | `19n + 31` | `64n + 512` | **yes**: difference `45n + 481 > 0` |
| `ct_eq_u64` | `3` | `64` | **yes**: `64 − 3 = 61` |

For comparison, the same functions today: the interpreter gives `25n + 29`,
`25n + 42` and `5` (with the entry call), and the symbolic executor, which the
Warrant `steps` obligation actually uses, gives `11n + 8`, `11n + 15` and `2`. All
three schedules satisfy the bounds. The bounds are loose by a factor of 3 to 6 in the
slope, so the schedule decision does not move any proved claim.

**Findings about these bounds. No bound was changed.**

1. **The rows `eq` and `eq_fixed` measure a different object from the one computed
   above.** The proof target is the **inlined** export (`warrant_proofs.rs`, `lower()`
   calls `inline_calls`), which replaces the `CallFunction` and its `Return` with
   parameter `Let`s and `Jump`s. The Lean claim of VER11a §8.5 is about the
   **un-inlined** program (frames, not inlining). Their costs differ by a small
   constant. This spec did not compute `ct_eq` or `ct_eq_fixed`, which are `ct_eq_equal_length`
   plus a wrapper, and the wrapper is bounded by a few tens of steps against 492
   spare. KRN-006a must recompute them from the un-inlined lowering and record which
   object the `steps` row bounds.
2. **The `heap` rows were never checked** (8.6), so the stack-versus-heap marking is
   what makes them true.
3. **`depth`**: the entry frame is depth 1. `ct_eq` calls one callee, so depth 2, within
   `depth <= 32`. `eq_u64` has depth 1, within `depth <= 8`.

### 8.10 Interpreter alignment: the differential test (KRN-010)

**Purpose.** To catch a disagreement between the Lean semantics and the reference
interpreter. This is evidence about the interpreter. It is not part of any proof's
trust (VER11a §6.5).

**Inputs.** Each case is `(program, entry function, argument buffers and scalars,
capability table, limits)`.

- **Programs:** (a) the lowered functions of `ct-compare`; (b) a hand corpus with one
  program per instruction and terminator kind and per stuck, trap and timeout path;
  (c) generated programs. Every program must be accepted by `TypeChecker::check_program`,
  because 8.5 is only sound for those. A generator must emit control flow that
  can loop, call, trap and get stuck.
- **Arguments:** buffers are allocated with `Interpreter::allocate_bytes`, which gives
  ids `1 .. k`. The Lean entry state uses the same bytes in `mem` and `allocs = k + 1`.
- **Limits:** large enough that none fires, for the main runs. Separate runs use a
  small `max_steps`, `max_call_depth` and `max_memory_bytes` to test `timeout`.
- **Run:** the interpreter, once. The Lean machine with fuel `n` equal to the
  interpreter's `machine_steps` (5.1), plus enough extra fuel for a second run that
  must not change the state, to test that a finished run stays finished.

**Compared, on every outcome:**

1. Outcome kind: `Success` with `ret`, `Trap` with `trap`, `Stuck` with `stuck`,
   `BudgetExceeded` with `timeout` (still `running`).
2. The trace, event by event (`BranchDecision` with `branch`, `MemoryAccess` with
   `memoryAccess` including the width, `CapabilityInvocation` with
   `capabilityInvocation`).
3. `steps`.
4. For `ret`: the returned value. It is not part of `obs`, but it is evidence about the
   value semantics.
5. The final memory: for every id, `size`, `alive`, `kind`, and the bytes; and `heap`
   against `ExecutionCost::heap_bytes`.
6. As a separate check, the same program run on its tagged and untagged form (8.6)
   gives the same interpreter outcome, since the tag must not affect `obs`.

**Allowed differences.** There are exactly these; any other disagreement is an
interpreter bug (8.0).

| # | Difference | Why it is allowed |
|---|---|---|
| D1 | Resource limits. `max_call_depth`, `max_memory_bytes` and `max_steps` end the interpreter's run as `timeout`; the Lean machine has only fuel. | They are limits of the tool, not of WIR. A `timeout` case compares the trace and `steps` of the interpreter at the stop with the Lean state after `machine_steps` steps. The interpreter's rule is: before every step, if `steps ≥ max_steps`, stop. A call that would exceed `max_call_depth`, or an `Alloc` that would exceed `max_memory_bytes`, stops before it runs. |
| D2 | The text of a trap message and the payload of `UndefinedBehavior`. | `obs` forgets both. |
| D3 | Programs the checker rejects. The interpreter stops with `Stuck(TypeMismatch)` or `Trap("... not found")`; Lean reads defaults. | Out of scope by 8.5. |
| D4 | The extra counters of `ExecutionCost` (`allocations`, `allocated_bytes`, `memory_reads`, `memory_writes`, `capability_calls`). | `obs` has no use for them. `heap_bytes` is compared (item 5). |
| D5 | The interpreter counts argument buffers against `max_memory_bytes`. | Resource limit (D1). They are never in `heap`. |
| D6 | The capability table is the test's own, which mirrors `DefaultCapabilityDispatcher` (`rand`, `host_log`). | The semantics takes the table as an input. |

The test is **not** a proof, and it does not run in CI until KRN-007 exists. Its
generator must be seeded and its corpus committed, so a failure reproduces.

### 8.11 The label table

The relational obligation of VER11a §8 needs facts about which values and which
pointers agree between the two runs. They are **data**, produced by the label
inference and the VC builder, **not proofs**. The table gives, for every program
point `(f, b, i)` (including the terminator point of 8.1), three lists of variables
(`VarId`s of 8.1):

| List | Meaning |
|---|---|
| `pub` | variables whose value is equal in both runs |
| `pubPtr` | variables that are `pub` and point into an allocation whose tag is **public** (so a load through them reads equal data) |
| `secPtr` | variables that are `pub` and point into an allocation whose tag is **secret** (so a store of a secret through them is allowed) |

`pubPtr` and `secPtr` are needed because the real lowering keeps the loop counter
in a stack slot and reloads `i` from it on every trip (8.6, `Stmt::For`). So "`i` is
public" is true only because that slot is public-tagged (KRN-S0 REPORT.md, F2).

**The table is untrusted, and the kernel decides it.** For each program point the
kernel checks, by `decide` on the concrete program and table, that the obligation
of that point holds under the table's facts. The obligation is the one of its
instruction or terminator kind, proved once over symbolic states in `warrant-lib`.
A wrong table makes some point fail; it can never make a false claim pass. VER11a
§9.2 follows this form: the VC builder emits the program and this table, and no
generated tactic text (KRN-S0 REPORT.md, F1). Two further facts the relation
carries, from the same report, are stated in VER11a: a callee's result is treated as
secret in the caller (`RetOK`), and the `∀ q ≥ allocs` invariant of 8.6.

Label inference must carry the pointer facts through copies: `let q = p` must put
`q` in the list that `p` is in.

### 8.12 What the semantics covers, and leaves out

Covered, for the first kernel version: the subset ct-compare's lowering uses (`Let`,
`Alloc`, `Load`, `Store`, `Free`, `CallFunction`, `Return`, `Jump`, `Branch`, `Trap`,
`Unreachable`; integer, bool and pointer values; `PtrOffset`, `PtrLen`). Written but
without kernel proofs yet (VER11a §6.4): `CallCapability`, tuples and dynamic values.
Nothing in this section is concurrent.

### 8.13 Interpreter changes this section requires (KRN-006a and KRN-010)

One list, for the person who writes the code. Each is already argued above.

1. `Alloc` carries `kind` and `secret`; `Memory` stores both; `heap_bytes` counts
   `Heap` only (8.6).
2. Charge by kind, once per completed step; charge nothing for a stuck step; drop the
   blanket per-instruction charge; the entry frame is free; `Trap` costs 1 (8.9).
3. Check the budget before every step, including terminators (8.9).
4. Record a memory event only after the access check **and** the decode or encode
   succeed; `Free` needs offset 0 (8.7).
5. An overflow in mode `Trap` ends with `trap`, not `stuck` (8.5).
6. Storing a pointer, and loading a `Ptr` type, are `stuck` (8.2).
7. Every outcome carries `cost` and `trace`; add `machine_steps` (8.8).
8. A call that would exceed `max_call_depth` stops before it runs (8.10, D1).
9. `check.rs` gains `UseBeforeDefinition`, `NoEntryBlock` and `BlockIdsNotDense` (8.5).
10. The stack-versus-heap marking in `lower.rs` comes from the construct, and the
    `name == "acc"` test goes (8.6).
11. `sym.rs` is aligned with 5.1 (8.9).

### 8.14 What the KRN-S0 spike differs on, and what VER11a still says

For KRN-007, the port of `KrnS0/Semantics.lean` changes:

| Spike | This section |
|---|---|
| `costRet := 2` | 1 |
| `Jump` appends a `branch` event | no event |
| `Free` requires `off < size`, so freeing a zero-size allocation is stuck | offset 0 and alive |
| memory cells hold `Val`, one per offset | bytes (8.2) |
| the pointer carries `len` | `PtrLen` reads the allocation |
| the event has no width | the access event has the width |
| `eval` is total, with no stuck case | `eval` returns value, `trap` or `stuck` (8.5) |
| `Outcome` has `running` | the kind of a running state is `timeout` |
| `secret : Bool` on the allocation, copied from the instruction | the same (8.6) |

VER11a still says things this section corrects, and this change does not edit that
spec. The follow-ups, for whoever amends it next: §8.4 says the loop body has
"bounds checks" and "bounds-check branches agree" (there are none, 8.7); §6.2 gives
`vars : VarId → Option Value` (total, 8.5) and names a `Status` with a trap reason;
§8.2's `Lockstep` can drop the `newEvents` and cost clauses and the `running`
guard once `R` carries trace and step equality; §8.3's `R` needs the
freshness field, the pointer facts (`pubPtr`, `secPtr`) and `RetOK`, and does not
need `WfState` (KRN-S0 REPORT.md, F2); §9.2 becomes the "emit data, check by
`decide`" form of 8.11 (F1); and §13 gains the proposed id
`wir.typing.checker-guarantees` of 8.5.
