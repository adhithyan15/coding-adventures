# LANG30 — IIR Codegen Adapters

## Status

Proposed.

---

## Motivation

LANG29 delivered four new Rust crates (`iir-to-beam`, `iir-to-wasm`,
`iir-to-jvm-class-file`, `iir-to-cil-bytecode`) that each lower an `IIRModule`
to a target VM bytecode.  Each crate exposes a `CodeGenerator<IIRModule, _>`
adapter (LANG20 protocol), but there is currently no single place where all
four are registered together.

A consumer wanting to "compile to any backend by name" must:

1. Import all four crates explicitly.
2. Match on a string to decide which to call.
3. Handle four different artifact types in a `match` arm.

This is boilerplate that every consumer will duplicate.  LANG30 eliminates it
by providing:

1. A **`build_iir_codegen_registry()`** factory that populates a
   `CodeGeneratorRegistry` with all four IIR generators.
2. A **`compile_iir()`** free function for the common case of "compile this
   `IIRModule` to whichever backend the caller names, give me a single result
   type."
3. An **`IIRBackendArtifact`** enum that wraps the four different artifact types
   so callers can pattern-match on them or serialize/display them uniformly.

---

## Scope

### In scope

- `iir-codegen-adapters` Rust crate
- `IIRBackendArtifact` enum with four variants
- `IIRAdapterError` error type
- `build_iir_codegen_registry() -> CodeGeneratorRegistry` — register all 4 IIR backends
- `compile_iir(module: &IIRModule, backend: &str) -> Result<IIRBackendArtifact, IIRAdapterError>` — dispatch by name
- `list_iir_backends() -> Vec<&'static str>` — enumerate known backend names
- Unit tests ≥ 85% coverage

### Out of scope

- Serialization of `IIRBackendArtifact` to disk.
- Any new IIR backends (those live in their own LANG-N crates).
- Runtime execution (that is `jit-core` / `vm-runtime` territory).

---

## Crate layout

```
code/packages/rust/iir-codegen-adapters/
  Cargo.toml
  BUILD
  README.md
  CHANGELOG.md
  src/
    lib.rs       — module-level docs, re-exports, public API
    artifact.rs  — IIRBackendArtifact enum + display
    error.rs     — IIRAdapterError enum
    registry.rs  — build_iir_codegen_registry()
    dispatch.rs  — compile_iir() + list_iir_backends()
  tests/
    test_adapters.rs  — ≥ 40 integration tests
```

---

## Public API

### `IIRBackendArtifact`

```rust
/// The output of compiling an `IIRModule` to one of the four IIR backends.
///
/// Each variant wraps the natural artifact type produced by that backend's
/// lowering pass.  The variant name matches the backend identifier string
/// returned by `list_iir_backends()`.
pub enum IIRBackendArtifact {
    /// BEAM module — produced by the `"iir-beam"` backend.
    Beam(BEAMModule),
    /// WebAssembly module — produced by the `"iir-wasm"` backend.
    Wasm(WasmModule),
    /// JVM class file — produced by the `"iir-jvm"` backend.
    Jvm(JvmClassFile),
    /// CIL program artifact — produced by the `"iir-clr"` backend.
    Clr(CILProgramArtifact),
}
```

Accessor helpers are provided for each variant:
- `as_beam(&self) -> Option<&BEAMModule>`
- `as_wasm(&self) -> Option<&WasmModule>`
- `as_jvm(&self)  -> Option<&JvmClassFile>`
- `as_clr(&self)  -> Option<&CILProgramArtifact>`
- `backend_name(&self) -> &'static str` — returns the backend identifier string

`Display` is implemented to show the backend name and a size hint (e.g.
`"Wasm(functions=3)"`, `"Jvm(methods=2, constant_pool=14)"`).

### `IIRAdapterError`

```rust
pub enum IIRAdapterError {
    /// The requested backend name is not registered.
    UnknownBackend {
        /// The name the caller provided.
        requested: String,
        /// All names currently registered.
        available: Vec<String>,
    },
    /// The module failed pre-flight validation for the chosen backend.
    ValidationFailed {
        /// The backend that rejected it.
        backend: String,
        /// Human-readable error list from `validate()`.
        errors: Vec<String>,
    },
    /// The lowering step returned an error after validation passed.
    LoweringFailed {
        backend: String,
        detail: String,
    },
}
```

`Display` is implemented; `std::error::Error` is derived.

### `build_iir_codegen_registry()`

```rust
/// Populate a `CodeGeneratorRegistry` with the four IIR backends.
///
/// The registry maps each backend's stable name to its code generator:
///
/// | Name        | Generator type           |
/// |-------------|--------------------------|
/// | `"iir-beam"` | `IIRBeamCodeGenerator`  |
/// | `"iir-wasm"` | `IIRWasmCodeGenerator`  |
/// | `"iir-jvm"`  | `IIRJvmCodeGenerator`   |
/// | `"iir-clr"`  | `IIRClrCodeGenerator`   |
///
/// The generators are instantiated with a placeholder module name
/// `"iir_module"`.  Callers that need a specific module name should
/// instantiate generators directly and register them manually.
pub fn build_iir_codegen_registry() -> CodeGeneratorRegistry
```

### `compile_iir()`

```rust
/// Compile `module` with the named backend.
///
/// This is the primary entry point for single-backend compilation:
///
/// ```rust
/// use interpreter_ir::IIRModule;
/// use iir_codegen_adapters::compile_iir;
///
/// let module: IIRModule = /* ... */;
/// let artifact = compile_iir(&module, "iir-wasm").unwrap();
/// let wasm = artifact.as_wasm().unwrap();
/// ```
///
/// # Errors
///
/// - `UnknownBackend` if `backend` is not one of the four registered names.
/// - `ValidationFailed` if the module fails the backend's pre-flight checks.
/// - `LoweringFailed` if lowering panics or returns an unexpected error.
pub fn compile_iir(
    module: &IIRModule,
    backend: &str,
) -> Result<IIRBackendArtifact, IIRAdapterError>
```

### `list_iir_backends()`

```rust
/// Return the stable names of all registered IIR backends, sorted alphabetically.
///
/// Currently: `["iir-beam", "iir-clr", "iir-jvm", "iir-wasm"]`.
pub fn list_iir_backends() -> Vec<&'static str>
```

---

## Design decisions

### Why an enum instead of a dyn trait?

The four artifact types (`BEAMModule`, `WasmModule`, `JvmClassFile`,
`CILProgramArtifact`) are from four different external crates with no shared
trait.  A closed `enum` is the idiomatic Rust solution when the set of variants
is known at compile time — it allows exhaustive matching and carries zero
indirection overhead compared to `Box<dyn Any>`.

### Why both a registry AND a dispatch function?

The `CodeGeneratorRegistry` is the power-user API: it allows type-erased storage
and retrieval with downcasting, enabling dynamic backend selection in a pipeline
driver that does not know which backends are available at compile time.

`compile_iir()` is the convenience API for the 80% case: "I have a module and a
backend name string — give me the artifact or an error."  It wraps the registry
and performs the downcast + lowering in one step.

### Module name placeholder

Generators registered via `build_iir_codegen_registry()` use `"iir_module"` as
the placeholder module name.  Callers that need a specific name should
instantiate generators directly.  A future `compile_iir_named()` overload can
accept a `module_name: &str` parameter; for now the placeholder is acceptable
because the name is cosmetic (it appears in WASM custom-name sections and JVM
`this_class_name`, not in the bytecode semantics).

### Validation before lowering

`compile_iir()` always calls `validate()` before `generate()`.  If validation
fails it returns `ValidationFailed` with the full error list.  This matches the
LANG20 contract and gives the caller one clean error type instead of a mix of
`Result<_, E>` types from four different crates.

---

## Dependencies

```toml
[dependencies]
interpreter-ir        = { path = "../interpreter-ir" }
codegen-core          = { path = "../codegen-core" }
iir-to-beam           = { path = "../iir-to-beam" }
iir-to-wasm           = { path = "../iir-to-wasm" }
iir-to-jvm-class-file = { path = "../iir-to-jvm-class-file" }
iir-to-cil-bytecode   = { path = "../iir-to-cil-bytecode" }
```

No external (crates.io) runtime dependencies.  The artifact types
(`BEAMModule`, `WasmModule`, `JvmClassFile`, `CILProgramArtifact`) are
re-exported from the four backend crates and transitively available.

---

## Test plan

The integration tests in `tests/test_adapters.rs` use a standard `minimal_module()`
fixture (one `ret_void` function).  Coverage goals:

| Test group | What is tested |
|-----------|----------------|
| Registry  | `build_iir_codegen_registry()` has 4 backends; names are correct |
| Registry  | Each backend can be retrieved and downcast to its concrete type |
| Dispatch  | `compile_iir(module, "iir-beam")` returns `IIRBackendArtifact::Beam` |
| Dispatch  | `compile_iir(module, "iir-wasm")` returns `IIRBackendArtifact::Wasm` |
| Dispatch  | `compile_iir(module, "iir-jvm")`  returns `IIRBackendArtifact::Jvm`  |
| Dispatch  | `compile_iir(module, "iir-clr")`  returns `IIRBackendArtifact::Clr`  |
| Dispatch  | Unknown backend → `IIRAdapterError::UnknownBackend` |
| Dispatch  | Invalid module → `IIRAdapterError::ValidationFailed` |
| Artifact  | `as_beam()` / `as_wasm()` / `as_jvm()` / `as_clr()` accessors |
| Artifact  | `backend_name()` returns the correct string for each variant |
| Artifact  | `Display` includes backend name |
| Error     | `IIRAdapterError::Display` shows useful context |
| List      | `list_iir_backends()` returns 4 sorted names |
| Arithmetic round-trip | `add(a, b)` module compiles to all 4 backends without error |

---

## Workflow

1. Feature branch: `feat/lang30-iir-codegen-adapters` (off `feat/lang29-iir-direct-backends`)
2. Commit 1: spec (this document)
3. Commit 2: `iir-codegen-adapters` crate + tests
4. Commit 3: workspace `Cargo.toml` update
5. Security review → push → PR
6. Note: this PR **depends on** LANG29 (#2708) merging first; rebase on `main`
   after #2708 lands.
