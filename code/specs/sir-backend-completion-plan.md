# SIR Backend Completion Plan — close every deferred item

**Status:** active. **Scope (user-confirmed 2026-06-21):** Python + TypeScript backends
only (Go/Rust stay out of scope). **SIR core schema changes ARE allowed** where a
faithful implementation requires them.

## Why this exists

An audit on 2026-06-21 found the merged backends accept only a slice of the SIR the
Ruby frontend produces. The frontend lowers the full surface, but:

- **Python backend** (`semantic-ir-to-python`) `ACCEPTED_FEATURES` =
  {Closures, Pairs, Symbols, Strings, DynamicTyping, OptionalTypeAnnotations,
  MutualRecursion, Globals, Floats, Sequences, Maps, ShortCircuit, StringInterpolation}.
  It **panics/rejects** on mutation (`MutableBindings`), loops (`Loops`),
  classes/modules/singleton (`Classes`/`Modules`), instance/class/const vars,
  exceptions (`TryCatch`), and routes `recv.meth` (`__method__`) + `&:sym`
  (`block_pass`) to `_sir_call_builtin`, which raises "unknown builtin" at runtime.
- **TypeScript backend** is narrower still — it does not even accept Floats,
  Sequences, Maps, ShortCircuit, StringInterpolation.
- The per-concern runtime packages from the original plan (`-oop`, `-exceptions`,
  `-pairs`, `-regex`, `-shell`, `-range`) **do not exist** — only `sir-runtime-core`
  with the minimal Lisp-era dispatch table (no `__method__`, `is_a?`, `LocalJumpError`,
  `sym_to_proc`).

So "nothing deferred" requires building out most of an imperative/OO backend across
two targets plus runtime support, plus resolving the frontend markers. This file is
the authoritative backlog. Execution: one PR at a time, specs→tests→impl→changelog→
README→/security-review→push→babysit, never self-merge.

## Verified-OPEN inventory (the things to finish)

Backend feature gaps (Python + TS):
- Mutation: `Assign` reassignment, `SeqSet`, `MapSet` (`MutableBindings`)
- Loops: `While`, `ForRange`, `ForEach` (`Loops`)
- OOP: `ClassDef`, `ModuleDef`, `SingletonClassDef`; `Instance`/`ClassVar`/`Const` scopes
- Method dispatch: `__method__` envelope (`recv.meth(args)`) incl. built-in/collection
  methods (`.each`/`.map`/`.to_s`/`.length`/…) and user-defined methods
- Exceptions: `TryCatch`, `raise`, rescue-type matching, exception base hierarchy,
  `LocalJumpError` (nil-block yield)
- `&:sym` symbol-to-proc (runtime)
- TS expression parity: Floats/Sequences/Maps/ShortCircuit/StringInterpolation

Frontend / SIR-core gaps:
- Block & lambda captures of outer locals (currently always empty → outer refs fail)
- Variadic param flag for `*args` / `**kwargs` (SIR `Param` has no variadic kind)
- Map has-key primitive → faithful hash-pattern key-presence
- First-class seq-slice (`__seq_slice__` marker) execution
- Find pattern `[*, x, *]` (`__pattern_match__` marker) execution
- `case/when` case-equality (`===`) vs current `==`
- `defined?` on never-bound local (validator rejects today)
- Splat without parens `puts *arr`
- Numeric: Rational `1r` / Complex `2i`, legacy octal `017`
- String fidelity: heredoc interpolation/quote-form/escape, backtick interpolation,
  string/regex escape unescaping, regex nested brackets

## Phased PR backlog (dependency order)

### Phase 0 — Spec & baseline
- **P0a** Rewrite `specs/sir-runtime.md` to the TRUE current contract + this roadmap;
  add the per-concern runtime-package contracts to be created. (this file is the plan;
  the runtime spec is the contract.)

### Phase A — TS expression parity with Python
- **A1** TS backend: accept + emit Floats, Sequences, Maps, ShortCircuit,
  StringInterpolation (native), reaching Python's expression baseline. Execution-proof
  through `node`.

### Phase B — Mutation & loops (both backends)
- **B1** `MutableBindings` + `Loops`: `Assign`/`SeqSet`/`MapSet`/`While`/`ForRange`/
  `ForEach` → native `=`, `s[i]=v`, `while`, `for`. Add to ACCEPTED_FEATURES; replace
  panic arms. Execution proofs both targets.

### Phase C — OOP (both backends + sir-runtime-oop)
- **C1** Create `sir-runtime-oop` (py + ts) full package scaffold; `ClassDef`/`ModuleDef`
  → native `class`/namespace; `Instance`/`ClassVar`/`Const` scopes.
- **C2** `SingletonClassDef`; `is_a?`/`kind_of?`/`instance_of?`/`class` via runtime-oop.
- **C3** Method dispatch: `__method__` → native `recv.meth(args)` where faithful; runtime
  `call_method`/`callMethod` for built-in/collection methods (`.each`/`.map`/`.to_s`/
  `.length`/…) and a registered user-method table. Closes the reported nil-dispatch boundary.

### Phase D — Exceptions (both backends + sir-runtime-exceptions)
- **D1** Create `sir-runtime-exceptions` (py + ts); `TryCatch` → `try/except`/`try/catch`
  with rescue-type match + finally; `raise`; SIR exception base hierarchy.
- **D2** `LocalJumpError` class; nil-block `yield` raises it (closes Q10a properly).

### Phase E — Closures & blocks faithful
- **E1** Frontend: compute block/lambda captures of outer locals (capture analysis),
  replacing the empty-captures v0; backends already emit captures.
- **E2** `&:sym` symbol-to-proc: frontend emits `sym_to_proc`; runtime builds a closure
  routing through method dispatch (depends on C3).

### Phase F — Method-dispatch completeness & defined?
- **F1** General `defined?(recv.meth)` runtime answer; `defined?` on never-bound local
  (frontend marker so validator accepts; runtime returns nil).

### Phase G — Pattern matching completeness (core changes allowed)
- **G1** Map has-key primitive (core) → hash-pattern key-presence enforcement.
- **G2** First-class seq-slice (core or runtime) → `__seq_slice__` executes.
- **G3** Find pattern `[*, x, *]` → faithful runtime matcher executes `__pattern_match__`.
- **G4** `case/when` case-equality (`===`): class/range/regex-aware via runtime.

### Phase H — Params (core change)
- **H1** SIR `Param` variadic kind; `*args`/`**kwargs` lower with flag; backends emit
  `*args`/`**kwargs`.
- **H2** Splat without parens `puts *arr` (frontend).

### Phase I — Literals & string fidelity
- **I1** Rational `1r` / Complex `2i` numeric markers + runtime; legacy octal `017`.
- **I2** Heredoc interpolation/quote-form/escape; backtick interpolation; string escape
  unescaping.
- **I3** Regex unescaping + nested-bracket tracking.

## Execution rules (unchanged)
- One PR at a time off latest origin/main; never self-merge; `--force-with-lease` only;
  scope `git add` to changed files; never commit `.venv`/`node_modules`/`dist`/`target`;
  commit via `git commit -F`; PR bodies end with the 🤖 Generated line; `/security-review`
  before every push; babysit each PR to green. Cargo crate version stays 0.1.0; bump
  CHANGELOG logical version only. Every new package needs BUILD, README, CHANGELOG,
  required_capabilities.json, tests ≥80% (target 95%), mypy --strict/ruff or tsc/vitest.
