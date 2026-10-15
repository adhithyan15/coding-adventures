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

## Bounded native word-call validation

Internal integer-word direct calls and function parameters now support
up to eight words on MsX64 and SysV. A fixed aligned outgoing tail follows
shadow space and stays below all negative RBP locals; callee extras spill from
RBP+16+shadow+8index. Frame/displacement arithmetic is checked; nine arguments
or formals refuse. Builtin physical-register marshalling is unchanged. Exact
both-ABI simulator controls check5/6/7/8 order, fullRAX, stackwords, shadow
clobbering, preserved locals, alignment/restoration, recursion and refusal.
Root metadata retainsp7 RBP-64 and actual external returnPC. Real Windows
six/eight-position and eight original captured/forwarded array rows execute
Exit42. Simulator evidence does not establish Linux/macOS hardware or forced
GC execution, and this bounded internal convention does not claim arbitrary
arity or native C floating argument interoperability.

Actual isolated source316e7878 validation completed: nine affected packages
1226PASS2FAIL1existingignore/81targets; only the obsolete five/seven-formal
refusal oracles failed. Full unfiltered LANG476/0/0, strict nine-package
alltargets, standalone Closure1093/0/0 across181targets and strict passed.
Spec-first test-only correction7126217d retains those accepted counts and eight,
adds exact ABI/got9/max8 plus public-refusal checks. New complete backend
85/0/0 over2 targets and strict nine-package alltargets passed.
Whole production prefix and every other Rust/instruction object remain exact;
unchanged runtime evidence is explicitly attributed to316e. These counts are
separate source receipts, not additive, and the original failed run stays failed.
The prior5ba run exhausted disk before tests; its failed receipts are preserved.
Retry disabled debug symbols/incremental compilation, preserving all inputs,
assertions and limits. Registry376 tuples remain exact. Miri
**DEFERRED/PENDING**, never passed; frozen run/input/target/receipts untouched.
General-hop32 resource debt, hosted CI/conflict/review/merge, full Closure parity
and output-source/full transformation lineage remain OPEN.
