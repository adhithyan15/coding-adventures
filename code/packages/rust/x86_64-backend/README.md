# x86_64-backend

x86-64 (AMD64) native-code backend for `jit-core` and `aot-core`.  Lowers
CIR to x86-64 machine code via `x86_64-encoder`.  Implements
[LANG43](../../../specs/LANG43-x86_64-backend.md).

## Stack position

```
IIRModule (interpreter-ir)
   │
   ▼ aot-core::specialise / jit-core::specialise
CIR (jit-core::cir)
   │
   ▼ x86_64-backend::X86_64Backend  (this crate)
Vec<u8> x86-64 machine code
   │
   ▼ code-packager::elf_object / pe_object / macho_object
runnable binary
```

## ABI support

V1 supports **both** ABIs in active use on x86-64 hosts:

| ABI | Used on | Arg regs (≤ N) | Notes |
|---|---|---|---|
| **System V AMD64** | Linux, macOS x86-64, FreeBSD | RDI, RSI, RDX, RCX, R8, R9 (6) | No shadow space; 128-byte red zone (unused in V1) |
| **Microsoft x64** | Windows | RCX, RDX, R8, R9 (4) | 32-byte shadow space allocated by caller; no red zone |

Construct with:

```rust
use x86_64_backend::{X86_64Backend, X86_64Abi};

let backend = X86_64Backend::with_abi(X86_64Abi::SysV);   // Linux
let backend = X86_64Backend::with_abi(X86_64Abi::MsX64);  // Windows
```

`X86_64Backend::new()` defaults to `SysV`.  `twig-aot` will pick the
right ABI based on `--target` (LANG46).

## V1 scope

Matches the aarch64-backend V1 baseline:

| Family | CIR mnemonics |
|---|---|
| Constants | `const_u8` … `const_u64`, `const_i8` … `const_i64`, `const_bool` |
| Integer arithmetic | `add_<ty>`, `sub_<ty>`, `mul_<ty>` |
| Comparisons | `cmp_eq_<ty>`, `cmp_ne_<ty>`, `cmp_lt_<ty>`, `cmp_le_<ty>`, `cmp_gt_<ty>`, `cmp_ge_<ty>` (signed and unsigned) |
| Control flow | `label`, `jmp`, `jmp_if_true`, `jmp_if_false` |
| Returns | `ret_<ty>`, `ret_void` |
| Moves | `mov_<ty>` |
| Type guards | `type_assert` (lowered to `UD2` trap — AOT has no deopt) |

Division, modulo, logical ops, shifts, calls, globals, and `io_out`
are added by subsequent waves (LANG43 phases 4–6).

## Register allocation

V1 uses **stack-spill** allocation: every CIR virtual register lives
at a fixed slot `[rbp - 8 - slot_idx*8]`.  `RAX`, `RCX`, and `RDX` are
reserved as scratch for every instruction emission:

- `RAX` — primary scratch + return register.
- `RCX` — shift-count register (needed for `SHL`/`SHR`/`SAR`-by-CL
  added in phase 4).
- `RDX` — high half of `RDX:RAX` for `IDIV` / `DIV` (phase 4).

Reserving these three up front means later phases never have to
shuffle scratch around to make room for division / shift operands.

## Prologue / epilogue

Both ABIs share the same prologue shape; only the spill arg-register
set differs.

```
;  System V                       MS x64
push rbp                          push rbp
mov  rbp, rsp                     mov  rbp, rsp
sub  rsp, <frame>                 sub  rsp, <frame> + 32   ; +32 shadow space (non-leaf)
mov  [rbp -  8], rdi              mov  [rbp -  8], rcx
mov  [rbp - 16], rsi              mov  [rbp - 16], rdx
mov  [rbp - 24], rdx              mov  [rbp - 24], r8
mov  [rbp - 32], rcx              mov  [rbp - 32], r9
mov  [rbp - 40], r8
mov  [rbp - 48], r9
<body>
mov  rsp, rbp                     mov  rsp, rbp
pop  rbp                          pop  rbp
ret                               ret
```

`<frame>` is chosen so RSP is 16-byte aligned at every `CALL` site.
Since `push rbp` shifts the inherited 8-mod-16 alignment to 0-mod-16,
`<frame> ≡ 0 (mod 16)`.

For Microsoft x64, V1 unconditionally reserves the 32-byte shadow
space in the prologue (treating every function as potentially
non-leaf).  Costs at most 32 bytes of unused stack per leaf function;
removes the need to reserve-per-call.

## Reference

- LANG43 spec (this repo)
- *System V Application Binary Interface, AMD64 Architecture Processor
  Supplement* — Matz et al.
- Microsoft Docs: "x64 calling convention"
