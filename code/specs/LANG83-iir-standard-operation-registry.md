# LANG83 — IIR standard operation registry and backend lowering contract

**Status:** Draft, 2026-10-19. Contract slice (PR1a) and first op family on
all nine backends (PR1b, see "PR1b — what landed") under LANG78.

## Why this exists

The dynamic-language frontends of LANG78 (JavaScript, Python, Ruby, Perl) need
far more than the scalar vocabulary the older frontends were built on: floor
division, overflow-checked arithmetic, output without a trailing newline,
defined runtime traps, string conversions, and later tagged dynamic values and
collections. Two directions were considered for supplying that vocabulary:

1. A **portable runtime** — a shared library of IIR helper functions (or IIR
   helper functions generated per frontend) that every backend would compile
   like user code.
2. **First-class operations** — every operation is an IIR instruction or a
   standard `call_builtin` name, and **every backend lowers it itself** into
   instructions it supports natively.

This spec adopts (2) and rejects (1). The direction, verbatim from the owner:

> "We should be able to support a rich array of operations in the frontend.
> And the backend should be able to convert these operations into instructions
> they support."
>
> "Do not do things like portable runtime."

So there is **no** shared IR-helper library and **no** per-frontend helper
generation. A frontend emits an operation from the registry; a backend either
lowers it or refuses it at compile time with a typed error. A refusal is
always a compile error, never a miscompile and never a silent fallback.

The consequence that makes this tractable is a *contract*: one registry that
names every standard operation, and one declaration per backend saying what it
does with each name. Adding an operation to the registry fails CI in every
backend crate until that backend lowers it or explicitly refuses it.

## Vocabulary

### Op versus Builtin

Every registry entry has a **kind**:

| Kind | IIR shape | Used for | Why |
| --- | --- | --- | --- |
| **Op** | `IIRInstr { op: "<name>", .. }` | arithmetic, comparison, conversion, strings, `trap` | Inline in every backend; visible to analyses (`has_side_effects`, `may_trap`, DCE) without decoding a call. |
| **Builtin** | `call_builtin "<name>", args…` | output (`print_i64`, `putchar`, `write_i64`) | Every backend already owns a builtin table and an output sink, and a builtin is the natural seam to a host stream. |

A registry entry is identified by its *name*. For an Op the name is
`instr.op`; for a Builtin it is the `Operand::Var` in `srcs[0]` of a
`call_builtin`. `interpreter_ir::op_registry::registry_name` performs that
extraction once, so no backend re-derives it.

### Naming rule: type first, never a dispatch prefix

New operation names are **type-first** (`i64_div_floor`, `f64_sin`), never
verb-first (`div_floor`). This is a correctness rule, not a style rule.

The native pipeline reaches the machine backends by name. `jit_core::specialise`
type-suffixes only the classic binary/unary IIR ops (`add` → `add_i64`), and
passes every other name through unchanged. Then `x86_64-backend`,
`aarch64-backend` and `jit-core`'s `GenericCirJit` dispatch on **prefixes**:
`op.starts_with("div_")` selects the hardware divide, `strip_prefix("add_")`
selects the adder, and so on for `add_ sub_ mul_ div_ mod_ neg_ not_ and_ or_
xor_ shl_ shr_ cmp_ ret_ mov_ const_ dyn_`.

A new op named `div_floor` therefore never reaches a "this op is unknown"
error. The x86_64 backend sees `div_floor`, matches `div_`, reads the suffix as
a type, and emits truncating `idiv` — a silent miscompile of every negative
quotient. A registry unit test (`op_registry::tests::names_avoid_dispatch_prefixes`)
rejects any registry name that starts with one of those prefixes, with an
explicit grandfather list for the six `cmp_*` comparison ops, which *are* the
family that prefix dispatches.

### Families

`Exists` = lowered by the standard backends today and registered in PR1a.
`PR1b` = registered in PR1a and lowered by **every** backend since PR1b
(`introduced_by: "LANG83 PR1b"`; the shared `NOT_YET_LOWERED_PR1B` reason is
no longer used by any backend). `Later` = planned; enters the
registry before any frontend emits it. `Unregistered` = exists in IIR today but
is not yet in the registry (it keeps working exactly as before; the registry
does not gate it).

| Family | Name | Kind | Status | Semantics |
| --- | --- | --- | --- | --- |
| Integer arithmetic | `add` `sub` `mul` | Op | Exists | Wrapping two's-complement on integers, IEEE on `f64`. |
| | `div` `mod` | Op | Exists | Truncating. Zero divisor and `MIN / -1` are **backend-dependent** today (see "Legacy edge behaviour"). |
| | `neg` | Op | Exists | Wrapping negation. |
| | `i64_div_floor` `i64_mod_floor` | Op | PR1b | Floor division / modulo (Python `//` `%`, Ruby `/` `%`). `b == 0` → `Trap.DivZero`; `MIN / -1` → `Trap.Overflow` for div, `0` for mod. |
| | `i64_div_trunc` `i64_mod_trunc` | Op | Later | Truncating with *defined* traps (the legacy `div`/`mod` with the edges fixed). |
| | `i64_add_ovf` `i64_sub_ovf` `i64_mul_ovf` `i64_neg_ovf` | Op | Later | Overflow → `Trap.Overflow` (bignum promotion is a frontend decision made by catching it). |
| | `i64_pow` | Op | Later | Integer power (a `pow` builtin exists on four backends only and is unregistered). |
| | `and` `or` `xor` `not` `shl` `shr` | Op | Unregistered | Bitwise; LLVM lacks `shl`/`shr` and the generic JIT lacks all six. Registered in a follow-up with honest per-backend declarations. |
| Comparison | `cmp_eq` `cmp_ne` `cmp_lt` `cmp_le` `cmp_gt` `cmp_ge` | Op | Exists | 0/1 (or `bool`) result. |
| Float | `f64_sqrt` `f64_sin` `f64_cos` `f64_ln` `f64_exp` `f64_atan` `f64_tan` | Op | Exists | IEEE / libm. |
| | `f64_pow` | Op | Exists | libm `pow`. |
| | `f64_div_floor` `f64_mod_floor` `f64_is_nan` | Op | Later | |
| Conversions | `int_to_real` | Op | Exists | Exact `i64 → f64`. |
| | `real_to_int_trunc` `real_to_int_floor` | Op | Exists | Non-finite or out-of-range → `Trap.ConvRange`. |
| | `i64_to_str`, `f64_to_str_py` `_js` `_rb` `_pl` | Op | Later | One op per *layout*, not a flags operand; every backend uses one internal shortest-digits core, with Rust `format!("{:e}")` digits as the oracle. |
| Output | `print_i64` | Builtin | Exists | Decimal value **plus `\n`**. Kept: BASIC, Oct, ALGOL and Ruby `puts` depend on it. |
| | `putchar` | Builtin | Exists | Low byte of the argument. |
| | `print_str` | Op | Exists | String bytes, no newline. |
| | `write_i64` | Builtin | PR1b | Decimal value, **no** newline (Perl `print`, Python `end=""`). |
| | `write_f64_py` `_js` `_rb` | Builtin | Later | |
| Strings | `str_const` `str_len` `str_index` `str_slice` `str_concat` `str_eq` `str_cmp` | Op | Exists | LANG-FULL E4 byte strings; `str_index`/`str_slice` out of range → `Trap.Bounds`. |
| Errors | `trap` | Op | PR1b | Terminator; no dest; `srcs = [Str(kind)]` with `kind` from the closed table below. |
| | `trap_msg` | Op | Later | `trap` plus a `str` message. |
| | `throw` `catch` `landingpad` | Op | Unregistered | AOT00 T2; defined, lowered by no backend. Once exceptions land, `trap k` is sugar for an uncaught `throw k`. |
| Dynamic / collections / closures | `box`/`unbox`, `dyn_*`, `alloc_array`…, LANG34 closures | — | Unregistered | Each new tagged-value, hash or list op enters the registry before any frontend uses it. |

### Legacy edge behaviour (why new ops exist instead of fixing `div`)

The legacy `div`/`mod` disagree across backends on their edges, and programs
already depend on some of those behaviours:

| Backend | `x / 0` | `MIN / -1` |
| --- | --- | --- |
| WASM | `i64.div_s` traps | traps |
| JVM | `ArithmeticException` | `MIN` |
| CLR | `DivideByZeroException` | `OverflowException` |
| BEAM | `badarith` | bignum `2^63` |
| vm-core | `VMError::DivisionByZero` | wraps to `MIN` (`mod`: `0`); a Rust panic before PR1b |
| GenericCirJit | `set_error` | `wrapping_div` |

Rather than silently change those programs, LANG83 adds new ops whose edges are
*defined* and identical everywhere. `i64_div_trunc` (Later) is the defined twin
of `div`.

### Trap semantics

A trap terminates the program. It is a **terminator**: control never falls
through a `trap`, exactly like `ret`. Before the process ends, every backend
must **flush all output written so far** in program order (see "Ordered
output"), then report the trap kind on stderr, then exit non-zero. A trap is
not catchable in PR1a/PR1b; when AOT00 T2 exceptions land, an uncaught `throw`
of the same kind must behave identically, so `trap` becomes sugar for it.

`interpreter_ir::exception_kind` holds the **closed** kind table. A `trap`
whose kind is not in the table is malformed IR.

How each column reports a trap (PR1b). Every one flushes earlier output first:

| Column | Trap sequence | Kind observable as |
| --- | --- | --- |
| native x86-64 / AArch64 | `mov edi/w0, id; call/bl __twig_trap; ud2/udf` — runtime does `fflush(stdout)`, `trap: <kind>` to stderr, `abort()` | stderr `trap: <kind>`, signal exit |
| LLVM | `call @__iir_trap(msg, len); unreachable` — module-local `linkonce_odr`: `fflush(NULL)`, `write(2, …)`, `abort()` | stderr `trap: <kind>` |
| WASM | `i32.const id; call $env.__trap; unreachable` | the host's `__trap` argument; the module traps even if the host returns |
| JVM | `System.out.flush(); throw new RuntimeException("<kind>")` | stderr `java.lang.RuntimeException: <kind>`, exit 1 |
| CLR | `Console.Out.Flush(); throw new System.Exception("<kind>")` | stderr `Unhandled exception. System.Exception: <kind>` |
| BEAM | `erlang:error('<kind>')` — the reason is the kind **atom** | the uncaught error's reason |
| VM | `VMError::Trap { kind }` | the error value |
| JIT | `TRAP <id>` bytecode; error slot `trap: <kind>` | `GenericCirJit::error` |

**Divergence from the PR1b plan:** the plan proposed
`erlang:error({iir_trap, Kind})` on BEAM. `iir-to-beam` has no
tuple-construction opcodes, and a registered kind atom (`'Trap.DivZero'`) is
already unambiguous, so the reason is the bare atom. A harness that needs to
tell a LANG83 trap from another error checks the atom against the closed table.

| Kind | Stable id | Raised by |
| --- | --- | --- |
| `Trap.Bounds` | 1 | `array_get`/`array_set`/`str_index`/`str_slice` out of range |
| `Trap.Null` | 2 | `unbox` of null |
| `Trap.DivZero` | 3 | integer division by zero |
| `Trap.ConvRange` | 4 | `real_to_int_*` of a non-finite / out-of-range value |
| `Trap.Overflow` | 5 | `i64_div_floor` `MIN / -1`; later the `*_ovf` family |
| `Trap.User` | 6 | a frontend's explicit abort (`die`, `raise` without a class, …) |

The ids are what native backends pass to the runtime (`mov edi, id; call
__twig_trap`); they are append-only and never renumbered. Language-specific
kinds (for example `Ruby.ZeroDivisionError`) are added to
`LANGUAGE_TRAP_KINDS` in the same file, also with append-only ids starting at
1000, by the PR that first emits them. Nothing else may be emitted as a trap
kind.

### Analysis predicates

`interpreter_ir::opcodes` gains:

- `may_trap(op)` — true for every op whose registry entry lists a trap kind,
  plus the pre-registry trap sites (`array_get`, `array_set`, `unbox`).
- `trap_kinds(op)` — the kinds an op may raise (from the registry).
- `is_terminator(op)` — `jmp`, `ret`, `ret_void`, `throw`, `trap`.
- `has_side_effects` now includes `trap`, `i64_div_floor` and `i64_mod_floor`,
  so a dead-code pass must keep them even when their result is unused (a
  removed trapping divide would change observable behaviour).
  **Divergence kept deliberately in PR1a:** the legacy `div`/`mod` are *not*
  added to `has_side_effects`, because the existing optimisers rely on them
  being pure and their trap behaviour is backend-dependent anyway.
- `is_value_producing` includes `i64_div_floor` and `i64_mod_floor`.
- `is_known_op` accepts every registry entry of kind Op (this also makes the
  long-standing `f64_*` ops "known" to the module validator; they were already
  emitted by ALGOL and lowered everywhere).

## Backend contract

### 1. Registry

`interpreter-ir/src/op_registry.rs`:

```rust
pub enum OpKind { Op, Builtin }
pub enum OpSupport { Lowered, Refused(&'static str) }
pub struct OpSpec {
    pub name: &'static str,
    pub kind: OpKind,
    pub family: OpFamily,
    pub arity: usize,                         // value operands (Builtin: excluding the name)
    pub dest: bool,
    pub operand_types: &'static [&'static str],
    pub may_trap: &'static [&'static str],    // kinds from exception_kind
    pub terminator: bool,
    pub introduced_by: &'static str,          // the spec slice that defined it
}
pub const STANDARD_OPS: &[OpSpec];
```

PR1a registers every `Exists` and `PR1b` row of the vocabulary table. The
registry's unit tests enforce: unique names; the type-first name rule; Builtin
entries never collide with an IIR mnemonic; every `may_trap` kind is in the
closed kind table; terminators have no dest.

### 2. Per-backend `op_support`

Each backend crate exports

```rust
pub const OP_SUPPORT: &[(&str, OpSupport)];
pub fn op_support(name: &str) -> OpSupport;
```

| Crate | Column | What "Lowered" means |
| --- | --- | --- |
| `x86_64-backend` | native x86-64 | The native AOT pipeline (`twig-aot`'s IIR rewriting + this backend's CIR lowering) compiles it. |
| `aarch64-backend` | native AArch64 | Same, for AArch64. |
| `iir-to-llvm` | LLVM | `lower_iir_to_llvm` emits it. |
| `iir-to-wasm` | WASM | `lower_iir_to_wasm` emits it. |
| `iir-to-jvm-class-file` | JVM | `lower_iir_to_jvm` emits it. |
| `iir-to-cil-bytecode` | CLR | The textual `emit_il` path (real CoreCLR, the matrix's CLR column) emits it. The binary `lower_iir_to_cil` path targets the 32-bit simulator and is narrower. |
| `iir-to-beam` | BEAM | `lower_iir_to_beam` emits it. |
| `vm-core` | VM | The dispatch table has a handler, or the default `BuiltinRegistry` registers the builtin. **Derived**, not hand-listed. |
| `jit-core` | JIT | `GenericCirJit` compiles it to JIT bytecode. An op that only runs because `JITCore` fell back to the vm-core interpreter tier is declared `Refused`, so interpreter fallback cannot masquerade as JIT support. |

`op_support` is **name-granular**. Operand-type refusals (for example the
generic JIT refusing `add` on `f64`, or BEAM refusing a `ref<…>` it has no
layout for) remain the job of each backend's validator, which already reports
them as typed errors.

Each crate has a unit test that runs `op_registry::check_op_support_table`
over its table: every `STANDARD_OPS` entry must be declared exactly once and
no undeclared name may appear. Adding a registry op therefore fails CI in all
nine crates until each one declares it.

**Derivation, not mirroring.** A hand-maintained list that mirrors code drifts
(see the lesson *"A hand-maintained list that mirrors what code does will
drift"*). Where a backend already owns an authoritative table, the Builtin rows
of `OP_SUPPORT` are checked against it by a unit test (native `V1_BUILTINS`,
LLVM `SUPPORTED_BUILTINS`, WASM/JVM/CIL/BEAM `CALL_BUILTIN_SUPPORTED_NAMES`),
and vm-core derives its answer from the dispatch table outright. Op rows are
checked behaviourally by `lang-aot/tests/op_conformance.rs`, which compiles a
case for every Lowered op through every code-generation backend. Replacing the
older tables with derivations from `OP_SUPPORT` (rather than checking agreement)
is incremental work for later PRs.

### 3. Compile-time refusal in `lang-aot`

`lang_aot::Backend` names the nine columns (`X86_64`, `Aarch64`, `Llvm`,
`Wasm`, `Jvm`, `Clr`, `Beam`, `Vm`, `Jit`) and maps each to its crate's
`op_support`. `lang_aot::check_module_for(backend, &IIRModule)` walks every
instruction, resolves its registry name, and collects every `Refused` entry
into `LangAotError::UnsupportedOps(UnsupportedOps { backend, refusals })`,
where each refusal names the function, the op and the backend's reason.
Instructions that are not registry entries are not gated (their existing
validators still apply).

Every code-generation entry point calls it **first**, before any lowering
pass: the IIR-level entry points `compile_iir_to_llvm`, `compile_iir_to_wasm`,
`compile_iir_to_jvm_class`, `compile_iir_to_cil_artifact`,
`compile_iir_to_cil_text`, `compile_iir_to_beam` and
`compile_iir_to_host_executable`, through which every `compile_source_to_*` /
`compile_file_to_*_executable` path now routes. The VM and JIT have no
lang-aot compile path; their harnesses call `check_module_for` directly.

### 4. Conformance suite

`lang-aot/tests/op_conformance.rs` holds an IIR-built case corpus (one or more
cases per registry entry) and three gates:

- `every_registry_op_has_cases` — no registry entry without a case.
- `every_lowered_op_runs_on_every_backend` — each case of a `Lowered` op
  compiles on every code-generation backend that declares it, and the output
  cases execute on every available column. Skip policy as
  `proven_columns_do_not_silently_skip`: WASM, VM and JIT are in-process and
  never skip; a toolchain column skips only when its tool is absent.
- `refused_ops_are_rejected_at_compile_time` — a case of a `Refused` op is
  rejected by `check_module_for` *and* by the backend's own compile entry point
  with `LangAotError::UnsupportedOps`, never compiled.

Expected outcomes are `Stdout`, `Exit` and `StdoutThenTrap(stdout, kind)`, and
the conformance runner's `Trapped` result carries the stdout written before the
trap, so "print, then trap" is checkable. (The older `lang_matrix` keeps its
data-less `Trapped` until the first language row that emits a PR1b op —
LANG83 PR2 — ports it.)

### 5. Ordered output stream

All output of a program — `print_i64`, `write_i64`, `putchar`, `print_str` —
is **one ordered byte stream**. A backend or harness may not keep a separate
buffer per builtin and pick one. PR1a fixes the three places that did:

- WASM harness (`lang_matrix::PrintHost`): `env.__print_i64` appended to an
  integer list while `putchar`/`__print_str` appended to bytes, and the bytes
  *won* whenever any existed, so a program mixing them lost every integer. Now
  `__print_i64` writes its decimal text and `\n` into the same byte buffer.
- VM/JIT harness runners: same either/or, same fix; and `vm-core`'s default
  `BuiltinRegistry` now provides `print_i64` and `putchar` (writing to the
  process stdout through the same `std::io::stdout()` handle as `print_str`),
  so an embedder no longer has to register them for a program to run.
- BEAM: `print_i64` used `erlang:display/1`, which writes straight from the
  emulator and bypasses the `io` server that `putchar`/`print_str` use. It now
  calls `io:format("~w~n", [X])`, byte-identical output for integers, through
  the same `io` server.

Native (`printf`/`fputc` on one `FILE*`), LLVM (`printf`/`putchar`), JVM
(`System.out`) and CLR (`Console.Out`) were already ordered.

### 6. Flush before trap

Every trap path flushes buffered output before terminating (PR1b): native
`__twig_trap` calls `fflush(stdout)`; LLVM emits `fflush(NULL)` before
`abort`; JVM/CLR flush their standard output object before throwing; BEAM's
`io` server is synchronous; WASM/VM/JIT harnesses already hold output in
memory. Test cases print, then trap, and assert both the stdout prefix and
the kind (`Expect::StdoutThenTrap`). One of them writes with `write_i64` and
`putchar`, which do not flush per call natively. The gate
`flush_before_trap_is_proven_on_every_executable_column` requires the proof on
WASM, VM and JIT always, and on LLVM, native and JVM whenever their toolchain
is present. Removing the `fflush` from `__twig_trap` makes the native proof
fail. This was checked by mutation.

## PR plan

- **PR1a — contract (this slice).** This spec; `op_registry.rs`; the
  `opcodes.rs` predicates; `Trap.Overflow`, `Trap.User`, stable kind ids and the
  language-kind table; `op_support` in all nine backend crates; vm-core default
  output builtins; `lang_aot::Backend`, `check_module_for` and the IIR-level
  entry points; `op_conformance.rs`; the ordered-output fixes with a mixed
  `print_i64`/`print_str` ALGOL matrix test and an IIR-level `print_i64` +
  `putchar` ordering case.
- **PR1b — first op family on every backend.** `write_i64`, `i64_div_floor`,
  `i64_mod_floor`, `trap`. Shared lowering shape: pre-check `b == 0` → trap
  `DivZero`; `b == -1 && a == MIN` → trap `Overflow` (mod: result `0`); then
  `q = a / b; r = a % b; if r != 0 && (r ^ b) < 0 { q -= 1; r += b }`. Native
  runtime gains `__twig_write_i64` and `__twig_trap(int32)` (flush, kind name to
  stderr, `abort`); LLVM emits self-contained `linkonce_odr` definitions so the
  harness C runtime becomes unnecessary; WASM imports `env.__write_i64` and
  `env.__trap`; JVM uses `Math.floorDiv/floorMod` after the pre-check and
  `RuntimeException(kind)`; CLR `Console.Write(int64)` and `Exception(kind)`;
  BEAM `io:put_chars(integer_to_list(X))` and `erlang:error({iir_trap, K})`;
  vm-core `VMError::Trap { kind }` (also fixing the `MIN / -1` panic); JIT
  `DIV_FLOOR_I64`/`MOD_FLOOR_I64`/`TRAP`. Cases: sign table (±7, ±2), zero
  divisor, `MIN / -1`, `MIN / 1`, print-then-trap. May split per backend.
- **PR2 — Ruby frontend** on these ops (`rb_int_div` = `i64_div_floor`, `puts`
  = `print_i64`, `StdoutThenTrap` rows).
- **PR3 — Perl** (`print` = `write_i64`).
- **PR4 — overflow family. PR5 — strings. PR6 — floats + Python/JavaScript
  formatting.**

## PR1b — what landed

All four PR1b operations are `Lowered` on all nine backends. The shared
lowering shape is "test the edges, then divide." No backend lets a hardware or
library divide see a zero divisor or `MIN / -1`. On those edges x86 raises
SIGFPE, LLVM has undefined behaviour, AArch64 returns 0 or `MIN`, WASM traps
without a kind, the JVM returns `MIN` or throws `ArithmeticException`, the CLR
throws, and BEAM returns a bignum or raises `badarith`.

| Backend | `i64_div_floor` / `i64_mod_floor` | `write_i64` |
| --- | --- | --- |
| x86-64 | `test`/`cmp` guards, `cqo; idiv`, fix-up via `xor`-sign test (R8 scratch) | `__twig_write_i64` (`printf("%lld")`, no per-call flush) |
| AArch64 | `cbnz`/`add #1`+`cbnz`/`cmp` guards, `sdiv`+`msub`, `eor`+`cmp #0` fix-up | `__twig_write_i64` |
| LLVM | `icmp`+`br` to trap blocks, `sdiv`/`srem`, `select` fix-up; mod divides by 1 for `b = -1` | module-local `@__iir_write_i64` |
| WASM | `i64.eqz`/`i64.eq` + `if` guards, `div_s`/`rem_s` with recomputed remainder (no scratch locals) | `env.__write_i64` |
| JVM | `lcmp`/`ifne` guards, then `Math.floorDiv`/`floorMod` | `System.out.print(J)V` |
| CLR (textual) | `bne.un` guards, `div`/`rem` fix-up; mod stores 0 for `b = -1` | `Console.Write(int64)` |
| BEAM | `is_eq_exact` guards, branchy fix-up over truncating `div`/`rem`, temp in an allocated (GC-rooted) register | `io:format("~w", [X])` |
| VM | `floor_divmod` handler | default builtin, same stdout handle |
| JIT | `DIV_FLOOR_I64` / `MOD_FLOOR_I64` bytecodes | `CALL_BUILTIN` to the embedder sink |

Supporting changes:

- `jit_core::CIRInstr::is_pure` treats `trap` and the floor ops as impure, so
  dead-code elimination keeps an unused trapping divide. A conformance case
  pins that.
- `lang_aot::concretize_scalar_any_for_jvm` keeps a module that uses
  `write_i64`, `i64_div_floor` or `i64_mod_floor` on the wide i64 model. These
  ops are defined on i64.
- vm-core's legacy `div`/`mod` wrap on `MIN / -1` instead of panicking the
  host.
- The binary CLR path, `lower_iir_to_cil` for the 32-bit simulator, does not
  lower the PR1b ops. It returns a typed `UnsupportedOp`. `op_support`
  describes the textual path (§2).
- `op_conformance` gains a JVM execution column: real `java`, verifier on,
  class files at version 49. It also adds the gates
  `pr1b_ops_are_lowered_on_every_backend` and
  `flush_before_trap_is_proven_on_every_executable_column`. A column that
  traps must report the kind; `None` is a failure.

Local evidence and its limits: native x86-64, LLVM (clang), WASM, JVM (JDK 21),
VM and JIT execute every case. CLR and BEAM are compile-only locally, with no
`dotnet` or `erl`, and CI executes them. Their unit tests pin the emitted
IL/BEAM. The BEAM tests also execute the emitted floor sequence on a model of
the opcodes it uses. AArch64 is structural: each expected word comes from
`aarch64-encoder`. The macOS ARM64 CI leg runs it.

`lang_matrix` keeps its data-less `RunResult::Trapped`, and its WASM/VM/JIT
hosts do not yet provide `write_i64`/`__trap`. No language row emits a PR1b op
yet. LANG83 PR2 (Ruby), the first that does, ports `StdoutThenTrap` and those
host imports.

## What PR1a deliberately does not do

- No backend lowers a PR1b op; all nine declare them `Refused`. (PR1b lowers
  all four everywhere.)
- `lang_matrix`'s `RunResult::Trapped` stays data-less; only the conformance
  runner carries stdout through a trap.
- The matrix runners are not moved into `tests/common/`; `op_conformance.rs`
  has its own compact IIR-level runners for the columns it executes (WASM, VM,
  JIT in-process; LLVM and native when the toolchain exists; PR1b adds the
  JVM). CLR and BEAM are compile-checked there.
- The older builtin tables are checked against `OP_SUPPORT`, not yet derived
  from it.
