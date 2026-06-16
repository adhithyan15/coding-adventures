# Changelog — iir-to-llvm

All notable changes to this crate are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.12.0] — 2026-06-16 — bitwise NOT (`not`) op

### Added — `not` (synthesised as `xor x, -1`)

LLVM has no `not` instruction, so the IIR `not` op was absent from this backend's
whitelist — the one backend of seven that lacked it. It now lowers to `xor x, -1`
(flip every bit). For a narrow unsigned width (`u4`/`u8`/`u16`/`u32`) it reuses the
E2 compute-wide+mask path — `xor i64 x, -1` then `and i64 …, <mask>` — so `~0u8` is
`255` (`-1 & 0xFF`), not the i64 all-ones. A full-width `i64`/`u64` `not` is a plain
`xor`. Added to `SUPPORTED_OPS`.

This **unblocks Nib N3-`~` and Oct O2-`~`** (their `compile_unary` lowers `~` to an
IIR `not`, which previously could not run on LLVM). **Verified on real `clang`**:
`not 0 : u8` returns exit `255`. New structural tests `not_u8_is_xor_minus1_then_masked`
and `not_i64_is_plain_xor_no_mask`; iir-to-llvm consumers (algol-iir-compiler, lang-aot)
green.

## [0.11.0] — 2026-06-15 — narrow unsigned arithmetic wraps mod-2ⁿ (LANG-FULL E2)

### Added — `u4`/`u8`/`u16`/`u32` results are masked back into their width

LANG-FULL **E2 — register width & wrap**, the LLVM column. A narrow unsigned
binary op (`add`/`sub`/`mul`/`div`/`mod` and `and`/`or`/`xor`) now computes at
`i64` and AND-masks the result into its declared width, so `200u8 + 100u8`
wraps to `44`:

```llvm
  %__nw1 = add i64 200, 100     ; compute wide (operands are i64 slots)
  %v     = and i64 %__nw1, 255  ; 300 & 0xFF = 44  ✓ wrapped to u8
```

**Why a value-mask, not a narrow-typed op.** Every IIR value rides a 64-bit
slot in this backend — arithmetic operands are `i64` SSA values (consts emit
`i64`; reassigned params become i64 stack slots). Typing the op at its narrow
LLVM width — `add i8 %a, %b` over two `i64` SSA values — is **invalid IR that
`clang` rejects** (the same shape as the AL5 `cmp`-truncation bug). So, exactly
like the VM, JIT, wasm, JVM, and CLR backends (and like this backend's own
byte-tape `store_byte` at the memory boundary), we compute wide and mask the
*value*:

| type_hint | mask         | example                |
|-----------|--------------|------------------------|
| `u4`      | `0xF`        | `15u4 + 1u4` → `0`     |
| `u8`      | `0xFF`       | `200u8 + 100u8` → `44` |
| `u16`     | `0xFFFF`     | `~0u16` → `65535`     |
| `u32`     | `0xFFFFFFFF` | wraps mod-2³²          |
| `u64`/`i*`/`f*` | —      | full word / signed / float: unchanged |

Signed narrow widths (`i8`/`i16`/`i32`) are left alone — E2 models unsigned
wrap; a signed wrap needs `trunc`+`sext`, out of scope.

Also adds `u4` (Nib's 4-bit nibble) to the supported type set — it has no
native LLVM width, so it rides an `i8` and the `& 0xF` mask enforces the range.

This corrects the earlier roadmap assumption that "LLVM already wraps natively
(u8→i8)": that was never executed (no frontend emitted narrow hints), and the
i64-slot value model means it does **not** hold. Verified by RUNNING the
emitted `.ll` through real `clang`: `200u8 + 100u8` returns exit `44`. New unit
tests: `e2_u8_add_computes_at_i64_then_masks`, `e2_u16_and_u4_masks_match_width`,
`e2_bitwise_u8_xor_masks`, `e2_wide_widths_emit_no_mask` (and the existing
`arith_div_unsigned_emits_udiv` updated to the i64-slot value model).

## [0.10.0] — 2026-06-13 — reassigned parameters become stack slots (LANG-FULL — LLVM first-class)

### Fixed — a reassigned function parameter is no longer silently dropped

`collect_slot_vars` promoted a variable to an `alloca` stack slot only when it was
the `dest` of **two or more** instructions. A parameter reassigned in the body —
e.g. `acc = acc + 6`, the shape of a loop accumulator — is the `dest` of only one
instruction, so it stayed a pure SSA value. Across a loop back-edge the
straight-line `const`/`mov` side-map is invalid, and the update was silently
dropped: the emitted IR computed `add %acc, 6` but never stored it back, so the
loop returned the unmodified incoming argument. (A `let` local works because its
declaration is a second `dest`; only parameters had this hole.)

A parameter's incoming binding **is** its first assignment, so:

- `collect_slot_vars` now seeds each i64-slot-compatible parameter with a count of
  1, so a single later reassignment crosses the `>= 2` promotion threshold. The new
  `param_slot_compatible` helper gates this to values that fit the i64 slot model
  (every integer width, `bool`, `any`, `symbol`, lisp `ref<Lispy…>`); a `float`/
  `double` parameter is **not** promoted (the i64 slot can't represent it — that is
  a separate concern under enabler E3, and is no worse than before).
- `lower_function` initialises each promoted parameter's slot from its incoming SSA
  argument at function entry (`store i64 %p, ptr %p.slot`), zero-extending a narrow
  `i1`/`i8`/`i16`/`i32` argument to the i64 slot width first.

Verified by RUNNING on real `clang`: a Nib program accumulating into a **parameter**
across a loop (`fn run(acc: u8) { for i in 0..7 { acc = acc + 6 } return acc }`)
now returns 42 — and is added to `lang-aot`'s `lang_matrix` battery across every
backend. New unit tests: `reassigned_parameter_is_promoted_to_a_stack_slot`,
`narrow_reassigned_parameter_is_zero_extended_into_its_slot`,
`non_reassigned_parameter_stays_pure_ssa`.

## [0.9.0] — 2026-06-12 (LLVM05 — byte-tape ops + Brainfuck I/O; LANG-MATRIX LM-L Brainfuck)

Adds the byte-tape memory ops and character I/O that Brainfuck needs, so the
LLVM column now covers Brainfuck — the last code-gen gap in that language's row.
Verified by RUNNING the Brainfuck cell `++++++++[>++++++++<-]>+.` on real `clang`
in `lang-aot/tests/lang_matrix.rs`: it prints `A`.

**New IIR opcodes** (added to `SUPPORTED_OPS` and `lower_instr`):

- `alloc_bytes dest <- size` → `%dest = call ptr @calloc(i64 size, i64 1)` — a
  zero-filled tape (Brainfuck cells start at 0). Declared once as
  `declare ptr @calloc(i64, i64)`. The tape base is a single-assignment value,
  so it is never a promoted stack slot.
- `load_byte dest <- base, idx` → `getelementptr i8` + `load i8` + `zext i8…i64`.
  The 8-bit cell becomes the uniform `i64` register width.
- `store_byte base, idx, val` (no dest) → `getelementptr i8` + `trunc i64…i8` +
  `store i8`. The `trunc` is what makes Brainfuck's 8-bit cell wrap-around fall
  out even though the surrounding arithmetic runs at `i64` width — "byte width
  only at the tape boundary."

**New `call_builtin`s** (added to `SUPPORTED_BUILTINS`):

- `putchar` (Brainfuck `.`) → `trunc i64…i32` + `call i32 @putchar(i32)`. Maps
  to libc directly (no host-runtime shim like `print_i64`'s `@__print_i64`).
- `getchar` (Brainfuck `,`) → `call i32 @getchar()` + `sext i32…i64`. EOF (`-1`)
  lands as `0xFF` after a subsequent `store_byte` truncation — the conventional
  Brainfuck behaviour. Declared as `declare i32 @putchar(i32)` / `@getchar()`.

**Bug fix — slot-dest SSA rename.** A variable assigned in 2+ instructions is
promoted to an `alloca i64` stack slot. Previously a value-producing op wrote
`%<var> = …` using the variable's name verbatim, so a slot variable that is the
dest of a real op (rather than only `const`/`mov`) emitted `%v = …` twice — which
LLVM rejects (*"multiple definition of local value named 'v'"*). Brainfuck's
`ptr`/`v` (incremented every command) are the first such case. `lower_instr_with_slots`
now lowers a clone of the instruction with a fresh SSA dest name and stores the
result into the original variable's slot. `const`/`mov` slot-dests (which emit no
`%dest =` line) are unaffected.

Six new tests in `tests/test_backend.rs` cover each emit case and the rename
regression.

## [0.8.0] — 2026-06-10 (McCarthy W13b — lisp lambda (F7) — LLVM COMPLETE)

Registers the universal exit-coercion runtime helper so the LLVM backend can
declare + call it: `LISPY_BUILTINS` gains `("lispy_to_exit_code",
"__twig_lispy_to_exit_code", 1)`. A lambda result is a `call` typed `any` whose
runtime tag is unknown at compile time; the shared `lower_lisp_repr` now emits
`lispy_to_exit_code` for it, and this entry lets the backend lower that to a
`call i64 @__twig_lispy_to_exit_code(i64)`. With it, **LLVM is McCarthy-complete
(F1–F7)** — verified by RUNNING in `lang-aot` (`lang-aot/tests/llvm_lambda.rs`).

## [0.7.0] — 2026-06-10 (McCarthy W13a — lisp symbols (F6))

`llvm_type_for("symbol")` now maps to `i64` — an interned McCarthy symbol is a
tagged 64-bit immediate (from `iir_builtin_lowering::intern_symbols`), so it flows
as a tagged word like `any`/`ref<Lispy…>`. With this, `(QUOTE A)`, symbol `EQ`, and
symbols inside `COND` all validate and lower. Verified by RUNNING in `lang-aot`
(clang + `lispy_runtime.c`): `(EQ (QUOTE A) (QUOTE A))`→1, `(EQ (QUOTE A) (QUOTE B))`→0.

## [0.6.0] — 2026-06-10 (McCarthy W12b-3 — `COND` via alloca SSA-merge — LLVM core F1–F5)

Lowers McCarthy `COND` (a cross-block value merge) and completes the LLVM core
(F1–F5).

- **Stack-slot promotion (`collect_slot_vars` + `lower_instr_with_slots`):** a
  variable assigned in 2+ instructions (a `COND` result written per clause) gets an
  entry `alloca`; each assignment becomes a `store i64 …, ptr %v.slot`, each read a
  `load i64, ptr %v.slot`. Single-assignment vars keep the `const`/`mov` side-map
  (fast path, no slot). This is the naive-frontend / `opt -mem2reg` pattern, so no
  PHI-predecessor analysis is needed.
- **Block-terminator hygiene (`FnState::block_open`):** a `label` reached while the
  current block is still open (its body was all tracked-not-emitted `const`/`mov`)
  emits an explicit fallthrough `br` first — no two labels back-to-back.
- **`jmp_if` void-cond:** when the `jmp_if_*` carries no operand type (`void`) — its
  condition is the `i64` 0/1 from `lispy_truthy` — it lowers to `icmp ne i64 %c, 0`
  instead of an invalid `trunc void`.
- Verified by RUNNING in `lang-aot` (clang + `lispy_runtime.c`):
  `(COND ((ATOM 7) 11) ((ATOM 8) 22))`→11, second-clause→22, nested `COND`→44.

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
