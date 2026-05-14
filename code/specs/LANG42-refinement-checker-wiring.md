# LANG42 — Refinement Obligation Checker Wired into `twig-aot`

## Motivation

LANG23 built the entire refinement-type infrastructure:

- `lang-refined-types` — `RefinedType`, `Predicate`, `Kind` data types
- `constraint-vm` + `constraint-engine` — DPLL SAT solver + Cooper's LIA
- `lang-refinement-checker` — four-tier checker (per-binding, function-scope,
  module-scope, program-scope)
- `twig-ir-compiler` — populates `IIRFunction::param_refinements` and
  `IIRFunction::return_refinement` from parsed type annotations

But the IIR never reaches the checker.  The annotations silently do nothing:
the `twig-aot` pipeline compiles functions, runs the AOT linker, and emits
machine code — never asking whether any refinement obligation is violated.

LANG42 wires the checker into the pipeline.  After `twig-ir-compiler` emits
an `IIRModule`, a new pre-codegen pass (`iir-refinement-pass`) scans call
sites and return sites, resolves argument evidence via lightweight constant
propagation, and discharges proof obligations through the existing
`lang-refinement-checker` API.

The result: a literal argument that provably violates a parameter annotation
becomes a **compile error with a counter-example** rather than silently
producing broken machine code.

---

## New crate: `iir-refinement-pass`

```
code/packages/rust/iir-refinement-pass/
  Cargo.toml
  CHANGELOG.md
  README.md
  src/
    lib.rs            — public API: check_module, RefinementMode, RefinementError
    const_prop.rs     — ConstMap: scan IIR instructions for compile-time literals
    call_checker.rs   — check call-site arguments against callee param_refinements
    ret_checker.rs    — check return values against function return_refinement
```

### Public API

```rust
/// Operating mode — how to handle UNKNOWN outcomes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RefinementMode {
    /// PROVEN_UNSAFE → compile error.  UNKNOWN → silent (emit runtime check
    /// when the runtime-check insertion pass is implemented; for now no-op).
    #[default]
    Lenient,
    /// PROVEN_UNSAFE → compile error.  UNKNOWN → compile error.
    /// Used by (typed strict) modules and for correctness-critical pipelines.
    Strict,
}

/// A refinement violation found during the pass.
#[derive(Debug, Clone)]
pub struct RefinementError {
    /// The function containing the violation.
    pub function: String,
    /// Human-readable description: which parameter or return, what annotation.
    pub site: String,
    /// Counter-example value that proves the violation.
    pub counter_example: i128,
    /// Full description from the checker.
    pub description: String,
}

/// Run the refinement obligation pass over all functions in `module`.
///
/// Returns a (possibly empty) list of violations.  The caller decides
/// whether to abort compilation (all errors) or emit warnings (lenient).
pub fn check_module(module: &IIRModule, mode: RefinementMode) -> Vec<RefinementError>;
```

---

## Algorithm

### Step 1 — Constant propagation map

For each function, scan its instruction list once and build:

```
ConstMap: HashMap<String, i128>
```

A variable `v` is entered into the map if and only if exactly one `const`
instruction in the function sets `dest = Some("v")` with `srcs[0] =
Operand::Int(n)`.  Re-assignments (multiple `const` to same dest) evict the
entry (conservative: treat as `Unconstrained`).

This is a simple, single-pass, intra-procedural constant map — no dataflow,
no join points, no loops.  It handles the common pattern:

```
const  arg0 = 500     ; Operand::Int(500) → ConstMap["arg0"] = 500
call   callee arg0    ; evidence for arg0 = Concrete(500)
```

### Step 2 — Call-site checking

For every `call` instruction (`op == "call"`):

1. `callee_name` = `srcs[0]` as `Var` string
2. Look up `callee_fn` in `module.functions` by name
3. If `callee_fn.param_refinements` is empty → skip (no annotations)
4. For each `(i, arg)` in `srcs[1..]` (argument list):
   - `annotation` = `callee_fn.param_refinements.get(i)` → if `None`, skip
   - Resolve evidence:
     - `Operand::Int(v)` → `Evidence::Concrete(v as i128)`
     - `Operand::Var(name)` + `name ∈ ConstMap` → `Evidence::Concrete(map[name])`
     - Otherwise → `Evidence::Unconstrained`
   - `outcome` = `Checker::check(annotation, evidence)`
   - `ProvenUnsafe(cx)` → push `RefinementError`
   - `Unknown(_)` and `mode == Strict` → push `RefinementError` (with no counter-example value, description = solver message)
   - `ProvenSafe` → silent

### Step 3 — Return-site checking

For every `ret` instruction (`op == "ret"`) in a function with
`return_refinement = Some(annotation)`:

1. Resolve evidence from `srcs[0]`:
   - `Operand::Int(v)` → `Evidence::Concrete(v as i128)`
   - `Operand::Var(name)` + `name ∈ ConstMap` → `Evidence::Concrete(map[name])`
   - Otherwise → `Evidence::Unconstrained`
2. Check and emit errors identically to call-site step.

---

## Integration into `twig-aot`

### Location in pipeline

```
twig-ir-compiler::compile_source → IIRModule  (has param_refinements, return_refinement)
       │
       ▼  ← NEW: iir_refinement_pass::check_module(&module, mode)
                  if errors → return Err(AotError::RefinementViolations(errors))
       │
       ▼
prepare_module_for_aot (lower_builtins, lower_global_io, …)
       │
       ▼
compile each function → ARM64 machine code
```

The pass runs on the **original IIR before any lowering**, so annotations
refer to the same variable names the compiler produced.  Running it after
`prepare_module_for_aot` would see rewritten instructions where the
correspondence between variable names and annotations is lost.

### New `AotError` variant

```rust
/// One or more refinement proof obligations were violated.
///
/// In lenient mode this only fires for PROVEN_UNSAFE outcomes.
/// In strict mode it also fires for UNKNOWN outcomes.
RefinementViolations(Vec<iir_refinement_pass::RefinementError>),
```

### `RefinementMode` exposure

`compile_macos_arm64_object` and `compile_module_macos_arm64_object` gain an
optional `refinement_mode: RefinementMode` parameter (default `Lenient`).
The internal helper `compile_module_to_text` threads it through.

---

## Worked example

```scheme
; source.twig
(define (ascii-info (codepoint : (Int 0 128)))
  codepoint)

(define (main)
  (ascii-info 200))   ; ← 200 violates [0, 128)
```

LANG42 output (lenient mode):

```
error[E0042]: refinement violation
  → function main, call to ascii-info, argument 0
  annotation: (Int 0 128)
  counter-example: value 200 violates annotation
```

```scheme
(define (clamp-byte (x : (Int 0 256)) -> (Int 0 256))
  x)

(define (main)
  (clamp-byte 42))    ; ← 42 ∈ [0, 256) → ProvenSafe → no error
```

No error emitted.

---

## Tests to add

### `iir-refinement-pass`

- `concrete_literal_violates_range` — call with literal out of range → `RefinementError`
- `concrete_literal_in_range` → no errors
- `const_tracked_variable_violates` — const then call via variable → caught
- `unconstrained_variable_lenient` — variable with unknown value → silent
- `unconstrained_variable_strict` — same in strict mode → error
- `return_literal_violates_return_type` → `RefinementError`
- `return_literal_satisfies_return_type` → no errors
- `unannotated_function_skipped` — no `param_refinements` → always 0 errors
- `membership_predicate_violation` — `(Int {1, 2, 5})` with value 3 → error
- `multiple_violations_all_reported` — two bad args → two errors

### `twig-aot`

- `refinement_violation_becomes_aot_error` — `compile_macos_arm64_object` with a
  violating source returns `Err(AotError::RefinementViolations(...))`
- `safe_annotated_program_compiles_ok` — annotated but valid program compiles

---

## What this does NOT do (future work)

- **CFG-based path-sensitive checking** (FunctionChecker, ModuleChecker) —
  requires building `CfgNode` trees from IIR.  Planned for LANG45.
- **Runtime check insertion** — UNKNOWN outcomes should insert a
  `type_assert`-style guard in the IIR.  Planned for LANG46.
- **Refinement narrowing** — proven-safe checks should narrow the downstream
  `RefinedType`.  Planned for LANG47.
- **`(typed strict)` module declaration** — the TW05-A syntax extension that
  sets the module-level default to `Strict`.  Planned for TW05-A.

---

## Files to create / update

| File | Action |
|------|--------|
| `code/specs/LANG42-refinement-checker-wiring.md` | CREATE (this file) |
| `code/packages/rust/iir-refinement-pass/Cargo.toml` | CREATE |
| `code/packages/rust/iir-refinement-pass/src/lib.rs` | CREATE |
| `code/packages/rust/iir-refinement-pass/src/const_prop.rs` | CREATE |
| `code/packages/rust/iir-refinement-pass/src/call_checker.rs` | CREATE |
| `code/packages/rust/iir-refinement-pass/src/ret_checker.rs` | CREATE |
| `code/packages/rust/iir-refinement-pass/CHANGELOG.md` | CREATE |
| `code/packages/rust/iir-refinement-pass/README.md` | CREATE |
| `code/packages/rust/twig-aot/Cargo.toml` | UPDATE — add `iir-refinement-pass` dep |
| `code/packages/rust/twig-aot/src/lib.rs` | UPDATE — call pass before codegen |
| `code/packages/rust/Cargo.toml` | UPDATE — add `iir-refinement-pass` to workspace |
