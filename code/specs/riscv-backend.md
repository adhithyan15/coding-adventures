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
| BASIC `PRINT 42` | `const_f64 42.0; call __basic_print_real; ...` | (refused — Dartmouth BASIC's only numeric type is REAL, and RV32I has no floating-point registers) |

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
15. [ ] **Wide register allocation:** spill live 64-bit register pairs and add
   pair-aware reloads, extending the scalar frame allocator to wide CIR values.
   Pair arithmetic, bitwise operations, shifts, division, and comparisons reload
   live spills today. Pair destinations can also evict scalar words from a
   pair slot, and scalar values use a reserved mixed-width register when all
   three pairs are live. Arbitrary mixed scalar/pair pressure still needs a
   general allocator.
16. [ ] **Complete wide spill coverage:** generalize mixed scalar/pair register
   allocation beyond pair destinations evicting scalar words and the reserved
   scalar register.
17. [ ] **Calls and modules:** lower direct calls, add relocations/linking for flat
   binaries, and preserve the RISC-V calling convention across calls.
18. [ ] **Host runtime ABI:** define simulator `ecall` services for exit and integer
   output, then lower language print primitives through that ABI.
19. [ ] **Memory and data:** globals, addresses, loads/stores, and a data-image
   loader for programs needing strings or arrays.
20. [ ] **BASIC real values:** Dartmouth BASIC currently lowers numeric values
   such as `PRINT 42` through `f64`; direct RISC-V execution needs either a
   floating-point ABI or an integer-only lowering path before those programs
   can run on the simulator.

Each item should land as a focused PR with an end-to-end fixture from the
highest-level language it enables. New constraints discovered while carrying
an item out belong in this list before the next item is selected.
