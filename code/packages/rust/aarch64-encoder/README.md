# aarch64-encoder

Pure-Rust ARM64 (AArch64) instruction encoder.  Stand-alone, no dependencies
beyond `std`.  Designed as the bottom-of-stack for any CIR → native-code
lowering in this repo (jit-core / aot-core).

## What's in scope

- 64-bit GPR-form integer instructions
- Branches with label-resolution at finish-time (no two-pass exposure to callers)
- Output: little-endian 32-bit instruction words concatenated as a `Vec<u8>`

Floats, atomics, SIMD, system-mode instructions, and large-immediate
addressing modes are out of scope today; they can be added incrementally.

## Where it sits in the stack

```
CIR (jit-core::cir)
   │
   ▼ aarch64-backend::compile
   │
   ▼ Assembler::add(...) / .ldr(...) / .b(label) ... .finish() ──► Vec<u8>
                                                               │
                                                               ▼
                                                   code-packager::macho64 / elf64 / pe
                                                               │
                                                               ▼
                                                       runnable binary
```

## Quick start

```rust
use aarch64_encoder::{Assembler, Reg};

// fn(a: u64, b: u64) -> u64 { a + b }
let mut a = Assembler::new();
a.add(Reg::X0, Reg::X0, Reg::X1);
a.ret();
let bytes = a.finish().unwrap();
assert_eq!(bytes.len(), 8);  // two 4-byte instructions
```

## Verification

Every encoded instruction has a unit test that asserts the produced
32-bit word against the expected bit pattern derived from the *ARM
Architecture Reference Manual for ARMv8-A* (DDI 0487).  This catches
field-offset and shift bugs at compile time of the test suite.

## Spec reference

Encoding details cite §C6 of the ARM ARM (DDI 0487).
