# Changelog — iir-to-llvm

All notable changes to this crate are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.5.0] — 2026-06-10 (McCarthy W12b-1 — tagged-word lisp `cons`/`car`/`cdr` → `__twig_lispy_*`)

Lowers the **tagged-word lisp** builtins to `call`s into the shared C runtime
(`twig-aot/runtime/lispy_runtime.c`) — the SAME runtime the native AOT path links,
so any lisp-family frontend inherits it.

- `LISPY_BUILTINS` table maps the `lispy_*` IIR names (from
  `iir_builtin_lowering::lower_heap_builtins_runtime`/`lower_lisp_repr`) to the
  runtime's `__twig_lispy_*` symbols: `cons`/`car`/`cdr`/`pair_p`/`equal`/`not`/
  `truthy`/`box_int`/`unbox_int`/`nil`. Each is `i64 (i64 × arity)` — a lisp value
  is a tagged 64-bit word.
- `call_builtin "lispy_*"` lowers to `%d = call i64 @__twig_lispy_*(i64 …)`; one
  `declare` per used builtin is emitted in the module header (first-seen order, deduped).
- `llvm_type_for`: `any` and a lisp reference (`ref<Lispy…>`) map to `i64` (the
  tagged word). A NON-lisp `ref<Foo>` stays `UnsupportedType`.
- **Verified by RUNNING** end-to-end in `lang-aot` (clang links `lispy_runtime.c`):
  `(CAR (CONS 7 9))`→7, `(CDR …)`→9, nested→2. Predicates (pair?/equal?/not, COND)
  are emitted but their tagged-boolean result handling is W12b-2.

## [0.4.0] — 2026-06-01 (LLVM04 — `call` + `call_builtin print_i64` + `lang-aot --emit=llvm-ir`)

### Added — user-defined `call`

Per-arg LLVM types come from a pre-built callee-signature side map:
`lower_iir_to_llvm` walks every function in the module once at the
start and stashes a `name → FnSig { param_types, return_type }` map.
Each `call` site looks up its callee in that map, validates the arg
count against the signature, and emits:

```llvm
%dest = call <ret_ty> @<callee>(<arg_ty> <arg>, ...)   ; non-void
        call void     @<callee>(<arg_ty> <arg>, ...)   ; void
```

Why pre-scan rather than synthesize from each call site's `type_hint`:
IIR's `call` carries only the **return** type in `type_hint`; param
types live on the *callee*.  Without pre-scan we'd need a second pass
or some hacky heuristic.

#### Validation

* `call`'s callee must exist in the module (else `UndefinedVariable`).
* Arg count must match the callee's param count (else `InvalidOperand`
  with an `arg-count` discriminator string).

### Added — `call_builtin "print_i64"` → extern `@__print_i64`

Completes the print_i64 trio across the four backend targets:

| Backend            | print_i64 lowering                                    |
|--------------------|-------------------------------------------------------|
| iir-to-wasm        | `env.__print_i64` host import                         |
| iir-to-jvm-class-file | `invokestatic env/BasicRuntime.println(J)V`         |
| iir-to-cil-bytecode | `call void env.BasicRuntime::PrintI64(int64)`        |
| **iir-to-llvm (this)** | `declare void @__print_i64(i64)` + `call void @__print_i64(i64 …)` |

The extern `declare` is emitted exactly **once** per module, at the
top, after the header.  `lower_iir_to_llvm` pre-scans the whole module
to decide whether to emit it (so the unused-builtin case doesn't pay
the extern cost).

#### Whitelist gate

* `SUPPORTED_BUILTINS = ["print_i64"]`.  Any other builtin name fails
  with `UnsupportedOp` — defence in depth even though `call_builtin`
  is in the validator whitelist.

### Tests added (45 total, was 37)

* `call` (4): non-void user fn typed call, void-return omits LHS,
  unknown callee → UndefinedVariable, arg-count mismatch error.
* `call_builtin` (4): print_i64 emits extern + call, declare emitted
  exactly once per module, declare omitted when print_i64 unused,
  unknown builtin name → UnsupportedOp.

[plan]: ../../../specs/MULTILANG-BACKEND-PLAN.md

## [0.3.0] — 2026-06-01 (LLVM03 — typed arithmetic + comparison + branches)

### Added — three op families

Implements item LLVM03 of the [multi-language backend plan][plan].  After
this release, the LLVM backend covers the IIR subset that BASIC, Twig,
Nib, and Oct front-ends actually emit for straight-line and branching
code (everything except `call`, `call_builtin`, and heap/memory ops —
those land in LLVM04).

#### Arithmetic — five op-families × signedness / float

| IIR op | Signed int | Unsigned int | Float |
|--------|------------|--------------|-------|
| `add`  | `add`      | `add`        | `fadd` |
| `sub`  | `sub`      | `sub`        | `fsub` |
| `mul`  | `mul`      | `mul`        | `fmul` |
| `div`  | `sdiv`     | `udiv`       | `fdiv` |
| `rem`  | `srem`     | `urem`       | `frem` |

Signedness comes from the IIR type_hint prefix (`i*` = signed, `u*` =
unsigned).  `add`/`sub`/`mul` are signedness-agnostic at the bit level
so they share opcodes.

#### Comparison — `icmp`/`fcmp` + automatic zext

| IIR op | i32 | u32 | f64 |
|--------|-----|-----|-----|
| `eq`   | `eq` | `eq` | `oeq` |
| `ne`   | `ne` | `ne` | `one` |
| `lt`   | `slt` | `ult` | `olt` |
| `le`   | `sle` | `ule` | `ole` |
| `gt`   | `sgt` | `ugt` | `ogt` |
| `ge`   | `sge` | `uge` | `oge` |

Both naked (`eq`) and `cmp_`-prefixed (`cmp_eq`) opcodes are accepted —
the latter were introduced in gap G1 for the wasm backend and we accept
them here for cross-backend consistency.

Float predicates use `o<pred>` (ordered) — NaN compares false.  This
matches the most common language-level expectation.

LLVM `icmp`/`fcmp` always return `i1`.  When the IIR type_hint is wider
than `i1`, we automatically emit a `zext` to widen.  The original `i1`
form is preserved in a sidecar `env_i1` map so a downstream
`jmp_if_true` / `jmp_if_false` can consume it directly without a
redundant `trunc` round-trip.

#### Control flow — three opcodes + auto-fallthrough

* `label "name"`           → `name:`
* `jmp "name"`             → `br label %name`
* `jmp_if_true cond, name` → `br i1 <cond_i1>, label %name, label %__fallN`
* `jmp_if_false cond, name`→ `br i1 <cond_i1>, label %__fallN, label %name`

Conditional branches require both arms in LLVM IR; IIR's `jmp_if_*` only
names one target.  We synthesize a fresh `__fallN` block immediately
after the branch, so the next IIR instruction lands in a valid basic
block.  No structural changes upstream are required.

#### Type system additions

* `llvm_type_for` now accepts `i1` and `bool` (both → LLVM `i1`).
  Enables comparison results to be requested at i1 width directly, with
  no zext.

#### Tests added (37 total, was 22)

* Arithmetic (6): add-i32, fadd-double, sdiv, udiv, srem/urem same
  module, const-operand inlining.
* Comparison (5): icmp eq i32 + zext, ult for u32, fcmp olt for f64,
  `cmp_`-prefix alias, no-zext when type_hint=i1.
* Control flow (4): label block header, unconditional br, jmp_if_true
  with fallthrough block, jmp_if_false swaps arms.

[plan]: ../../../specs/MULTILANG-BACKEND-PLAN.md

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
