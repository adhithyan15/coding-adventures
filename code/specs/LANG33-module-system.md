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
/// 1. Collect the export map: `HashMap<(module_name, public_name), &IIRFunction>`.
/// 2. For each import in each module, resolve it from the export map.
///    - Type-check parameter and return types if provided.
///    - Accumulate `LinkError::Unresolved` if not found.
/// 3. Merge all functions into a single `IIRModule`:
///    - Rename colliding private functions by prefixing with `"<module>::"`.
///    - Rewrite call instructions that reference imported functions to use the
///      merged name.
/// 4. Preserve `entry_point` from the first module that has one.
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

    /// Two modules export the same (module_name, function_name) pair.
    DuplicateExport {
        module_name:   String,
        function_name: String,
    },

    /// A call instruction inside a function references a name that is neither
    /// a local function nor a declared import.
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
5. BEAM: a module with explicit `exports` emits only those functions in ExpT.
6. WASM: a module with imports emits a WASM import section with correct entries,
   and function index offsets are applied correctly.
7. ≥ 30 tests in `iir-linker/tests/test_linker.rs`.
8. ≥ 5 tests each for BEAM/WASM/JVM/CLR export/import round-trips.
9. `cargo build --workspace` is clean.

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
