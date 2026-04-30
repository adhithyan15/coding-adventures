# LANG24 — Constraint-VM
## (a generic SAT/SMT-class constraint executor, the way Logic-VM is for Prolog)

## Overview

Constraints — "find values that make this predicate true," "prove
this predicate is unsatisfiable," "compute the strongest postcondition
of this expression given these inputs" — show up in many places in a
modern programming-language toolchain.  Refinement types
([LANG23](LANG23-refinement-types-and-solver-tier.md)) are one
consumer.  Property-based test generation, build-system version
resolution, register allocation, dead-code elimination, symbolic
execution, runtime user-facing `(solve ...)` builtins — every one of
them either *is* a constraint-solving problem or has one as a
sub-problem.

The pattern this repo has used for analogous problems is to build a
**generic VM** with **language-style frontends** layered on top.
Logic programming (Prolog) ships as `logic-core` (data + algorithms)
+ `logic-instructions` (IR) + `logic-engine` (search backend) +
`logic-vm` (instruction-stream executor); the Prolog frontend is just
one consumer.

LANG24 specifies the same shape for constraint solving:

```
   ┌─ constraint-core ─────────┐    primitives: predicate AST,
   │  predicates, theories,    │    normalization, factorization,
   │  normalisation, congruence│    theory-dispatch trait
   └────────────┬──────────────┘
                │
                ▼
   ┌─ constraint-instructions ─┐    IR — `DECLARE_VAR`, `ASSERT`,
   │  the constraint-VM IR     │    `CHECK_SAT`, `GET_MODEL`,
   └────────────┬──────────────┘    `PUSH_SCOPE`, `POP_SCOPE`, …
                │
                ▼
   ┌─ constraint-engine ───────┐    solver tactics: SAT (CDCL),
   │  pluggable solver backends│    LIA (Cooper), LRA (simplex),
   └────────────┬──────────────┘    arrays, bit-vectors, EUF
                │
                ▼
   ┌─ constraint-vm ───────────┐    instruction-stream executor:
   │  walks the IR + drives    │    walks programs, dispatches
   │  the engine               │    opcodes to the engine
   └────────────┬──────────────┘
                │
   ┌────────────┴───────────────────────────────────────────────────┐
   │  Consumers (each lowers its problem to constraint-instructions):│
   │                                                                 │
   │  • lang-refinement-checker (LANG23 — type-check refinements)    │
   │  • prop-test (future — generate inputs satisfying invariants)   │
   │  • semver-resolver (future — Cargo/npm version solving)         │
   │  • symbolic-exec (future — path-condition solving)              │
   │  • smt-lib-import (industry-standard input format)              │
   │  • lispy/twig `(solve ...)` runtime builtin                     │
   └─────────────────────────────────────────────────────────────────┘
```

The consumers don't know they share a backend.  They write to
`constraint-instructions` and ask `constraint-vm` for satisfiability
/ a model / a proof.  The VM is language-agnostic and theory-agnostic;
the engines are theory-specific tactics that compose.

LANG24 ships the substrate.  LANG23's `lang-smt` crate, originally
specced as part of refinement types, is **extracted to here** — the
refinement-checker becomes a thin frontend that lowers
`Predicate` to `ConstraintInstructions`.

---

## Why this is needed now

LANG23 (refinement types) explicitly specified a hand-rolled
`lang-smt` crate as part of its scope.  As soon as a second use case
showed up — and once you start looking, they're everywhere — bundling
the solver inside one consumer becomes a coupling problem.  Two
options:

1. **Copy-paste the solver into each consumer.**  Five crates with
   five subtly-different LIA solvers; bug fixes propagate manually.
   This is the failure mode every monolithic-language project has
   eventually regretted.
2. **Extract the solver into a shared substrate.**  One implementation,
   N consumers, the LANG-VM "build the universal substrate" ethos
   applied to constraint solving.

LANG24 is option 2.  LANG23 becomes the first consumer; future
consumers plug in without solver duplication.

The Logic-VM precedent makes the architecture decision easy: this
repo *already* knows how to build a "VM for X" cleanly.  `logic-core`
+ `logic-instructions` + `logic-engine` + `logic-vm` worked; the
parallel `constraint-*` family will work the same way.

### What changes for LANG23

LANG23's `lang-smt` crate is removed from its scope.  Refinement
type-checking becomes:

```
refinement annotation in Twig source
    ↓ frontend lowering
RefinedType { kind, predicate }                    (still LANG23)
    ↓ refinement checker
ConstraintInstructions: ASSERT predicate; CHECK_SAT  (NEW: LANG24)
    ↓ constraint-vm
SAT / UNSAT / UNKNOWN
    ↓ refinement checker decides
PROVEN_SAFE / PROVEN_UNSAFE / UNKNOWN (compile error / strip check / runtime check)
```

The LANG23 spec gets an addendum noting this dependency change.

---

## Relationship to existing specs

| Spec | What it gives | What LANG24 changes |
|------|---------------|---------------------|
| LP00–LP08 | Logic-VM stack: `logic-core`/`logic-engine`/`logic-instructions`/`logic-vm` | **Architectural precedent.**  Constraint-VM mirrors the layering exactly. |
| LANG01 | InterpreterIR for executable programs | Constraint-VM has its own IR (`ConstraintInstructions`); the two IRs don't overlap. |
| LANG02 | vm-core (program interpreter) | Constraint-VM is a separate VM; it doesn't run on vm-core.  The `(solve ...)` runtime builtin invokes constraint-vm via a normal builtin call. |
| LANG20 | LangBinding trait | The `(solve ...)` builtin is registered through the binding's normal builtin-resolution path; no LangBinding extension needed. |
| LANG22 | Typing spectrum + AOT/JIT/PGO | When AOT-PGO promotes a refinement-typed function, codegen invokes constraint-vm at compile time to discharge proof obligations.  No runtime cost. |
| LANG23 | Refinement types + solver tier | **Removed:** `lang-smt` crate.  **Added:** "consume LANG24" via `lang-refinement-checker`.  See §"What changes for LANG23" above. |

---

## Architecture

### The four-crate layered stack (mirrors LP00/07/08 exactly)

```
┌──────────────────────────────────────────────────────────────────┐
│  constraint-core                                                 │
│    Pure data + algorithms.  Predicate AST, theory enum,          │
│    normalisation passes (NNF, CNF, simplification),              │
│    congruence closure for EUF, term ordering for AC theories.    │
│    No I/O, no solver state.  ~3-5 KLOC.                          │
└──────────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌──────────────────────────────────────────────────────────────────┐
│  constraint-instructions                                         │
│    The IR.  Same role as `interpreter-ir` for the LANG-VM and    │
│    `logic-instructions` for the Logic-VM.  Opcodes:              │
│      DECLARE_VAR / DECLARE_FN  — introduce a variable or         │
│                                  uninterpreted function          │
│      ASSERT predicate          — add a constraint                │
│      CHECK_SAT                 — ask: is the conjunction         │
│                                  satisfiable?                    │
│      GET_MODEL                 — extract a satisfying assignment │
│                                  (only legal after SAT)          │
│      GET_UNSAT_CORE            — extract the minimal             │
│                                  unsat-explaining subset         │
│      PUSH_SCOPE / POP_SCOPE    — incremental solving (push       │
│                                  before exploration, pop to undo)│
│      RESET                     — clear all assertions            │
│      SET_LOGIC                 — declare which theories the      │
│                                  program uses (QF_LIA, QF_LRA,   │
│                                  QF_BV, QF_AUFLIA, …)            │
│      ECHO / SET_OPTION         — diagnostics + tuning            │
│    ~500-line crate; pure data shape.                             │
└──────────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌──────────────────────────────────────────────────────────────────┐
│  constraint-engine                                               │
│    Pluggable solver tactics, one per theory:                     │
│      sat-tactic        — DPLL with conflict-driven clause        │
│                          learning (CDCL).  The boolean core.     │
│      lia-tactic        — Cooper's algorithm for linear integer   │
│                          arithmetic.  Decidable, complete.       │
│      lra-tactic        — simplex / Fourier-Motzkin for linear    │
│                          real arithmetic.                        │
│      array-tactic      — congruence closure + array axioms.     │
│      bv-tactic         — bit-blasting (theory of bit-vectors).   │
│      euf-tactic        — equality + uninterpreted functions.     │
│      qe-tactic         — quantifier elimination (Presburger).   │
│    Tactics compose via `Theory::join` — DPLL(T) / Nelson-Oppen.  │
│    ~5-15 KLOC depending on which theories are enabled.           │
└──────────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌──────────────────────────────────────────────────────────────────┐
│  constraint-vm                                                   │
│    Instruction-stream executor.  Walks `ConstraintInstructions`  │
│    one opcode at a time, dispatches each to the engine,          │
│    accumulates state (asserted predicates + push/pop scopes),    │
│    surfaces tracing for the equivalent of LANG18 coverage and    │
│    LANG11 profiling.  ~500 LOC; the easy crate.                  │
└──────────────────────────────────────────────────────────────────┘
```

### Data flow per consumer

```text
Consumer                       lowers to                     consumes from
────────────────────────────  ──────────────────────────────  ─────────────
Refinement checker (LANG23)    obligation graph from CFG  →   SAT / UNSAT
                                                                / counter-example

Property-test generator        invariant + free vars      →   model = test inputs

Semver resolver                version constraints        →   SAT / UNSAT (and minimal
                                                                conflict set)

Symbolic execution             path conditions            →   per-path SAT / UNSAT

(solve …) runtime builtin       user-supplied predicate    →   model OR enumeration
```

Each consumer is a thin lowering; the heavy lifting lives in
constraint-vm.  Bug fixes in the SAT solver fix every consumer at
once.  Adding a new theory (say, strings, for regex constraints) ships
to every consumer without per-consumer changes.

---

## Theories supported (versioned scope)

| Theory | Decidable? | Tactic | v1 ship | Future |
|--------|-----------|--------|---------|--------|
| Boolean (SAT) | Yes (NP-complete) | DPLL/CDCL | ✅ | – |
| LIA — linear integer arithmetic | Yes | Cooper / Omega | ✅ | – |
| LRA — linear real arithmetic | Yes (in P, simplex) | Simplex | – | v2 |
| Theory of arrays (read-write) | Yes | Congruence closure + array axioms | – | v2 |
| Bit-vectors (BV) | Yes | Bit-blasting → SAT | – | v2 |
| EUF — equality + uninterpreted functions | Yes | Congruence closure | – | v2 |
| Strings | Decidable fragments only | Word eqs / regex | – | v3 |
| Quantifiers (∀∃) | Undecidable in general | Quantifier elimination + heuristics | – | v3 |
| Non-linear arithmetic | Undecidable in general | Interval arithmetic + heuristics | – | v3 |
| Floating-point | Decidable but expensive | FPA → bit-vectors | – | v3 |

**v1 is intentionally narrow.**  SAT + LIA covers refinement types'
v1 vocabulary completely (which is exactly the spec's design — see
LANG23 §"Predicate vocabulary").  Other consumers can ship the
moment SAT + LIA are working; richer theories arrive incrementally.

The theory enum is `#[non_exhaustive]` so adding theories doesn't
break consumers.

---

## ConstraintIR shape

Layered exactly like `interpreter-ir` for code:

```rust
/// One constraint-VM instruction.  Mirrors LP07's IIR shape.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum ConstraintInstr {
    /// Introduce a variable of the given sort.  Sort is one of
    /// Bool, Int, Real, BitVec(width), Array(idx_sort, val_sort),
    /// or Uninterpreted(name).
    DeclareVar { name: String, sort: Sort },

    /// Introduce an uninterpreted function symbol.
    /// `f: Sort1 × Sort2 × … → SortN`.
    DeclareFn { name: String, arg_sorts: Vec<Sort>, ret_sort: Sort },

    /// Assert that `pred` holds.  Multiple Asserts conjoin.
    Assert { pred: Predicate },

    /// Ask the engine: is the conjunction of all currently-asserted
    /// predicates satisfiable?
    CheckSat,

    /// After a CheckSat that returned SAT, extract the satisfying
    /// assignment.  Returns Map<VarName, Value>.
    GetModel,

    /// After a CheckSat that returned UNSAT, extract the minimal
    /// subset of asserted predicates that explains unsatisfiability.
    GetUnsatCore,

    /// Push a new scope; subsequent Asserts can be undone by Pop.
    /// Maps to incremental-solving stack semantics.
    PushScope,
    PopScope,

    /// Clear all assertions.  Equivalent to `(reset)` in SMT-LIB.
    Reset,

    /// Declare which theory family the program uses.  Allows the
    /// VM to pick the right tactics + reject input that uses
    /// unsupported features.
    SetLogic { logic: Logic },

    /// Print a diagnostic to the trace channel.  Maps to SMT-LIB
    /// `(echo "...")` and Logic-VM's tracing convention.
    Echo { msg: String },

    /// Tune solver behaviour: timeout, model-completeness,
    /// random seed, etc.
    SetOption { key: String, value: OptionValue },
}

/// The predicate AST.  Same lattice as LANG23's `Predicate` but
/// extracted to constraint-core so other consumers reuse it.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum Predicate {
    /// Boolean literal (true / false).
    Bool(bool),

    /// Variable reference (declared via DeclareVar).
    Var(String),

    /// Integer literal.
    Int(i128),

    /// Real literal (rational; arbitrary precision via num-rational).
    Real(Rational),

    /// Application of an uninterpreted function (DeclareFn).
    Apply { f: String, args: Vec<Predicate> },

    /// Boolean operators.
    And(Vec<Predicate>),
    Or(Vec<Predicate>),
    Not(Box<Predicate>),
    Implies(Box<Predicate>, Box<Predicate>),
    Iff(Box<Predicate>, Box<Predicate>),

    /// Equality and disequality.
    Eq(Box<Predicate>, Box<Predicate>),
    NEq(Box<Predicate>, Box<Predicate>),

    /// Linear arithmetic.
    Add(Vec<Predicate>),
    Sub(Box<Predicate>, Box<Predicate>),
    Mul { coef: i128, term: Box<Predicate> }, // linear: only int × var
    Le(Box<Predicate>, Box<Predicate>),
    Lt(Box<Predicate>, Box<Predicate>),
    Ge(Box<Predicate>, Box<Predicate>),
    Gt(Box<Predicate>, Box<Predicate>),

    /// Conditional (ite — if-then-else for predicates).
    Ite(Box<Predicate>, Box<Predicate>, Box<Predicate>),

    /// Quantifiers (v3).  `forall x: Sort. body`.
    Forall { var: String, sort: Sort, body: Box<Predicate> },
    Exists { var: String, sort: Sort, body: Box<Predicate> },

    /// Array operations (v2).
    Select { arr: Box<Predicate>, idx: Box<Predicate> },
    Store { arr: Box<Predicate>, idx: Box<Predicate>, val: Box<Predicate> },
}
```

`Predicate` is `non_exhaustive`; new theories add new variants
without breaking consumers that match on the existing set.

### Why a separate `constraint-instructions` crate?

Because the IR is consumed by *more than one driver*:

- **constraint-vm** — the canonical executor.  Walks instructions,
  drives the engine.
- **smt-lib serialiser** — emits standard SMT-LIB v2 textual format.
  Lets `constraint-vm` programs interoperate with industry solvers
  (Z3, CVC5) for verification; lets users import existing SMT-LIB
  benchmarks.
- **z3-bridge** (optional v2 fallback) — translates
  ConstraintInstructions to Z3's native AST when our hand-rolled
  engine times out on a hard query.  Only active if Z3 is linked;
  default builds don't depend on Z3.
- **debugger / coverage / profiler** — tools that observe a
  constraint-program's execution use the IR as their grammar (the
  LANG18 / LANG11 / debug-sidecar patterns generalised to
  constraints).

Decoupling instructions from the executor mirrors LP07 vs LP08
exactly.

---

## Constraint-VM execution model

The VM is a simple dispatch loop, mirroring LP08's design:

```rust
/// One activation of the constraint-VM.  Holds the engine + scope
/// stack + tracing.
pub struct ConstraintVM {
    /// The pluggable solver engine.  Composed of theory tactics.
    engine: Box<dyn Engine>,
    /// Push/pop stack of asserted-predicate sets.
    scopes: Vec<ScopeFrame>,
    /// Trace channel — every instruction's effect for debug /
    /// coverage tooling.
    trace: TraceSink,
    /// Active logic family (SAT-only / QF_LIA / QF_AUFLIA / …).
    logic: Logic,
    /// Configurable options (timeout, model completeness, …).
    options: SolverOptions,
}

impl ConstraintVM {
    pub fn execute(&mut self, program: &[ConstraintInstr]) -> Result<VmReport, VmError> {
        for instr in program {
            self.step(instr)?;
        }
        Ok(self.snapshot())
    }

    pub fn step(&mut self, instr: &ConstraintInstr) -> Result<(), VmError> {
        // dispatch — same shape as LP08's step()
        match instr {
            ConstraintInstr::Assert { pred } => self.engine.assert(pred)?,
            ConstraintInstr::CheckSat => { self.last_check = self.engine.check_sat()?; }
            // …
        }
        Ok(())
    }
}
```

The VM owns the engine (`Box<dyn Engine>`) so different deployments
can swap it: hand-rolled for default builds, Z3-backed for opt-in
"hard query" handling, MockEngine for tests.

### Resource limits (the same defensive-cap pattern as
`twig-vm::dispatch`)

| Constant | Default | What it bounds |
|----------|---------|----------------|
| `MAX_VM_INSTRUCTIONS` | 2²⁴ | Per-`execute` instruction count |
| `MAX_ASSERTED_PREDICATES` | 2¹⁶ | Predicates in scope at once |
| `MAX_SCOPE_DEPTH` | 256 | Push-depth |
| `MAX_VARIABLES` | 2¹⁶ | DeclareVar count |
| `SOLVER_TIMEOUT_MS` | 5000 | Per-CheckSat wall-clock cap (configurable per-call) |

Every cap is public + unit-tested.  Solver timeout returns
`SatResult::Unknown` rather than blocking forever.

---

## Engine tactics

Each tactic implements a small trait:

```rust
pub trait Tactic: Send + Sync {
    /// Add a predicate to the tactic's working set.  Returns Err
    /// if the predicate is outside the tactic's vocabulary.
    fn assert(&mut self, pred: &Predicate) -> Result<(), TacticError>;

    /// Returns Sat / Unsat / Unknown for the conjunction of
    /// currently-asserted predicates.
    fn check_sat(&mut self, timeout_ms: u64) -> Result<SatResult, TacticError>;

    /// Get the current model (only valid after Sat).
    fn get_model(&self) -> Option<Model>;

    /// Get an UNSAT core (only valid after Unsat).  Optional —
    /// tactics that don't support cores return None.
    fn get_unsat_core(&self) -> Option<Vec<Predicate>> { None }

    /// Push / pop scopes.  Tactics that don't support incremental
    /// solving use a slow-path: snapshot/restore the working set.
    fn push(&mut self) {}
    fn pop(&mut self) {}
}
```

Tactics compose via Nelson-Oppen / DPLL(T): the SAT tactic drives the
search, and theory tactics handle leaves of theories they own.  The
`Engine` orchestrates the composition.

### v1 tactic implementations

- **`SatTactic`** (CDCL): clauses, watched literals, two-watched
  scheme, conflict-driven backjumping, restart heuristic.  ~2-3
  KLOC; well-understood.  Reference: MiniSat for the algorithm
  shape.
- **`LiaTactic`** (Cooper's algorithm): quantifier elimination over
  Presburger arithmetic.  Complete for closed integer-arithmetic
  formulas.  ~1-2 KLOC.  Reference: original Cooper 1972 paper +
  Bjorner's modern presentation.

Both ship as Lispy programs running on the LANG VM (eat-own-dogfood),
with a Rust shim providing the FFI surface — same pattern as the
solver-as-Lispy proposal in LANG23.  Hand-rolled in Rust first; the
Lispy port comes later as an optimization (`PR 24-J` parallel to
LANG23's PR 23-J).

---

## Per-frontend lowering

Each consumer is a thin crate that emits ConstraintInstructions:

### Refinement type checker (LANG23)

```rust
// In `lang-refinement-checker`:
fn discharge_obligation(
    obligation: &RefinementObligation,
    vm: &mut ConstraintVM,
) -> Result<DischargeOutcome, RefinementError> {
    let mut program = Vec::new();
    program.push(ConstraintInstr::PushScope);
    for (var, type_) in &obligation.in_scope {
        program.push(ConstraintInstr::DeclareVar {
            name: var.clone(),
            sort: lower_sort(&type_.kind),
        });
        if let Some(pred) = &type_.predicate {
            program.push(ConstraintInstr::Assert { pred: lower_pred(pred) });
        }
    }
    // The obligation itself: assert the *negation* and check sat.
    // SAT means counter-example exists → PROVEN_UNSAFE.
    // UNSAT means no counter-example → PROVEN_SAFE.
    program.push(ConstraintInstr::Assert {
        pred: Predicate::Not(Box::new(lower_pred(&obligation.must_hold))),
    });
    program.push(ConstraintInstr::CheckSat);
    program.push(ConstraintInstr::PopScope);

    let report = vm.execute(&program)?;
    match report.last_check {
        SatResult::Unsat => Ok(DischargeOutcome::ProvenSafe),
        SatResult::Sat => Ok(DischargeOutcome::ProvenUnsafe(report.last_model)),
        SatResult::Unknown => Ok(DischargeOutcome::Unknown),
    }
}
```

The refinement checker now owns *zero* solver code.  It owns
*lowering* code: refinement-language predicates → ConstraintIR
predicates.

### Property-based test generation (future)

```rust
// User writes a Lispy property:
//   (property (forall (x : (Int 0 100))) (= (square x) (* x x)))
// We want to generate inputs that violate it (if any).
fn generate_counterexample(prop: &Property, vm: &mut ConstraintVM)
    -> Option<TestInput>
{
    let mut program = Vec::new();
    for var in prop.free_vars() {
        program.push(ConstraintInstr::DeclareVar {
            name: var.name.clone(),
            sort: lower_sort(&var.sort),
        });
        if let Some(bound) = &var.bound {
            program.push(ConstraintInstr::Assert { pred: lower_pred(bound) });
        }
    }
    // Assert the *negation* of the property — find a witness that
    // violates it.
    program.push(ConstraintInstr::Assert {
        pred: Predicate::Not(Box::new(lower_pred(&prop.body))),
    });
    program.push(ConstraintInstr::CheckSat);
    program.push(ConstraintInstr::GetModel);
    let report = vm.execute(&program).ok()?;
    if report.last_check == SatResult::Sat {
        Some(TestInput::from_model(report.last_model))
    } else {
        None  // Property holds for all bounded inputs.
    }
}
```

### Semver / build resolver (future)

```rust
// Given user requirements like:
//   foo: ">=1.2,<2.0"
//   bar: "^3.4"
// And a registry of available versions, find an assignment that
// satisfies them.
fn resolve(requirements: &[Requirement], registry: &Registry,
           vm: &mut ConstraintVM) -> Option<ResolutionPlan> {
    let mut program = Vec::new();
    for pkg in requirements.iter().map(|r| r.package_name()) {
        // One Int variable per package representing the chosen version index.
        program.push(ConstraintInstr::DeclareVar {
            name: format!("v_{pkg}"),
            sort: Sort::Int,
        });
    }
    for req in requirements {
        program.push(ConstraintInstr::Assert {
            pred: lower_semver_constraint(req, registry),
        });
    }
    // Plus transitive constraints derived from each candidate version's deps.
    // …
    program.push(ConstraintInstr::CheckSat);
    program.push(ConstraintInstr::GetModel);
    let report = vm.execute(&program).ok()?;
    (report.last_check == SatResult::Sat).then(|| ResolutionPlan::from(report))
}
```

### `(solve ...)` runtime builtin (Lispy/Twig)

```scheme
;; The user-facing API for constraint solving inside a running
;; LANG-VM program.  Calls into constraint-vm via the same
;; LangBinding builtin-resolution path as `+` or `cons`.
(solve '(int) '(and (>= x 1) (<= x 100) (= (modulo x 7) 3)))
;; → 3   (the smallest x satisfying the constraint)

(solve-all '(int) '(and (>= x 1) (<= x 5)))
;; → (1 2 3 4 5)

(solve-sat? '(and (> x 5) (< x 3)))
;; → #f
```

The Lispy frontend's runtime registers these as builtins via
`LispyBinding::resolve_builtin`.  Each invocation builds a
ConstraintInstructions program, invokes the embedded constraint-vm,
returns a value.

### SMT-LIB import (industry standard)

```sh
$ constraint-vm --input benchmark.smt2
sat
((x 7) (y 13))
```

A small parser turns SMT-LIB v2 text into ConstraintInstructions.
Lets us run the standard SMT-LIB benchmark suite as part of CI to
catch regressions.

---

## Use case catalog (why decoupling matters)

| Use case | Predicate vocabulary | Notes |
|----------|---------------------|-------|
| Refinement type checking (LANG23) | LIA + Boolean + EUF | The motivating case |
| Property-based test generation | Same as the language under test | Generates concrete inputs that violate properties |
| Semver / build dependency resolution | Boolean + small LIA | Same shape as Cargo's resolver |
| Symbolic execution (security analysis) | Path-condition theory + heap model | Klee-style |
| Whole-program reachability | Boolean + LIA | "is this branch dead?" |
| Constant folding in optimizer | LIA | "x + 0 = x" type rewrites — small queries that benefit from a real solver |
| Register allocation | Boolean (graph colouring) | Many compilers already use SAT/IP for this |
| Test minimisation (delta debugging) | Boolean | "find the smallest failing input" — UNSAT-core driven |
| User-facing `(solve …)` builtin | Whatever the user wrote | Exposes the substrate to the language directly |
| Linker constraint solving | Boolean + LIA (offsets) | Symbol resolution + section layout |
| Scheduler / instruction-scheduling | LIA | Dependency-graph + latency constraints |

Every entry uses the same backend.  A theory addition (say, strings
in v3) ships value to *every* row that needs strings — the
network-effect benefit of the substrate approach.

---

## Crate structure

```
code/packages/rust/
├─ constraint-core/                  NEW
│   Predicate AST, Sort, Logic, theory enum, normalisation
│   passes (NNF, CNF, simplification), congruence closure helper.
│   Pure data; no solver state.  ~3-5 KLOC.
│
├─ constraint-instructions/          NEW
│   ConstraintInstr, programs, scope semantics.  ~500 LOC; pure
│   data shape, no logic.  Mirrors `interpreter-ir` for code.
│
├─ constraint-engine/                NEW
│   Tactic trait + v1 implementations:
│     sat-tactic (CDCL)
│     lia-tactic (Cooper)
│   Plus the Engine combinator that does Nelson-Oppen / DPLL(T)
│   to mix theories.  ~5-10 KLOC.
│
├─ constraint-vm/                    NEW
│   Instruction-stream executor.  ~500 LOC.
│
├─ smt-lib-format/                   NEW (parallel deliverable)
│   Read/write SMT-LIB v2 textual format.  Pure data; lets CI
│   benchmark against industry suite.
│
└─ Consumers (each ~1-2 KLOC of lowering):
    ├─ lang-refinement-checker  → consumes constraint-vm  (LANG23)
    ├─ prop-test                → future
    ├─ semver-resolver          → future
    └─ symbolic-exec            → future
```

The four `constraint-*` crates form a vertically-decomposed stack
analogous to the four `logic-*` crates already in the repo.  Same
naming, same layering, same interface boundaries.

---

## Migration path

LANG24 is **independent of the LANG20 PR 4-8 stream that's in flight**.
Can land in parallel with the AOT/dev-tools/refinement streams.  The
sequencing of LANG24's own PRs:

| PR | Scope | Acceptance | Unblocks |
|----|-------|-----------|----------|
| 24-A | `constraint-core`: Predicate AST, Sort, Logic, normalisation passes.  Pure data, no solver. | Round-trip every Predicate variant through normalisation; CNF conversion produces equivalent formulas. | 24-B, 24-C, 24-D, all consumers. |
| 24-B | `constraint-instructions`: the IR.  Pure data. | All opcodes round-trip through Debug + an early text serialiser. | 24-D, 24-E, smt-lib-format. |
| 24-C | `constraint-engine` v1: sat-tactic (CDCL) + lia-tactic (Cooper).  Hand-rolled in Rust. | Discharges all 100 standard SMT-LIB QF_LIA benchmarks correctly. | 24-D, all consumers. |
| 24-D | `constraint-vm`: instruction-stream executor.  Drives the engine. | Executes a hand-built ConstraintInstructions program; surfaces SAT / UNSAT / UNKNOWN; respects scope push/pop. | All consumers. |
| 24-E | `smt-lib-format`: read/write SMT-LIB v2 textual format. | Round-trips a corpus of SMT-LIB benchmarks. | CI benchmarking, industry interop. |
| 24-F | First consumer: extract `lang-smt` out of LANG23.  `lang-refinement-checker` lowers refinement obligations to ConstraintInstructions. | LANG23 refinement-type tests pass against the new backend. | LANG23's roadmap re-enabled. |
| 24-G | Lispy `(solve …)` runtime builtins: routes through constraint-vm. | All five `solve*` shapes from §"Per-frontend lowering" pass tests. | User-facing constraint solving in LANG-VM programs. |
| 24-H | Z3 fallback bridge (opt-in cargo feature): translates ConstraintInstructions to Z3 AST for hard queries. | Hard SMT-LIB benchmarks our hand-rolled engine times out on get answered correctly via the bridge. | Production deployments needing >LIA theories. |
| 24-I | Engine v2 tactics: LRA (simplex), arrays, EUF, BV. | Each new tactic ships with its corresponding SMT-LIB benchmark coverage. | More-expressive consumers; richer property-test generation. |
| 24-J | Re-implement `sat-tactic` + `lia-tactic` as Lispy programs running on the LANG-VM (eat-own-dogfood).  Rust impls become fallback. | Performance parity within 2× of the Rust impl on the v1 benchmark. | The LANG-VM JIT becomes the constraint-engine's JIT for free. |

**MVP at PR 24-D** unblocks every consumer (PR 24-F starts the day
after, fixing LANG23's roadmap; PR 24-G ships the user-facing
builtin).  PRs 24-A through 24-D are the **critical path**; 24-E
through 24-J are parallel additions.

### Highest-leverage first PRs

- **PR 24-A** (`constraint-core`) and **PR 24-B**
  (`constraint-instructions`) are pure data crates.  They ship
  with no dependencies on each other; they ship *immediately*
  given the spec; downstream PRs build against them.  By the
  parallel-execution rule, these are the top-priority unblockers.
- **PR 24-C** (`constraint-engine` v1) is the heaviest implementation
  PR in the family but unblocks every consumer.  Land third.
- After 24-D, **all consumers are simultaneously startable**:
  24-E (smt-lib import), 24-F (refinement checker), 24-G (`solve`
  builtin), and any future consumer.

### Update needed in LANG23

LANG23 §"Solver architecture" and §"Migration path" call for a
`lang-smt` crate.  When LANG24 lands, LANG23 gets an addendum:

```diff
- 6. **`lang-smt` crate, LIA fragment**.  Hand-rolled Cooper algorithm
-    in Rust...
+ 6. **CONSUMER OF LANG24**.  `lang-refinement-checker` lowers
+    refinement obligations to LANG24's ConstraintInstructions and
+    delegates to constraint-vm.  No solver code lives in LANG23.
```

The `lang-smt` symbol becomes an alias for `constraint-vm` for
backwards-compatibility during transition; future PRs delete the
alias.

---

## Risk register

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Hand-rolled SAT solver has subtle correctness bugs | Medium-High | All consumers see wrong answers | Differential testing against Z3 via PR 24-H bridge (cross-check, not dependency); SMT-LIB benchmark suite as acceptance gate; aggressive fuzz-testing |
| Hand-rolled LIA solver too slow on real workloads | Medium | Refinement type-check times grow | Z3 bridge as escape hatch (PR 24-H); profile-guided optimisation of hot paths; defer-to-Z3 timeout configurable per consumer |
| Theory composition (DPLL(T)) bugs | Medium | Mixed-theory queries get wrong answers | One tactic at a time in v1; enable theory composition (Nelson-Oppen) only in v2 with explicit acceptance tests for cross-theory examples |
| Lispy self-hosting bootstrap problem (PR 24-J only) | Lower | Constraint-VM can't bootstrap | Keep Rust solver as fallback; the Lispy version is an optimisation, not a dependency |
| SMT-LIB compatibility drift from spec | Low | Consumers can't share benchmarks | Conformance test suite from the official SMT-LIB benchmarks; CI gate |
| Constraint-VM's instruction stream gets out of sync with engine capabilities | Medium | Programs assert predicates the active tactics can't handle | `SetLogic` instruction declares required theories; VM rejects programs whose logic isn't supported by the loaded engines |
| Resource caps too tight for real consumers | Low (numbers chosen generously) | Spurious refusals | All caps configurable per VM-instance; reasonable defaults plus an override API |
| Cross-language predicate vocabulary divergence | Lower | Refinement annotations in different frontends don't share the same lattice | The `Predicate` enum is canonical; each frontend lowers to it.  No frontend extends `Predicate` privately. |

---

## Open questions

1. **Embedded constraint-vm vs. external solver process.**  Should
   constraint-vm run in-process for refinement type-checking
   (fast, no IPC) or as a separate process (isolation, OOM
   protection)?  **Recommendation:** in-process by default,
   process-isolated as an opt-in cargo feature for hostile-input
   contexts (e.g. running user-supplied `(solve …)` queries from
   untrusted input).

2. **Caching discharged obligations across builds.**  The same
   refinement obligation discharged in build N+1 with the same
   inputs has the same answer.  Should constraint-vm offer a
   built-in cache, or is that the consumer's job?  **Recommendation:**
   consumer's job; constraint-vm exposes deterministic execution
   so consumers can hash inputs and cache.

3. **Concurrent solving.**  Multiple parallel `check-sat` calls
   from different threads on different programs — should
   constraint-vm be `Sync`?  **Recommendation:** v1 is single-
   threaded per VM instance; instances are independent so callers
   can run multiple in parallel.

4. **Floating-point arithmetic.**  Theory of FP is decidable but
   slow.  Should v1 ship LRA (linear *real*, ie. rationals) and
   defer FP entirely?  **Recommendation:** yes; FP is v3.

5. **Quantifiers.**  The undecidable case.  E-matching + heuristic
   instantiation is what Z3 uses; reproducing it well is large
   research.  **Recommendation:** v3 at the earliest; out of scope
   for v1/v2.

6. **Profile-guided solver tuning.**  LANG22's `.ldp` artefact
   format already contains observation data.  Could the solver
   read it to bias its heuristics on hot paths?
   **Recommendation:** explore in v2; not blocking v1.

7. **Symbolic-execution path explosion.**  Symbolic execution
   typically forks at every branch, producing exponential paths.
   Constraint-vm needs efficient path-condition incremental
   solving (push/pop heavy use).  **Recommendation:** ensure v1's
   push/pop is efficient — this is one of the load-bearing use
   cases.

8. **Should constraint-vm run on the LANG-VM (Lispy backend) for
   tooling parity?**  PR 24-J explores this.  **Recommendation:**
   defer to v2; the Rust impl is the substrate during v1.

9. **Cross-binding constraint vocabulary.**  Refinement predicates
   in Lispy and Ruby should mean the same thing.  How do we
   prevent each binding extending Predicate privately?
   **Recommendation:** Predicate is `non_exhaustive` but
   read-only-extension by binding-private types is forbidden by
   convention — bindings either use the canonical lattice or
   contribute upstream.

10. **Tooling integration with LANG18 coverage / LANG07 LSP.**
    Constraint programs deserve coverage measurement (which
    branches of the SAT decision tree got hit?) and LSP-style
    error reporting (which clause is unsatisfiable?).
    **Recommendation:** v2 PR.  The trace channel exists;
    tools just need to consume it.

---

## Acceptance criteria

LANG24 is "done" — locked enough for implementation — when:

1. **Four-crate stack mirrors LP00/07/08** with exact responsibility
   split (§"Architecture").
2. **Theory scope is versioned** with v1 = SAT + LIA committed
   (§"Theories supported").
3. **ConstraintIR shape is defined** with the full opcode + Predicate
   enum (§"ConstraintIR shape").
4. **Engine tactic trait is specified** with composition semantics
   (§"Engine tactics").
5. **Each consumer's lowering is shown** at a level of detail that
   makes the solver-decoupling concrete (§"Per-frontend lowering").
6. **Use-case catalog enumerates the consumers** with their predicate
   vocabularies (§"Use case catalog").
7. **Migration path sequences ~10 PRs** with the data crates as
   highest-leverage unblockers (§"Migration path").
8. **LANG23 dependency change is called out** so the existing spec
   gets updated (§"Update needed in LANG23").

This document satisfies all eight.

---

## Out of scope (named for clarity)

- **MaxSAT and weighted constraint solving.**  v1 is satisfiability
  + ranged search.  Optimisation problems wait for v3.
- **First-order theorem proving (e.g. resolution-based provers).**
  Constraint-VM is decidable-fragment-focused.  General theorem
  proving is a different research area.
- **Distributed solving.**  Single-process, single-VM-instance is
  the design.  Distributing a SAT solver across nodes is a
  research project.
- **Custom theory plugins from outside the project.**  v1 ships
  fixed tactics.  External plugins via dynamic-loading aren't
  specified.
- **Floating-point arithmetic.**  v3.
- **Differential privacy / probabilistic constraints.**  Different
  problem space; out of scope.
- **Closed-world theorem proving for arbitrary languages.**  We
  prove obligations the consumer asks about; we don't try to
  prove arbitrary correctness statements.
