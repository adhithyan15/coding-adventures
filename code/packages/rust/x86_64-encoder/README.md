# x86_64-encoder

Pure-Rust x86-64 (AMD64 / Intel 64) instruction encoder.  Stand-alone, no
dependencies beyond `std`.  Designed as the bottom-of-stack for any CIR →
native-code lowering in this repo (jit-core / aot-core) on Linux x86-64 and
Windows x86-64 hosts.

Implements [LANG44](../../../specs/LANG44-x86_64-encoder.md).

## What's in scope (V1)

- 64-bit (long mode) GPR-form integer instructions
- REX-prefixed encoding for full access to R8..R15
- Branches with label-resolution at finish-time — always emits `Rel32` /
  `disp32` forms so the byte length of every instruction is known the
  moment its first byte is written (no two-pass shortening)
- External (cross-function and runtime) relocations surfaced as opaque
  `ExternalReloc` records that the packager translates to OS-specific
  relocation type IDs
- Output: byte-stream `Vec<u8>` — no headers, no executable wrapping

Floats / SSE / AVX, atomics, system instructions, and 16-/32-bit legacy
modes are **out of scope** for V1; they can be added incrementally.  The
encoder is intentionally OS-agnostic and ABI-agnostic — ABI choice (System
V on Linux, Microsoft x64 on Windows) is a backend concern; object format
choice (ELF vs PE/COFF) is a packager concern.

## Where it sits in the stack

```
CIR (jit-core::cir)
   │
   ▼ x86_64-backend::compile  (System V or MS x64 ABI)
   │
   ▼ Assembler::mov_r64_r64(...) / .cmp(...) / .jcc(cc, label) ... .finish()
   │                                                              │
   │                                                              ▼
   │                                                          Vec<u8>
   │                                                              │
   │                                                              ▼
   │                                       code-packager::elf_object / pe_object
   │                                                              │
   │                                                              ▼
   │                                                       cc / link.exe
   │                                                              │
   │                                                              ▼
   │                                                  runnable native binary
   ▼
(macOS ARM64 path uses aarch64-encoder + macho_object — unchanged)
```

## Quick start

```rust
use x86_64_encoder::{Assembler, Reg};

// fn(a: u64, b: u64) -> u64 { a + b }
// System V: a in RDI, b in RSI, return in RAX
//   mov rax, rdi
//   add rax, rsi
//   ret
let mut a = Assembler::new();
a.mov_r64_r64(Reg::Rax, Reg::Rdi);
a.add(Reg::Rax, Reg::Rsi);
a.ret();
let bytes = a.finish().unwrap();
assert_eq!(bytes, vec![0x48, 0x89, 0xF8, 0x48, 0x01, 0xF0, 0xC3]);
```

## Why "always emit Rel32 / disp32"

x86-64 has both 8-bit and 32-bit displacement forms for branches and
memory addressing.  Picking the right one in a single forward pass needs
either a worst-case overestimate (sometimes-too-long branches) or a
relaxation pass.  V1 picks the always-long form: 6 bytes per `Jcc rel32`
and 7 bytes per `MOV r64, [rbp - disp32]` regardless of how small the
displacement actually is.

The cost is a few extra bytes per branch / memory access; the benefit is
that label resolution is a straight "patch the rel32 field at the
recorded byte offset" pass with no width juggling.  A future PR can add a
relaxation pass if the size cost matters.

## Testing

Coverage target: **≥ 95%**.  Every public method has at least one
byte-exact test asserting against a hand-verified reference (cross-checked
with `llvm-mc -triple=x86_64 --show-encoding` during development).

`iced-x86` is a `dev-dependency` used to round-trip the emitted bytes
back through a decoder and confirm the mnemonic + operands match
expectations.  Production builds keep the zero-runtime-dep guarantee.

## Reference

- *Intel® 64 and IA-32 Architectures Software Developer's Manual, Volume
  2* — instruction set reference.
- *AMD64 Architecture Programmer's Manual, Volume 3* — instruction
  encoding details.
- LANG44 spec — repo-local coverage and design decisions.
