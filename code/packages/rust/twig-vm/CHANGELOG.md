# Changelog — twig-vm

## [0.5.0] — 2026-04-30

### Added — PR 7 of LANG20: persistent inline-cache slot machinery

Replaces the per-dispatch stack-allocated IC from PR 6 with a
**persistent, per-call-site IC table** indexed by
`IIRInstr::ic_slot`.  The hot-site invariant: two activations of
the same `send` / `load_property` / `store_property` instruction
share *one* IC instance, so an observation on call N benefits
on call N+1 (the V8-style fast path the JIT eventually compiles
against).

- **`ICTable` struct** — `HashMap<String, Vec<InlineCache<LispyICEntry>>>`
  keyed by `(function_name, ic_slot)`.  Dense vec per function
  (slot N at index N); first access to a slot allocates a fresh
  `InlineCache::new()`; subsequent accesses return the same
  instance.  Public API: `new()`, `get_or_alloc()`, `get()`,
  `slot_count()`, `total_slots()`.

- **`run_with_state(module, &mut Globals, &mut ICTable)`** — new
  entry point that accepts both per-VM tables.  Per-run lifetime
  for now; PR 8+ moves both to a long-lived `LangVM` so the
  cache survives across `run` calls.

- **Dispatcher routes through the table** — `exec_send` /
  `exec_load_property` / `exec_store_property` now take
  `(ic_table, fn_name)`.  When `instr.ic_slot` is `Some(slot)`,
  the handler calls `ic_table.get_or_alloc(fn_name, slot)` and
  passes the resulting `&mut InlineCache` to the binding.  When
  `None`, it falls back to a stack-allocated fresh IC (PR 6
  behaviour, backwards-compatible).

- **`run`** + **`run_with_globals`** retained as wrappers that
  allocate a fresh `ICTable` and delegate to `run_with_state`.
  No breaking change to existing PR 5/6 callers.

### IIR additions (in `interpreter-ir`)

- **`IIRInstr::ic_slot: Option<u32>`** field added per LANG20
  §"IIR additions".  Defaults to `None` in `IIRInstr::new` —
  every existing call site stays correct without modification.

- **`IIRInstr::with_ic_slot(slot)`** builder method.  Used by
  language frontends that emit IC-owning opcodes; tests use it
  to verify the table-routing path.

### Tests

- twig-vm: **103 unit + 2 doc tests** (13 new for PR 7) — all
  pass on stable Rust.  (11 PR-7 mechanics tests + 2 security-
  review boundary tests for the new caps.)

11 new tests cover:
- `ICTable` mechanics in isolation: starts empty, allocates on
  first access, dense-per-function, separate-functions-don't-
  share, repeated-access-returns-same-instance.
- Dispatcher routing: `send` / `load_property` / `store_property`
  with `ic_slot=Some(...)` populates the table; `ic_slot=None`
  leaves the table empty (backward-compat).
- The hot-site invariant: a `load_property` in a function
  called twice consults the same IC instance — verified via
  `note_hit` persistence between calls.
- `IIRInstr::ic_slot` round-trip: defaults to `None`,
  `with_ic_slot` sets the field.

### Security review fixes applied

- **HIGH #1 — unbounded `ic_slot` permits ~256GB allocation**:
  `ICTable::get_or_alloc(fn_name, slot)` previously called
  `Vec::resize_with(slot + 1, …)` with no cap on `slot`;
  `IIRInstr::ic_slot: Option<u32>` allows values up to
  `u32::MAX`, so a malformed IIR with `ic_slot = u32::MAX - 1`
  would attempt a multi-hundred-GB allocation.  Capped at
  `MAX_IC_SLOTS_PER_FUNCTION = 2¹⁶` matching
  `MAX_REGISTERS_PER_FRAME`.  `get_or_alloc` now returns
  `Result<&mut InlineCache, RunError>` so callers explicitly
  handle the cap; `?` propagation gives a clean
  `MalformedInstruction` error.

- **MEDIUM #2 — unbounded function-name HashMap growth**:
  `ICTable::by_function: HashMap<String, _>` had no cap.
  Capped at `MAX_IC_FUNCTIONS = 2¹⁶` distinct function names.
  Re-accessing existing functions still works (cap is on
  distinct keys, not on accesses).

- **TODO comment for serialisation** — `ic_slot` is a static
  field that probably should serialise to disk for AOT
  pipelines, but the current `interpreter-ir::serialise`
  module doesn't.  Documented as a follow-up for LANG22 AOT.

### Hardening

- Clippy-clean on twig-vm.  (Pre-existing `approx_constant`
  warnings on PI literals in `interpreter-ir/serialise.rs`
  predate this PR and are not addressed here.)
- Three independent caps now defend the IC subsystem from
  malformed IIR DoS:
  - `MAX_IC_SLOTS_PER_FUNCTION = 2¹⁶` (per-function vec size)
  - `MAX_IC_FUNCTIONS = 2¹⁶` (distinct function-name keys)
  - `MAX_REGISTERS_PER_FRAME = 2¹⁶` (already enforced for
    `send`/`call` srcs)
- All resource limits from prior PRs still enforced.
- The new opcodes don't add any unsafe; no cargo-geiger
  impact.
- Miri verification on twig-vm runs in CI.

## [0.4.0] — 2026-04-30

### Added — PR 6 of LANG20: send / load_property / store_property opcodes

Wires the three method-dispatch opcodes through the existing
`LangBinding` trait machinery.  For Lispy these surface as
`NoSuchMethod` / `NoSuchProperty` runtime errors (correct
behaviour for a language without method dispatch); the value of
PR 6 is making the OPCODE PATH live, so a future Ruby-binding or
JS-binding gets dispatch for free without further dispatcher
changes.

- **`send recv selector args...`** — extracts the receiver from
  `srcs[0]`, the symbol-id selector from `srcs[1]` (lowered via
  PR 5's string-as-symbol convention), and any remaining args
  from `srcs[2..]`.  Allocates a per-instruction
  `InlineCache<LispyICEntry>` (PR 7 makes it persistent), calls
  `LispyBinding::send_message`, stores the result in `dest`.

- **`load_property obj key`** — same shape with a single key.
  Allocates a fresh IC, calls `LispyBinding::load_property`.

- **`store_property obj key value`** — three-src side-effecting
  opcode (no dest).  Allocates a fresh IC, calls
  `LispyBinding::store_property`.

- **`read_symbol_arg` helper** — shared by all three opcodes.
  Reads a register's value, asserts it's a symbol, returns the
  `SymbolId`.  A non-symbol selector / key surfaces as
  `Runtime(TypeError(...))` with a descriptive message.

- **10 new tests** covering all three opcodes' happy path
  (returns NoSuchMethod / NoSuchProperty as appropriate),
  arity validation (`send` requires 2+ srcs, `load_property`
  requires exactly 2, `store_property` requires exactly 3),
  type validation (selector / key must be a symbol), and a
  DoS-guard test for `send` with `srcs.len() >
  MAX_REGISTERS_PER_FRAME`.  Tests hand-build minimal
  IIRModules since `twig-ir-compiler` doesn't emit these
  opcodes yet (no `(send obj msg ...)` form in Twig source —
  that's a future PR).

### Security review fixes applied

- **DoS via unbounded args allocation in `exec_send`**: a
  hand-built `send` instruction with millions of srcs would
  OOM via `Vec::with_capacity(srcs.len() - 2)` before the
  per-arg loop.  Capped at `MAX_REGISTERS_PER_FRAME` (2¹⁶).
  Well-formed IIR never approaches this — function arities
  are bounded by source syntax and a Twig function can't have
  65k arguments — so the cap is purely defensive.  Found by
  PR 6 security review.
- **`DispatchCx::new_for_test()` documented**: the only
  constructor for `DispatchCx` today is doc-hidden and
  test-suffixed.  Added a `TODO` comment noting that this
  call site needs to migrate to the production constructor
  when `DispatchCx` grows real fields (LANG20 PR 8+ — vm-core
  wiring).

### IC parameter handling (PR 6 vs PR 7)

PR 6 allocates an `InlineCache<LispyICEntry>` fresh per
dispatch and discards it after the call.  PR 7 (IC machinery)
introduces a per-call-site IC table indexed by
`IIRInstr::ic_slot` (LANG20 §"IIR additions") — at that point
the IC allocation moves to table lookup but the trait calls
stay identical.  No breaking change.

### Tests across the diff

- twig-vm: **90 unit + 2 doc** — all pass on stable Rust.
  Miri verification deferred to CI.
- lispy-runtime: 105 unit + 1 doc — unchanged (PR 6 is
  dispatcher-only; lispy-runtime's binding methods already
  return the right errors from PR 2).

### Hardening

- Clippy-clean.
- All resource limits from prior PRs (`MAX_DISPATCH_DEPTH`,
  `MAX_INSTRUCTIONS_PER_RUN`, `MAX_REGISTERS_PER_FRAME`) still
  enforced.
- The new opcodes don't add any unsafe; no `cargo-geiger` impact.

## [0.3.0] — 2026-04-29

### Added — PR 5 of LANG20: closures, top-level value defines, quoted symbols

This PR completes the **full Twig surface language** under the
tree-walking dispatcher.  Programs using `lambda`, `(define x value)`,
or quoted symbols (`'foo`) now run end to end, alongside everything
PR 4 already supported.

- **String-as-symbol convention for `const Operand::Var(text)`**:
  the dispatcher interns `text` and stores
  `LispyValue::symbol(intern(text))` rather than refusing.  Lispy
  has no string type yet; the IR compiler routes string-shaped
  literals (function names, global names, quoted-symbol names)
  through this path.  When a real string value lands in a future
  PR, only this single arm changes; the IR compiler already emits
  the right operand shape.

- **`make_symbol` / `make_closure` / `make_builtin_closure`
  builtins** added to `lispy-runtime`.  Registered in
  `LispyBinding::resolve_builtin`.  All three are normal
  context-free `BuiltinFn`s — they don't need access to the
  dispatcher, only to the heap allocators (`alloc_closure`,
  `alloc_builtin_closure`).

- **`apply_closure` opcode** handled inline in the dispatcher (it
  needs the IIRModule reference for user-fn lookup and the
  dispatcher for recursion).  Routes through
  `LispyBinding::resolve_builtin` for builtin-wrapping closures
  (detected via `CLOSURE_FLAG_BUILTIN`); for user-fn closures it
  prepends the closure's captures to the user-supplied args and
  recurses into `dispatch` with the resolved IIRFunction.

- **`global_set` / `global_get` opcodes** handled inline (they
  need access to the per-run globals table).  Backed by a new
  `Globals` struct (`HashMap<SymbolId, LispyValue>`) threaded
  through `dispatch` as a `&mut` parameter.  Per-run lifetime
  for now; will move to per-VM state when LANG20 PR 7+ adds a
  long-lived `LangVM`.

- **Closure heap layout extended** in `lispy-runtime::heap`:
  `Closure._reserved` renamed to `Closure.flags` and given bit 0
  (`CLOSURE_FLAG_BUILTIN`) to distinguish user-fn closures from
  builtin-wrapping closures.  New `alloc_builtin_closure(name)`
  factory and `Closure::is_builtin()` accessor.

- **New public API**:
  - `dispatch::Globals` — the globals-table struct (constructable,
    inspectable; future-proofing for LANG20 PR 7+).
  - `dispatch::run_with_globals(module, &mut globals)` — entry
    point that accepts a caller-supplied table.  Tests use this
    to verify table threading; future per-VM state will use it
    to persist globals across runs.

- **18 new tests** covering quoted symbols, anonymous lambdas
  (no capture, single capture, multi capture, nested), closure-
  returning functions (curried add, make-adder), higher-order
  passing both user-fn and builtin closures, top-level value
  defines (read, used in function, overwrite), error paths
  (`apply_closure` on non-closure, `global_get` on undefined
  name), and the `Globals` struct directly.

### Removed — placeholder error path retired

- `RunError::UnsupportedOpcode("const with string operand
  (closures / globals / symbols — PR 5+)")` — no longer
  returnable; the `Operand::Var(text)` arm of `exec_const` now
  produces a symbol value.

### Changed — error type evolution

- `RunError` is now `#[non_exhaustive]` so future variants
  (sent/load/store opcodes in PR 6, deopt in PR 8+) don't
  break callers.

### Added — `RunError` variants

- `UndefinedGlobal(String)` — `global_get` of a name never
  written.  Includes the demangled name for diagnostics.
- `NotCallable(String)` — `apply_closure` of a non-closure
  value.  Surfaces user-visible "X is not a function".

### Tests across the diff

- twig-vm: **80 unit + 2 doc** — all pass on stable Rust and
  Miri.  18 new for PR 5; 62 PR-4 tests untouched.
- lispy-runtime: **105 unit + 1 doc** — unchanged at the test
  level (4 new closure-/symbol-builtin tests added; 4 prior
  PR-4 tests retired as obsolete with the `flags` field
  rename).

### Hardening

- Clippy-clean on all touched crates.
- Miri-clean — closure heap allocations + builtin-flag reads +
  `as_closure` walks all exercised under Miri without UB.
- All resource limits from PR 4 (`MAX_DISPATCH_DEPTH`,
  `MAX_INSTRUCTIONS_PER_RUN`, `MAX_REGISTERS_PER_FRAME`) still
  enforced; closures don't bypass them — `apply_closure`
  recursion uses the same `dispatch` recursion as direct `call`.

## [0.2.0] — 2026-04-29

### Added — PR 4 of LANG20: real dispatch loop

- **`dispatch` module** — first version of the VM that actually
  *runs* a Twig program end to end.  Replaces the PR-3
  `evaluate_call_builtin` 1-instruction helper with a tree-walking
  dispatcher covering the IIR opcodes emitted by
  `twig-ir-compiler` for the closure-free / globals-free /
  symbols-free subset:
  - `const` — bind register ← `Int` / `Bool` immediate
  - `call_builtin` — resolve through `LispyBinding`, materialise
    args via `operand_to_value`, dispatch
  - `call` — resolve callee in the module's `functions` table,
    recurse into a fresh `Frame`
  - `jmp` / `jmp_if_false` / `label` — control flow with O(1)
    label resolution (label index built once per function on
    entry)
  - `ret` — return the operand value to the caller
- **`TwigVM::run(source)`** — public end-to-end entry point.
  Compiles + dispatches in one shot, returns a `LispyValue`.
- **`TwigRunError`** — combined error type that wraps
  `TwigCompileError` and `RunError` so callers don't need to
  juggle two error families.
- **Resource limits**:
  - `MAX_DISPATCH_DEPTH = 256` — caps recursion depth so
    adversarial input can't blow the host stack.
  - `MAX_INSTRUCTIONS_PER_RUN = 2²⁰` — caps total instructions
    per top-level run as a backstop against infinite loops in
    hand-built malformed IIR.
  - `MAX_REGISTERS_PER_FRAME = 2¹⁶` — caps the up-front
    `HashMap` allocation per `Frame` so a hand-built module
    with `register_count = usize::MAX` cannot abort the
    process at allocation time before any instruction tick
    fires.  Added in response to a security review finding.
- **`_move` and `make_nil` builtins** added to `lispy-runtime`
  (and registered in `LispyBinding::resolve_builtin`).  These are
  infrastructure builtins emitted by `twig-ir-compiler` for
  type-preserving register copies (`if` / `let` lowering) and
  nil materialisation.  They are not part of the Lispy surface
  language but appear in the lowered IIR.
- **End-to-end tests** that prove canonical Twig programs run
  through the full pipeline:
  - arithmetic: `(+ 2 3)` → 5, `(+ (* 2 3) (- 10 4))` → 12
  - control flow: `(if (< 1 2) 100 200)` → 100, with bool literals
  - locals: `(let ((x 5)) (* x x))` → 25, nested `let`
  - sequencing: `(begin 1 2 3)` → 3
  - direct call: `(define (square x) (* x x)) (square 7)` → 49
  - direct call w/ multiple args: `(add3 1 2 3)` → 6
  - recursion: `(fact 5)` → 120, `(fib 10)` → 55
  - mutual recursion: `(is_even 10)` → `#t`
  - cons family: `(car (cons 1 2))` → 1, `(pair? (cons 1 2))` → `#t`
  - Scheme truthiness: `(if 0 1 2)` → 1 (only `#f` and `nil`
    branch — 0 is truthy)

### Removed

- **`evaluate.rs` module** (`evaluate_call_builtin`,
  `EvaluateError`) — the PR-3 1-instruction placeholder is no
  longer needed now that `dispatch` interprets full programs.
  All its tests are subsumed by `dispatch::tests` and
  `tests::run_*`.

### Changed

- **`lispy-runtime::LispyBinding::resolve_builtin`** — extended
  with `_move` and `make_nil` arms.  No source-language users —
  the IR compiler emits these names from `if` / `let` lowering.
- **`lang-runtime-safety` CI workflow** — extended Miri
  coverage to include `twig-vm` so the dispatcher's integration
  with lispy-runtime's tagged-pointer code is exercised under
  Miri on every PR (lispy-runtime's own Miri suite tests the
  binding in isolation; this catches UB in the seam).

### Tests across the diff

- twig-vm: **62 unit + 2 doc tests** — all pass on stable Rust
  and Miri.  35 PR-3 tests retired (subsumed by dispatch tests).
  Two extra tests added for the security-review fixes
  (`frame_caps_register_count`,
  `build_label_index_rejects_duplicate_labels`).
- lispy-runtime: 105 unit tests (4 new for the `_move` /
  `make_nil` builtins).

### Hardening

- Clippy-clean on all touched crates (one pre-existing
  `unused_parens` warning in `operand.rs` cleaned up while the
  file was being edited — independent of PR 4).
- Miri-clean on twig-vm dispatcher tests including:
  - factorial recursion (frames + arg copy)
  - mutual recursion (cross-function frame creation)
  - cons / car / cdr (heap pointer round-trips)
  - infinite-recursion guard (depth cap fires before host stack
    overflow)
- Resource limits (`MAX_DISPATCH_DEPTH`, `MAX_INSTRUCTIONS_PER_RUN`,
  `MAX_REGISTERS_PER_FRAME`) unit-tested directly so a future
  change can't silently raise them.
- `build_label_index` errors on duplicate label names (instead of
  silently shadowing), so a hand-built module that violates the
  fresh-label invariant fails fast instead of mis-routing
  `jmp` instructions.  Added in response to a security review
  finding.

## [0.1.0] — 2026-04-29

### Added

- **PR 3 of [LANG20](../../../specs/LANG20-multilang-runtime.md)
  §"Migration path"** — runtime wiring between the Twig frontend and
  the LANG-runtime substrate.
- `TwigVM` facade — currently stateless; PR 4 will add per-VM
  state (frame stack, register file scratch, JIT promotion
  thresholds).  Provides:
  - `TwigVM::new()` — zero-cost constructor.
  - `TwigVM::compile(source)` — calls `twig_ir_compiler::compile_source`
    with `module_name = "twig"`.
  - `TwigVM::compile_with_name(source, name)` — explicit module name.
  - `TwigVM::resolve_builtin(name)` — proxies through
    `<LispyBinding as LangBinding>::resolve_builtin`.
- `operand::operand_to_value` — converts IIR `Operand` →
  `LispyValue`.  The per-language seam between the language-
  agnostic IIR and the Lispy runtime's tagged-i64 representation.
  Handles Int (with range-check against the 61-bit tagged-int
  range), Bool, Var (via caller-supplied frame_lookup callback),
  and the special-cased `nil` name.  Float operands return a
  `RuntimeError::TypeError("flonum")` since Lispy doesn't yet
  have flonums.
- `evaluate::evaluate_call_builtin` — 1-instruction evaluator
  proving the substrate composes.  Takes a `call_builtin` IIR
  instruction, resolves the builtin via `LispyBinding`,
  materialises argument operands into `LispyValue`s, and
  dispatches.  Real interpretation lands in PR 4 (vm-core
  wiring); this evaluator is the integration test surface.
  - `EvaluateError` enum with variants for unsupported opcode,
    missing/non-Var builtin name, unknown builtin, operand
    conversion failure, builtin runtime error.
- 35 unit + 2 doc tests.  Coverage includes the full Twig source
  → IIR → LispyValue evaluation pipeline for arithmetic, cons,
  comparisons, and predicates.

### Changed

- **`twig-lexer` and `twig-parser` now compile their grammars at
  build time.**  Earlier drafts called `std::fs::read_to_string`
  on `code/grammars/twig.tokens` / `twig.grammar` at every lexer/
  parser construction.  A new `build.rs` in each crate calls
  `grammar_tools::token_grammar::parse_token_grammar` (or the
  parser-grammar equivalent) and `grammar_tools::codegen::*` to
  emit Rust source code that reconstructs the parsed grammar as a
  `OnceLock<…>` static.  `lib.rs` `include!`s the generated code
  and exposes the grammar via `pub fn twig_token_grammar()` /
  `pub fn twig_parser_grammar()`.
  Result: zero runtime file I/O, Miri-compatible without
  `-Zmiri-disable-isolation`, builds catch malformed grammars
  early.

### Uses (existing): `grammar_tools::compiler`

- The build scripts call `grammar_tools::compiler::compile_token_grammar`
  / `compile_parser_grammar` — the canonical grammar-to-Rust
  compiler that already lived in the workspace.  An earlier draft
  of this PR added a duplicate `codegen` module; that has been
  removed in favour of using the existing one.  The lib.rs of
  twig-lexer / twig-parser wraps the compiler-generated
  `token_grammar()` / `parser_grammar()` constructors in a
  `OnceLock<…>` so the struct is materialised exactly once per
  process — the generated code constructs eagerly each call, so
  the OnceLock ensures we don't redo it on every
  `create_twig_lexer` / `create_twig_parser` invocation.

### Tests across the diff

- twig-vm: 35 unit + 2 doc — all pass on stable Rust **and Miri**.
- grammar-tools: 144 unit + 3 doc (codegen module adds 8 of those).
- twig-lexer: 18 unit + 1 doc, identical surface — grammar source
  is now compiled at build time but `tokenize_twig` /
  `create_twig_lexer` API unchanged.
- twig-parser: 33 unit + 2 doc, identical surface.
- twig-ir-compiler: 45 unit + 2 doc, identical (compiles against
  the build-time grammar via twig-parser).

### Hardening

- Clippy-clean across all touched crates.
- Miri runs cleanly on `lang-runtime-core`, `lispy-runtime`, and
  `twig-vm` — no UB, no isolation-disable required.
