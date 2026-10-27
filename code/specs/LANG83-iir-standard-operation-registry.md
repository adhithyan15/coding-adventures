# LANG83 — IIR standard operation registry and backend lowering contract

**Status:** Draft, 2026-10-19. Contract slice (PR1a), first op family on
all nine backends (PR1b, see "PR1b — what landed") and the overflow-checked
arithmetic family (PR4, see "PR4 — the overflow family") and Python/JavaScript
shortest round-trip float output (PR6a, see "PR6a — shortest round-trip float
output") and non-trapping IEEE `f64` arithmetic with `f64_signbit` (PR6c, see
"PR6c — IEEE f64 arithmetic and the sign bit") under LANG78. PR7 (planned,
see "PR7 — language neutrality") removes every language-specific op, builtin
and trap kind.

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

**Amended by PR7 (2026-10-26).** The owner later added: "the IIR should not
contain any language specific instructions. And IIR should be low level
enough. The IIR compiler is responsible for converting things." Both
directions still hold, and they divide the work as follows:

- A computation with **one language-independent definition** is a registry
  op that every backend lowers, such as shortest round-trip digits, floor
  division or I/O.
- A **language's rule** is code the IIR compiler emits from those ops, such
  as Python's `repr` thresholds, `Infinity` versus `inf`, or Lisp
  truthiness. When the code is long, the compiler emits it as a function in
  its own module.

What stays rejected is direction (1) proper: a *shared* helper library, and
helpers that stand in for an operation a backend should lower. See "PR7 —
language neutrality".

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
for the float-output slice (`introduced_by: "LANG83 PR6a"`). `PR6c` = registered in one
slice and lowered by every backend whose floats are IEEE-754 binary64; BEAM
declares the arithmetic `Refused` (see "PR6c"). `Later` = planned; enters the
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
| | `f64_add` `f64_sub` `f64_mul` `f64_div` | Op | PR6c | IEEE-754 binary64 `+ - × ÷` with round-to-nearest-even. **Never traps**: `1/0` is `+Infinity`, `-1/0` is `-Infinity`, `0/0` and `Infinity - Infinity` are `NaN`, an overflowing result is `±Infinity`, and the sign of a zero result follows IEEE (`-1 × 0` is `-0`). Type-first twins of the classic `add`/`sub`/`mul`/`div` on `f64`, whose zero-divisor edge is backend-dependent (see "Legacy edge behaviour"). BEAM: `Refused`. |
| | `f64_neg` | Op | PR6c | IEEE negation: flips the sign bit only, so `f64_neg 0` is `-0` and `f64_neg -0` is `0` (unlike `0 - x`). BEAM: `Refused`. |
| | `f64_signbit` | Op | PR6c | The sign bit of an `f64` as an integer `0`/`1`: `1` for `-0`, `-Infinity` and every negative value; `0` for `0`, `+Infinity` and every positive value. The only way to tell `-0` from `0` without dividing by it. Lowered on all nine backends. |
| | `f64_div_floor` `f64_mod_floor` `f64_is_nan` | Op | Later | Python float `//` `%` (`fmod`, then a sign fix-up; `b == 0` → `Trap.DivZero`). Deferred to PR6b: WASM has no `fmod` instruction, so it needs its own exact remainder routine. `f64_is_nan` is superseded by PR7's `f64_class`. |
| Conversions | `int_to_real` | Op | Exists | Exact `i64 → f64`. |
| | `real_to_int_trunc` `real_to_int_floor` | Op | Exists | Non-finite or out-of-range → `Trap.ConvRange`. |
| | `f64_class` `f64_digits` `f64_exp10` | Op | PR7b | Class (`0` zero, `1` finite, `2` infinite, `3` NaN), shortest round-trip significand and leading-digit exponent. Each compiler builds its language's layout from these; see "PR7 — language neutrality". Lowered on all nine backends (BEAM: classes 0 and 1 only, its floats being finite). |
| | `i64_to_str`, `f64_to_str_py` `_js` `_rb` `_pl` | Op | Later | **Withdrawn by PR7** (`i64_to_str` stays): a layout is a language rule, so it belongs in a compiler-emitted helper. One op per *layout*, not a flags operand; every backend uses one internal shortest-digits core. The oracle is host `python3` / `node`. Rust `format!("{:e}")` gives the same digit *length* but rounds an exact tie up, not to even (see "PR6a"). |
| Output | `print_i64` | Builtin | Exists | Decimal value **plus `\n`**. Kept: BASIC, Oct, ALGOL and Ruby `puts` depend on it. |
| | `putchar` | Builtin | Exists | Low byte of the argument. |
| | `print_str` | Op | Exists | String bytes, no newline. |
| | `write_i64` | Builtin | PR1b | Decimal value, **no** newline (Perl `print`, Python `end=""`). |
| | `write_f64_py` | Builtin | PR6a | **Removed in PR7e** (language-specific). Python `repr(float)` (= `str` = what `print` writes), **no** newline: shortest round-trip digits; fixed notation when the decimal exponent `e` satisfies `-4 <= e < 16`, else `d.ddde±XX` (at least two exponent digits); integral values keep `.0`; `-0.0`, `inf`, `-inf`, `nan`. |
| | `write_f64_js` | Builtin | PR6a | **Removed in PR7e** (language-specific). JavaScript `Number.prototype.toString()` (what `console.log` writes for a number), **no** newline: the same digits; fixed when `-7 < e < 21`, else `d.ddde±X` (no padding); integral values print bare (`1`, not `1.0`); `-0` → `0`, `Infinity`, `-Infinity`, `NaN`. |
| | `write_f64_rb` | Builtin | Later | **Withdrawn by PR7**: the Ruby compiler emits a helper. Ruby `Float#to_s` (fixed when `-4 <= e < 16`, `1.0e+16` style otherwise). |
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
| `Trap.Unbound` | 7 | reading a name that is not bound yet or at all (JavaScript's `ReferenceError`, Python's `NameError`; LANG84 phase 6e). Named `Trap.ReferenceError` until **PR7d renamed it**, same id. |

The ids are what native backends pass to the runtime (`mov edi, id; call
__twig_trap`); they are append-only and never renumbered. Language-specific
kinds (for example `Ruby.ZeroDivisionError`) were once to be added to a
`LANGUAGE_TRAP_KINDS` table in the same file, with append-only ids starting
at 1000. **PR7d retired that table** before anything used it: a compiler
maps its language's error onto the neutral kind for its condition (Python's
`ZeroDivisionError` is `Trap.DivZero`; JavaScript's `ReferenceError` and
Python's `NameError` are `Trap.Unbound`), and a catchable language exception
is a frontend-built value, never a trap kind (see "PR7 — language
neutrality"). `exception_kind::is_trap_kind` is the whole closed table.
Nothing else may be emitted as a trap kind.

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
- **PR3 — Perl** (`print` = `write_i64`). Landed as LANG84 phase 4a: the
  Perl frontend emits `call_builtin write_i64` (no more `pl_print_int`), and
  eleven `lang_matrix` rows run it; the matrix WASM host and VM/JIT runners
  now provide `write_i64` on the ordered sink. No Perl row emits a floor op
  or traps yet (the subset has no `/`, `%` or `int()`).
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
- **PR6c — IEEE `f64` arithmetic and the sign bit** (`f64_add`, `f64_sub`,
  `f64_mul`, `f64_div`, `f64_neg`, `f64_signbit`), the prerequisite for
  LANG84 JavaScript 6e (`console.log(-0)` prints `-0`; `1/0` prints
  `Infinity`). See "PR6c — IEEE f64 arithmetic and the sign bit".
- **PR7 — language neutrality** (PR7a–PR7h): `write_f64_py`/`write_f64_js`
  give way to the neutral `f64_class`/`f64_digits`/`f64_exp10` plus
  compiler-emitted layout helpers; `Trap.ReferenceError` becomes
  `Trap.Unbound` (landed, PR7d); Lisp-family semantics leave the `dyn_*` runtime; a
  registry test forbids language names. See "PR7 — language neutrality".

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
host imports. (Done: see "PR2 — what landed".)

## PR2 — what landed

LANG84 phase 5a. `ruby-iir-compiler` emits `i64_div_floor` for `/`,
`i64_mod_floor` for `%` and `print_i64` for `puts` (plus `putchar 10` for
`puts()`); the Ruby-named builtins (`rb_int_div`, `rb_puts_int`, …) are gone.
A zero divisor is a run-time `Trap.DivZero`, as Ruby's `ZeroDivisionError` is,
not a compile error. `Language::Ruby` has thirteen `lang_matrix` rows on all
eight executable columns, and `check_module_for` passes on all nine backends.

The matrix now carries the trap data the conformance suite already had:

- `Expect::StdoutThenTrap(stdout, kind)` and `RunResult::TrappedAfter {
  stdout, kind }`. The data-less `Expect::Trap` / `RunResult::Trapped` stay
  for the older rows (bounds traps whose kind no row asserts yet).
- Kind sources: native, LLVM, JVM and CLR stderr (the first `Trap.` word);
  WASM's `env.__trap` id through `exception_kind::trap_kind_name` (the matrix
  `PrintHost` now resolves `env.__trap`); `VMError::Trap { kind }` on the VM
  and JIT; on BEAM a `try … catch error:K` launcher prints the reason atom
  between sentinels.
- Four Ruby rows trap: output then a zero divisor, a zero modulus, a zero
  divisor in a second `puts` argument (nothing printed, because Ruby evaluates
  every argument first), and a divisor that is zero only after evaluation.

Local evidence: native x86-64, LLVM, WASM, JVM, VM and JIT executed all 78 of
their Ruby cells, 24 of them trapping with the right stdout and kind. CLR and
BEAM are compile-checked locally and execute in CI.

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

## PR4's first frontend: Python (LANG84 phase 7a)

`python-iir-compiler`'s release path is the first frontend to emit the PR4
ops. Python `+ - *` and unary `-` on `i64` are `i64_add_ovf`, `i64_sub_ovf`,
`i64_mul_ovf` and `i64_neg_ovf`. Where CPython would produce an `int`
outside `i64`, the program prints its output so far and stops with
`Trap.Overflow`. That is the frontend's stated divergence until traps are
catchable and it can promote to a bignum. `//` and `%` are the PR1b floor
ops. A divisor known to be `-1` is folded (`a // -1` is `i64_neg_ovf a`,
`a % -1` is `0`), so `MIN // -1` and `MIN % -1` never reach a backend's
divide.

`lang_matrix` carries the trap data through the same `StdoutThenTrap` port
as PR2 (Ruby). Five Python rows are the first matrix rows whose kind is
`Trap.Overflow`: `MAX + 1`, `MIN - 1`, `3037000500²`, `-MIN` and `MIN // -1`.
Each one checks the stdout before the trap and the kind on every column.
Locally, native, LLVM, WASM, JVM, VM and JIT reported the right kind for
all of them. CLR and BEAM run in CI.

## PR6a — shortest round-trip float output

> **History.** This section records what PR6a built. LANG83 PR7e removed the
> two builtins it describes, `write_f64_py` and `write_f64_js`: each named a
> source language and carried that language's layout into all nine backends,
> which "PR7 — language neutrality" rules out. What survives is the digit
> core below ("The digits", and the "Digits from" column of the per-backend
> table). Since PR7b each backend exposes it as `f64_class`, `f64_digits` and
> `f64_exp10`; since PR7c-js / PR7c-py the two layouts live in the compilers,
> as javascript-iir-compiler's `__js_write_number` and python-iir-compiler's
> `__py_write_float`. The 90-value table under "Conformance cases" stays the
> oracle; `op_conformance` now prints it through those helpers. See "What
> PR7e landed".

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
3.0+'s `"R"` and Erlang's `float_to_list(F, [short])` (Ryu). The JVM backend
does not rely on `Double.toString`, because a version-49 class may run on a
JVM older than 19 (see the JVM row below). Two host formatters differ, so the
backends that use them correct for it:

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
| LLVM | module-local IR (`linkonce_odr` in PR6a, `internal` since PR7b): `@__iir_bn_*` limb routines and `@__iir_f64_digits` (the core, bignums in `alloca [40 x i64]`) | `@__iir_write_f64(double, i32 layout)`; libc `putchar` |
| WASM | in-module functions: `$__iir_bn_*` limb routines and the core, bignums in a 1 664-byte region reserved at the end of the module's static data (so neither the bump allocator nor string data can overlap it) | in-module `$__iir_write_f64(f64, i32 layout)`; one `env.putchar` per byte |
| JVM | a search in the helper, with Java 5 APIs only, so the output does not depend on the JDK. `exact = new BigDecimal(Math.abs(x))` is the double's exact value. For p = 1, 2, … 17 it computes `lo = exact.round(new MathContext(p, FLOOR))` and `hi = exact.round(new MathContext(p, CEILING))`, and stops at the first p where `lo.doubleValue()` or `hi.doubleValue()` equals `|x|`. If only one reads back, that one wins. If both do, the closer to `exact` wins, and on an exact tie the even last digit. Both neighbours are needed because the round-trip interval is lopsided at a power of two. Any p-digit decimal in the interval implies that `lo` or `hi` is in it, so the first such p is the shortest length. `Double.toString` is NOT used: it is shortest only on JDK 19+ (JDK-4511638), and CI's Windows job runs a pre-19 `java`. | synthetic private static method `__iir_write_f64(DI)V`: `picked.stripTrailingZeros()` gives `D` (`unscaledValue()`) and `e10 = len(D) - 1 - scale()`; fixed layout is `toPlainString()`, sci is built with a `StringBuilder`; one `System.out.print(String)` |
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

The gate `pr6a_ops_are_lowered_on_every_backend` pinned that all nine backends
lower both builtins. It also required both table cases to pass on every
executable column whose toolchain is present. (PR7e replaced it with
`pr7e_float_writers_are_gone_and_the_layouts_run_everywhere`, which runs the
same table through the compilers' helpers.)

Local evidence and its limits are those of PR4: native x86-64, LLVM (clang),
WASM, JVM (JDK 21), VM and JIT execute every case. Beyond the 90-value table,
four implementations were checked on the full prototype corpus (535 432 values
against `python3`, 402 112 against `node`), with no mismatch:

- the native runtime's C (also clean under ASan/UBSan);
- the LLVM IR at `-O0` and `-O2`;
- the emitted WASM module, run on V8;
- the JVM helper on JDK 21. After the fix below it was re-checked with
  `-Xverify:all` on JDK 8, 17 and 21, on a 775 126-value corpus (the one above,
  plus every binade boundary `2^k` and its neighbours, random bit patterns,
  random subnormals and random short decimals), with no mismatch.

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
- *JVM.* `Double.toString` was planned as the whole digit source. The first
  implementation used it, with a one-digit neighbour check because it prints
  at least two digits. CI's Windows job runs the runner's preinstalled `java`,
  which is older than 19. There `Double.toString` is not shortest: it printed
  `1.9999999999999998e+23` where Python and JS print `2e+23`. So the helper
  now finds the digits itself with the `BigDecimal` FLOOR/CEILING search in
  the JVM row above. That search also covers one-digit results, so the
  neighbour check is gone.
- *JIT.* `JITCore`'s optimiser propagates a `const_f64` into the writer's
  operand, so the JIT accepts a float literal there as well as a float
  register.
- *Table size.* The plan named 40 random values. The table has 60: 40 bit
  patterns and 20 short decimals, because short decimals exercise the fixed
  layouts that random bit patterns rarely reach.

## PR6c — IEEE f64 arithmetic and the sign bit

JavaScript numbers are IEEE-754 binary64, and a JavaScript program observes
every IEEE edge: `console.log(1 / 0, -1 / 0, 0 / 0)` prints
`Infinity -Infinity NaN`, and `console.log(-0, 0 * -1)` prints `-0 -0`
(`console.log` formats with `util.inspect`, which keeps the sign of zero;
`String(-0)` is `"0"`). The classic `add`/`sub`/`mul`/`div` cannot carry that
contract: `div` is registered `DIV_ZERO_ONLY` and its zero-divisor edge differs
by backend (see "Legacy edge behaviour"). And nothing in the registry exposes
the sign of a zero: `write_f64_js` is `String(x)` and prints `-0` as `0`, and
the textbook `1 / x < 0` test divides by zero. PR6c adds six type-first ops
whose edges are IEEE's and identical everywhere.

| Op | Operands → result | Semantics |
| --- | --- | --- |
| `f64_add a, b` | `f64, f64 → f64` | IEEE `a + b`, round to nearest, ties to even |
| `f64_sub a, b` | `f64, f64 → f64` | IEEE `a - b` |
| `f64_mul a, b` | `f64, f64 → f64` | IEEE `a × b` |
| `f64_div a, b` | `f64, f64 → f64` | IEEE `a ÷ b`; a zero divisor gives `±Infinity` or `NaN`, never a trap |
| `f64_neg a` | `f64 → f64` | flip the sign bit (so `-0` ↔ `0`, and `-NaN` is a NaN) |
| `f64_signbit a` | `f64 → int` | the sign bit as `0`/`1` |

None of them traps (`may_trap: []`), so they are pure and dead-code
elimination may remove one whose result is unused.

**The IEEE edges every lowering must reproduce:**

| Expression | Result | `f64_signbit` of it |
| --- | --- | --- |
| `f64_div 1, 0` / `f64_div -1, 0` | `+Infinity` / `-Infinity` | `0` / `1` |
| `f64_div 1, -0` | `-Infinity` | `1` |
| `f64_div 0, 0`, `f64_sub inf, inf`, `f64_mul 0, inf` | `NaN` | not specified |
| `f64_add 1e308, 1e308` | `+Infinity` (overflow) | `0` |
| `f64_mul -1, 0` / `f64_mul -1, -0` | `-0` / `0` | `1` / `0` |
| `f64_add -0, -0` / `f64_add -0, 0` | `-0` / `0` | `1` / `0` |
| `f64_sub 0, 0` | `0` | `0` |
| `f64_neg 0` / `f64_neg -0` | `-0` / `0` | `1` / `0` |
| `f64_add 0.1, 0.2` | `0.30000000000000004` | `0` |

**NaN payloads and the sign of a NaN are not observable.** IEEE leaves the
sign of `0/0` to the implementation (x86 SSE produces the "default NaN" with
the sign bit *set*; AArch64 and WASM produce it clear), so `f64_signbit` of a
NaN is unspecified, and every writer prints any NaN as `NaN`. A frontend that
must print a NaN's sign (no current language does) has to test for NaN first.

One frontend leans on NaN signs today. JavaScript's `console.log` writes the
`-` of `-0` when `f64_signbit x > f64_signbit (f64_add x, 0)` (LANG84 6e).
For a NaN `x` that is correct only if `f64_add` keeps the NaN operand's sign.
Every backend's hardware and runtime does that today: x86 SSE, AArch64 with
default-NaN mode off, LLVM, the JVM, the CLR, wasmtime without NaN
canonicalisation, and the VM's Rust `f64`. The `infinities_and_nan` matrix
rows check it on every column that runs `f64_*`. A backend that canonicalises
NaN results could print `-NaN` for `0/0`. Such a backend needs a dedicated
negative-zero test op before it can claim JavaScript.

### Per-backend lowering

| Backend | `f64_add`/`sub`/`mul`/`div` | `f64_neg` | `f64_signbit` |
| --- | --- | --- | --- |
| x86-64 | SSE2 `addsd`/`subsd`/`mulsd`/`divsd` (the existing `f64` binop) | load the bits into `rax`, `xor` with `0x8000_0000_0000_0000` | load the bits into `rax`, `shr rax, 63` |
| AArch64 | `fadd`/`fsub`/`fmul`/`fdiv d0, d0, d1` (the existing `f64` binop) | load the bits into `x0`, `eor` with `0x8000_0000_0000_0000` | load the bits into `x0`, `lsr` by 63 |
| LLVM | `fadd`/`fsub`/`fmul`/`fdiv double` | `fneg double` | `bitcast double to i64`, `lshr i64 …, 63` |
| WASM | `f64.add`/`sub`/`mul`/`div` | `f64.neg` | `i64.reinterpret_f64`, `i64.const 63`, `i64.shr_u` |
| JVM | `dadd`/`dsub`/`dmul`/`ddiv` | `dneg` | `Double.doubleToRawLongBits(D)J`, `bipush 63`, `lushr` |
| CLR (textual) | `add`/`sub`/`mul`/`div` on `float64` | `neg` | `BitConverter::DoubleToInt64Bits(float64)`, `ldc.i4.s 63`, `shr.un` |
| BEAM | **`Refused`** (below) | **`Refused`** | `is_lt x, 0.0` → `1`; else `is_eq_exact x, -0.0` → `1`; else `0` |
| VM | Rust `f64` `+ - * /` on `Value::Float` | Rust `-x` | `x.is_sign_negative()` |
| JIT | `ADD_F64`/`SUB_F64`/`MUL_F64`/`DIV_F64` bytecodes on the registers' bit patterns | `NEG_F64` (`xor` of bit 63) | `SIGNBIT_F64` (`bits >> 63`, unsigned) |

Every operand is read before the destination is written, so a destination
that aliases an operand (`x = f64_add x, y`) is safe everywhere.

**BEAM refuses the arithmetic.** An Erlang float is always finite: `'+'/2`,
`'-'/2`, `'*'/2` and `'/'/2` raise `badarith` instead of producing an
infinity or a NaN, and `0.0 / 0.0` does the same. There is no float term to
hold the result. So BEAM declares the five arithmetic ops
`Refused("BEAM floats have no ±Infinity or NaN …")`, and `lang-aot` rejects a
module that uses them with `UnsupportedOps` *at compile time*, which LANG84's
stage B allows. It does **not** lower them to the classic BIFs and accept a
`badarith` on the edges, because B counts a run-time failure as a failure.
`f64_signbit` needs no non-finite value, so BEAM lowers it with two
comparisons against literal-table floats. It relies on OTP 27, where `-0.0`
and `0.0` are distinct terms (`0.0 =:= -0.0` is `false`); CI runs OTP 27, and
PR6a's `-0.0` output already depends on it.

**The JIT carries floats as bit patterns.** PR6a let one float shape through
the i64-only register file: a `const_f64` or `f64` parameter consumed by a
float writer. PR6c widens that set to the destinations of the five arithmetic
ops: an `f64` register is any `const_f64` destination, `f64` parameter or
`f64_*` arithmetic destination, and it holds the value's bits. The new
bytecodes reinterpret those bits, compute in Rust `f64`, and store the
result's bits back. A float literal that constant propagation folded into an
operand is materialised into a fresh register first, as PR6a does for the
writers. Any other use of an `f64` register still refuses the function, so
integer code can never read float bits as an integer. `f64_signbit` writes an
ordinary integer register. One more bytecode came with it: `RET_F64` returns a
float register as `Value::Float`, so `ret_f64` of a float register compiles
too (before PR6c every function returning an `f64` fell back to the
interpreter, and the JIT's declaration test — one function per registry op
that returns the op's result — could not have compiled `f64_add` without it).

Two smaller implementation notes. On native x86-64 and AArch64, `f64_neg` and
`f64_signbit` also accept a float literal that constant propagation folded
into the operand (it is materialised as its `to_bits()` immediate); an
integer literal is refused rather than read as float bits. On BEAM, the five
refused ops are also listed in `validate_for_beam`'s `UNSUPPORTED_OPS`, so a
direct `iir-to-beam` caller that skips `lang-aot` gets the same refusal.

### Conversions on the JIT

Until this follow-up, the JIT declared `int_to_real`, `real_to_int_trunc` and
`real_to_int_floor` `Refused(INTERPRETER_TIER_ONLY)`. Any function that mixed
an integer with a float therefore ran on the interpreter tier. That covered
Python's `1 + 2.5`, Dartmouth BASIC's `INT`, and ALGOL's `entier`. All three
fit the bit-pattern scheme above:

| Op | Bytecode | Source | Destination | Semantics |
| --- | --- | --- | --- | --- |
| `int_to_real` | `I64_TO_F64 <dst> <a>` | an integer register, or an integer literal that constant propagation folded in | a float register | Rust `a as f64`: round to nearest, ties to even. It is exact for \|a\| ≤ 2⁵³, and `2⁵³ + 1` becomes `2⁵³`, as on every other backend (`cvtsi2sd`, `scvtf`, `sitofp`, `f64.convert_i64_s`, `l2d`, `conv.r8`) |
| `real_to_int_trunc` | `F64_TO_I64_TRUNC <dst> <a>` | a float register, or a folded float literal | an integer register | `t = x.trunc()`, then the VM's checked narrowing |
| `real_to_int_floor` | `F64_TO_I64_FLOOR <dst> <a>` | the same | an integer register | `t = x.floor()`, then the same check |

Because `int_to_real` writes a float register, its destination joins the
JIT's set of `f64` registers. Each of the two narrowing ops writes an
ordinary integer register. So a name that some op also writes as a float
refuses the function, the rule `f64_signbit` already follows. A source of
the wrong class also refuses the function: a float register or float literal
given to `int_to_real`, or an integer given to a narrowing op. No float bits
are ever read as an integer, and no integer is ever read as float bits.

**An integer register must hold an integer.** The class rules above cover
values the function computes. Values that enter from outside are checked when
they arrive:
- Every parameter has a checked prologue entry: `PARAM_I64 <reg>` for a
  non-`f64` parameter, `PARAM_F64 <reg>` for an `f64` one.
  - A `PARAM_I64` argument must be `Int` or `Bool`. Anything else fails the
    call before the body runs. A missing argument leaves the register 0.
  - Arguments beyond the declared parameters are ignored, as the VM ignores
    them.
- A `call_builtin` result stored in a register must be `Int`, `Bool`, or a
  void `Null` (stored as 0).

Both used to collapse to 0, so `int_to_real` of a parameter that received
`Float(2.5)` returned `0.0` where the VM raises "operand is not an integer".
It now fails closed. An `f64` parameter is still checked by its `PARAM_F64`
prologue instruction.

**The narrowing failure matches the VM, not `Trap.ConvRange`.** vm-core's
`real_to_i64_checked` rejects a non-finite `t` with
`VMError::Custom("real_to_int: operand is not finite")`. It rejects a `t`
outside `[-2⁶³, 2⁶³)` with `VMError::Custom("real_to_int: {t} is outside the
i64 range")`. The JIT writes exactly that text to its error slot and stops,
and `error_from_message` passes it through unchanged as the same
`VMError::Custom`. This way the two tiers fail identically, which is what the
JIT's declaration rule requires.

The registry table above names `Trap.ConvRange` (id 4) for this case. No
backend reports it today:
- the VM raises the custom error above;
- native x86-64 executes `ud2` and AArch64 executes `udf`;
- the CLR throws `OverflowException` from `conv.ovf.i8`.

Moving every backend to the typed trap is a separate change. The JIT follows
the VM until that change lands.

### Conformance cases

`op_conformance` gains three cases:

- **Edges.** Every row of the edge table above (plus `f64_neg -inf`), each
  result printed with `write_f64_js` and — except for the NaN rows, whose
  sign is unspecified — its sign with `write_i64`: on the eight IEEE columns
  this checks `Infinity`, `-Infinity`, `NaN`, signed zeros and overflow. BEAM
  is covered by `refused_ops_are_rejected_at_compile_time`. (Since PR7e,
  which removed `write_f64_js`, each result is printed as `f64_class`,
  `f64_digits` and `f64_exp10` instead; the aliasing case likewise.)
- **Sign bit.** `f64_signbit` of `-0`, `0`, `-2.5`, `2.5`, the smallest
  subnormals of both signs and `±1.7976931348623157e308`, built from constants
  only, so the case is lowered on all nine backends (BEAM included).
- **Aliasing.** `x = f64_add x, x`, then `f64_mul`, `f64_sub`, `f64_neg` and
  `f64_div` all with `x` as both destination and operand, so a lowering that
  writes the destination before reading an operand fails.

The gate `pr6c_ops_follow_the_ieee_contract` pins the support matrix (all six
lowered on the eight IEEE backends; the five arithmetic ops refused and
`f64_signbit` lowered on BEAM), requires all three cases to pass on every
executable column whose toolchain is present, and checks the BEAM compile
entry point: the two arithmetic cases are refused with `UnsupportedOps`, the
sign-bit case compiles.

Local evidence and its limits are those of PR6a: native x86-64, LLVM (clang),
WASM, JVM (JDK 21), VM and JIT execute every case; CLR, BEAM and AArch64 are
structural here (pinned IL text, BEAM instruction streams and AArch64 words)
and execute in CI once LANG84 6e's JavaScript rows emit these ops.

## PR7 — language neutrality: the IIR names no source language

### The principle

The IIR is a low-level, language-neutral machine. **No registry op, builtin
or trap kind may carry the semantics of one source language.** That rules
out a language's name (`write_f64_py`), a language's layout rule (Python's
`repr`), a language's coercion (Lisp truthiness) and a language's error
class (`ReferenceError`). The **IIR compiler** of each language turns source
semantics into neutral ops. When the translation is long, as number
formatting is, the compiler emits a **helper function into the module it
produces**. Nothing is shared at run time: there is no portable language
runtime, and no backend contains a line of code that knows which language it
is compiling.

Dartmouth BASIC already works this way. `portable_numeric_print_helpers()` in
`dartmouth-basic-iir-compiler` emits `__basic_print_uint`, `__basic_print_int`
and `__basic_print_real` into every module that prints. The helpers are built
only from `const`, `cmp_*`, `add`/`sub`/`mul`/`div`, `int_to_real`,
`real_to_int_trunc`, `jmp`/`jmp_if_false`/`label`, `call` and `putchar`, so
BASIC's `PRINT` layout runs on all nine backends with no backend support. PR7
makes that the rule for every frontend.

What counts as language-neutral:

- An op whose semantics a hardware manual, IEEE 754 or a libc page states
  without naming a language: `i64_div_floor`, `f64_signbit`, `f64_sqrt`,
  `putchar`. Floor division is Python's `//`, but it is also a mathematical
  operation with one definition, and several languages share it.
- A trap kind that names a machine-level condition (a divisor is zero, a name
  is unbound). It never names a language's exception class.
- An op whose name tells where it came from (`introduced_by: "ALGOL AL8"` on
  `f64_sin`) is fine. The **semantics** are what must be neutral, not the
  history.

**Relation to "no portable runtime".** "Why this exists" rejected IIR helper
functions. PR7's helpers are a different thing:

- **Not shared.** Each compiler emits its own, and no library links them.
- **Stand in for no op.** Each one composes registry ops that every backend
  already lowers, the way the code for an `if` does. A layout helper is the
  compiler translating `print(x)`, just as BASIC's `PRINT` already is.
- **Need no backend change.** A backend never compiles them as a special
  case.

The line between the two is **cost of definition, not cost of code**. The
shortest-digits search is a bignum algorithm with one universal definition,
so it stays an op. Choosing where the decimal point goes is a language's
rule, so it moves to the compiler.

**Alternative considered: a parameterised layout op.** A neutral
`write_f64_layout x, fixed_lo, fixed_hi, exp_min_digits, flags` would keep
the module smaller. It was rejected for three reasons:

- Each new language adds a knob to all nine backends. Ruby needs `1.0e+16`'s
  forced `.0`, and `%g` strips zeros differently.
- The knobs are language rules written as numbers.
- Python's and JavaScript's needs already differ in four places (fixed range,
  exponent padding, integral `.0`, `-0`).

With helpers, a new language's float output changes no backend at all.

### Audit (main at d1730421d3)

| Item | Where | Emitted by | Verdict and replacement |
| --- | --- | --- | --- |
| `write_f64_py` | `op_registry.rs` (PR6a, Builtin); lowered by all nine backends: `vm_core::float_format::python_repr`, jit-core `CALL_BUILTIN_F64`, `twig_runtime.c` `__twig_write_f64_py`, iir-to-llvm / iir-to-wasm `lang83_f64.rs` / iir-to-jvm-class-file / iir-to-cil-bytecode / iir-to-beam `__iir_write_f64(x, layout)` | python-iir-compiler (release path) | **Language-specific** (Python `repr` layout). Replaced by `f64_class` + `f64_digits` + `f64_exp10` and a compiler-emitted helper (below). *Removed in PR7e* (registry, all nine backends, vm-core's default builtins, the harness sinks). |
| `write_f64_js` | same, layout 1 | javascript-iir-compiler 0.6 | **Language-specific** (`Number::toString` layout). Same replacement. *Removed in PR7e.* |
| `write_f64_rb`, `f64_to_str_py` `_js` `_rb` `_pl` | vocabulary table, `Later` | none | **Language-specific by design.** Never registered. Ruby `Float#to_s` uses the same three ops. Perl `%.15g` and Python 2 / 3.0a1 `%.12g` use `f64_digits_prec` (below, `Later`). String-producing formatting becomes a helper that appends to a runtime string once a neutral byte-append string op exists (PR5 family). |
| `Trap.ReferenceError` (id 7) | `exception_kind.rs`, `twig_runtime.c` kind table, this spec | javascript-iir-compiler | **Language-named.** The condition, reading an unbound name, is neutral (Python `NameError`, a Lisp unbound variable). Renamed `Trap.Unbound`, keeping **id 7**, in PR7d. *Landed.* |
| `LANGUAGE_TRAP_KINDS` (empty) | `exception_kind.rs`, "Trap semantics" above, LANG84 C7.4 | none | **A language-specific mechanism** (`Ruby.ZeroDivisionError`). Retired in PR7d. A compiler maps a language's error onto a neutral kind. A catchable language exception is a value the frontend builds once AOT00 T2 exceptions land; it is never an IIR trap kind. *Landed:* the table, `LANGUAGE_TRAP_KIND_ID_BASE` and `is_registered_trap_kind` are gone. |
| `py_float_div`, `py_name_error`, `py_print_float`, `py_print_empty`, `py_print_two_floats`, `py_print_three_floats` | `python-iir-compiler/src/lib.rs`, registered on the VM only by the LANG79 pilot's `compile_source` | the LANG79 pilot path | **Language-specific**, unregistered, VM only. Retired in PR7g: the pilot entry points move onto the release path (`f64_div` with a `b == 0` check that traps `Trap.DivZero`; `trap Trap.Unbound`; the PR7b print helper). |
| `dyn_truthy`, `dyn_equal`, `dyn_not`, `dyn_to_exit_code` | `iir-builtin-lowering/src/dyn_repr.rs`, lispy-runtime | Twig, McCarthy Lisp (through the `+`/`cons`/`null?` lowering) | **Language-specific semantics in a runtime op.** Lisp truthiness (only nil and `#f` are false), `equal?`'s structural equality and Lisp's exit-code coercion are each one language family's rules. Replaced in PR7f by neutral representation primitives plus compiler-emitted helpers (below). *Landed in PR7f-2:* truthiness and `not` inline, `__lisp_equal` and `__lisp_exit_code` module functions; the four are gone from the runtime and every backend (see "What PR7f-2 landed"). |
| `dyn_cons`, `dyn_car`, `dyn_cdr`, `dyn_pair_p`, `dyn_null_p`, `make_nil`, `make_symbol` | `dyn_repr.rs`, `heap.rs`, lispy-runtime | Twig, McCarthy Lisp | **Neutral semantics, Lisp vocabulary.** A two-field heap cell and a nil sentinel are a representation, not a language rule. Renamed in PR7f to `dyn_pair_new`, `dyn_pair_first`, `dyn_pair_rest`, `dyn_is_pair`, `dyn_is_nil`, `dyn_nil` and `dyn_symbol`, then registered. *Landed in PR7f-1,* with the new `dyn_tag` and the existing `dyn_box_int` / `dyn_unbox_int` (see "What PR7f-1 landed"). |
| Source-level builtin names `+ - * / % = < <= > >= cons car cdr pair? null? equal? not` | emitted as `call_builtin` by twig-ir-compiler, mccarthy-lisp, and the seven CAS frontends (axiom, maple, macsyma, derive, reduce, wolfram, maxima); rewritten by the shared `iir-builtin-lowering` pass | as listed | **Source vocabulary reaching the IIR.** The module is not neutral until that pass runs, and the pass's `is_lisp_function_call` and `rename_lisp_not` know Lisp. PR7f moves the translation into the frontends: the shared pass becomes a compiler-side library those frontends call, and the module they hand to a backend holds registry ops only. |
| `print_i64`, `putchar`, `getchar`, `print_str`, `write_i64`, `str_*`, `i64_*`, `cmp_*`, `f64_*` arithmetic and math, `int_to_real`, `real_to_int_*`, `trap`, `Trap.Bounds` … `Trap.User` | registry | many | **Neutral.** They stay. `print_i64` appends `\n`, a convenience every integer-printing frontend uses; it is not a language rule. |
| `input_i64`, `input_more`, `input_str`, `getchar` | lang-aot harnesses, flow-matic and brainfuck compilers | flow-matic, brainfuck | **Neutral but unregistered.** Registered with honest `op_support` in PR7h. |
| `print`, `double`, `noop`, `assert_eq` | `vm-core/src/builtins.rs` defaults | tests and doc examples | Neutral test fixtures. Out of scope: no frontend emits them and the registry does not gate them. |
| `__twig_write_i64`, `__twig_trap`, `__twig_write_f64_*` | native runtime symbols in `twig_runtime.c` | backends, not frontends | A historical **symbol** prefix. The runtime is language-neutral. `__twig_write_f64_*` disappears in PR7e (*landed*), and renaming the rest is cosmetic. |

### The neutral float primitives

Every PR6a backend already computes the shortest digits `D` and the
leading-digit exponent `e10`, and only then applies a layout ("The host
formatters fix only the digits"). PR7 cuts each writer at that boundary.
The digits become three registry ops, and the layout moves into the
compilers.

| Name | Kind | Signature | Semantics |
| --- | --- | --- | --- |
| `f64_class` | Op | `f64 → i64` | `0` for `±0`, `1` for a finite non-zero value (subnormals included), `2` for `±Infinity`, `3` for NaN. Never traps. With `f64_signbit` it identifies every special value. It replaces the planned `f64_is_nan` (`f64_class x == 3`). |
| `f64_digits` | Op | `f64 → i64` | For class 1, the shortest round-trip decimal significand `D` of `|x|` as an integer with no trailing zeros: shortest, then closest, ties to even, exactly PR6a's rule. `1 ≤ D < 10^17`. For classes 0, 2 and 3 the result is `0`. The sign is not part of the result; use `f64_signbit`. |
| `f64_exp10` | Op | `f64 → i64` | For class 1, the exponent `e10` of `D`'s leading digit: `|x| = D × 10^(e10 − n + 1)`, where `n` is the number of digits of `D`. So `1.5` gives `D = 15`, `e10 = 0`; `5e-324` gives `D = 5`, `e10 = -324`; `1e21` gives `D = 1`, `e10 = 21`. For classes 0, 2 and 3 the result is `0`. Range `-324 ≤ e10 ≤ 308`. |
| `f64_digits_prec` | Op | `(f64, i64 p) → i64` | `Later`. `|x|` correctly rounded to `p` significant digits (`1 ≤ p ≤ 17`), round-half-even on the exact binary value, trailing zeros stripped. `f64_exp10_prec` is its exponent twin. These serve `%.12g` (Python 2, 3.0a1) and `%.15g` (Perl). They enter the registry with the first frontend that needs them. |

They are **Ops, not Builtins**: each is pure, writes nothing and never traps.
`f64_digits` and `f64_exp10` each run the digit search on its own. A backend
may compute both in one helper and remember the last argument. The
conformance cases test only the values, never how often the search runs.

**Per-backend lowering** (each reuses its PR6a core):

| Backend | `f64_class` | `f64_digits` / `f64_exp10` |
| --- | --- | --- |
| x86-64 / AArch64 | inline bit test on the raw pattern in a GPR (exponent field all ones → 2 or 3, all zero with zero fraction → 0) | `__twig_f64_digits(bits)` / `__twig_f64_exp10(bits)` in the native runtime: the C core, minus the layout. The runtime's own `__twig_` namespace: an `__iir_` name would collide with the LLVM module's core (see below). |
| LLVM | inline `bitcast` + integer compares | the existing `@__iir_f64_digits` core, returning `D` and `k − 1` through two wrappers (`@__iir_f64_shortest_digits` / `@__iir_f64_shortest_exp10`); every float helper has `internal` linkage |
| WASM | inline `i64.reinterpret_f64` + compares | the existing in-module core, two in-module `(f64) -> i64` wrappers |
| JVM | `Double.doubleToRawLongBits` + compares | the existing `BigDecimal` search, now its own method `__iir_f64_shortest`: `unscaledValue().longValue()` and `len − 1 − scale()` |
| CLR | `BitConverter.DoubleToInt64Bits` + compares | the existing `"R"` parse, minus the layout, now its own method `__iir_f64_shortest(float64, int32&)` |
| BEAM | finite only, so `0` for `±0.0` (two `is_eq_exact` tests against the literal-table zeros), else `1`. Classes 2 and 3 cannot occur; the validator already refuses non-finite constants. | the writer's `float_to_list(X, [short])` parse, now its own function `'__iir_f64_shortest'(X) -> [Neg, E \| D]`, returning `D` (`list_to_integer`) or `e10`, or `0` when no digits survive |
| VM | Rust `f64::classify` | `vm_core::float_format::shortest_digits(x) -> (u64, i32)`, split out of `python_repr` / `js_to_string` |
| JIT | new `CLASS_F64` bytecode, float register → integer register, shaped like `SIGNBIT_F64` | new `DIGITS_F64` / `EXP10_F64` bytecodes that call the same Rust core. After PR7e the JIT needs no `CALL_BUILTIN_F64` for output, because the layout helper is integer code. Where the JIT refuses a helper's `call`, that function falls back to the interpreter, as it does today. |

On BEAM all three are lowerable, unlike the PR6c arithmetic. A JavaScript
module whose numbers are all literals already reaches BEAM, and it keeps
doing so.

**What PR7b landed, and where it diverged from the plan above.** All three
ops are registered (`introduced_by: "LANG83 PR7b"`, family `Float`, integer
result like `f64_signbit`) and lowered on all nine backends. Every backend's
PR6a digit core now serves both the writers and the new ops, and the
writers' output is byte-identical (every PR6a table and model test passes
unchanged).

- *Shared cores.* Where the PR6a digits lived inside the writer, they moved
  into a helper of their own that the writer and the ops call: JVM
  `__iir_f64_shortest(D)Ljava/math/BigDecimal;`, CLR
  `string __iir_f64_shortest(float64, int32&)` (the exponent comes back
  through the managed pointer), BEAM `'__iir_f64_shortest'/1` returning
  `[Neg, E | D]`. LLVM and WASM split their module-local code into a
  computation part (big integers, core, the two wrappers) and an output part
  (layout, `putchar`), so a module that asks for digits but prints no float
  carries no output code and, on WASM, imports no `env.putchar`. The native
  runtime adds `__twig_f64_digits` / `__twig_f64_exp10` beside the writers.
  VM: `vm_core::float_format::shortest_digits` now returns `(u64, i32)`; the
  string form is private.
- *WASM wrappers* are in-module functions, not exports (nothing outside the
  module calls them).
- *Names and linkage (security review).* The native runtime helpers were
  first named `__iir_f64_digits` / `__iir_f64_exp10`. The LLVM module's own
  3-argument digit core is also `@__iir_f64_digits`, and PR6a gave it
  `linkonce_odr` linkage, which a strong definition can interpose on. Any LLVM
  program that also calls a `@__twig_*` helper links `twig_runtime.c`, so the
  C function silently replaced the core: `write_f64_py 0.1` printed `0.0` and
  `123.456` crashed. The fix has two parts. The runtime helpers are now
  `__twig_f64_*`. Every LLVM float helper (`lang83_f64.rs`) is now
  `define internal` and its constants `private`, so no linked object can
  replace them; each module carries its own copy. An `op_conformance` case
  combines a string op (which links the runtime on the LLVM column) with the
  writers and the digit ops on every column, and
  `llvm_float_helpers_cannot_be_interposed_by_the_native_runtime` pins the
  linkage and the naming rule. PR1b's `@__iir_write_i64` / `@__iir_trap`
  stay `linkonce_odr`: the runtime defines neither name.
- *BEAM* parses the signed value (as the writer always has) rather than
  `abs(X)`: the sign is peeled off before the digits, so `D` and `e10` are the
  same.
- *JIT* opcodes: `CLASS_F64` 0x5D, `DIGITS_F64` 0x5E, `EXP10_F64` 0x5F, so
  that 0x59–0x5C stay free for an in-flight branch's int⇄float bytecodes.
- *`f64_class` on the integer side* is branch-free everywhere it has
  compares that yield 0/1: `(m ≠ 0) + (m ≥ 0x7FF0…0) + (m > 0x7FF0…0)` on the
  magnitude `m` (x86-64 `setcc`, AArch64 `cset`, LLVM `icmp`/`zext`, WASM,
  CLR `cgt.un`), and on the JVM `lcmp(m, 0) + lcmp(m, 0x7FF0…0) + 1`.
- *JVM wide model.* `lang-aot`'s JVM scalar concretizer now treats every
  registry Op with an `f64` operand as wide-model evidence (next to the
  `i64` ones), so a module whose only real is a literal propagated into
  `f64_digits` (up to 10^17) is not narrowed to `int`.
- *Conformance.* PR6a has one 90-value table with a Python and a JavaScript
  column; PR7b's `FLOAT_DIGITS_TABLE` gives `(class, D, e10)` for the same
  values, generated by host `python3` and re-derived by
  `float_digits_table_matches_host_python3`. The cases are one per op over
  the 87 finite values (all nine backends) plus one case for `±Infinity` and
  NaN (the eight IEEE backends), each value printed with `write_i64`. Each
  table case reuses one result variable for every row, because the JIT's
  256-register file cannot hold 87 results plus the literals constant
  propagation materialises. The gate `pr7b_ops_are_lowered_on_every_backend`
  pins the matrix, requires the four cases on every executable column whose
  toolchain is present (WASM, VM, JIT, LLVM, native x86-64 and JVM locally),
  and checks that BEAM compiles the finite cases and refuses the non-finite
  one. CLR, BEAM and AArch64 are structural locally, as in PR6a: the CLR IL
  model and the BEAM opcode-and-BIF model now follow calls between the
  helpers and run the three ops over the table in every host layout.

### The layouts, as compiler-emitted helpers

Each compiler emits **one helper function per layout**, only into modules
that print a float, and calls it from every print site. It does not inline
the layout. The layout is 80–150 instructions, and inlining it at every
`print` would multiply code size without saving anything a backend cannot
already get from a direct `call`. The helper's name is module-local and
belongs to the compiler (for example `__py_write_float`, `__js_write_number`).
The no-language-names rule applies to registry ops, builtins and trap kinds,
not to functions a compiler defines.

The helpers use only integer ops (`div`/`mod` on non-negative operands with
non-zero power-of-ten divisors, so the legacy edges never arise), `cmp_*`,
`jmp`/`jmp_if_false`/`label`, `call` and `putchar`. They share a digit
writer that both compilers emit:

```text
fn write_digits(D, n, from, to):    # write D's digits from..to-1 (0 = leading); positions ≥ n are '0'
    for i in from..to: putchar('0' + (i < n ? (D div 10^(n-1-i)) mod 10 : 0))
fn digit_count(D): n = 1; while D >= 10^n: n += 1; return n
fn write_uint(u): …                 # BASIC's __basic_print_uint, unchanged
```

**Python `repr(float)`** (what `print` writes). This is the PR6a
`write_f64_py` row, moved into python-iir-compiler:

```text
c = f64_class x;  s = f64_signbit x
c == 3: "nan"                          c == 2: s ? "-inf" : "inf"
c == 0: s ? "-0.0" : "0.0"
D = f64_digits x;  e = f64_exp10 x;  n = digit_count(D);  if s: '-'
if -4 <= e < 16:                       # fixed
    if e >= 0:  write_digits(D, n, 0, e+1); '.'; if n > e+1: write_digits(D, n, e+1, n) else '0'
    else:       "0."; (-e-1) × '0'; write_digits(D, n, 0, n)
else:                                  # scientific
    write_digits(D, n, 0, 1); if n > 1: '.'; write_digits(D, n, 1, n)
    'e'; e < 0 ? '-' : '+'; if |e| < 10: '0'; write_uint(|e|)
```

**JavaScript `Number::toString`**, moved into javascript-iir-compiler:

```text
c == 3: "NaN"                          c == 2: s ? "-Infinity" : "Infinity"
c == 0: "0"                            # String(-0) is "0"
D, e, n as above;  if s: '-'
if -7 < e < 21:
    if e >= n-1: write_digits(D, n, 0, e+1)           # integral: no ".0"; zeros past n
    elif e >= 0: write_digits(D, n, 0, e+1); '.'; write_digits(D, n, e+1, n)
    else:        "0."; (-e-1) × '0'; write_digits(D, n, 0, n)
else:
    write_digits(D, n, 0, 1); if n > 1: '.'; write_digits(D, n, 1, n)
    'e'; e < 0 ? '-' : '+'; write_uint(|e|)           # no padding
```

**`console.log(-0)` writes `-0`.** At each `console.log` argument the
compiler emits `if f64_class x == 0 and f64_signbit x == 1: '-'` before the
call. This replaces the 6e test `f64_signbit x > f64_signbit (x + 0)`. That
test needed `f64_add`, which BEAM refuses, and it relied on `f64_add` keeping
a NaN operand's sign. The class test needs neither, so `console.log(x)` with a
computed `x` lowers on BEAM too.

Ruby `Float#to_s` (fixed when `-4 <= e < 16`, else `1.0e+16`) becomes a
third helper in the Ruby compiler when Ruby gains floats. No backend changes.

**What PR7c-js landed, and where it diverged from the plan above.**
`javascript-iir-compiler` 0.7.0 emits no `write_f64_js`. A module that
prints a number carries `__js_write_number(x: f64) -> i64` and the three
shared helpers, as `__js_digit_count(D)`, `__js_write_digits(D, n, from, to)`
and `__js_write_uint(u)`, written in the crate's `number_to_string` module.
Each `console.log` argument is one `call __js_write_number`; a module that
prints no number (only `console.log()`, or a trap before the first print)
carries none of them. All 38 `lang_matrix` JavaScript rows print what they
printed before on every column that ran (native x86-64, LLVM, WASM, JVM, VM
and JIT locally, 228 cells, matching host node v22), and a crate test
compares 1,200 values, each positive and negative, with vm-core's PR6a
formatter byte for byte.

- *The `-0` test is one compare.* `f64_class x == 0 and f64_signbit x == 1`
  is emitted as `cmp_gt (f64_signbit x) (f64_class x)`. The sign bit is 0 or
  1 and the class is 0 to 3, so `s > c` holds exactly when `s = 1` and
  `c = 0`. A literal, or a binding initialized straight from one, still
  skips the test, as in 6e.
- *`q − (q div 10) × 10`, not `q mod 10`.* The values agree for `q ≥ 0`.
  The generic JIT has no `mod` bytecode, so with `mod` it would refuse the
  digit writer. As written, it compiles.
- *The run of zeros after `0.`* is `write_digits(D, n, n, n − e − 1)`.
  Positions past `D`'s end are `'0'`, so no separate loop is needed.
- *`write_uint`* is BASIC's recursive `__basic_print_uint`, unchanged.
  `digit_count` stops at `p = 10^17`, since `D < 10^17`.
- *Typing.* The helpers are marked `FullyTyped`, like BASIC's. Every
  instruction has an `i64`, `f64` or `void` hint, and each helper returns
  `i64` from one `ret`.
- *Which rows BEAM runs did not change.* The subset has no computed value
  without arithmetic: a value is a literal, a binding of one, or an `f64_*`
  result. So the same 11 literal-only rows list BEAM, and the other 27 are
  refused at compile time. The refusal now names only the five arithmetic
  ops, never `f64_class`, `f64_signbit`, `f64_digits` or `f64_exp10`. The new
  `javascript_prints_lower_on_beam_once_the_arithmetic_is_gone` replaces each
  row's arithmetic with constants and lowers every module to BEAM, the sign
  tests and the helpers included.
- *JIT.* `main`, `__js_write_number` and `__js_write_uint` contain a `call`,
  which the generic JIT has no bytecode for, so they run on the interpreter
  tier. `__js_digit_count` and `__js_write_digits` compile, and
  `execute_with_jit` compiles them eagerly. The 6e test that every row's
  `main` compiles is replaced by
  `javascript_rows_run_with_their_call_free_number_helpers_compiled_by_the_jit`,
  which pins that split and checks that the compiled tier writes the
  significant digits.
- `write_f64_js` stays registered and lowered everywhere, and the `lang_matrix`
  VM and JIT harnesses keep their sinks. They are removed in PR7e.
  `run_module` registers only `putchar`.
- `release_classes.csv` is regenerated. The classes partition the editions
  as before: es2016–es2020 and es2021–es2025.

**What PR7c-py landed, and where it diverged from the plan above.**
`python-iir-compiler` 0.4.0 emits no `write_f64_py` on the release path. A
module that prints a float carries `__py_write_float(x: f64) -> i64` and
the three shared helpers, as `__py_digit_count(D)`,
`__py_write_digits(D, n, from, to)` and `__py_write_uint(u)`, written in
the crate's `float_repr` module. Every float `print` is one
`call __py_write_float` followed by `putchar 10`. All PR7c-js precedents
hold: the digit is `q − (q div 10) × 10`, the run of zeros after `0.` is
`write_digits(D, n, n, n − e − 1)`, each helper returns `i64` from one
`ret` and is `FullyTyped`. All 138 `lang_matrix` Python rows print what
they printed before on every column that ran (native x86-64, LLVM, WASM,
JVM, VM and JIT locally: 828 cells, 372 trapping as expected), and a crate
test compares 1,248 values, each positive and negative, with vm-core's
PR6a `python_repr` and with host `python3` byte for byte.

- *No call-site sign test.* Python's `repr` prints the sign of every zero
  (`-0.0`), so the helper writes it (the `c == 0` branch above), and a
  print site is the `call` alone. Only JavaScript's `console.log` needs the
  class-and-sign test.
- *One result register.* Every `call __py_write_float` writes its unused
  `0` into the same register, `py_repr`, so a float print costs `main` no
  VM register. vm-core's frame holds at most 65,536 named registers
  (`VMFrame::MAX_FRAME_REGISTERS`); a fresh register per call would have
  shrunk the largest float program vm-core can run by a quarter.
- *Python 2 and 3.0a1 are untouched.* Their profiles (`py2`, `py30`) have
  `Floats::None`: a `FLOAT` token and a true `/` are refused, so their
  modules never print a float and never carry the helpers. Their `%.12g`
  waits for `f64_digits_prec`, as planned. Every `py-int-print` and
  `py2-int-print` module is byte-identical to before, and the release
  sweep confirms it: only the 77 `py-float-print` digests in
  `code/grammars/python/release_classes.csv` changed, and the classes
  partition the releases as before ({3.6.0, 3.12.3, 3.13.16, 3.11.17,
  3.16.0a1} and {3.5.10}).
- *The step budget is re-derived.* A print went from one VM step to up to
  700 (`-1.7976931348623157e+308`; `1.5` takes 134). The release runner's
  flat budget of `MAX_IIR_INSTRUCTIONS` (1M), sized when a print was one
  step, would have refused the 2,048-line 64 KiB program of
  `print(-1.2345678901234567e-308)` that the default frontend budgets
  accept (1,423,362 steps). The budget is now derived from the module
  run: `len(main) + 1 + (calls of __py_write_float in main) × 1,024`,
  capped at `MAX_IIR_INSTRUCTIONS × (1 + 1,024 / 4)`. It is sound for
  the compiler's output because `main` is straight-line and
  `MAX_STEPS_PER_FLOAT_PRINT` (1,024) bounds one print. The security
  review rejected the first version, a flat 257M ceiling: a hand-built
  `label L; jmp L` handed to the public `run_module` spun for about 26 s
  in a release build. Now it stops after 3 steps. A test measures the
  dearest print over the sweep and pins it between half the bound and
  the bound; another runs the largest program the default budgets accept,
  and a trusted-budget one at vm-core's register limit; a third hands
  `run_module` looping modules and requires the step-limit error from a
  budget of their own size.
- *Which rows BEAM runs did not change.* The five literal-only float rows
  still list all eight columns and lower on BEAM, helpers included; the 33
  others are refused at compile time naming only the five `f64_*`
  arithmetic ops. `python_float_prints_lower_on_beam_once_the_arithmetic_is_gone`
  replaces each float row's arithmetic with constants and lowers every
  module to BEAM.
- *JIT.* As for JavaScript, `main`, `__py_write_float` and
  `__py_write_uint` contain a `call` and run on the interpreter tier;
  `__py_digit_count` and `__py_write_digits` compile.
  `python_float_rows_run_with_their_call_free_float_helpers_compiled_by_the_jit`
  pins that split for every float row and checks that the compiled tier
  writes the significant digits.
- `write_f64_py` stays registered and lowered everywhere, and the
  `lang_matrix` VM and JIT harnesses keep their sinks; PR7e removes them.
  `run_module` registers only `print_i64` and `putchar`. The LANG79 pilot
  (`compile_source`, `py_print_float`) is PR7g's.

**Conformance.** PR6a's 90-value table (its Python and JavaScript
columns) stays the oracle. The PR7b
`op_conformance` cases print `f64_class`, `f64_digits` and `f64_exp10` of
each table value with `write_i64`, against expectations generated from host
`python3` (`repr` split at the digits). Each frontend's `lang_matrix` rows
then show that its helper reproduces the PR6a output byte for byte on every
column.

### Other replacements

- **`Trap.Unbound`, id 7.** The kind table's row changes its name, not its
  id. Native code passes ids, so `twig_runtime.c` changes one string.
  BEAM's reason atom, the JVM/CLR exception message and the VM's
  `VMError::Trap { kind }` all print the new name, so rows that expect
  `trap: Trap.ReferenceError` change in the same PR. The JavaScript
  frontend's `run_source` keeps recording V8's message
  (`ReferenceError: z is not defined`). That message is the compiler's
  knowledge, and the IIR never carries it. *Landed in PR7d* (below).
- **Lisp-family dynamic values (PR7f).** The registry gains the
  representation primitives only:
  - `dyn_box_int`, `dyn_unbox_int`, `dyn_tag x → i64` (the NaN-box tag
    number);
  - `dyn_pair_new`, `dyn_pair_first`, `dyn_pair_rest`, `dyn_is_pair`,
    `dyn_is_nil`;
  - `dyn_nil`, `dyn_symbol`.

  Each Lisp-family compiler emits its own module-local helpers for the
  semantic operations. `truthy` is `not (dyn_is_nil x or x is #f)`; `equal?`
  is a recursive walk over `dyn_tag`, `dyn_pair_first`/`rest` and `cmp_eq`
  on raw bits; `not`; and the exit-code coercion. A JavaScript or Python
  frontend that adopts dynamic values later writes its own truthiness the
  same way, from the same primitives.
- **Source-level builtin names** stop reaching the IIR. `iir-builtin-lowering`
  becomes a compiler-side library. The Twig, McCarthy Lisp and CAS compilers
  call it before they return the module, so `check_module_for` sees
  registry ops only. The LANG84 C7.1 clause "a pre-registry instruction
  that the registry does not gate" shrinks accordingly.

**What PR7d landed, and where it diverged from the plan above.**

- `interpreter_ir::exception_kind::TRAP_REFERENCE_ERROR` is now
  `TRAP_UNBOUND` (`"Trap.Unbound"`), still the seventh entry of
  `ALL_TRAP_KINDS` and so still id 7. `"Trap.ReferenceError"` is no longer
  a kind: `is_trap_kind` and `trap_kind_id` refuse it.
- `LANGUAGE_TRAP_KINDS` and `LANGUAGE_TRAP_KIND_ID_BASE` are removed, and
  with them `is_registered_trap_kind`, which had become `is_trap_kind`
  under another name. *Divergence:* the plan named only the table. The
  function went too, so that no API is left that reads as if a second,
  language-owned set of kinds could exist. Its six callers (vm-core's
  `trap` handler, the CLR, BEAM, JVM and WASM `trap` lowerings, and the
  registry's `trap_kinds_are_registered` test) call `is_trap_kind`, which
  accepts the same set; their error messages are unchanged.
  `trap_kind_name` returns `None` for every id outside 1–7, including the
  retired 1000+ range.
- Each backend prints the kind through the shared table, so no backend
  source changed for the rename itself: the LLVM `@__iir_trap_msg.7`
  string, the JIT error slot, the JVM and CLR exception messages, the
  BEAM reason atom and the VM's `VMError::Trap { kind }` all read
  `Trap.Unbound`. Native and WASM code pass id 7, unchanged;
  `twig_runtime.c`'s `TWIG_TRAP_KINDS[6]` changes its string, and
  `runtime_trap_kind_table_matches_exception_kind` pins it (it no longer
  checks the language table, which is gone).
- javascript-iir-compiler's `REFERENCE_ERROR_TRAP` is `UNBOUND_TRAP`.
  `run_source` still reports V8's sentences
  (`ReferenceError: z is not defined`,
  `ReferenceError: Cannot access 'a' before initialization`), mapping
  `trap: Trap.Unbound` back to the message it recorded; `run_module`
  alone reports `trap: Trap.Unbound`. The two `lang_matrix` trap programs
  expect `StdoutThenTrap("1", "Trap.Unbound")` on every column.
- Kept as `ReferenceError`, because they are JavaScript-language text,
  not trap kinds: V8's messages and the compiler's comments about
  JavaScript semantics, `ReferenceError` in the JavaScript compiler's list
  of runner globals, python-iir-compiler's list of Python builtin names,
  and the Closure-compiler passes, Mosaic emitters, test262 expectations
  and `lattice-ast-to-css` text, none of which reach the IIR.
- The `es-number-log` corpus has no failing case, so no release-sweep
  module changed and `release_classes.csv` is unchanged.

**What PR7e landed, and where it diverged from the plan above.**

- *The registry.* `write_f64_py` and `write_f64_js` are gone from
  `STANDARD_OPS`, with `LANG83_PR6A` and `OpSpec::is_pr6a`. Nothing emitted
  them: both release frontends print with their own helpers since PR7c, and
  no other frontend or compiler in the repository named them. (The LANG79
  Python pilot's `py_*` builtins are separate, unregistered and PR7g's.)
- *All nine backends.* Each drops both rows from its `OP_SUPPORT` and the
  writer half of its float code, and keeps the PR6a digit core behind
  `f64_digits` / `f64_exp10`. A module that still calls a writer is refused
  at compile time on every code-generation backend (an unknown builtin) and
  fails on the VM (no such builtin), so it never compiles into a call that
  nothing defines.
  - VM: vm-core's default builtins, `FloatLayout`, `format_f64`,
    `python_repr` and `js_to_string` are removed; `shortest_digits` and the
    three query functions stay.
  - JIT: `CALL_BUILTIN_F64` (0x51) is removed and the byte stays unassigned,
    so stale bytecode naming it is an unknown opcode. A `call_builtin` of a
    float register now refuses the function like any other non-float use.
    The layout helpers' call-free functions compile, as before.
  - Native: `__twig_write_f64_py` / `_js` and the C layout leave
    `twig_runtime.c`; the x86-64 and AArch64 builtin tables drop both names.
  - LLVM / WASM: the writer halves of `lang83_f64.rs` (`@__iir_write_f64`;
    `$__iir_write_f64` and its two `putchar` helpers) are removed. The digit
    core never imported `putchar`, so the WASM module of a digit query
    imports nothing for it.
  - JVM / CLR / BEAM: `__iir_write_f64` is removed; `__iir_f64_shortest` and
    the digit and exponent queries stay. *Divergence:* BEAM also stops
    interning the writer's own imports (`lists:duplicate/2`,
    `lists:nthtail/2`, `erlang:integer_to_list/1`) and literals, and the
    shared parse's frame shrinks from 10 to 8 Y slots, the two the writer
    alone read.
- *The no-language-names test.* `op_registry::language_name_violation`
  splits a name into words, on `_`, `.` and `-` and at camelCase
  boundaries (the whole part is checked too, so `TypeScript` is caught), and
  refuses a word that is in `LANGUAGE_TOKENS` (ignoring case and trailing
  digits, so `python3` and `js2` are caught) or ends in `error` or
  `exception`. The tokens include the runtimes a writer might name instead
  of the language (`node`, `cpython`) and `ts`, `typescript`, `php`,
  `ocaml`, `clojure`, `scala`.
  `no_registry_name_or_trap_kind_names_a_language` runs it over every
  `STANDARD_OPS` name and every `exception_kind` name, and
  `the_language_name_rule_rejects_the_retired_names` pins `write_f64_py`,
  `write_f64_js`, `py_print_float`, `Trap.ReferenceError`,
  `Ruby.ZeroDivisionError` and others. vm-core's
  `default_builtins_name_no_language` covers every default builtin (through
  the new `BuiltinRegistry::names()`), and `lang-aot`'s `lang_matrix` VM and
  JIT runners and `op_conformance` runners check every builtin they register
  on every run (`GenericCirJit::builtin_names()` is new for the JIT side).
  *Divergences:* the exception-class half is a shape rule, not a list:
  any part ending in `error` or `exception` is refused, which catches
  `ReferenceError`, `NameError` and `ZeroDivisionError` and every class
  named the same way, and costs nothing, since a trap kind names a
  condition. The token list leaves out the languages whose names are
  ordinary words (`Oct`, `Nib`, `REDUCE`, `Derive`), so that a neutral op
  such as `i64_to_oct` or `vec_reduce` stays possible.
- *Harnesses.* The `write_f64_js` / `write_f64_py` sinks are gone from
  `lang_matrix`'s VM runner and both halves of its JIT runner, and from
  `op_conformance`'s. Every `lang_matrix` row prints what it printed before.
- *Conformance: the PR6a table stays the oracle.* The two writer table
  cases became four helper cases: each layout's helper, taken verbatim from
  the module its compiler produces for a one-print program, prints the
  finite rows (all nine backends; BEAM compile-checked) and, separately,
  `±Infinity` and NaN (the eight IEEE backends). Each expects the PR6a
  strings, which `float_table_matches_host_python3_and_node` still re-checks
  against host `python3` and `node`. So every value the writer tables
  covered is still checked byte for byte, in both layouts, on every column.
  The ordered-stream and computed-float cases print through the helpers too,
  and so does the PR7b linked-runtime regression case. The gate
  `pr7e_float_writers_are_gone_and_the_layouts_run_everywhere` replaces
  `pr6a_ops_are_lowered_on_every_backend`.
  - *Divergence: the JIT column.* A helper case's `main` makes a `call`,
    which the generic JIT has no bytecode for, so the runner now requires
    every **call-free** function to be JIT-compiled (`main` for every other
    case, the digit counter and digit writer for a helper case) and, when
    the output holds a digit, that a compiled function actually ran (its
    `exec_count`).
  - *Divergence: PR6c.* The IEEE cases printed their results with
    `write_f64_js`. They now print `class digits exp10` and the sign bit
    with `write_i64` (`-Infinity` is `2 0 0 1`, `-0` is `0 0 0 1`), which
    keeps their `main` call-free, so the JIT column still runs the IEEE ops
    as compiled bytecode. The backends' own PR6c tests do the same.
- *Frontend oracles.* The JavaScript and Python crates compared their
  helpers, over 1,200 and 1,248 values, with vm-core's `format_f64` /
  `python_repr`. Each now carries the layout as a small Rust oracle over
  `vm_core::float_format::shortest_digits`, pinned to host `node` /
  `python3` strings.
- *No IIR changed.* No frontend's output changed, so no IIR digest and no
  `release_classes.csv` changed.
- Only `interpreter-ir` is in `twig-vm`'s dependency graph (`cargo tree -p
  twig-vm -i <crate>`); vm-core, jit-core and the backends are not, despite
  the note below. The local Miri script applies because of
  `interpreter-ir`.

**What PR7f-1 landed, and where it diverged from the plan above.**

PR7f is split into three PRs, stacked: PR7f-1 renames and registers the
representation primitives (no IIR a frontend produces changes, no output
changes), PR7f-2 moves the language-semantic runtime calls into
compiler-emitted helpers, and PR7f-3 moves the source-name translation into
the frontends.

- *The registry.* A new family, `OpFamily::Dynamic`, holds ten Builtins
  (`introduced_by: "LANG83 PR7f"`):

  | Builtin | Operands → result | Meaning |
  | --- | --- | --- |
  | `dyn_box_int` | `i64` → dyn | an integer (`|n| < 2^60`) as a dynamic value, `n << 3` |
  | `dyn_unbox_int` | dyn → `i64` | a dynamic integer's value, arithmetic `>> 3` |
  | `dyn_tag` | dyn → `i64` | the low three bits: 0 integer, 1 nil, 2 symbol, 3 false, 5 true, 7 pair |
  | `dyn_pair_new` | dyn, dyn → dyn | a fresh, garbage-collected pair |
  | `dyn_pair_first` / `dyn_pair_rest` | dyn → dyn | a pair's fields; undefined for a non-pair |
  | `dyn_is_pair` / `dyn_is_nil` | dyn → dyn | the pair and nil tests, as the dynamic booleans (words 5 and 3) |
  | `dyn_nil` | → dyn | nil (the word 1) |
  | `dyn_symbol` | literal → dyn | the symbol a name denotes |

  `dyn_box_int` and `dyn_unbox_int` keep their names (they were already
  neutral); `dyn_tag` is new. The operand type `"dyn"` is new.
  *Divergence:* all ten are **Builtins**, not Ops. Every backend that lowers
  them does so as a call into the tagged-word runtime, through the builtin
  table it already has. Their names start with `dyn_`, a dispatch prefix, so
  they join `GRANDFATHERED_PREFIXED_NAMES`. The native backends' `dyn_` arm
  (call `__dyn_<name>`) *is* their lowering, and it fires only for a name in
  the backend's builtin table, so an unregistered `dyn_` name is still
  refused (`dyn_new` is still a name-rule violation).
- *Renamed everywhere they are lowered or handled.* `iir-builtin-lowering`'s
  native runtime-call path (`heap.rs`'s `RUNTIME_RENAMES`, `dyn_repr.rs`,
  `list_ops.rs`, `closure_heap.rs`, `dynamic_arith.rs`); the x86-64 and
  AArch64 `V1_BUILTINS`; iir-to-llvm's `DYN_BUILTINS`; `dynval_runtime.c`
  (`__dyn_cons` … → `__dyn_pair_new` …, `__dyn_make_symbol` →
  `__dyn_symbol`, and the new `__dyn_tag`); the `dynval-runtime` crate's
  `extern "C"` functions and its `LispyBinding` names (`make_nil` /
  `make_symbol` → `dyn_nil` / `dyn_symbol`, which twig-vm resolves);
  `lang_aot::jit_lisp`'s callbacks; and the tests that name any of them.
  `lower_dyn_repr` boxes an integer atom flowing into `dyn_tag` (so `dyn_tag
  5` is 0, not the `#t` tag of the raw word 5) without treating its integer
  result as tagged.
- *Support is honest.* Native x86-64 and AArch64 and LLVM, the three
  backends that link `dynval_runtime.c`, lower all but `dyn_symbol`.
  `dyn_symbol` is refused there (`SYMBOLS_ARE_INTERNED_CONSTANTS`): a symbol
  reaches them as `const Var(name) : symbol`, which `intern_symbols` turns
  into a tagged immediate, and nothing lowers a symbol built at run time.
  WASM, JVM, CLR, BEAM, the VM and the JIT refuse all ten
  (`NO_TAGGED_WORD_RUNTIME`): they hold dynamic values structurally
  (`alloc` / `field_*` / `is_null`, then a GC struct, an object, an Erlang
  term or a `vm-core` heap cell), and none links the tagged-word runtime.
  The JIT's `CALL_BUILTIN` would run a registered handler, but the LANG83
  harnesses register none for these; `jit_lisp`'s McCarthy embedding does,
  and it is not the JIT column. vm-core's `every_standard_op_is_executed`
  pins that `Dynamic` is the one family the VM refuses.
- *Nothing a frontend emits changed.* Twig, McCarthy Lisp and the CAS
  frontends still emit `cons` / `car` / `null?` …, which pass
  `check_module_for` ungated and are renamed to the registered names by
  the tagged-word pipelines afterwards (PR7f-3 moves that translation into
  the frontends). So no IIR digest and no `lang_matrix` row changed.
- *Conformance.* One case runs every primitive but `dyn_symbol` — a pair of
  two boxed integers taken apart, the tags of a pair, an integer and nil,
  the nil and pair tests on a pair, nil and an integer, and a two-element
  list — and prints each result; it runs on LLVM and native x86-64. A second
  case is refused everywhere (`dyn_symbol`). The gate
  `pr7f_dynamic_values_run_on_the_tagged_word_backends` pins the support
  matrix and checks that WASM, JVM, CLR, BEAM, the VM and the JIT refuse the
  case with `UnsupportedOps` before any pass. `dynval_runtime_golden`'s
  `c_tag_of_every_kind_matches_rust` pins `__dyn_tag` against the Rust tag
  constants.
- *Not yet neutral:* the frontends' nil literal is typed `ref<LispyPair>`,
  a Lisp word in a type hint. Type names are outside the name rule, which
  covers registry ops, builtins and trap kinds; it is noted here for a
  later pass.

**What PR7f-2 landed, and where it diverged from the plan above.**

- *The rules are compiler-emitted IIR.* `iir-builtin-lowering`'s new
  `lisp_semantics` module writes Lisp's four value rules over the PR7f-1
  primitives:

  | Rule | Emitted as | Built from |
  | --- | --- | --- |
  | truthiness (false iff the word is nil or `#f`) | inline, 5 instructions: `(x ≠ 3) × (x ≠ 1)` over the whole word | `const`, `cmp_ne` (typed `i64`), `mul` |
  | `not` | inline, 4 more: `5 − 2 × truthy` (the words of `#t` / `#f`) | the above, `sub` |
  | `equal?` | `__lisp_equal(a, b)`, a recursive module function | `dyn_tag`, `dyn_pair_first`, `dyn_pair_rest`, `cmp_eq` on the words, `call` |
  | exit code | `__lisp_exit_code(x)`, a module function | `dyn_tag`, `dyn_unbox_int`, `cmp_eq`, `mov` |

  `lower_dyn_repr` (the tagged-word pass the native, LLVM and McCarthy-JIT
  pipelines run) marks each use with a `call` of the rule's name while it
  classifies registers — exactly where it emitted `dyn_truthy` /
  `dyn_not` / `dyn_equal` / `dyn_to_exit_code` before — then expands the two
  inline rules and appends the two functions a module calls, once each.
  `equal?` is no longer renamed by `lower_heap_builtins_runtime`; it keeps its
  name (retyped `ref<any>`) for `lower_dyn_repr`. The helpers are appended
  after the pass's analysis, and a module that already holds them has them
  set aside while it is classified, so the pass never boxes their constants.
- *Divergence: two rules are inline, not helper functions.* A probe of
  `JITCore::is_compiled` over the McCarthy JIT test programs showed that a
  `call` of a helper sends its caller to the interpreter tier (the generic
  JIT has no `call` bytecode): `main` of `(ATOM 7)` and the lambda of
  `((LAMBDA (X) (ATOM X)) 7)` had compiled before. Inline, every instruction
  typed `i64`, they still compile. `equal?` must recurse, so `main` of
  `(EQ 7 7)` / `(EQ 7 8)` and the lambda of `((LAMBDA (X Y) (EQ X Y)) 3 3)`
  now run on the interpreter tier — the only change in the compiled set,
  with the same results. The exit-code switch applies only to a `call`'s
  result, so its caller already contained a call. The helper functions
  contain `jmp`/`label`, so they are `PartiallyTyped` and run on the
  interpreter tier too.
- *Removed everywhere.* `__dyn_truthy`, `__dyn_not`, `__dyn_equal` and
  `__dyn_to_exit_code` leave `dynval_runtime.c` (and its golden test); the
  four rows leave the x86-64 and AArch64 `V1_BUILTINS` and iir-to-llvm's
  `DYN_BUILTINS`, so a module that still calls one is refused at compile
  time; `jit_lisp` drops its four callbacks and registers `dyn_tag` and
  `dyn_is_nil` instead; `lang-aot`'s narrowing evidence lists drop them.
  None of the four was a registry entry, so no `op_support` row changed.
- *Names and linkage.* The helpers are `__lisp_equal` and
  `__lisp_exit_code`, the Lisp lowering's own namespace: no linked runtime
  exports `__lisp_*` (the runtimes use `__twig_*`, `__dyn_*`, `__gc_*`, the
  LLVM float helpers `__iir_*`), so no runtime symbol can be bound in their
  place. `helper_names_avoid_runtime_namespaces` pins that.
- *Typing.* Helper parameters and the pair fields they read are `ref<any>`,
  so the precise collector treats them as roots; a native function entry
  polls a safepoint, and a compacting collection there must relocate the
  pairs `__lisp_equal` walks. The atom comparison is typed `i64`: a
  `ref<any>`-typed `cmp_eq` was accepted by LLVM but refused by the native
  x86-64 backend (a comparison's hint is its operand type).
- *Step budgets.* The runners that execute this IIR are the native and LLVM
  executables (no budget), `jit_lisp`'s JIT (fuel counts backward jumps; the
  rules add none) and its vm-core interpreter tier (no instruction cap). The
  `lang_matrix` VM and JIT columns run Lisp-family programs through the
  structural lowering, which this PR does not touch. So no budget changed.
- *Tests.* `lisp_semantics` runs the four rules on a reference machine over
  every kind of value — integers (0 included), nil, `#t`, `#f`, symbols,
  pairs equal in structure but not identity, nested lists — against the
  rules stated independently. `op_conformance`'s new case prints the four
  rules over the same kinds of value on LLVM and native x86-64, and
  `pr7f2_lisp_rules_are_module_code_over_the_primitives` checks its expected
  output against `dynval-runtime`'s Rust `LispyValue` (`is_truthy`,
  `builtins::not`, `builtins::equal_p`), the twin of the removed C, and that
  the LLVM module defines the helpers and references no removed runtime
  rule. Every `lang_matrix` row and every McCarthy LLVM / native / JIT test
  prints what it printed before.
- *Security review: truthiness reads the whole word.* The first draft
  tested `dyn_tag x` (`(tag ≠ 3) × (tag ≠ 1)`). That is not the rule the
  removed `__dyn_truthy` (or the generic JIT's `is_truthy`) applied: only
  the *words* 1 and 3 are false, and a malformed word with their tag — a
  raw 9, 11, 17, 19, which Twig produces by passing an unboxed parameter
  into `dyn_pair_new` — is true. `(define (f n) (if (car (cons n (list)))
  1 2)) (f 9)` exited 1 before PR7f-2 and 2 with the draft on LLVM and
  native x86-64. Truthiness now compares the word itself (`cmp_ne x, 3`,
  `cmp_ne x, 1`, typed `i64`); `lang_matrix`'s
  `twig_truthiness_of_a_raw_word_matches_the_runtime_rule_it_replaced`
  pins the old exit codes.
- *Security review: compiler-reserved names.* The pass recognises its
  markers and helpers by name, and Twig's identifiers can spell them:
  `(define (__lisp_equal a b) 42)` stood in for `equal?`, and
  `(define (__lisp_not x) 0) (__lisp_not 5)` was expanded as Lisp's `not`.
  Two layers now refuse it. `lisp_semantics::check_reserved_names` (run
  first by `lower_dyn_repr`, which now returns `Result`) refuses a module
  that defines or calls one of the four names it recognises (`__lisp_truthy`,
  `__lisp_not`, `__lisp_equal`, `__lisp_exit_code`), unless the function is
  exactly the helper the pass emits (so a lowered module lowers again).
  Only those four: the pass runs on Nib's and ALGOL's modules too, where a
  function merely prefixed `__lisp_` collides with nothing. The Twig
  frontend reserves the `__lisp_` and `__dyn_` prefixes for every name it
  reads, with a source position; McCarthy's atoms are upper-case and cannot
  spell either. `__dyn_` has the same exposure — `length` binds to an
  existing `__dyn_list_length`, closures to an existing
  `__dyn_call_closure`, and `__dyn_pair_new` … are runtime symbols — and is
  closed at the Twig frontend. *Follow-up (landed):* the list and closure
  passes now refuse a module that holds a different function under one of
  their helpers' names (`helper_names::HelperNameError`, plumbed like
  `ReservedNameError`). A second review found two more shadowing routes, also
  closed. First, `twig-aot`'s link step patched a runtime call to a program
  function of the same name: `(define (__twig_print_i64 n) 0)` swallowed
  `print`. It now refuses a function named like a runtime symbol, or two
  functions of one name. Second, a program could define `__lambda_0`, the
  name of a lifted lambda. Twig now also reserves `__lambda_` and `__twig_`.
- *Not moved yet.* The managed backends (WASM, JVM, CLR, BEAM) still lower
  the source-level `pair?`, `not` and `equal?` builtins in their own code
  (their value model is structural, and `lower_dyn_repr_structural` leaves
  those names for them). That is PR7f-3's "source-level builtin names".

### Migration, as a PR sequence

Each PR keeps main green on all nine columns. None removes an op while
anything still emits it.

1. **PR7a — this spec** (and the LANG84 principle statement).
2. **PR7b — the three ops on every backend.** `f64_class`, `f64_digits`
   and `f64_exp10` are registered (`introduced_by: "LANG83 PR7b"`) and
   lowered on all nine. Each backend's PR6a core is refactored so that
   `write_f64_*` and the new ops share it; the output is unchanged.
   `op_conformance` gains the class/digits/exponent tables. JIT: the
   three bytecodes. CLR and BEAM are structural locally and executed in
   CI, as in PR6a. *Landed;* see "What PR7b landed" under "The neutral
   float primitives".
3. **PR7c-js — javascript-iir-compiler emits its helper.** `console.log`
   calls the helper; the `-0` test becomes the class test; BEAM accepts
   computed values. All 38 `lang_matrix` JavaScript rows stay
   byte-identical. BEAM still refuses only `f64_*` arithmetic, but a
   `console.log` of a computed value no longer needs `f64_add` for its sign. `release_classes.csv` is regenerated, since the IIR
   changes. *Landed;* see "What PR7c-js landed" under "The layouts, as
   compiler-emitted helpers".
4. **PR7c-py — python-iir-compiler does the same** on the release path.
   Its `lang_matrix` rows stay byte-identical. *Landed;* see "What
   PR7c-py landed" under "The layouts, as compiler-emitted helpers".
5. **PR7d — `Trap.ReferenceError` → `Trap.Unbound`, and retire
   `LANGUAGE_TRAP_KINDS`.** The kind keeps id 7. It lands before PR7e,
   because PR7e's test rejects the old name. *Landed;* see "What PR7d
   landed" under "Other replacements".
6. **PR7e — remove `write_f64_py` and `write_f64_js`** from the registry,
   all nine backends, `twig_runtime.c`, vm-core's default builtins, the
   lang-aot and `lang_matrix` harness sinks, and jit-core's
   `CALL_BUILTIN_F64` if nothing else uses it. Add the
   **no-language-names test** to `op_registry`: every `STANDARD_OPS` name,
   every vm-core default builtin, every lang-aot harness builtin and every
   `exception_kind` name is split into words (on `_` and `.`; as landed,
   also `-` and camelCase boundaries), and no word may be a
   language token (`py`, `python`, `js`, `javascript`, `ecma`, `rb`,
   `ruby`, `pl`, `perl`, `lisp`, `scheme`, `twig`, `basic`, `algol`,
   `cobol`, `fortran`, …) or a known language exception class
   (`ReferenceError`, `NameError`, `ZeroDivisionError`). Rewrite PR6a's
   section as history and point it here. The Lisp-vocabulary `dyn_*`
   names are unregistered until PR7f, so the test covers them when PR7f
   registers them under their new names. *Landed;* see "What PR7e landed"
   under "Other replacements".
7. **PR7f — Lisp-family neutrality.** Register the representation
   primitives, rename the Lisp-vocabulary `dyn_*`, and move
   truthiness/`equal?`/`not`/exit-code into compiler helpers. Move
   `iir-builtin-lowering` behind the Twig, McCarthy Lisp and CAS
   frontends. This is the largest slice. It may split per frontend; the
   McCarthy Lisp and CAS compilers are smaller than Twig. Twig's
   `twig-aot` native runtime and lispy-runtime rename their symbols
   together. *Split in three:*
   - **PR7f-1** — register the ten `Dynamic` primitives and rename the
     Lisp-vocabulary ones everywhere they are lowered (the native
     runtime and `dynval-runtime` together). *Landed;* see "What PR7f-1
     landed" under "Other replacements".
   - **PR7f-2** — the language-semantic runtime calls (`dyn_truthy`,
     `dyn_equal`, `dyn_not`, `dyn_to_exit_code`) become compiler-emitted
     IIR built from the primitives. *Landed;* see "What PR7f-2 landed".
   - **PR7f-3** — the source-name translation moves into the frontends.
8. **PR7g — retire the LANG79 Python pilot builtins** (`py_*`) by putting
   `compile_source` on the release path.
9. **PR7h — register `getchar`, `input_i64`, `input_more`, `input_str`**
   with honest `op_support`.

PR7b–PR7e touch `interpreter-ir`, which `twig-vm` depends on, so each runs
the local Miri script per CLAUDE.md. (`iir-to-*`, `jit-core` and `vm-core`
are not in `twig-vm`'s graph.) PR7f-1 touches `interpreter-ir`,
`dynval-runtime`, `twig-ir-compiler` and `twig-vm` itself, all in the graph
(`cargo tree -p twig-vm -i <crate>`), so the script applies. PR7f-2 touches
none of them: `iir-builtin-lowering`, `lang-aot`, `twig-aot`, the native and
LLVM backends are not in `twig-vm`'s graph.

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
