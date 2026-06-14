# `iir-refinement-pass`

**LANG42** — pre-codegen refinement obligation checker for the Twig AOT
pipeline.

## What it does

LANG23 built a complete refinement-type infrastructure:

- `lang-refined-types` — `RefinedType`, `Predicate`, `Kind` data types
- `constraint-vm` + `constraint-engine` — DPLL SAT solver + Cooper's LIA
- `lang-refinement-checker` — per-binding `Checker` + function/module/program
  scoped checkers
- `twig-ir-compiler` — populates `IIRFunction::param_refinements` and
  `return_refinement` from parsed type annotations

However, the IIR never reached the checker: refinement annotations were parsed
and stored but never validated.  Any violation silently produced broken machine
code.

`iir-refinement-pass` wires the checker into the pipeline.  It runs **before
any lowering passes**, directly on the original IIR emitted by the compiler,
so variable names still correspond to their annotations.

## Algorithm

For each function in the module:

1. **Constant-propagation map** — one forward scan of `const` instructions
   builds `HashMap<String, i128>` of compile-time integer literals.  A
   variable is tracked only if it is assigned exactly once; re-assignments are
   conservatively evicted.

2. **Call-site checking** — for every `call` instruction:
   - Look up the callee's `param_refinements`
   - Resolve each argument's evidence (Concrete if literal/ConstMap hit;
     Unconstrained otherwise)
   - Call `lang_refinement_checker::Checker::check(annotation, evidence)`
   - `ProvenUnsafe` → `RefinementError` with a counter-example value

3. **Return-site checking** — same as call-site but for `ret` instructions
   checked against the function's `return_refinement`.

## Modes

| Mode | `ProvenUnsafe` | `Unknown` |
|---|---|---|
| `Lenient` (default) | compile error | silent |
| `Strict` | compile error | compile error |

Use `Strict` for `(typed strict)` modules (LANG TW05-A) or
correctness-critical pipelines.

## Usage

```rust
use iir_refinement_pass::{check_module, RefinementMode};

let errors = check_module(&iir_module, RefinementMode::Lenient);
if !errors.is_empty() {
    for e in &errors {
        eprintln!("{e}");
    }
    return Err(AotError::RefinementViolations(errors));
}
```

## Where it fits in the pipeline

```
twig-ir-compiler::compile_source → IIRModule
       │
       ▼  ← iir_refinement_pass::check_module  (LANG42, this crate)
              if errors → Err(AotError::RefinementViolations)
       │
       ▼
prepare_module_for_aot (lower_builtins, …)
       │
       ▼
compile each function → ARM64 machine code
```

## Future work

- **CFG-based path-sensitive checking** (`FunctionChecker`, `ModuleChecker`)
  — requires building `CfgNode` trees from IIR.  Planned for LANG45.
- **Runtime check insertion** — `Unknown` outcomes should insert a
  `type_assert`-style guard.  Planned for LANG46.
- **Refinement narrowing** — proven-safe checks should narrow the downstream
  `RefinedType`.  Planned for LANG47.
- **`(typed strict)` module declaration** — TW05-A syntax extension that sets
  the module-level default to `Strict`.
