# LANG33 — Module System at the IIR Level

## Why this spec exists

LANG32 wired global variables and I/O into all four native backends.  The
remaining gap before Twig programs can call each other across compilation units
is a **module system**.  Today every `IIRModule` is self-contained: all the
functions it calls must be defined inside it.  Cross-module references (e.g. a
math library calling into a list library, or a Twig REPL importing a pre-compiled
helper module) are impossible.

This spec adds exports and imports at the `IIRModule` level and a new
`iir-linker` crate that resolves cross-module references into a single linked
module — or, for backends that support it natively (WASM, JVM, CLR), into a
multi-unit artifact that the runtime resolves at load time.

---

## Design goals

1. **Free for every language** — Twig, NIB, Prolog, Brainfuck all get a module
   system without any language-specific work once the IIR layer is complete.
2. **Minimal surface** — the first version covers the 80 % case: function exports
   and imports.  Global variable exports are deferred to LANG33b.
3. **Backward compatible** — existing `IIRModule` structs with empty
   `exports`/`imports` continue to work identically.
4. **Static linking default** — `iir-linker` produces a single merged
   `IIRModule`.  Dynamic linking (WASM import sections, JVM `invokeinterface`)
   is backend-specific and tracked in LANG33b.

---

## New IIRModule fields

```rust
pub struct IIRModule {
    // ── existing ────────────────────────────────────────────────────────────
    pub name:        String,
    pub functions:   Vec<IIRFunction>,
    pub entry_point: Option<String>,
    pub language:    String,

    // ── LANG33 (new) ────────────────────────────────────────────────────────

    /// Functions this module makes visible to other modules.
    ///
    /// An empty list means the module exports nothing (a pure program, not
    /// a library).  Backends that have a native export concept (WASM, JVM)
    /// use this list to populate their export sections; BEAM generates an
    /// `ExpT` chunk entry for each name.
    ///
    /// The default (`Vec::new()`) preserves backward compatibility: modules
    /// built before LANG33 export nothing, which is the same as before.
    pub exports: Vec<IIRExport>,

    /// Functions this module requires from other modules.
    ///
    /// During static linking (`iir-linker`), each import is resolved to a
    /// function in a peer `IIRModule`.  During JIT/interpreter execution,
    /// unresolved imports cause a `LinkError` at load time.
    ///
    /// The default (`Vec::new()`) preserves backward compatibility.
    pub imports: Vec<IIRImport>,
}
```

### `IIRExport`

```rust
/// A function that this module makes visible to other modules.
#[derive(Debug, Clone, PartialEq)]
pub struct IIRExport {
    /// The function name as it appears in `IIRModule::functions`.
    ///
    /// Must refer to a function that exists in the same module.
    /// Validated by `IIRModule::validate()`.
    pub function_name: String,

    /// Optional alias to publish under a different public name.
    ///
    /// `None` → export under the same name as `function_name`.
    /// `Some("public_add")` → export the function as `"public_add"` while
    /// the internal name remains `function_name`.
    ///
    /// Backends use `alias.as_deref().unwrap_or(&function_name)` as the
    /// external symbol name.
    pub alias: Option<String>,
}

impl IIRExport {
    pub fn new(function_name: impl Into<String>) -> Self {
        IIRExport { function_name: function_name.into(), alias: None }
    }

    pub fn with_alias(mut self, alias: impl Into<String>) -> Self {
        self.alias = Some(alias.into());
        self
    }

    /// The name external callers use (alias if set, otherwise function_name).
    pub fn public_name(&self) -> &str {
        self.alias.as_deref().unwrap_or(&self.function_name)
    }
}
```

### `IIRImport`

```rust
/// A function required from another module.
#[derive(Debug, Clone, PartialEq)]
pub struct IIRImport {
    /// The module that provides this function.
    ///
    /// For static linking, this must match the `name` of a peer `IIRModule`
    /// passed to `iir_linker::link`.  For dynamic linking (WASM/JVM), this
    /// becomes the module/class name in the import section.
    pub module_name: String,

    /// The function name as published by the exporting module (the `public_name`
    /// of the corresponding `IIRExport`).
    pub function_name: String,

    /// The local alias used inside *this* module's instructions.
    ///
    /// Call instructions use this name as their callee.  During linking,
    /// the linker inlines or redirects to the actual function.
    ///
    /// `None` → use `function_name` locally.
    pub local_alias: Option<String>,

    /// Expected parameter types for type-checking during linking.
    ///
    /// Empty means "don't check" (trusted import).
    pub param_types: Vec<String>,

    /// Expected return type.
    ///
    /// `"any"` means "don't check".
    pub return_type: String,
}

impl IIRImport {
    pub fn new(
        module_name: impl Into<String>,
        function_name: impl Into<String>,
        return_type: impl Into<String>,
    ) -> Self {
        IIRImport {
            module_name: module_name.into(),
            function_name: function_name.into(),
            local_alias: None,
            param_types: Vec::new(),
            return_type: return_type.into(),
        }
    }

    /// The name call instructions in this module use to refer to the imported
    /// function.
    pub fn local_name(&self) -> &str {
        self.local_alias.as_deref().unwrap_or(&self.function_name)
    }
}
```

---

## Updated `IIRModule::validate()`

The existing validator is extended with two new checks:

| Check | Error key | Condition |
|-------|-----------|-----------|
| ExportNotFound | `ExportNotFound` | An export's `function_name` does not appear in `self.functions` |
| DuplicateExport | `DuplicateExport` | Two exports have the same `public_name()` |

Imports are **not** validated here (they require peer modules, which `validate`
doesn't have access to).  Import resolution is the linker's job.

---

## New crate: `iir-linker`

### Crate layout

```
code/packages/rust/iir-linker/
  Cargo.toml
  BUILD
  CHANGELOG.md
  README.md
  src/
    lib.rs       — re-exports + module-level doc
    error.rs     — LinkError enum
    resolve.rs   — import/export resolution
    merge.rs     — module merging (static linking)
    linker.rs    — IIRLinker struct + link() entry point
  tests/
    test_linker.rs — ≥ 30 tests
```

### API

```rust
/// Static link two or more `IIRModule`s into one merged module.
///
/// # Algorithm
///
/// 0. Reject two input modules with the same `name`
///    (`LinkError::DuplicateModuleName`).  Exports and import rewrites are
///    keyed by module name, so two modules called `util` would share one
///    set of import rewrites: an import in one could override the other's
///    own function of the same name.  (#18429)
/// 1. Collect the export map: `HashMap<(module_name, public_name), &IIRFunction>`.
///    - The same key twice → `LinkError::DuplicateExport`.
///    - The same *public name* from two different modules →
///      `LinkError::ConflictingExport`.  Exported functions keep their bare
///      public name in the merged module, so only one function may own it;
///      before #18429 both kept the name and every reference — including
///      the second module's own calls — ran the first module's function.
///      Export aliases count by their public (aliased) name.  One module
///      holding two functions of the same name (Twig's user `main` beside
///      its synthesized `main`) is not a conflict.
/// 2. For each import in each module, resolve it from the export map.
///    - Type-check parameter and return types if provided.
///    - Accumulate `LinkError::Unresolved` if not found.
///    - Check that the merged entry point — the `entry_point` of the first
///      module that has one — names a function that module defines or the
///      local name of one of its imports; otherwise
///      `LinkError::UnresolvedEntryPoint`.  (Before #18429 an unresolved
///      name stayed bare and could bind to another module's private
///      function.)  Later modules' entry points are unused and unchecked.
///    - Check every *function reference* — the operands step 3 rewrites
///      (`call` callee, `alloc_closure` body, `task_spawn` / `group_spawn`
///      target).  Inside module `m` each must name a function `m` defines
///      or the local name of one of `m`'s imports; anything else is
///      `LinkError::UndeclaredCall`, reported once per
///      `(module, function, name)`.  (#18432: before this, nothing raised
///      `UndeclaredCall`.  A name that was neither stayed bare and bound to
///      whichever function owned it after merging — so with two libraries
///      each holding a private `helper`, an app's un-imported `call helper`
///      ran the first library's private function.)  This is what makes
///      "private by default" (design goal of `exports`) hold at link time.
///      Builtins are unaffected: producers reach them with `call_builtin`,
///      not `call`, and no VM resolves an unknown `call` target to a
///      builtin.
/// 3. Merge all functions into a single `IIRModule`:
///    - Rename colliding private functions by prefixing with `"<module>::"`.
///      If that name is taken too (another module exported an alias spelled
///      `"<module>::<fn>"`), append `$0`, `$1`, … until the name is free.
///      The search always ends — of `taken + 1` candidates one is free — so
///      it needs no bound and never panics.  (#18432: it used to `assert!`
///      a bound of *functions + 1*, but export aliases claim names without
///      adding functions, so a handful of crafted aliases panicked
///      `link()` in release builds too.)
///    - Rewrite every operand that *names a function* — `call` callee
///      (`srcs[0]`), `alloc_closure` body name (`srcs[0]`, an
///      `Operand::Str`), `task_spawn` fn (`srcs[0]`) and `group_spawn`
///      fn (`srcs[1]`) — through the module's rename map: its own renamed
///      functions plus its imports (imports win on a clash).  Data operands
///      such as `const(Str(..))` are never rewritten.  (#18421: earlier
///      versions rewrote only `call` callees of imported names, so a renamed
///      private function — e.g. a second module's `__lambda_0` — was still
///      referenced by its old bare name and silently bound to another
///      module's function.)  The spawn operands are function *names*, never
///      closure registers — LANG28 pins this contract (#18429), because the
///      rewrite matches on operand text.
/// 4. Preserve `entry_point` from the first module that has one, mapped
///    through that module's rename map.
/// 5. Return the merged module (or all errors if any were found).
pub fn link(modules: &[IIRModule]) -> Result<IIRModule, Vec<LinkError>>;

/// Link-and-fail-fast variant.  Returns `Err` with the first error encountered.
pub fn link_strict(modules: &[IIRModule]) -> Result<IIRModule, LinkError>;

/// Verify that all imports in `module` are satisfied by the exports in `providers`.
///
/// Does not merge or rewrite — useful for pre-flight checking in the REPL.
pub fn verify_imports(module: &IIRModule, providers: &[&IIRModule]) -> Vec<LinkError>;
```

### `LinkError`

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum LinkError {
    /// An import could not be resolved.
    Unresolved {
        importing_module: String,
        import_module:    String,
        import_function:  String,
    },

    /// An import's expected param types don't match the export's actual types.
    TypeMismatch {
        importing_module: String,
        exporting_module: String,
        function:         String,
        expected:         Vec<String>,
        actual:           Vec<String>,
    },

    /// The same (module_name, function_name) pair is exported twice.
    DuplicateExport {
        module_name:   String,
        function_name: String,
    },

    /// Two different modules export the same public name (#18429).
    ConflictingExport {
        function_name: String,
        first_module:  String,
        second_module: String,
    },

    /// Two input modules share a name (#18429).
    DuplicateModuleName {
        module_name: String,
    },

    /// The entry module's entry_point is neither defined nor imported by
    /// that module (#18429).
    UnresolvedEntryPoint {
        module_name: String,
        entry_point: String,
    },

    /// A function reference (`call` callee, `alloc_closure` body, spawn
    /// target) names something that is neither a function of its module
    /// nor the local name of a declared import — e.g. another module's
    /// private function (#18432).
    UndeclaredCall {
        in_module:   String,
        in_function: String,
        callee:      String,
    },
}
```

---

## Updated `IIRModule::new()`

```rust
pub fn new(name: impl Into<String>, language: impl Into<String>) -> Self {
    IIRModule {
        name:        name.into(),
        functions:   Vec::new(),
        entry_point: Some("main".to_string()),
        language:    language.into(),
        exports:     Vec::new(),   // ← LANG33
        imports:     Vec::new(),   // ← LANG33
    }
}
```

All existing call sites that construct `IIRModule { ... }` with struct literal
syntax must add `exports: vec![], imports: vec![]`.  The workspace-level `cargo
build` enforces this.

---

## Per-backend changes

### BEAM (`iir-to-beam`)

BEAM already has a native export table (`ExpT` chunk).  Currently the backend
exports every function.  With LANG33:

- Only functions in `module.exports` are added to the export table.
- If `exports` is empty, fall back to exporting all functions (backward compat).
- Cross-module calls become `call_ext` instructions using the import table.
  For the static-linking path, `iir-linker` merges modules first, so the
  backend never sees an unresolved import.

### WASM (`iir-to-wasm`)

WASM has native import/export sections.  With LANG33:

- `module.exports` → WASM `Export` entries (kind = Function).
- `module.imports` → WASM `Import` entries (module = `import.module_name`,
  name = `import.function_name`), with the function type derived from
  `param_types` / `return_type`.
- Function index map: imported functions occupy indices 0..N-1; defined
  functions start at N.  (Same `fn_idx_base` logic as the LANG32 io_out
  import, generalised.)

### JVM (`iir-to-jvm-class-file`)

- Exported functions are marked `public static`.
- Imported functions become `invokestatic` calls with a Methodref CP entry
  pointing to the exporting class (module name → class name mapping).
- For the static-linking path, all functions are in one class so no
  cross-class references are needed.

### CLR (`iir-to-cil-bytecode`)

- Exported functions get `.method public static`.
- Imported functions become `call` instructions with a MemberRef token
  pointing to the exporting assembly.
- Static linking collapses everything into one assembly.

---

## Serialisation (`interpreter-ir/src/serialise.rs`)

The `serialise_module` / `deserialise_module` functions are updated to include
`exports` and `imports`.  Binary format:

| Field | Tag | Format |
|-------|-----|--------|
| exports length | `0x10` | u32 little-endian |
| each export | — | len-prefixed UTF-8 function_name + len-prefixed UTF-8 alias (empty = None) |
| imports length | `0x11` | u32 little-endian |
| each import | — | len-prefixed module_name + len-prefixed function_name + len-prefixed local_alias + u32 param_count + (len-prefixed type) × N + len-prefixed return_type |

---

## Acceptance criteria

1. `IIRModule` with non-empty `exports`/`imports` round-trips through
   `serialise`/`deserialise` losslessly.
2. `iir-linker::link` merges two modules where module A imports a function
   exported by module B.
3. `iir-linker::link` returns `LinkError::Unresolved` when an import has no
   matching export.
4. `iir-linker::link` returns `LinkError::TypeMismatch` when param/return
   types don't match.
5. `iir-linker::link` returns `LinkError::ConflictingExport` when two
   modules export the same public name, `LinkError::DuplicateModuleName`
   when two input modules share a name, and
   `LinkError::UnresolvedEntryPoint` when the entry point does not resolve
   inside its module (#18429).
5a. `iir-linker::link` returns `LinkError::UndeclaredCall` when a function
   reference names another module's private function (or any name the
   module neither defines nor imports), and returns a renamed module — not
   a panic — when export aliases occupy every `"<module>::<fn>$N"` name the
   old counter bound allowed (#18432).
6. BEAM: a module with explicit `exports` emits only those functions in ExpT.
7. WASM: a module with imports emits a WASM import section with correct entries,
   and function index offsets are applied correctly.
8. ≥ 30 tests in `iir-linker/tests/test_linker.rs`.
9. ≥ 5 tests each for BEAM/WASM/JVM/CLR export/import round-trips.
10. `cargo build --workspace` is clean.

---

## Sister specs

| Spec | Scope |
|------|-------|
| LANG31 | iir-builtin-lowering Phase 1+2; four e2e pipeline crates |
| LANG32 | global variables + I/O at IIR level |
| **LANG33 (this)** | module system: exports/imports at IIR level; `iir-linker` |
| LANG34 | Closures at IIR level: `alloc_closure`/`call_closure` opcodes |
| LANG35 | Real-VM integration tests (erl/java/dotnet/wasmtime) |

---

## What is NOT in LANG33

- **Global variable exports** — a global defined in module A and read in module
  B requires sharing the process-dictionary atom (BEAM) or WASM global import.
  This is LANG33b.
- **Dynamic library loading** — `dlopen`-style late binding is LANG36.
- **Circular imports** — the linker rejects them with a clear error;
  support requires lazy binding and is future work.
- **Versioning / semver** — no version field on `IIRImport`; reserved for the
  package manager layer (outside this spec).
