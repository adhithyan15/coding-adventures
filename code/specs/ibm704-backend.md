# `ibm704-backend` spec

> **Status:** v0.2.0 executable minimal output, 2026-08-27.

## Purpose

IBM 704 implementation of `jit_core::backend::Backend`. It lowers supported
typed CIR into the canonical 36-bit words and big-endian transport defined by
`ibm704-encoder`.

## Supported CIR

| CIR family | Lowering |
|---|---|
| `const_*` with integer/bool value 0–32767 | `CLA` from a literal-pool word |
| `ret_*` of the current accumulator variable | `HTR 0` |
| `ret_void` or empty CIR | `HTR 0` |
| Anything else | `BackendError::UnsupportedOp` |

CONS, arithmetic, branches, calls, locals, and multi-register allocation remain
outside this minimal historical-backend increment.

## Layout algorithm

Lowering is two-pass:

1. Validate CIR and collect one machine operation per supported CIR operation.
2. Assign each constant a literal address immediately after all instructions,
   relocated by the function's absolute load address.
3. Emit canonical instructions followed by raw positive sign-magnitude words.

The combined module instruction and literal count must not exceed 32,768 words.
The CIR length is bounded before it is used as an allocation capacity, and
oversized output returns `BackendError::ProgramTooLarge` before narrowing any
address to 15 bits. Module emitters call `compile_at` with each concatenated
function's word offset so all `CLA` addresses remain absolute.

## Canonical output

`const_i64 v=42; ret_i64 v` produces:

| Address | Word | Transport |
|---:|---|---|
| 0 | `CLA 2` (`0x1_4000_0002`) | `01 40 00 00 02` |
| 1 | `HTR 0` (`0`) | `00 00 00 00 00` |
| 2 | positive sign-magnitude `42` | `00 00 00 00 2A` |

## Backend trait surface

| Method | Behavior |
|---|---|
| `name()` | `"ibm704"` |
| `compile(ir)` | `Some(bytes)` for supported CIR; `None` on errors |
| `compile_function(ctx, ir)` | Same lowering as `compile` |
| `run(binary, args)` | Panics with the documented emit-only message |

## Error variants

| Variant | Trigger |
|---|---|
| `UnsupportedOp(String)` | Unsupported CIR or return of a non-current variable |
| `InvalidOperand(String)` | Missing/wrong operand shape |
| `ImmediateOutOfRange(i64)` | Constant outside 0–32767 |
| `ProgramTooLarge(usize)` | Instructions plus literals exceed 32K words |

## Required tests

- Empty/void returns emit canonical `HTR 0`.
- Integer and boolean constants address exact literal-pool entries.
- Multiple constants receive distinct pool addresses.
- Unsupported operations and operands report typed errors.
- Constant and program-size bounds fail closed.
- `lang-aot` Twig and McCarthy Lisp smoke tests pin the 15-byte `42` program.
