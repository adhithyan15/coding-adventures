# LANG70 — TW05-P Part 1: Generated Symbol Registration

> **LANG69 (TW05-O) is merged.**  This spec covers TW05-P Part 1:
> registering generated record accessors, record predicates, and union
> variant predicates in `TypeEnv` during Pass 1 so that modules using their
> own generated symbols can run in `(typed strict)` mode.

---

## Background

### The generated-symbol gap

After LANG69 (TW05-O), `TypeEnv::new()` pre-registers all Twig runtime
builtins.  Pass 1 (`collect_forms`) also registers:

- Record constructors: `Span`, `Token`, `IirBuilder` → `TwigKind::Record(name)`
- Union type names: `TokenKind`, `Expr` → `TwigKind::Union(name)`
- Union variant constructors: `TkInteger`, `IntLit` → `TwigKind::Function { arity }`

**Not yet registered:**

- Record predicates: `span?`, `token?`, `iirinstr?` — `{lowercase(name)}?`
- Record field accessors: `span-start`, `token-kind`, `iirbuilder-reg-count` —
  `{lowercase(name)}-{field}`
- Union variant predicates: `TkInteger?`, `SevError?`, `IntLit?` — `{variant}?`
- Union variant field accessors: `intlit-value`, `ifexpr-cond` —
  `{lowercase(variant)}-{field}`

These are emitted by `twig-ir-compiler` as `call_builtin` instructions, and
their names follow exact conventions (see below).  Without registration the
type checker reports "unresolved variable" for each generated-symbol call,
blocking strict mode for any module that uses accessors or predicates.

### Naming conventions (matching `twig-ir-compiler/src/compiler.rs`)

From the IR compiler's `collect_forms` and `compile_record_def`:

| Kind | Convention | Example |
|------|-----------|---------|
| Record constructor | `RecordName` (as-is) | `Token` |
| Record predicate | `{to_lowercase(RecordName)}?` | `token?` |
| Record field accessor | `{to_lowercase(RecordName)}-{field_name}` | `token-kind` |
| Union type | `UnionName` (as-is) | `TokenKind` |
| Union variant constructor | `VariantName` (as-is) | `TkInteger` |
| Union variant predicate | `{VariantName}?` (NOT lowercased) | `TkInteger?` |
| Union variant field accessor | `{to_lowercase(VariantName)}-{field_name}` | `intlit-value` |

Note: record predicates use `to_lowercase(RecordName)`, but union variant
predicates use the **original case** of `VariantName`.  This asymmetry
matches the IR compiler's code at lines 348 and 355 of `compiler.rs`.

### Which modules this unblocks

After LANG70, the following modules can use `(typed strict)`:

| Module | Reason strictly safe after LANG70 |
|--------|----------------------------------|
| `span.tw` | ✅ already strict (LANG69) |
| `token.tw` | 0 `define` forms — trivially safe |
| `ast.tw` | 0 `define` forms — trivially safe |
| `iir-types.tw` | 0 `define` forms — trivially safe |
| `diagnostic.tw` | Only calls own constructors (`Diagnostic`, `SevError`, …) |
| `iir-builder.tw` | Calls own accessors (`iirbuilder-name`, `iirbuilder-reg-count`, …) — safe after accessor registration |

Modules that remain `(typed lenient)` — they call names from **imported**
modules (needs TW05-P Part 2 / LANG71):

| Module | Unresolved imports |
|--------|--------------------|
| `lexer.tw` | `make-span`, `Token`, `TkLParen`, … from span/token |
| `cst-parser.tw` | Token predicates, AST constructors from token/ast |
| `parser.tw` | Token predicates, AST constructors from token/ast |
| `emit.tw` | AST predicates, IirInstr from ast/iir-types/iir-builder |
| `main.tw` | Everything from all modules |

---

## Changes

### 1. `twig-type-checker/src/env.rs`

#### `TypeEnv::register_record` — add accessor + predicate registration

```rust
pub fn register_record(&mut self, r: &twig_parser::RecordDef) {
    let field_names: Vec<String> = r.fields.iter().map(|f| f.name.clone()).collect();
    self.records.insert(r.name.clone(), field_names);
    // Constructor: stored as Record kind (already present).
    self.globals
        .insert(r.name.clone(), TwigKind::Record(r.name.clone()));

    // ── NEW: register generated symbols ─────────────────────────────────
    // Naming mirrors twig-ir-compiler/src/compiler.rs lines 343-348.
    let prefix = r.name.to_lowercase();
    // Predicate: <lower(RecordName)>?
    self.globals
        .insert(format!("{prefix}?"), TwigKind::Any);
    // Field accessors: <lower(RecordName)>-<field_name>
    for f in &r.fields {
        self.globals
            .insert(format!("{prefix}-{}", f.name), TwigKind::Any);
    }
}
```

#### `TypeEnv::register_union` — add variant predicate + variant field accessor registration

```rust
pub fn register_union(&mut self, u: &twig_parser::UnionDef) {
    let variant_names: Vec<String> = u.variants.iter().map(|v| v.name.clone()).collect();
    self.unions.insert(u.name.clone(), variant_names);
    self.globals
        .insert(u.name.clone(), TwigKind::Union(u.name.clone()));
    for v in &u.variants {
        // Variant constructor (already present).
        self.globals.insert(
            v.name.clone(),
            TwigKind::Function { arity: v.fields.len() },
        );
        // ── NEW: variant predicate: <VariantName>?  (original case, not lowercased)
        // Mirrors twig-ir-compiler/src/compiler.rs line 355:
        //   format!("{}?", variant.name)
        self.globals
            .insert(format!("{}?", v.name), TwigKind::Any);
        // ── NEW: variant field accessors: <lower(VariantName)>-<field_name>
        // Mirrors twig-ir-compiler/src/compiler.rs lines 357-359:
        //   let vprefix = variant.name.to_lowercase();
        //   format!("{vprefix}-{}", f.name)
        let vprefix = v.name.to_lowercase();
        for f in &v.fields {
            self.globals
                .insert(format!("{vprefix}-{}", f.name), TwigKind::Any);
        }
    }
}
```

### 2. Convert 5 modules to `(typed strict)`

Change the `(typed lenient)` declaration to `(typed strict)` in:
- `code/twig/compiler/token.tw`
- `code/twig/compiler/ast.tw`
- `code/twig/compiler/iir-types.tw`
- `code/twig/compiler/diagnostic.tw`
- `code/twig/compiler/iir-builder.tw`

### 3. Tests in `twig-type-checker/src/lib.rs`

Add `mod tw05p1_tests` with the following tests:

| Test | What it verifies |
|------|-----------------|
| `record_predicate_resolves_in_strict` | `(record R (x : int)) (define (f r) (r? r))` → ok |
| `record_accessor_resolves_in_strict` | `(record R (x : int)) (define (f r) (r-x r))` → ok |
| `union_variant_predicate_resolves_in_strict` | `(union U (A) (B)) (define (f v) (A? v))` → ok |
| `union_variant_field_accessor_resolves_in_strict` | `(union U (A (x : int))) (define (f v) (a-x v))` → ok |
| `diagnostic_tw_strict_mode_compiles` | Full `diagnostic.tw` snippet in strict mode → ok |
| `iir_builder_tw_strict_mode_compiles` | Full `iir-builder.tw` snippet in strict mode → ok |

### 4. Version bump

- `twig-type-checker`: `0.7.0` → `0.8.0`
- Update `CHANGELOG.md` for twig-type-checker

---

## Acceptance Criteria

1. `cargo test -p twig-type-checker -- tw05p1` — all 6 tests pass.
2. `cargo test -p twig-type-checker` — all 98 tests pass (92 prior + 6 new).
3. `cargo test -p twig-module-driver` — all 79 tests still pass.
4. `cargo build -p twig-type-checker` — clean build.

---

## What this does NOT change

- Modules that call imported names (`lexer.tw`, `parser.tw`, `emit.tw`,
  `cst-parser.tw`, `main.tw`) remain `(typed lenient)` — converting them
  requires registering exported names from imported modules (TW05-P Part 2,
  LANG71).
- The `twig-module-driver` is unchanged — it does not yet propagate type
  information from imported modules to the type checker of importing modules.
- `TwigKind::Any` is used for all generated symbols (not `Function { arity }`)
  because the type checker does not verify accessor return types or enforce
  field-count arity on generated functions.

---

## Files Changed

| File | Change |
|------|--------|
| `code/specs/LANG70-tw05p1-generated-symbol-registration.md` | **new** |
| `code/packages/rust/twig-type-checker/src/env.rs` | add accessor+predicate registration |
| `code/packages/rust/twig-type-checker/src/lib.rs` | add `tw05p1_tests` (6 tests) |
| `code/packages/rust/twig-type-checker/Cargo.toml` | version bump 0.7.0 → 0.8.0 |
| `code/packages/rust/twig-type-checker/CHANGELOG.md` | prepend entry |
| `code/twig/compiler/token.tw` | `(typed lenient)` → `(typed strict)` |
| `code/twig/compiler/ast.tw` | `(typed lenient)` → `(typed strict)` |
| `code/twig/compiler/iir-types.tw` | `(typed lenient)` → `(typed strict)` |
| `code/twig/compiler/diagnostic.tw` | `(typed lenient)` → `(typed strict)` |
| `code/twig/compiler/iir-builder.tw` | `(typed lenient)` → `(typed strict)` |

---

## Commit Sequence

1. `docs(specs)` — `LANG70-tw05p1-generated-symbol-registration.md`
2. `feat(twig-type-checker)` — register generated symbols in `register_record`
   / `register_union` + tests
3. `feat(twig)` — convert 5 modules to `(typed strict)`

---

## Verification

```bash
cargo test -p twig-type-checker -- tw05p1      # 6 new tests pass
cargo test -p twig-type-checker                # all 98 tests pass
cargo test -p twig-module-driver               # 79 tests pass
cargo build -p twig-type-checker               # clean build
```
