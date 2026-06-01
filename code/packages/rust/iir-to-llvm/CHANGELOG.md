# Changelog — iir-to-llvm

All notable changes to this crate are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.1.0] — 2026-06-01 (LLVM01 — crate skeleton)

### Added — empty-module emission

First release.  Implements item LLVM01 of the
[multi-language backend plan][plan]: a crate skeleton that emits a valid
**empty** LLVM textual IR (`.ll`) module — a `; ModuleID = '<name>'`
comment plus a `target triple = "<triple>"` directive.

#### Public surface

```rust
pub struct IIRLlvmConfig {
    pub module_name: String,
    pub target_triple: String,
}
impl IIRLlvmConfig {
    pub fn new(module_name: impl Into<String>) -> Self;
    pub fn with_target(self, triple: impl Into<String>) -> Self;
}

pub enum IIRLlvmError {
    ValidationFailed(Vec<String>),
    UnsupportedOp     { function: String, op: String },
    UnsupportedType   { function: String, type_hint: String },
    InvalidOperand    { function: String, detail: String },
}

pub fn validate_for_llvm(module: &IIRModule) -> Vec<String>;
pub fn lower_iir_to_llvm(
    module: &IIRModule,
    cfg: &IIRLlvmConfig,
) -> Result<String, IIRLlvmError>;
```

#### What is NOT in v0.1.0

- **No instruction lowering.**  Function bodies in the input `IIRModule`
  are ignored.  v0.2.0 (LLVM02) starts lowering `ret_void` / `ret` /
  `const` / `mov`.
- **No `lang-aot --backend=llvm` wiring.**  Deferred to LLVM04.
- **No `llvm-sys` dependency.**  Textual `.ll` only — see the README and
  spec for the rationale.

#### Why textual `.ll`?

- Zero build-time dep: CI doesn't need LLVM installed.
- The output is the human-readable form — `assert!`-able in tests.
- Adding a sibling `llvm-sys` emitter later is a non-breaking change.

#### Why a fixed default `target_triple`?

The default is the literal string `"x86_64-unknown-linux-gnu"` rather
than a host-derived value.  Reasons:

- Test output is byte-identical across CI runners.
- Cross-compilation footguns are avoided — the user opts into a host
  override via `.with_target(...)` rather than receiving it implicitly.

#### Tests added

* `validate_returns_empty_for_empty_module`
* `output_contains_module_id_comment`
* `output_contains_target_triple`
* `output_starts_with_comment_or_target` (LLVM01 acceptance criterion)
* `default_config_has_nonempty_triple`
* `new_sets_module_name_keeps_default_triple`
* `errors_display_without_panic`

[plan]: ../../../specs/MULTILANG-BACKEND-PLAN.md
