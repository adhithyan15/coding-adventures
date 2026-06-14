# Changelog — iir-to-riscv

All notable changes to this crate are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.4.0] — 2026-06-03 — **DEPRECATED** (Phase 7, FINAL lane of historical-arch backend migration)

This crate is now deprecated.  Use `riscv-encoder` for byte
encoding and `riscv-backend` for CIR lowering via the
`jit_core::backend::Backend` trait.

### What changed

* `lower_iir_to_riscv` is marked `#[deprecated]` at the API
  level.  Existing call sites continue to compile (lang-aot has
  already migrated off it; downstream consumers can take their
  time).
* Tests get `#![allow(deprecated)]` so the deprecation warning
  doesn't break the build.
* The module-level docstring now opens with the deprecation
  notice and a pointer to the new crates.

### Why

The architectural correctness migration documented in
`code/specs/HISTORICAL-ARCH-BACKEND-MIGRATION.md` moves every
arch backend from the IIR (dynamic-typed) layer to the CIR
(monomorphised, typed) layer behind a single `Backend` trait.
RV32I was the original mistake from the A1+ cascade that started
this whole pattern — Phase 7 closes the loop and completes the
migration.

### Compatibility

No behavioural change.  `lower_iir_to_riscv` still produces the
same `Vec<u32>` it produced in v0.3.3.  The new
`riscv-backend::compile` produces byte-for-byte-identical output
for the ops it covers (`const_*` + `ret_*` + `ret_void`), and
returns `BackendError::UnsupportedOp` for the rest — which the
lang-aot e2e tests treat as expected gaps with their
`UnsupportedOp` fallback.

## [0.3.3] — 2026-06-02 (A1++.5.5.5 — call args + non-void returns)

### Added — full call ABI (up to 8 args, non-void returns)

Removes the v0.3.2 first-slice restrictions: `call dest, callee(arg1,
arg2, …)` now supports up to 8 arguments per the RV32I calling
convention (`a0..a7`) and non-void return values bound via `dest`.

### Two-phase move-through-temp (the swap-clobbering fix)

Naive sequential moves corrupt arguments when an arg's source register
coincides with another arg's *target* a-register.  The classic example:
calling `f(y, x)` from within a function where `x` lives in `a0` and
`y` lives in `a1`.  Naive `mv a0, a1; mv a1, a0` gets `f(y, y)` because
the first `mv` clobbers `a0` before the second `mv` reads it.

Fix: two-phase scheme.

* **Phase 1** copies each arg source into a fresh scratch temp from
  `TEMP_REGISTERS[next_temp .. next_temp + arg_count]`.  Sources read
  BEFORE any `a*` write.
* **Phase 2** copies each scratch temp into the corresponding `a_i`.
  Temps are still untouched because phase 1 wrote into disjoint slots.

The scratch slots are **transient** — not added to `state.env`, no
permanent reservation — but they still need to be available from the
pool.  `next_temp + arg_count > 7` yields `OutOfRegisters`.

For literal args (`Operand::Int`, `Operand::Bool`) we materialise the
constant directly into the scratch temp via the existing
`emit_const_i32` path (re-using lui+addi for wide consts) and the
canonical bool encoding.

### Non-void returns

After the patched `jal`, the callee's return value lives in `a0`.  When
`dest` is `Some`, we allocate a fresh temp via the existing
`alloc_temp` path and emit `addi dest_reg, a0, 0`.

### New error variant

* `UnsupportedCallShape` now also fires when `args.len() > 8`
  (RV32I `a0..a7` only).  Stack-based arg passing lands in A1++.6.

### Tests added (38 total, was 36)

* `call_with_one_const_arg_emits_arg_setup` — pins the exact two-phase
  move sequence (`addi t1, t0, 0; addi a0, t1, 0`) and the resolved
  `jal ra, -24` offset.
* `call_with_non_void_return_binds_dest_from_a0` — pins the
  post-jal `addi t0, a0, 0` binding.
* `call_too_many_args_is_rejected_as_unsupported_shape` (9-arg case).
* `call_with_too_many_scratch_temps_needed_is_rejected` (5 locals
  already allocated + 3-arg call → `OutOfRegisters`).

Pre-existing tests in section 16 (`cross_function_void_call_*`,
`leaf_function_*`, `undefined_callee_*`) continue to pass — they
exercise the 0-arg/void degenerate case.

The two restriction tests from v0.3.2
(`call_with_args_is_rejected_*` and `call_with_non_void_return_is_rejected_*`)
are removed because their preconditions no longer reject.

## [0.3.2] — 2026-06-02 (A1++.5.5 first slice — cross-function `call` (0-arg, void))

### Added — cross-function `call` with module-level resolution

First slice of A1++.5.5: lands `call` of user-defined functions with
the smallest meaningful restrictions (0 args + void return).  Args +
non-void returns land in A1++.5.5.5.  Stack spilling lands in A1++.6.

### What's emitted per `call`

A function that contains at least one `call` gets a **call-frame
prologue and epilogue**:

```text
caller:
    addi sp, sp, -16              ; prologue
    sw   ra, 12(sp)

    …caller body…
    jal  ra, +offset              ; call site — patched at module level
    …more caller body…

    lw   ra, 12(sp)               ; epilogue (before every ret/ret_void)
    addi sp, sp, 16
    jalr x0, x1, 0                ; ret
```

Leaf functions (no `call` in body) skip the prologue/epilogue entirely
— preserves the single-word `ret` shape from earlier slices.

### Module-level call-site resolution (pass 1 + pass 2)

`lower_iir_to_riscv` becomes two-pass:

1. **Pass 1**: walk every function, record its start byte in
   `function_starts: HashMap<String, usize>`; per-function lowering
   returns `(Vec<u32>, Vec<CallSite>)`; the module-level loop
   collects all call sites into a global list with their owning
   function's start byte snapshotted.
2. **Pass 2**: for each call site, compute
   `offset = callee_start - call_site_byte`, range-check ±1 MiB,
   re-emit via `encode_jal(ra, offset)`, write back into the words
   vector.

Inter-function branches (`jmp`/`jmp_if_*`) still resolve in pass 1
because they're function-local.

### Public error variants added

* `IIRRiscvError::UndefinedCallee` — `call` to a name that isn't a
  function in the module.
* `IIRRiscvError::CallOutOfRange` — target so far away it would
  overflow `jal`'s ±1 MiB range.
* `IIRRiscvError::UnsupportedCallShape` — first-slice restriction:
  current scope is 0 args + void return.

### Tests added (36 total, was 31)

* `cross_function_void_call_resolves_jal_offset` — pins the exact
  7-word two-function sequence (callee ret + caller prologue + jal -12
  + epilogue + ret), proving the module-level resolver produced the
  right PC-relative offset.
* `leaf_function_still_omits_prologue` — regression test for the
  single-word leaf shape.
* `call_with_args_is_rejected_as_unsupported_shape` (1-arg case).
* `call_with_non_void_return_is_rejected_as_unsupported_shape`.
* `undefined_callee_is_rejected_at_module_level` — error path.

### What is NOT in this PR (deferred to A1++.5.5.5 / A1++.6)

* **Call arguments** — A1++.5.5.5.  Two-phase mv-through-temp dance
  to avoid the swap-clobbering problem.
* **Non-void return values** — A1++.5.5.5.  Allocate a temp for the
  dest and `addi dest, a0, 0` after the `jal`.
* **Stack-spilling register allocator** — A1++.6.

## [0.3.1] — 2026-06-02 (A1++.5 control-flow slice — `label` / `jmp` / `jmp_if_*` with two-pass label resolution)

### Added — control flow within a single function

Lands the control-flow piece of the A1++ bundle.  Cross-function `call`
and stack spilling remain deferred to A1++.5.5 — splitting them keeps
each PR's review surface focused.

| IIR op | RV32I lowering |
|--------|----------------|
| `label "L"` | byte-offset marker, emits zero machine words |
| `jmp "L"` | `jal x0, +offset` (J-type, ±1 MiB range) |
| `jmp_if_true cond, "L"` | `bne cond, x0, +offset` (B-type, ±4 KiB range) |
| `jmp_if_false cond, "L"` | `beq cond, x0, +offset` (B-type, ±4 KiB range) |

### Implementation — two-pass label resolution

Single traversal of IIR, two visits of the words vector:

1. Each `label` records `labels[name] = out.len() * 4` (current byte
   offset).  Zero machine words emitted.
2. Each branch (`jmp` / `jmp_if_*`) pushes a placeholder zero word and
   records `(word_idx, target_label, BranchKind)` in
   `state.branches`.
3. After all instructions are lowered, `lower_function` walks
   `state.branches` and re-emits each placeholder via the right
   encoder with the resolved offset.

This keeps the per-instruction lowerer simple — no special "is this a
forward branch?" logic — and produces the same byte layout as a real
two-pass assembler.

### New error variants

* `IIRRiscvError::UndefinedLabel` — branch to a `label` that was never
  defined in the same function.
* `IIRRiscvError::BranchOutOfRange` — target so far away it would overflow
  the encoding (`±4096` bytes for `beq`/`bne`, `±1 MiB` for `jal`).

### Tests added (31 total, was 26)

* `jmp_around_a_dead_block_patches_jal_with_real_offset` — pinned the
  exact `jal x0, +8` encoding for a forward jump over a dead block.
* `jmp_if_true_emits_bne_with_resolved_offset`
* `jmp_if_false_emits_beq_with_resolved_offset`
* `backward_jmp_emits_negative_offset` — infinite loop case (`jal x0, +0`).
* `undefined_label_is_rejected` — error path.

### What is NOT in this PR (deferred to A1++.5.5)

* **Cross-function calls.**  `call` with `jal` + `ra` save/restore needs
  a real stack frame.
* **Stack-spilling register allocator.**  Locals beyond `t0..t6` still
  produce `OutOfRegisters`.
* **64-bit register-pair handling.**

## [0.3.0] — 2026-06-02 (A1++ first slice — wide consts + comparisons + `ecall print_i64`)

### Added — three more op families on RV32I

A1++ as originally scoped was a large bundle (comparisons, branches,
calls, stack spilling, wide consts, ecall).  This release lands the
three self-contained pieces:

1. **Wide constants via `lui + addi`.**  Any i32 literal now lowers
   cleanly.  Values that fit in i12 still use the single-`addi` form;
   anything wider gets the canonical `lui rd, upper20; addi rd, rd,
   lower12` sequence with the standard carry adjustment when low12 is
   negative.  Values outside i32 still produce `ImmediateOutOfRange`
   (64-bit register-pair handling lands in A1++.5).
2. **Comparisons** — `eq`/`ne`/`lt`/`le`/`gt`/`ge` (and their
   `cmp_`-prefixed aliases per gap G1) producing i32 `0`/`1` results
   in a register:
   * `lt`/`gt` → `slt` (or `sltu` for `u*` operands)
   * `le`/`ge` → `slt` + `xori 1`
   * `eq` → `xor` + `sltiu 1`
   * `ne` → `xor` + `sltu x0, t`
   All idioms reuse the dest register for the synth intermediate so no
   extra temp is needed — important until A1++.5 ships stack spilling.
3. **`call_builtin "print_i64"` via `ecall`** — completes the
   cross-backend print_i64 parity:

   | Backend            | Print sentinel |
   |--------------------|----------------|
   | iir-to-wasm        | `env.__print_i64` host import |
   | iir-to-jvm-class-file | `invokestatic env/BasicRuntime.println(J)V` |
   | iir-to-cil-bytecode | `call void env.BasicRuntime::PrintI64(int64)` |
   | iir-to-llvm        | `declare void @__print_i64(i64)` + extern call |
   | **iir-to-riscv (this)** | `ecall` with `a7 = ECALL_PRINT_I64_NUM = 1` |

   We pick syscall `1` (Linux `__NR_write` slot) because a future
   real-syscall pass can fold this into `write(2)` without a
   convention change.

### What is NOT in this PR (deferred to A1++.5)

* **Branches and label resolution.**  `label` / `jmp` / `jmp_if_*`
  need a two-pass over the function body to compute PC-relative
  offsets for `beq`/`bne`/`jal`.
* **Calls + stack spilling.**  Cross-function `jal` + `ra` save/restore
  needs a real stack frame, which forces an allocator rework.
* **64-bit register-pair handling.**  i64 literals + arithmetic on
  RV32 needs register pairs.

Splitting A1++ this way keeps the data-flow + ecall slice independently
reviewable — branches and stack spilling are orthogonal concerns that
benefit from landing as their own PR.

### Tests added (26 total, was 17)

* Wide const (2): `4096` → just `lui` (no `addi`); `4097` → `lui + addi`.
* Comparisons (5): signed `slt`, unsigned `sltu`, `eq` synthesis,
  `ne` synthesis, `cmp_`-prefix alias parity.
* `ecall print_i64` (2): exact 5-word sequence pinned; unknown builtin
  rejected.

Pre-existing tests rewritten:
* `const_out_of_imm12_range_is_rejected` → `const_out_of_i32_range_is_rejected`
  (the threshold moved up from i12 to i32 after lui+addi landed).
* `validate_rejects_unsupported_op` flipped to use `safepoint`
  (since `call_builtin` is now supported).

## [0.2.0] — 2026-06-02 (A1+ — const/mov/add/sub/ret + linear register allocator)

### Added — first real instruction lowering

| IIR op | RV32I lowering |
|--------|----------------|
| `const dest, Int(n)` (12-bit imm) | `addi rd, x0, n` |
| `add dest, a, b`  | R-type `add rd, rs1, rs2` |
| `sub dest, a, b`  | R-type `sub rd, rs1, rs2` |
| `mov dest, src`   | `addi rd, rs1, 0` (canonical move) |
| `ret <var>` (int) | `addi a0, var_reg, 0` (skipped when var already in a0) + `jalr x0, x1, 0` |
| `ret_void`        | `jalr x0, x1, 0` |

#### Register allocation (linear, no spilling)

* Function parameters land in `a0..a7` (`x10..x17`) per the RISC-V
  calling convention; the validator caps params at 8 (`TooManyParams`
  error otherwise).
* Locals get the next free temp from
  `[t0, t1, t2, t3, t4, t5, t6]` = `[x5, x6, x7, x28, x29, x30, x31]`.
  Pool exhaustion → `OutOfRegisters`.  A real stack-spilling allocator
  lands in A1++.

#### Type rules

Supported: `void`, `i8`, `u8`, `i16`, `u16`, `i32`, `u32`.  Everything is
treated as a 32-bit value at this scope (RV32I native width).
`i64`/`u64`/`f32`/`f64` are deferred — 64-bit needs register pairs and
floats need the F-extension.

#### Why skip the `mv` when `ret`'ing the first param

`identity(x: i32) -> i32 { ret x }` lowers to just one word — the
canonical `ret` (`0x0000_8067`).  We don't emit `mv a0, a0` because
`x` already lives in `a0` (it's the first param).  This is a small but
visible win for the trivial pass-through case.

#### Public error variants added

* `IIRRiscvError::UndefinedVariable` — var used before bound.
* `IIRRiscvError::TooManyParams` — `>8` function params.
* `IIRRiscvError::OutOfRegisters` — `>7` locals after params.
* `IIRRiscvError::ImmediateOutOfRange` — `const` value outside
  `[-2048, 2047]`.  `lui+addi` synthesis lands in A1++.

#### Tests added (17 total, was 6)

* Validator (5): empty, accept, reject-op, reject-type, reject-too-many-params.
* Empty module emits no words (contract for trivial input).
* `ret_void`-only function emits just the canonical `0x0000_8067`.
* `const` + `ret`: pinned the exact three-word sequence
  `addi t0, x0, 7; addi a0, t0, 0; jalr x0, x1, 0`.
* `const` of `-2048` (smallest valid imm12) lowers cleanly.
* `const` of `4096` (overflow) → `ImmediateOutOfRange`.
* `add` of two params: pinned the exact `add t0, a0, a1` word.
* `sub` of two params: pinned the exact `sub t0, a0, a1` word.
* `mov` produces the canonical `addi rd, rs1, 0`.
* Identity-of-first-param skips the redundant `mv` (one-word output).
* Register-pool exhaustion is rejected with `OutOfRegisters`.
* Config + error-display smoke.

#### Why scope branches & comparisons to A1++

The data-flow core (arith + ret with register allocation) is its own
review surface.  Branches add label resolution and PC-relative offset
computation — orthogonal concerns that benefit from landing as a
separate slice.

[plan]: ../../../specs/MULTILANG-ARCHITECTURE-BACKENDS.md

## [0.1.0] — 2026-06-01 (A1 — crate skeleton)

### Added — `ret`-only emission

First release.  Implements item A1 of the
[multi-language architecture backends plan][plan]: a crate skeleton
that lowers any IIR module to a single RV32I `ret` instruction
(`jalr x0, x1, 0`, encoded as `0x0000_8067`).

#### Public surface

```rust
pub struct IIRRiscvConfig { pub module_name: String }
impl IIRRiscvConfig {
    pub fn new(module_name: impl Into<String>) -> Self;
}

pub enum IIRRiscvError {
    ValidationFailed(Vec<String>),
    UnsupportedOp     { function: String, op: String },
    UnsupportedType   { function: String, type_hint: String },
    InvalidOperand    { function: String, detail: String },
}

pub fn validate_for_riscv(module: &IIRModule) -> Vec<String>;
pub fn lower_iir_to_riscv(
    module: &IIRModule,
    cfg: &IIRRiscvConfig,
) -> Result<Vec<u32>, IIRRiscvError>;
```

#### Why an architecture backend?

The wasm / JVM / CLR / BEAM / LLVM backends all target *software*
runtimes that own register allocation and instruction selection.
RISC-V is the first **architecture** backend: output is real hardware
ISA, decodeable by the in-tree `riscv-simulator` (RV32I + M-mode
traps), QEMU, or a physical SiFive / Espressif RISC-V chip.

Strategic priority: RISC-V is the most open of the architecture
candidates (royalty-free spec, broad simulator availability, growing
hardware footprint).  A2-A5 (Intel 8008, ARMv7, Intel 4004, GE-225)
follow the same shape once A1's lessons are baked in.

#### Why `Vec<u32>` output, not textual assembly?

* **Round-trips with `riscv-simulator`** — it consumes raw 32-bit words.
* **Deterministic test surface** — `assert!(words[0] == 0x0000_8067)`
  is unambiguous; assembly syntax has GNU vs LLVM divergence.
* **No textual-format coupling.**

A textual `.s` emitter can be added as a sibling later without
breaking callers.

#### What is NOT in v0.1.0

* **No instruction lowering.**  Function bodies in the input
  `IIRModule` are ignored.  v0.2.0 (A1+) lowers function
  entry/exit prologue/epilogue + arithmetic + cmp + control flow.
* **No `lang-aot --target=riscv32`.**  Deferred to v0.4.0 (A1+++).
* **No external linker integration.**  Output is raw words; downstream
  linkers / loaders are the caller's responsibility.

#### Tests added (6 total)

* `validate_returns_empty_for_empty_module`
* `lower_emits_exactly_one_word`
* `lower_emits_the_canonical_ret_word` (exact `0x0000_8067`)
* `default_config_has_nonempty_module_name`
* `new_sets_module_name`
* `errors_display_without_panic`

[plan]: ../../../specs/MULTILANG-ARCHITECTURE-BACKENDS.md
