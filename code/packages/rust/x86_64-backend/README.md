# x86_64-backend

Register the two-argument str_index checked runtime builtin in the generic ABI table.

Register the three-argument str_slice runtime builtin for generic ABI marshalling.

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

## Scope

Matches the aarch64-backend baseline through LANG38:

| Family | CIR mnemonics |
|---|---|
| Constants | `const_u8` … `const_u64`, `const_i8` … `const_i64`, `const_bool` |
| Integer arithmetic | `add_<ty>`, `sub_<ty>`, `mul_<ty>` |
| Division | `div_<ty>` (IDIV / DIV via RDX:RAX), `mod_<ty>` |
| Logical | `and_<ty>`, `or_<ty>`, `xor_<ty>`, `not_<ty>` |
| Shifts | `shl_<ty>`, `shr_<ty>` (SAR for `i*`, SHR for `u*`) |
| Unary | `neg_<ty>` |
| Calls | `call callee_name, arg0, …, argN` — `CALL rel32` (self-recursive: label fixup; cross-function: `PltRel32` external reloc) |
| Globals | `global_load name → dest`, `global_store name, val` — `LEA r64, [RIP+_twig_globals]` + `MOV r64, [r64 + slot*8]` (`PcRel32` reloc) |
| I/O | `io_out val` — load arg into RDI (SysV) / RCX (MS x64), `CALL __twig_print_i64` (`PltRel32` reloc) |
| Comparisons | `cmp_eq_<ty>`, `cmp_ne_<ty>`, `cmp_lt_<ty>`, `cmp_le_<ty>`, `cmp_gt_<ty>`, `cmp_ge_<ty>` (signed and unsigned) |
| Control flow | `label`, `jmp`, `jmp_if_true`, `jmp_if_false` |
| Returns | `ret_<ty>`, `ret_void` |
| Moves | `mov_<ty>` |
| Type guards | `type_assert` (lowered to `UD2` trap — AOT has no deopt) |
| Lisp runtime (LANG77) | `call_builtin "lispy_*"` → `CALL __twig_lispy_*` — cons/car/cdr, ATOM/EQ predicates, COND truthiness, and `lispy_to_exit_code` (W14b — the polymorphic lambda-result exit coercion) |


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

### Arguments beyond the registers

Calls and functions may take more arguments than the ABI's integer
registers (six for SysV, four for Microsoft x64). The overflow arguments
travel on the stack in the standard layout:

```
; caller, for a call with n_stack overflow arguments
sub  rsp, align16(shadow + 8 * n_stack)
mov  rax, <arg k>  ;  mov [rsp + shadow + 8*k], rax     ; for each k
<load register arguments>
call callee
add  rsp, align16(shadow + 8 * n_stack)

; callee prologue, after the register spills
mov  rax, [rbp + 16 + shadow + 8*k]  ;  mov [rbp - slot], rax
```

`shadow` is 0 on SysV and 32 on Microsoft x64. RAX is the copy scratch
because it is never an argument register. Every source lives in an
`rbp`-relative slot, so moving `rsp` for the outgoing area does not
disturb them.

For Microsoft x64, V1 unconditionally reserves the 32-byte shadow
space in the prologue (treating every function as potentially
non-leaf).  Costs at most 32 bytes of unused stack per leaf function;
removes the need to reserve-per-call.

## Reference

- LANG43 spec (this repo)
- *System V Application Binary Interface, AMD64 Architecture Processor
  Supplement* — Matz et al.
- Microsoft Docs: "x64 calling convention"

FLOW-MATIC EOF support (VM-039a): `input_more()` uses a non-consuming stdio
peek alongside `input_i64`. The native/LLVM input stream proof includes empty
input and repeated EOF reads; this does not change existing integer parsing.

## LANG83 operation support

`x86_64_backend::op_support::op_support(name)` (and `OP_SUPPORT`) is this backend's answer,
`Lowered` or `Refused(reason)`, for every operation in the
[LANG83](../../../specs/LANG83-iir-standard-operation-registry.md) standard registry
(`interpreter_ir::op_registry::STANDARD_OPS`). "Lowered" means the native AOT pipeline that ends in this backend (twig-aot's IIR rewriting, then `jit_core::specialise`, then this crate) compiles the op. This crate dispatches CIR ops by name prefix (`div_`, `add_`, …), which is why LANG83 requires new op names to be type-first. `lang-aot` refuses a
module that uses a refused op at compile time, with a typed error. The four
LANG83 PR1b ops (`write_i64`, `i64_div_floor`, `i64_mod_floor`, `trap`) are
refused until PR1b lowers them.
