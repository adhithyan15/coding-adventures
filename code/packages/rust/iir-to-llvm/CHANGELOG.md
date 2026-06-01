# Changelog — iir-to-llvm

All notable changes to this crate are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.2.0] — 2026-06-01 (LLVM02 — function signatures + ret/ret_void/const/mov)

### Added — function lowering and four instructions

Implements item LLVM02 of the [multi-language backend plan][plan].  This
release extends the v0.1.0 skeleton with the smallest set of instructions
that produces a runnable LLVM module:

| IIR op     | Lowering strategy                                      |
|------------|--------------------------------------------------------|
| `const`    | tracked in a name→operand map, no LLVM line emitted    |
| `mov`      | aliases dest to source's operand, no LLVM line emitted |
| `ret_void` | `  ret void`                                           |
| `ret`      | `  ret <ty> <operand>`                                 |

Sample output (`fn answer() -> i64 { const v = 42; ret v }`):

```llvm
; ModuleID = 'iir_module'
target triple = "x86_64-unknown-linux-gnu"

define i64 @answer() {
  ret i64 42
}
```

#### Design choices

* **`const`/`mov` are side-map operations, not LLVM lines.**  An obvious
  alternative is to emit `%dest = add <ty> 0, <src>` for both, but that
  produces no-op SSA assignments that `opt -mem2reg` would have to
  immediately clean up.  The side-map approach gives output that already
  looks like what hand-written `.ll` looks like.
* **Signless integer types.**  IIR's `u32` and `i32` both lower to LLVM
  `i32` — LLVM has no signedness in types.  The sign manifests in the
  opcode (`sdiv` vs `udiv`, `slt` vs `ult`) and will be picked up in
  LLVM03 when arithmetic lowering arrives.
* **Float literal format.**  We emit `{:e}` scientific notation (e.g.
  `1.5e0`), which round-trips through `f64::to_string` for finite values
  and is unambiguously parsed by LLVM.

#### Public surface added

* `IIRLlvmError::UndefinedVariable { function, name }` — surfaced when
  `ret` references a name that was never `const`/`mov`/param-bound.

#### Validator rules (`validate_for_llvm`)

* `SUPPORTED_OPS` whitelist: `["const", "mov", "ret", "ret_void"]`.
  Anything else → `UnsupportedOp`.
* Type rules: `void`, `i{8,16,32,64}`, `u{8,16,32,64}`, `f32`, `f64`.
  Anything else (incl. `ref<…>`, `str`, `bool`, `any`, `polymorphic`)
  → `UnsupportedType`.
* Checks run on: return type, every param type, every instruction's
  `type_hint`.  Errors aggregate; the lowerer fails fast with
  `ValidationFailed(Vec<String>)` if any are present.

#### Tests added (22 total, was 7)

* Function signature lowering (4): void/no-params, i32 with 2 params,
  float types, u32+i32 → i32 mapping.
* ret_void / ret (4): emission, const-inlined, param-register,
  undefined-var error.
* const / mov (3): no LLVM line for `const`, mov chains, mov of a param.
* Validator (4): accept-supported, reject-op, reject-ret-type, reject-param-type.

#### Not yet in v0.2.0

* Arithmetic, comparisons, branches — LLVM03.
* `call` and `call_builtin print_i64` extern decl — LLVM04.
* `lang-aot --backend=llvm` wiring — LLVM04.

[plan]: ../../../specs/MULTILANG-BACKEND-PLAN.md

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
