# `riscv-backend` spec

> **Status:** v0.1.0 — Phase 7 (FINAL lane) of the historical-arch
> backend migration, 2026-06-03.

## Purpose

RV32I implementation of the `jit_core::backend::Backend` trait.
Mirror of `aarch64-backend` / `x86_64-backend` / `ge225-backend` /
`intel4004-backend` / `armv7-backend` / `intel8008-backend`.

Lowers `Vec<CIRInstr>` (typed, monomorphised) to a little-endian
`Vec<u8>` of RV32I machine code via `riscv-encoder`.

## Why this crate exists (Phase 7 of the historical-arch backend migration)

RV32I was the **original** historical-arch lane (the A1+ cascade
from May 2026).  It shipped at the wrong layer — `iir-to-riscv`
consumed dynamic IIR (`add a b` with unknown operand types) and
emitted bytes directly.  That bypassed `aot_core::infer` and the
shared `Backend` trait — every other arch backend would have to
redo type inference and re-implement the registry hookup.

Phase 7 corrects that.  `riscv-backend` consumes CIR (typed:
`add_i64`, `cmp_lt_u32`) and plugs into the same registry as
`aarch64-backend` / `x86_64-backend`.  The architectural
correctness win — every arch backend uses the same
`aot_core::infer` + `aot_core::specialise` + `Backend::compile`
pipeline — is delivered as Phase 7 lands, closing the historical
migration.

## Current scope - executable scalar core

The initial migration shipped a minimal emitter. The current increment keeps
its byte-for-byte compatibility while adding an executable scalar RV32I core:

| CIR op family | Lowering |
|---------------|----------|
| `const_*` (RV32-width literal) | `addi` or `lui` + `addi` |
| Scalar integer ops | `add`, `sub`, `and`, `or`, `xor`, shifts, `neg`, and `not` |
| Scalar comparisons | Signed or unsigned RV32I comparison sequences |
| `ret_*` | `addi a0, src_reg, 0` + `jalr x0, x1, 0` |
| `ret_void` | `jalr x0, x1, 0` |
| Empty CIR body | `jalr x0, x1, 0` |

The backend accepts up to eight integer parameters through the standard
`a0` through `a7` starter ABI. Unsupported CIR operations and types return a
`BackendError` from the inherent `compile()` method, or `None` from the
`Backend::compile` trait method. AOT treats `None` as a per-function compile
failure; JIT keeps execution on the interpreter tier.

## Wire format

Each instruction is a 32-bit RV32I word, flattened to little-endian bytes per
the RISC-V spec. Per-function byte streams can be concatenated directly;
`lang-aot` writes them straight to disk as a flat `.bin`.

## Pinned byte sequences

| Program | CIR | Emitted bytes |
|---------|-----|---------------|
| Twig `42` | `const_i64 v=42; ret_i64 v` | `[0x93, 0x02, 0xA0, 0x02, 0x13, 0x85, 0x02, 0x00, 0x67, 0x80, 0x00, 0x00]` |
| `ret_void` only | `ret_void` | `[0x67, 0x80, 0x00, 0x00]` |
| Empty CIR | (none) | `[0x67, 0x80, 0x00, 0x00]` |
| BASIC `PRINT 42` | target-only `const_i64 42; call __basic_print_int; ...` | host byte output `42\\n` |

## Backend trait surface

| Trait method | Behaviour |
|--------------|-----------|
| `name()` | returns `"riscv32"` |
| `compile(ir)` | returns `Some(bytes)` for supported CIR ops; `None` otherwise |
| `compile_function(ctx, ir)` | Uses `FunctionContext` parameters to map integer arguments to `a0` through `a7` |
| `run(binary, args)` | Loads the flat binary into `riscv-simulator`, passes up to eight integer arguments in `a0` through `a7`, and returns the final `a0` value |

## Error variants

| `BackendError` variant | Trigger |
|------------------------|---------|
| `UnsupportedOp(String)` | CIR operation outside the scalar core |
| `UnsupportedType(String)` | Type needing a representation beyond one RV32 register |
| `UnsupportedFloat { site, ty }` | A floating-point type (`f32`/`f64`) reached the backend. RV32I is the base *integer* ISA and has no floating-point registers; floats need the `F`/`D` extensions (RV32F/RV32D). `site` names the CIR op or parameter that carried it. Distinct from `UnsupportedType` because the fix is retarget-or-soft-float, not "write the missing lowering" |
| `InvalidOperand(String)` | Malformed CIR operands or destinations |
| `UndefinedVariable(String)` | A variable has no allocated source register |
| `ImmediateOutOfRange(i64)` | A compatibility `i64` literal cannot fit in RV32 |
| `OutOfRegisters` | A wide register-pair allocation cannot fit the starter pool |
| `TooManyArguments(usize)` | More than eight starter-ABI parameters or arguments |
| `CallOutOfRange` | A linked direct-call target is beyond the `jal` range |
| `ExecutionDidNotHalt` | Simulator step limit was exceeded |

## Tests

Focused tests pin the Twig `42` byte sequence, execute its actual `lang-aot`
output through the simulator, and cover arithmetic, comparisons, 32-bit
literals, narrow integer masking, starter-ABI parameters, scalar stack spills,
unsupported wide arithmetic, unsupported calls, and bounded simulator execution.

## Executable scalar core

The first post-migration increment expands the minimal emitter into an
executable scalar RV32I core. It supports 32-bit integer and boolean
constants, arithmetic, bitwise operations, shifts, comparisons, up to eight
ABI arguments, and return values. `run_binary` and `Backend::run` execute the
generated bytes in `riscv-simulator`; the simulator now reports whether the
program halted and how many instructions it executed.

Word-sized `i64` and `u64` constants preserve the historical Twig `42` byte
sequence. Wider constants and `add` / `sub` / `mul` now use low/high register pairs;
the simulator runner exposes the pair through `a0` and `a1`. Other wide
operations remain intentionally rejected until their pair-aware sequences are
implemented.

## Simulator host ABI

Programs running under the in-tree simulator can request host services with
`ecall` while `mtvec` is zero. `a7` selects the service and arguments use the
standard `a0` / `a1` registers:

| Service | `a7` | Arguments | Effect |
|---------|------|-----------|--------|
| Exit | `1` | `a0`: signed status | Halts the guest and records `Exit(status)` |
| Write signed integer | `2` | `a1:a0`: signed `i64` | Records `WriteI64(value)` and continues |
| Write byte | `3` | `a0`: low byte | Records `WriteByte(value)` and continues |
| Read byte | `4` | none | Returns the next input byte in `a1:a0`, or signed `-1` at EOF |

Installing an M-mode trap vector leaves architectural `ecall` trap behavior
unchanged. `riscv_backend::run_binary` exposes `WriteI64` values through
`RunResult::output` and clears `a7` before its private return trampoline, so
a program's last host service cannot be replayed as the terminal halt.

`call_builtin "print_i64", value` lowers to the write service. This gives Nib
programs a source-to-RV32I-to-simulator printing path without requiring an
external runtime image.

`call_builtin "putchar", value` and `call_builtin "getchar"` lower to the
byte services. `run_binary_with_input` supplies deterministic input bytes and
`RunResult::byte_output` exposes produced bytes for source-level tests.

## Module globals

`compile_module` collects `global_load` and `global_store` names in stable
first-seen order across the linked CIR module. Each name owns an aligned,
zero-initialized eight-byte slot appended after the text image. Scalar globals
use the low word; `i64` and `u64` globals use both words. The global name and
its CIR type must agree at every access. Lowered code materializes the final
slot address after module layout, so calls share one storage location while
the selected entry function can seed language-level static initializers.

## Prioritized backlog

1. [x] **Control flow:** label binding and branch backpatching for CIR conditional
   jumps, followed by boolean source-language conditional end-to-end tests.
2. [x] **Wide value core:** register-pair lowering for full-width `i64`/`u64`
   constants, addition, subtraction, returns, and a Nib arithmetic fixture.
3. [x] **Wide comparisons:** pair-aware signed and unsigned comparisons plus a
   numeric Nib conditional source-to-simulator fixture.
4. [x] **Wide bitwise operations:** pair-aware `and`, `or`, `xor`, and `not`,
   plus a Nib bitwise source-to-simulator fixture.
5. [x] **Wide shifts:** pair-aware logical and arithmetic shifts for an RV32I
   shift count, including counts crossing the 32-bit word boundary.
6. [x] **Nib shift frontend:** `<<` / `>>` syntax lowers to typed IIR shifts,
   with a Nib source-to-RV32I-simulator fixture covering both operations.
7. [x] **Wide multiplication:** RV32M `mul` / `mulhu` lowering for full-width
   signed and unsigned values, including a Nib source-to-simulator fixture.
8. [x] **Chained wide shifts:** reuse a dead left-hand register pair for a
   following wide right shift, with direct CIR and Nib source-to-simulator
   fixtures for `(1 << 6) >> 1`. General spilling remains the allocator item.
9. [x] **RV32M divide/modulo substrate:** encoder and simulator support for
   `div` / `divu` / `rem` / `remu`, including defined zero-divisor behavior.
10. [x] **Wide unsigned division and modulo:** pair-aware restoring division for
   `u64`, including RV32M-compatible zero-divisor results.
11. [x] **Wide signed division and modulo:** normalize signs around the unsigned
   pair loop, including `i64::MIN / -1` behavior.
12. [x] **Nib division frontend:** Nib `/` already lowers to typed `div`; add a
   source-to-RISC-V-simulator fixture to keep that end-to-end path covered.
13. [x] **Nib modulo frontend:** `%` lowers to typed `mod`, with a
   source-to-RISC-V-simulator fixture.
14. [x] **Scalar register allocation:** spill live RV32 values to stack slots and
   emit a frame, removing the six-temporary limit for scalar CIR.
15. [x] **Wide register allocation:** spill live 64-bit register pairs and add
   pair-aware reloads, extending the scalar frame allocator to wide CIR values.
   Pair arithmetic, bitwise operations, shifts, division, and comparisons reload
   live spills. Allocation tracks the three physical pair slots, evicts scalar
   occupants when necessary, and preserves live aliases when a pair spills.
16. [x] **Complete wide spill coverage:** mixed scalar/pair allocation uses
   coherent pair slots, reclaims dead values of either width, and spills live
   scalar or wide values as needed under arbitrary temporary pressure.
17. [x] **Calls and modules:** module-local calls link as PC-relative `jal`,
   preserve `ra`, marshal scalar or `i64`/`u64` pair arguments through `a0`
   through `a7`, and save register-resident scalar or pair values live across a
   call. Nib argument and live-value calls execute from the selected entry
   function in the simulator.
18. [x] **Host runtime ABI:** simulator `ecall` services expose exit and signed
   64-bit integer output; `print_i64` lowers through that ABI and Nib output is
   captured by the source-to-simulator fixture.
19. [x] **Module globals:** collect typed `global_load` / `global_store` names
   across a linked module, append zero-initialized 64-bit slots after code, and
   lower scalar and pair accesses. Nib `static` values survive linked calls in
   the source-to-simulator fixture.
20. [x] **Static byte buffers:** lower compile-time `alloc_bytes` requests plus
    zero-extending `load_byte` and truncating `store_byte` operations into an
    appended, zero-filled module-data region for tape-like payloads. Brainfuck
    mutation now compiles through `lang-aot` and executes in the simulator.
21. [x] **Dynamic byte allocation and bounds:** module-local non-constant
    `alloc_bytes` requests use a zero-filled bump heap under the in-tree
    simulator. Byte accesses retain their allocation length and halt through
    host exit status `2` when the offset is out of range.
22. [x] **Wide reassignment:** allow a word-sized `i64`/`u64` value to widen or
    spill when an in-place wide operation reuses its destination; Brainfuck
    pointer motion now reaches the byte bounds guard from source.
23. [x] **Escaped byte-buffer ABI:** byte buffers use the low/high words of
    their existing `i64`/`u64` representation for address/length, so checked
    access survives moves, calls, returns, and globals without a separate ABI.
24. [x] **Data images:** append deduplicated initialized UTF-8 byte images to
    the module alongside code. `str_const` produces their address/length pair,
    giving string and array runtimes addressable initialized program data.
25. [x] **Host character I/O:** byte-oriented input/output services back
    `getchar` / `putchar`; Brainfuck `.` now emits observable simulator output.
26. [x] **BASIC integral literal output:** target-only lowering rewrites a
   finite whole-number literal passed directly to `PRINT` from the frontend's
   `f64` representation to the integer print ABI, allowing it to execute in
   the simulator without pretending fractional REAL values are supported.
27. [x] **Call argument ABI:** marshal scalar and pair-value arguments into
   `a0` through `a7`, including word-sized wide values, and preserve the narrow
   CIR view of ABI-normalized wide parameters.
28. [x] **Typed CIR moves:** lower scalar and wide `mov_*` copies, including
   copies with a live wide source. Nib `let` bindings can now flow into direct
   calls and execute in the simulator.
29. [x] **Incoming parameter call preservation:** save values live across a
   direct call before argument marshalling overwrites incoming `a0` through
   `a7` ABI registers. Recursive integer formatting keeps its caller state.
30. [ ] **BASIC integral expressions:** give BASIC a target-specific checked
   integer representation for variables, arithmetic, control flow, and `LET`,
   so non-literal whole-number programs do not depend on the `f64` frontend
   representation.
31. [ ] **BASIC fractional REAL ABI:** choose and implement either soft-float
   or an RV32 floating-point target and preserve Dartmouth BASIC's fractional
   arithmetic and formatting semantics end to end.

Each item should land as a focused PR with an end-to-end fixture from the
highest-level language it enables. New constraints discovered while carrying
an item out belong in this list before the next item is selected.
