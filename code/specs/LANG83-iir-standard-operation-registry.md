# LANG83 — IIR standard operation registry and backend lowering contract

**Status:** Draft, 2026-10-19. Contract slice (PR1a), first op family on
all nine backends (PR1b, see "PR1b — what landed") and the overflow-checked
arithmetic family (PR4, see "PR4 — the overflow family") and Python/JavaScript
shortest round-trip float output (PR6a, see "PR6a — shortest round-trip float
output") under LANG78.

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
no longer used by any backend). `PR4` = registered *and* lowered by every
backend in the same slice (`introduced_by: "LANG83 PR4"`). `PR6a` = the same,
for the float-output slice (`introduced_by: "LANG83 PR6a"`). `Later` = planned; enters the
registry before any frontend emits it. `Unregistered` = exists in IIR today but
is not yet in the registry (it keeps working exactly as before; the registry
does not gate it).

| Family | Name | Kind | Status | Semantics |
| --- | --- | --- | --- | --- |
| Integer arithmetic | `add` `sub` `mul` | Op | Exists | Wrapping two's-complement on integers, IEEE on `f64`. |
| | `div` `mod` | Op | Exists | Truncating. Zero divisor and `MIN / -1` are **backend-dependent** today (see "Legacy edge behaviour"). |
| | `neg` | Op | Exists | Wrapping negation. |
| | `i64_div_floor` `i64_mod_floor` | Op | PR1b | Floor division / modulo (Python `//` `%`, Ruby `/` `%`). `b == 0` → `Trap.DivZero`; `MIN / -1` → `Trap.Overflow` for div, `0` for mod. |
| | `i64_div_trunc` `i64_mod_trunc` | Op | Later | Truncating with *defined* traps (the legacy `div`/`mod` with the edges fixed). Not part of PR4: no frontend needs them yet (Python and Ruby divide with the floor ops). |
| | `i64_add_ovf` `i64_sub_ovf` `i64_mul_ovf` | Op | PR4 | Exact `a + b`, `a - b`, `a * b`; a result outside `[i64::MIN, i64::MAX]` → `Trap.Overflow` (bignum promotion is a frontend decision made by catching it, once traps are catchable). |
| | `i64_neg_ovf` | Op | PR4 | Exact `-a`; only `-i64::MIN` overflows → `Trap.Overflow`. |
| | `i64_pow` | Op | Later | Integer power (a `pow` builtin exists on four backends only and is unregistered). |
| | `and` `or` `xor` `not` `shl` `shr` | Op | Unregistered | Bitwise; LLVM lacks `shl`/`shr` and the generic JIT lacks all six. Registered in a follow-up with honest per-backend declarations. |
| Comparison | `cmp_eq` `cmp_ne` `cmp_lt` `cmp_le` `cmp_gt` `cmp_ge` | Op | Exists | 0/1 (or `bool`) result. |
| Float | `f64_sqrt` `f64_sin` `f64_cos` `f64_ln` `f64_exp` `f64_atan` `f64_tan` | Op | Exists | IEEE / libm. |
| | `f64_pow` | Op | Exists | libm `pow`. |
| | `f64_div_floor` `f64_mod_floor` `f64_is_nan` | Op | Later | Python float `//` `%` (`fmod`, then a sign fix-up; `b == 0` → `Trap.DivZero`). Deferred to PR6b: WASM has no `fmod` instruction, so it needs its own exact remainder routine. |
| Conversions | `int_to_real` | Op | Exists | Exact `i64 → f64`. |
| | `real_to_int_trunc` `real_to_int_floor` | Op | Exists | Non-finite or out-of-range → `Trap.ConvRange`. |
| | `i64_to_str`, `f64_to_str_py` `_js` `_rb` `_pl` | Op | Later | One op per *layout*, not a flags operand; every backend uses one internal shortest-digits core. The oracle is host `python3` / `node`. Rust `format!("{:e}")` gives the same digit *length* but rounds an exact tie up, not to even (see "PR6a"). |
| Output | `print_i64` | Builtin | Exists | Decimal value **plus `\n`**. Kept: BASIC, Oct, ALGOL and Ruby `puts` depend on it. |
| | `putchar` | Builtin | Exists | Low byte of the argument. |
| | `print_str` | Op | Exists | String bytes, no newline. |
| | `write_i64` | Builtin | PR1b | Decimal value, **no** newline (Perl `print`, Python `end=""`). |
| | `write_f64_py` | Builtin | PR6a | Python `repr(float)` (= `str` = what `print` writes), **no** newline: shortest round-trip digits; fixed notation when the decimal exponent `e` satisfies `-4 <= e < 16`, else `d.ddde±XX` (at least two exponent digits); integral values keep `.0`; `-0.0`, `inf`, `-inf`, `nan`. |
| | `write_f64_js` | Builtin | PR6a | JavaScript `Number.prototype.toString()` (what `console.log` writes for a number), **no** newline: the same digits; fixed when `-7 < e < 21`, else `d.ddde±X` (no padding); integral values print bare (`1`, not `1.0`); `-0` → `0`, `Infinity`, `-Infinity`, `NaN`. |
| | `write_f64_rb` | Builtin | Later | Ruby `Float#to_s` (fixed when `-4 <= e < 16`, `1.0e+16` style otherwise). |
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
| GenericCirJit | `VMError::DivisionByZero` (via the error slot; was fail-open before the PR1b security review) | `wrapping_div` |

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
| JIT | `TRAP <id>` bytecode; error slot `trap: <kind>`, drained by `JITCore` after every run | `VMError::Trap { kind }` from `JITCore::execute` / `execute_with_jit` |

**JIT traps fail closed (PR1b security review).** `Backend::run` returns a
plain `Value`, so `GenericCirJit` records a trap in its error slot. The first
PR1b cut stopped there: the JIT handler vm-core called returned the placeholder
`Value::Null`, nothing on the call path read the slot, and an *interpreted*
caller of a trapping JIT-compiled function kept running (and `execute_with_jit`
returned `Ok`). vm-core JIT handlers are now fallible
(`Fn(&[Value]) -> Result<Value, VMError>`); after every run `JITCore` drains
the slot through `Backend::take_error` (`trap: <kind>` → `VMError::Trap`,
`div_*` by zero → `VMError::DivisionByZero`, anything else →
`VMError::Custom`), and `call` propagates it with `?`. A JIT trap therefore
stops the whole program exactly like an interpreted one.

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
| `Trap.Overflow` | 5 | `i64_div_floor` `MIN / -1`; the PR4 `i64_*_ovf` family |
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
- `has_side_effects` now includes `trap`, `i64_div_floor` and `i64_mod_floor`
  (and, since PR4, the four `i64_*_ovf` ops), so a dead-code pass must keep
  them even when their result is unused (a removed trapping divide or add
  would change observable behaviour).
  **Divergence kept deliberately in PR1a:** the legacy `div`/`mod` are *not*
  added to `has_side_effects`, because the existing optimisers rely on them
  being pure and their trap behaviour is backend-dependent anyway.
- `is_value_producing` includes `i64_div_floor` and `i64_mod_floor` (and
  every other registry Op with a dest, so the PR4 ops with no edit).
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
- **PR4 — overflow family** (`i64_add_ovf`, `i64_sub_ovf`, `i64_mul_ovf`,
  `i64_neg_ovf`), registered and lowered on all nine backends in one slice;
  see "PR4 — the overflow family". `i64_div_trunc` / `i64_mod_trunc` stay
  `Later`.
- **PR5 — strings. PR6 — floats + Python/JavaScript formatting.** PR6 is
  split: **PR6a** (`write_f64_py`, `write_f64_js` and each backend's
  shortest-digits core; see "PR6a — shortest round-trip float output") and
  **PR6b** (`f64_div_floor`, `f64_mod_floor`, `f64_is_nan`, `i64_to_str` and
  the `f64_to_str_*` string-producing family, which needs PR5's runtime
  strings on every backend).

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

## PR4 — the overflow family

Python's and Ruby's integers are unbounded. A frontend that keeps small
integers in an `i64` needs to know, *exactly*, when an operation leaves the
`i64` range, so it can promote to a bignum (later, by catching the trap) or
report an error. PR4 adds the four checked operations that make that
observable on every backend:

| Op | Result | Traps `Trap.Overflow` when |
| --- | --- | --- |
| `i64_add_ovf a, b` | `a + b` | the exact sum is outside `[MIN, MAX]` |
| `i64_sub_ovf a, b` | `a - b` | the exact difference is outside the range |
| `i64_mul_ovf a, b` | `a * b` | the exact product is outside the range (`MIN * -1` included) |
| `i64_neg_ovf a` | `-a` | `a == MIN` |

They are type-first (`i64_add_ovf`, never `add_ovf`): the native backends and
the generic JIT dispatch on `add_`, `sub_`, `mul_` and `neg_` prefixes, and an
`add_ovf` would be lowered as a *wrapping* add with a nonsense type suffix. The
registry test `names_avoid_dispatch_prefixes` already lists `add_ovf` and
`neg_ovf` as names it must reject. They pass through `jit_core::specialise`
unchanged (it type-suffixes only the classic `add`/`sub`/`mul`/`neg`).

No other trap kind is possible, and no operand edge is undefined, so the
registry entries list `may_trap: [Trap.Overflow]`. Like the PR1b ops they are
side-effecting (`has_side_effects`, `CIRInstr::is_pure` → impure), so
dead-code elimination keeps an unused checked add whose only observable effect
is its trap, and they are wide-model evidence for
`concretize_scalar_any_for_jvm` (an op *defined* on i64 must not be narrowed to
`int`). That evidence is now derived from the registry — every Op with an
`i64` operand type — instead of a hand-kept name list.

### Shared lowering shape

"Detect, then trap; never let a wrapped value escape." Each backend computes
the overflow condition with whatever its target offers, branches to the same
trap sequence PR1b established (flush, report the kind, terminate), and only
then writes the destination. A destination that aliases an operand
(`x = i64_add_ovf x, y`) is therefore safe everywhere: the operands are read
before the store.

| Backend | `i64_add_ovf` / `i64_sub_ovf` | `i64_mul_ovf` | `i64_neg_ovf` |
| --- | --- | --- | --- |
| x86-64 | `add`/`sub r64` sets OF; `jno` over the trap | `imul r64, r64` sets OF exactly when the signed 128-bit product does not fit; `jno` | `neg r64` sets OF only for `MIN`; `jno` |
| AArch64 | `adds`/`subs` set V; `b.vc` over the trap | `mul` (low half) + `smulh` (high half); `cmp hi, lo, asr #63`; `b.eq` over the trap | `negs` (= `subs xd, xzr, xm`) sets V only for `MIN`; `b.vc` |
| LLVM | `@llvm.sadd.with.overflow.i64` / `@llvm.ssub.with.overflow.i64`; `extractvalue` the flag; `br` to a trap block | `@llvm.smul.with.overflow.i64` | `@llvm.ssub.with.overflow.i64(0, a)` |
| WASM | no flag: sign test `((a ^ r) & (b ^ r)) < 0` for add, `((a ^ b) & (a ^ r)) < 0` for sub, with `r` recomputed (no scratch local) | no wide multiply: `a == -1 && b == MIN` → trap; else `a != 0 && (a * b) / a != b` → trap | `a == MIN` → trap; else `0 - a` |
| JVM | the WASM sign test on `long`, `ifge` over the trap | the WASM guards with `lmul`/`ldiv` | `lcmp` against `MIN` |
| CLR (textual) | the WASM sign test on `int64`, `bge` over the trap | the WASM guards (the `-1 × MIN` guard also keeps `div` from throwing) | `bne.un` against `MIN` |
| BEAM | exact bignum `+`/`-` (`gc_bif2`), then `is_lt` range checks against `MIN` and `MAX` | exact bignum `*`, same range checks | `0 - a`, same range checks |
| VM | `checked_add` / `checked_sub` / `checked_mul` / `checked_neg` → `VMError::Trap { kind: "Trap.Overflow" }` | | |
| JIT | `ADD_OVF_I64` / `SUB_OVF_I64` / `MUL_OVF_I64` / `NEG_OVF_I64` bytecodes (`checked_*`); error slot `trap: Trap.Overflow`, drained by `JITCore` | | |

Why the multiply guard is exact on WASM/JVM/CLR: if no overflow happened, the
wrapped product `p` equals `a * b`, so `p / a == b`. If it did, `p` differs
from `a * b` by a non-zero multiple of 2^64, which is larger than `|a|`, so
`p / a` cannot truncate back to `b`. The one product whose check would itself
overflow (`p = MIN`, divided by `a = -1`) is exactly the `-1 × MIN` case, which
is tested first.

**Divergences from the PR4 plan, and why.**

- *JVM:* the plan allowed `Math.addExact` / `subtractExact` /
  `multiplyExact` / `negateExact`. They throw `ArithmeticException`: no kind,
  and no `System.out` flush. Turning that into the LANG83 trap needs an
  exception-table entry, which `iir-to-jvm-class-file` does not emit. So PR4
  pre-checks, exactly as PR1b's floor division does. It needs no JDK 9+
  method (`Math.multiplyHigh`), so the class still runs on every JVM the
  backend targets.
- *CLR:* the plan allowed `add.ovf` / `sub.ovf` / `mul.ovf`, which throw
  `System.OverflowException`. Mapping that to the kind needs a `.try`/`catch`
  region, which the textual path has never emitted, and CoreCLR is stricter
  about protected-region stack shape than `ilasm` (Perl 4a). With no local
  `dotnet`, PR4 uses the branch-around shape PR1b already proved in CI. The
  observable result is identical: `Console.Out.Flush()`, then
  `System.Exception("Trap.Overflow")`.
- The binary CLR path (`lower_iir_to_cil`, the 32-bit simulator) refuses the
  PR4 ops with a typed `UnsupportedOp`, like the PR1b ops. `op_support`
  describes the textual path (§2).

### Conformance cases

`op_conformance` gains, for each op, cases at the edges of the range: `MAX + 0`,
`MAX - 1 + 1`, `MIN + (-1)`, `MAX + 1`, `MIN - 1`, `MAX - (-1)`, `MIN - 0`,
`MIN * 1`, `MIN * -1`, `-1 * MIN`, `2^32 * 2^31` (= `2^63`, one past `MAX`),
`-2^32 * 2^31` (= `MIN`, which fits), `3_037_000_500²` (just past `MAX`),
`neg MIN`, `neg MAX`, `neg 0`. Every trapping case prints first and then traps
(`Expect::StdoutThenTrap`), and one case discards its result (DCE must keep it).
One case uses no `print_i64`: it writes with `putchar` only, so the JVM narrowing
pass is actually exercised. The gate `pr4_ops_are_lowered_on_every_backend`
pins that every backend lowers all four.

Local evidence and its limits are those of PR1b: native x86-64, LLVM (clang),
WASM, JVM, VM and JIT execute every case. CLR, BEAM and AArch64 are structural
locally (IL text, BEAM instruction streams, and AArch64 words compared against
`aarch64-encoder`'s own encodings, which gains `adds`, `subs`, `negs`, `smulh`
and `cmp … asr #imm` for this PR), and execute in CI.

## PR6a — shortest round-trip float output

Python's `print(x)` and JavaScript's `console.log(x)` for a float both print
the **shortest** decimal string that reads back as exactly `x`. They agree on
the digits and disagree on the layout. PR6a adds one output builtin per
layout. Each writes to the program's single ordered stream without a newline,
like `write_i64`:

| | `write_f64_py` (Python `repr`) | `write_f64_js` (JS `Number#toString`) |
| --- | --- | --- |
| fixed notation when | `-4 <= e < 16` | `-7 < e < 21` |
| `1e16` | `1e+16` | `10000000000000000` |
| `1e21` | `1e+21` | `1e+21` |
| `1.5e-7` | `1.5e-07` (two exponent digits at least) | `1.5e-7` |
| `1.0`, `100.0` | `1.0`, `100.0` | `1`, `100` |
| `0.0` / `-0.0` | `0.0` / `-0.0` | `0` / `0` |
| `inf` / `-inf` / `nan` | `inf` / `-inf` / `nan` | `Infinity` / `-Infinity` / `NaN` |

Here `e` is the decimal exponent of the first significant digit
(`123.4` has `e = 2`). Sci notation puts a `.` after the first digit only when
more digits follow, then `e`, then the sign, which is always present.

### The digits: shortest, then closest, ties to even

For finite non-zero `x`, the digit string `D` is the shortest one such that
`D × 10^k` rounds to `x` under round-half-even. If several strings of that
length qualify, it is the one closest to `x`. On an exact tie the last digit
is even. Python's `repr` and ECMAScript's `Number::toString` produce exactly
this. So, per their documentation, do Java 19+'s `Double.toString`, .NET Core
3.0+'s `"R"` and Erlang's `float_to_list(F, [short])` (Ryu). Two host
formatters differ, so the backends that use them correct for it:

- **Rust's `{:e}` rounds an exact tie up.** Against host `python3` on 535 432
  values, Rust's digits differed on 94, and every one was such a tie. `2^-25`
  is exactly `2.98023223876953125e-08`: Python prints `…312e-08` and Rust
  `…313e-8`. The same happens for any double near `1e15` with a `.25` or `.75`
  fraction. So Rust's `{:e}` is NOT the oracle the vocabulary table first
  named; host `python3` and `node` are.
- **Java never prints fewer than two digits**, so `Double.MIN_VALUE` is
  `4.9E-324` where Python and JavaScript print `5e-324`. This only matters
  for the deepest subnormals, where a one-digit decimal reads back.

The **one internal core** (the vocabulary table's "one shortest-digits core")
is the Burger–Dybvig free-format algorithm (Steele–White / Dragon4 family)
over exact big integers. It is short, needs no tables, and is exact by
construction. Where a backend has no host routine that already produces these
digits, it carries this core as its own module-local code:

```text
x = f · 2^e   (f the 53-bit significand, e the binary exponent; subnormals e = -1074)
even   = f is even               (round-half-even: boundaries count as inside)
closer = f == 2^52 and the biased exponent > 1   (the gap below x is half the gap above)
r  = f · 2^(max(e,0) + 1 + closer)      s  = 2^(max(-e,0) + 1 + closer)
m+ = 2^(max(e,0) + closer)              m- = 2^max(e,0)
           so r/s = x, and m+/s, m-/s are the half-gaps above and below x
k  = floor((e + 52) · 1233 / 4096)      an estimate of the decimal exponent;
                                        scale s by 10^k (k >= 0) or r, m+, m- by 10^-k
fix k up:   while r + m+ >= s (> s when odd):     s *= 10; k += 1
fix k down: while 10(r + m+) < s (<= s when odd): r, m+, m- *= 10; k -= 1
loop:
    r, m+, m- *= 10;  d = r div s;  r = r mod s           d is 0..9
    low  = r <= m- (< when odd)       high = r + m+ >= s (> when odd)
    neither: emit d and continue
    low only: emit d;   high only: emit d + 1
    both: compare 2r with s: less → d, greater → d + 1, equal → whichever is even
x = 0.D × 10^k, so the layout exponent is e10 = k - 1
```

`d + 1` never carries: the invariant `r + m+ < s` entering each iteration
gives `d + 1 <= 9`. The big integers stay below 2^1084 for every finite
double (measured over every power of two, both ends of the subnormal range,
and 200 000 random bit patterns), so the module-local versions use a fixed 40
limbs of 32 bits (1 280 bits), with each limb in a 64-bit word so `limb × m +
carry` never overflows. A multiply by a power of two or of ten is done 31 bits
or 10^9 at a time, so the worst case (`5e-324`) takes about 35 + 36 limb
passes, not 1 400.

A Python prototype of exactly this core plus the two layouts was checked
against host `python3` `repr` on 1.2 million values (every power of two,
subnormals, `±max`, random bit patterns, random integers and short
decimals) and against host `node` `String(x)` on 400 000 values, with no
mismatch.

### Per-backend design

No backend links a shared helper library: each one either calls its own host's
shortest formatter and re-lays out the result, or carries the core as code it
emits itself.

| Backend | Digits from | Layout and output |
| --- | --- | --- |
| x86-64 / AArch64 | `__twig_write_f64_py` / `__twig_write_f64_js` in the native runtime (`twig_runtime.c`, the same place PR1b put `__twig_write_i64`): the core above in C, on stack bignums | C; `putchar` on the same `stdout` `FILE*` as `printf`. The argument is the raw bit pattern in a GPR (`rdi` / `x0`), so the generic `call_builtin` marshalling is unchanged. |
| LLVM | module-local `linkonce_odr` IR: `@__iir_bn_*` limb routines and `@__iir_f64_digits` (the core, bignums in `alloca [40 x i64]`) | `linkonce_odr @__iir_write_f64(double, i32 layout)`; libc `putchar` |
| WASM | in-module functions: `$__iir_bn_*` limb routines and the core, bignums in a 1 664-byte region reserved at the end of the module's static data (so neither the bump allocator nor string data can overlap it) | in-module `$__iir_write_f64(f64, i32 layout)`; one `env.putchar` per byte |
| JVM | `Double.toString(Math.abs(x))`: JDK 19+ implements Raffaello Giulietti's shortest algorithm (JDK-4511638). CI runs JDK 21. When two digits survive, the helper tries the two one-digit neighbours with `BigDecimal.doubleValue` and keeps the closer one that reads back (even on a tie), because Java never prints one digit. | synthetic private static method `__iir_write_f64(DI)V`: `new BigDecimal(s).stripTrailingZeros()` gives `D` (`unscaledValue()`) and `e10 = len(D) - 1 - scale()`; fixed layout is `toPlainString()`, sci is built with a `StringBuilder`; one `System.out.print(String)` |
| CLR (textual) | `Math.Abs(x).ToString("R", CultureInfo.InvariantCulture)`: shortest round-trip on .NET Core 3.0+ | IL helper `__iir_write_f64(float64, int32)` that parses the mantissa / `E` exponent with `IndexOf`, `Substring`, `Remove`, `TrimStart('0')`, `TrimEnd('0')` and `Int32.Parse`, then `Console.Write(string)` for each piece |
| BEAM | `erlang:float_to_list(F, [short])`: shortest round-trip since OTP 25 (Ryu). CI runs OTP 27. | module function `'__iir_write_f64'/2` (exported, because `ir-to-beam` writes the Code chunk's function count as the export count): `string:split` on `e` and `.`, `string:trim` of `0`s, `list_to_integer`, then `io:put_chars/1` per piece |
| VM | Rust `format!("{:e}", x)` for the shortest digits. Then Rust's exact `{:.1100e}` expansion detects an exact tie: one digit longer than the shortest, ending in `5`. On a tie the module takes the even neighbour whenever it also reads back. | `vm_core::float_format::{python_repr, js_to_string}`; default builtins on the same `stdout` handle as `write_i64` |
| JIT | the same Rust functions, through the embedder-registered `write_f64_py` / `write_f64_js` handler | new `CALL_BUILTIN_F64` bytecode, which passes the register as `Value::Float`. `const_f64` compiles to the register's bit pattern, and a float literal that the optimiser's constant propagation folded into the call is materialised the same way. An `f64` parameter is loaded by a `PARAM_F64` prologue instruction. That instruction requires a `Value::Float` argument and fails the call closed otherwise (security review); other parameters keep the old integer seeding. Any other use of an `f64` register still refuses compilation, so it falls back to the interpreter, as it did before. |

The host formatters fix only the **digits**. Each backend re-derives `D` and
`e10` from the host's string and applies the LANG83 layout itself, so the
host's own layout rules (Java's `1.0E16`, .NET's `1E+16`, Erlang's `1.0e16`)
never reach the output.

**BEAM has no non-finite floats.** The emulator raises `badarith` before an
`inf` or `nan` can exist, so `'__iir_write_f64'` has no `inf`/`nan` arms; `-0.0` is
a distinct float since OTP 27 and keeps its sign through `float_to_list`.

**Python and JavaScript only.** Ruby's `Float#to_s` (`write_f64_rb`) and
Perl's `%.15g` layout (`f64_to_str_pl`) use the same core with other
layouts. They stay `Later` until their frontends emit them.

### Conformance cases

`op_conformance` gains one case per layout that prints a 90-value table, each
value followed by `putchar('\n')`. The table holds the layout edges and the
hard digits:

- `0.0`, `-0.0`, `±inf` and `nan` (BEAM cannot represent the last three);
- `1.0`, `100.0`, `0.1`, `1e15`, `1e16`, `1e20`, `1e21`;
- `1e-4`, `1e-5`, `1e-6`, `1e-7`, `1.5e-7`, `123.456`, `-2.5`;
- `5e-324`, `2.2250738585072014e-308`, `1.7976931348623157e308`;
- the round-half-even ties `2^-25` and `1072012167405248.25`;
- `2^53 + 2`, `2e23` (the value JDK < 19 printed long), `0.1 + 0.2`, `1e-300`;
- 40 random finite bit patterns and 20 random short decimals from fixed
  seeds.

The expected strings are **generated by host `python3` and `node`** and
checked in. The test `float_table_matches_host_python3_and_node` re-runs both
oracles whenever they are installed and fails if a checked-in string drifts.

Two more cases cover output that is not a table:

- One interleaves the writers with `write_i64`, `print_i64` and `putchar`, so
  the ordered-stream contract covers them.
- One prints computed floats (`0.1 + 0.2`, `int_to_real 7`), not only
  constants.

The gate `pr6a_ops_are_lowered_on_every_backend` pins that all nine backends
lower both builtins. It also requires both table cases to pass on every
executable column whose toolchain is present.

Local evidence and its limits are those of PR4: native x86-64, LLVM (clang),
WASM, JVM (JDK 21), VM and JIT execute every case. Beyond the 90-value table,
four implementations were checked on the full prototype corpus (535 432 values
against `python3`, 402 112 against `node`), with no mismatch:

- the native runtime's C (also clean under ASan/UBSan);
- the LLVM IR at `-O0` and `-O2`;
- the emitted WASM module, run on V8;
- the JVM helper on JDK 21.

The VM/JIT formatter was checked the same way. CLR, BEAM and AArch64 are
structural here. Their unit tests pin the emitted IL, BEAM instructions and
AArch64 words. The CLR helper is checked by an IL stack-depth model: every
label is reached at depth 0, no instruction underflows, and the declared
`.maxstack` covers the peak. This matters because CoreCLR rejects IL that
`ilasm` accepts (Perl 4a). The BEAM helper runs on an opcode-and-BIF model
whose `float_to_list(F, [short])` is fed every plausible Erlang layout of the
same digits (`1.0e16`, `1.0e+16`, `10000000000000000.0`) and must produce
Python's and JavaScript's strings for each. CI executes CLR and BEAM through
`lang_matrix`'s real `dotnet` and `erl` columns once a language row emits
these builtins (LANG83 PR2 for Python).

### Divergences from the PR6a plan, and why

- *The oracle.* The plan named Rust's `format!("{:e}")` digits as the oracle.
  Rust rounds an exact tie up, so the oracle is host `python3` / `node`. The
  VM/JIT formatter takes Rust's digits and applies round-half-even with Rust's
  exact expansion (see "The digits").
- *JVM.* `Double.toString` was planned as the whole digit source. It prints at
  least two digits, so the helper adds the one-digit neighbour check.
- *JIT.* `JITCore`'s optimiser propagates a `const_f64` into the writer's
  operand, so the JIT accepts a float literal there as well as a float
  register.
- *Table size.* The plan named 40 random values. The table has 60: 40 bit
  patterns and 20 short decimals, because short decimals exercise the fixed
  layouts that random bit patterns rarely reach.

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
