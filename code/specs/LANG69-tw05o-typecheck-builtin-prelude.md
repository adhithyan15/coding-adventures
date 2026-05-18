# LANG69 — TW05-O: Type-Checker Builtin Prelude + span.tw Strict Mode

> **LANG68 (TW05-N) is merged.**  This spec covers TW05-O: pre-registering
> all Twig runtime builtins in the type-checker's global environment so that
> calls to builtins no longer produce "unresolved variable" warnings, and
> converting `span.tw` to `(typed strict)` as the first strict-mode compiler
> module.

---

## Background

### The builtin-resolution gap

`twig-type-checker` builds its global environment (`TypeEnv`) by walking
module-level `(define ...)`, `(record ...)`, and `(union ...)` forms.  Runtime
builtins (`+`, `car`, `null?`, `string-append`, …) are **not** pre-populated
in `TypeEnv::new()` — the comment in `lib.rs` reads:

> "in a real program those come from the standard prelude"

Because there is no prelude today, every call to a builtin emits:

```
twig type warning: unresolved variable `null?`
twig type warning: unresolved variable `car`
```

These are harmless in `(typed lenient)` mode (ok is always `true`) but they
are fatal in `(typed strict)` mode.

### `and` / `or` are special-cased in the IR compiler, not the grammar

`(and x y)` and `(or x y)` are parsed as regular call-expressions
(`Apply { fn: VarRef("and"), … }`).  The IR compiler special-cases them in
`compile_apply` before the builtin lookup; they never appear in the `BUILTINS`
const.  From the type-checker's perspective they look like calls to user-
defined functions, so they also need to be pre-registered.

### `span.tw` — simplest compiler module with zero imports

`span.tw` defines only `Span` (a record) plus two functions:

```scheme
(define (make-span sid s e)
  (if (and (>= s 0) (<= s e)) (Span sid s e) nil))
(define (dummy-span) (Span 0 0 0))
```

The only external names it calls are `and`, `>=`, `<=`.  After builtin
pre-registration these all resolve, so `span.tw` is the ideal first module to
switch to `(typed strict)`.

---

## Changes

### 1. `twig-type-checker/src/env.rs` — `TypeEnv::register_builtins`

Add a new private method `TypeEnv::register_builtins` and call it from
`TypeEnv::new()`:

```rust
pub fn new() -> Self {
    let mut env = TypeEnv::default();
    env.register_builtins();
    env
}

/// Pre-populate `globals` with every Twig runtime builtin so that calls to
/// builtins resolve to `TwigKind::Any` instead of "unresolved variable".
///
/// ## Design note
///
/// Builtins are registered as `Any` rather than `Function { arity }` because:
/// - Several builtins are variadic (`list`, `string-append`).
/// - Arity enforcement for builtins is already done by the IR compiler.
/// - Registering as `Any` suppresses "unresolved variable" errors without
///   introducing false arity mismatches.
///
/// Callers may shadow any of these with explicit `(define ...)` stubs in
/// test prelude code; `collect_forms` (Pass 1) overwrites `globals` entries
/// with the stub's kind just like any other define.
///
/// ## What is included
///
/// - All 43 names from the `BUILTINS` const in `twig-ir-compiler`.
/// - `and` and `or` — special-cased in the IR compiler (`compile_apply`)
///   but parsed as regular call-expressions by `twig-parser`, so the type
///   checker sees them as unresolved variables without this registration.
fn register_builtins(&mut self) {
    let names: &[&str] = &[
        // Arithmetic / comparison (TW00 core + LANG52)
        "+", "-", "*", "/", "=", "<", ">", "<=", ">=",
        "modulo", "remainder", "quotient",
        // Cons cells
        "cons", "car", "cdr",
        // Predicates
        "null?", "pair?", "number?", "symbol?", "not", "boolean?",
        "equal?", "list?",
        // List stdlib (LANG52)
        "list", "length", "append", "reverse", "list-ref", "assoc",
        // Symbol utilities
        "symbol-append",
        // Conversions
        "number->string", "string->symbol", "symbol->string",
        // String and char operations (LANG58)
        "string-length", "string-ref", "substring", "string-append",
        "string->number", "string=?", "string<?", "string>?",
        "char->integer", "integer->char",
        "char-alphabetic?", "char-numeric?", "char-whitespace?",
        // I/O
        "print",
        // Host I/O (LANG52)
        "host/write_string", "host/read_line", "host/read_file",
        // Higher-order list operations (LANG55)
        "map", "filter", "fold-left", "fold-right",
        // Special forms that parse as calls (LANG52)
        "and", "or",
    ];
    for &name in names {
        self.globals.insert(name.to_owned(), TwigKind::Any);
    }
}
```

### 2. `code/twig/compiler/span.tw`

Change the module declaration from:

```scheme
(module compiler/span
  (typed lenient)
  …)
```

to:

```scheme
(module compiler/span
  (typed strict)
  …)
```

This is possible because:
- `Span` constructor is registered by `collect_forms` via `register_record`.
- `and`, `>=`, `<=` are now pre-registered as builtins.
- No names from imported modules are used (span.tw has no imports).

### 3. Tests in `twig-type-checker/src/lib.rs`

Add `mod tw05o_tests` with the following tests:

| Test | What it verifies |
|------|-----------------|
| `builtin_arithmetic_resolves` | `(+ 1 2)` in strict mode → `ok: true` |
| `builtin_list_ops_resolve` | `(null? nil)`, `(car (list 1))` → ok |
| `builtin_string_ops_resolve` | `(string-length "hi")` → ok |
| `builtin_and_or_resolve` | `(and #t #f)`, `(or #f #t)` → ok |
| `builtin_host_io_resolves` | `(host/read_file "x")` → ok (no arity error) |
| `builtin_hof_resolves` | `(map nil nil)` → ok |
| `builtin_does_not_block_stub_shadow` | Explicit `(define (+ a b) 0)` shadows pre-registered `+`; call still ok |
| `span_tw_strict_mode_compiles` | Compile span.tw (copied to a temp string) in strict mode → ok |

### 4. Version bumps

- `twig-type-checker`: bump minor (e.g. `0.x.y → 0.(x+1).0`)
- Update `CHANGELOG.md` for twig-type-checker

---

## Acceptance Criteria

1. `cargo test -p twig-type-checker -- tw05o` — all 8 tests pass.
2. `cargo test -p twig-module-driver` — all 79 tests still pass; no
   "unresolved variable" warnings in stderr.
3. `cargo build -p twig-type-checker` — clean build.

---

## What this does NOT change

- Modules other than `span.tw` remain `(typed lenient)` — converting them
  requires also registering imported names (multi-module strict mode is
  TW05-P).
- Record accessors and predicates (`span-start`, `Span?`, …) are not
  pre-registered — they are emitted by the IR compiler as `call_builtin` and
  the type checker does not check them independently.
- `TwigKind::Function { arity }` is not used for builtins — variadic builtins
  (`list`, `string-append`) would cause false arity errors.

---

## Files Changed

| File | Change |
|------|--------|
| `code/specs/LANG69-tw05o-typecheck-builtin-prelude.md` | **new** |
| `code/packages/rust/twig-type-checker/src/env.rs` | add `register_builtins`, call from `new()` |
| `code/packages/rust/twig-type-checker/src/lib.rs` | add `tw05o_tests` (8 tests) |
| `code/packages/rust/twig-type-checker/Cargo.toml` | version bump |
| `code/packages/rust/twig-type-checker/CHANGELOG.md` | prepend entry |
| `code/twig/compiler/span.tw` | `(typed lenient)` → `(typed strict)` |

---

## Commit Sequence

1. `docs(specs)` — `LANG69-tw05o-typecheck-builtin-prelude.md`
2. `feat(twig-type-checker)` — `register_builtins` in `TypeEnv::new()` + tests
3. `feat(twig)` — `span.tw` → `(typed strict)`

---

## Verification

```bash
cargo test -p twig-type-checker -- tw05o      # 8 new tests pass
cargo test -p twig-type-checker               # all existing tests still pass
cargo test -p twig-module-driver              # 79 tests pass; no unresolved warnings
cargo build -p twig-type-checker              # clean build
```
