# LANG44 — `x86_64-encoder`: X86-64 Instruction Encoder

**Status:** Draft — 2026-05-14

> The x86-64 port spans four specs.  Reading order is encoder → backend
> → object format → twig-aot driver:
>
> - **LANG44** (this) — pure instruction encoder.  ABI-agnostic.
> - **LANG43** — CIR → x86-64 lowering, with both **System V** (Linux)
>   and **Microsoft x64** (Windows) ABIs in V1.
> - **LANG45** — object-file emitters (`elf_object.rs`, `pe_object.rs`)
>   that wrap the encoder's `.text` bytes with relocations the system
>   linker on each OS expects.
> - **LANG46** — `twig-aot` multi-target dispatch, per-target runtime
>   archive build, and linker integration (`cc` on Linux,
>   `link.exe` / `lld-link` on Windows).
>
> Number LANG41 in this repo is already claimed by the AOT runtime
> library work (twig-aot 0.1.8 etc.) — that's why this spec lands at
> LANG44.

## Motivation

The LANG VM today has exactly one mature native backend: `aarch64-backend`
on top of `aarch64-encoder`.  ARM64 covers Apple Silicon and modern Linux
ARM hosts, but the *most common* desktop and server architecture in the
field remains x86-64 (AMD64 / Intel 64).  CI runners, default cloud VMs,
and the majority of contributor laptops run x86-64.

To bring AOT and JIT compilation to those hosts, the LANG VM needs an
x86-64 backend.  Following the established two-package pattern, this
spec defines the lower of the two layers: a pure instruction encoder
that produces raw `.text` bytes for the x86-64 instruction set.

The companion spec **LANG43** defines `x86_64-backend`, which lowers
CIR onto the encoder.  This spec stops at "given a method like
`mov_r64_imm64(Reg::Rax, 42)`, what bytes come out?"

The encoder is **OS-agnostic and ABI-agnostic** — the same bytes
encode on Linux, Windows, and macOS.  ABI choice (which registers
hold arguments, whether a shadow space exists) lives in the backend.
Object-format choice (ELF vs PE/COFF, which relocation type IDs to
use) lives in `code-packager` per LANG45.

## Non-goals

- **Lowering**: no CIR awareness.  That belongs in `x86_64-backend`.
- **Packaging**: no ELF / Mach-O / COFF wrappers.  `code-packager`
  handles that.
- **Relocations beyond branch fixups**: external symbol relocations
  (CALL to runtime, RIP-relative globals) are surfaced as opaque slots
  that the backend records and the packager resolves.  Branch
  destinations *internal* to a function are resolved at `finish()`
  time, exactly like in `aarch64-encoder`.
- **AVX/SSE/x87**: floats and SIMD are out of scope for V1 (the
  AArch64 encoder also defers floats).
- **16-bit / 32-bit legacy modes**: only 64-bit (long) mode is
  encoded.
- **Microsoft x64 ABI quirks**: 32-byte home/shadow stack space,
  unwind data tables (`.pdata` / `.xdata` in PE/COFF) — those are
  layered above the encoder.  The encoder produces the same bytes
  regardless of host OS; the choice of ABI is a *backend* concern
  (LANG43); object-format reloc IDs are a *packager* concern (LANG45).

## ISA reference

All encodings cite **Intel® 64 and IA-32 Architectures Software
Developer's Manual, Volume 2** (Vol. 2A/2B/2C/2D, instruction set
reference) and **AMD64 Architecture Programmer's Manual, Volume 3**.
The encoder uses the *Intel* mnemonic forms in API names; bit layouts
follow the Intel manual.

## Why x86-64 is harder than ARM64 to encode

| Aspect | ARM64 | x86-64 |
|---|---|---|
| Instruction width | Fixed 4 bytes | Variable 1–15 bytes |
| Operand encoding | Bit-packed fields | Prefixes + opcode + ModR/M + SIB + disp + imm |
| Register file | 31 GPRs, regular numbering | 16 GPRs (8 legacy + 8 via REX.R/REX.B), irregular high/low byte aliases |
| Immediate forms | One per family, sign-extended | Multiple sizes per family, varied sign/zero-extension |
| Special clobbers | None for arithmetic | `IDIV` clobbers RDX:RAX; shifts use CL; CMPS uses RSI/RDI |
| PC-relative addressing | Explicit (ADRP/ADR) | Implicit via `[RIP + disp32]` ModR/M form |
| Alignment | 4-byte instructions | Bytes anywhere — no alignment to worry about, but jump targets can land mid-instruction (we don't) |

The encoder's job is to hide as much of this as possible behind a
typed API, while remaining a thin shim — no register allocation, no
peephole optimisation, just bytes.

---

## Package layout

```
code/packages/rust/x86_64-encoder/
├── Cargo.toml          # zero external deps (matches aarch64-encoder)
├── README.md
├── CHANGELOG.md
└── src/
    └── lib.rs
```

## Public API surface

The shape mirrors `aarch64_encoder` so a reader who knows that crate
can navigate this one immediately.

### Registers

64-bit GPR encoding:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reg {
    Rax, Rcx, Rdx, Rbx,
    Rsp, Rbp, Rsi, Rdi,
    R8,  R9,  R10, R11,
    R12, R13, R14, R15,
}
```

Notes:

- Encoding order is the Intel ModR/M numeric order (`RAX=0, RCX=1,
  RDX=2, RBX=3, RSP=4, RBP=5, RSI=6, RDI=7`).  R8..R15 are `0..7`
  *plus* a REX bit (R/B/X extension).
- The encoder normalises the REX bit internally; callers always pass
  the logical register.
- `RSP` and `RBP` have special ModR/M behaviour (`mod=00 r/m=100`
  forces SIB; `mod=00 r/m=101` forces RIP-relative).  The encoder
  emits a SIB or non-zero displacement to side-step these rules
  whenever the caller picks `RSP`/`RBP` as a base.

8-bit register low halves are not exposed in V1.  We sign- or
zero-extend during 8/16/32-bit ops by clearing the upper bits via the
64-bit form (e.g., `mov r/m32, ...` zeroes the upper 32 bits as a
matter of the ISA).

### Condition codes

x86-64 condition codes are 4-bit tags on `Jcc` / `SETcc` /
`CMOVcc`.  We mirror the ARM `Cond` enum:

```rust
pub enum Cond {
    O = 0x0, No = 0x1,
    B = 0x2, Ae = 0x3,           // unsigned below / above-or-equal
    E = 0x4, Ne = 0x5,
    Be = 0x6, A = 0x7,           // unsigned below-or-equal / above
    S = 0x8, Ns = 0x9,
    P = 0xA, Np = 0xB,
    L = 0xC, Ge = 0xD,           // signed less / greater-or-equal
    Le = 0xE, G = 0xF,
}
```

### Labels and fixups

Verbatim copy of the AArch64 design: `LabelId`, `create_label`,
`bind`, internal `Fixup { word_idx, target, kind }`, resolved in
`finish()`.

x86-64 has *two* near-branch displacement widths we care about:

- `Rel8`  — `Jcc rel8`, `JMP rel8` (signed -128..+127, 1 byte)
- `Rel32` — `Jcc rel32`, `JMP rel32`, `CALL rel32` (signed ±2 GiB)

V1 emits **Rel32 forms only**, even for tiny forward jumps.  This
wastes 4 bytes per branch versus the optimal short form, but avoids
the two-pass shortening problem entirely.  A future PR can add a
relaxation pass; the cost is bounded and predictable.

### Errors

```rust
pub enum EncodeError {
    UnboundLabel(LabelId),
    LabelAlreadyBound(LabelId),
    ImmediateOutOfRange { op: &'static str, bits: u32, value: i64 },
    BranchOutOfRange { bits: u32, delta_bytes: i64 },
    // x86-64-specific:
    InvalidEffectiveAddress { reason: &'static str },
}
```

The `BranchOutOfRange.delta_bytes` is in *bytes*, not words — x86-64
displacements are byte counts.

### External relocations

`ExternalReloc` mirrors the AArch64 encoder's analogue and tracks the
information a packager needs to patch a `CALL rel32` or
`MOV r64, [RIP+disp32]` later:

```rust
pub struct ExternalReloc {
    /// Byte offset in `.text` where the 32-bit field lives.
    pub patch_offset: usize,
    /// Logical symbol name (e.g., "__twig_print_i64").
    pub symbol: String,
    /// Reloc kind: PLT call, RIP-relative load, GOT load, …
    pub kind: ExternalRelocKind,
    /// Offset adjustment applied to the resolved address before patching.
    pub addend: i32,
}

pub enum ExternalRelocKind {
    /// PC-relative branch.  Maps to `R_X86_64_PLT32` on ELF,
    /// `IMAGE_REL_AMD64_REL32` on PE/COFF, `X86_64_RELOC_BRANCH` on
    /// Mach-O.  Used by `CALL rel32` to external functions.
    PltRel32,
    /// PC-relative 32-bit displacement.  Maps to `R_X86_64_PC32` on
    /// ELF, `IMAGE_REL_AMD64_REL32` on PE/COFF, `X86_64_RELOC_SIGNED`
    /// on Mach-O.  Used for RIP-relative loads/stores of globals when
    /// the symbol resolves locally.
    PcRel32,
    /// RIP-relative GOT load.  Maps to `R_X86_64_REX_GOTPCRELX` on
    /// ELF.  On PE/COFF and Mach-O this collapses to a regular
    /// PC-relative reloc (no separate GOT).  Used for possibly-external
    /// globals.
    GotPcRel32,
}
```

The encoder produces the same 32-bit slot regardless of which kind is
chosen; the packager (LANG45) translates the abstract kind to the
right OS-specific relocation type ID at emit time.

---

## Coverage (V1)

V1 covers exactly the same logical surface as `aarch64-encoder` after
LANG38 + LANG39 + LANG40 lands, so the backend has a one-to-one
mapping target.

| Family | Methods | Notes |
|---|---|---|
| Move immediate | `mov_r64_imm32`, `mov_r64_imm64` | `mov r64, imm32` zero-extends; `MOVABS r64, imm64` for full-width |
| Move register | `mov_r64_r64` | `MOV r/m64, r64` form |
| Arithmetic (reg) | `add`, `sub`, `imul` | 64-bit `r/m64, r64` and `r/m64, r/m64` |
| Arithmetic (imm) | `add_imm32`, `sub_imm32` | Sign-extended imm32 |
| Division | `idiv`, `div` | Implicit RAX/RDX dividend; backend sequences `cqo`/`xor rdx,rdx` |
| Sign extension | `cqo` | RAX → RDX:RAX before `IDIV` |
| Compare | `cmp`, `cmp_imm32` | `CMP` is `SUB` with discarded result |
| Logical (reg) | `and_`, `or_`, `xor_`, `not_` | 64-bit |
| Shifts (variable) | `shl_cl`, `shr_cl`, `sar_cl` | Shift amount in CL only (x86 ISA constraint) |
| Shifts (imm) | `shl_imm8`, `shr_imm8`, `sar_imm8` | 8-bit immediate |
| Unary | `neg_` | Two's-complement negate |
| Set on condition | `setcc(Cond, Reg)` | 8-bit destination; zero-extend with `movzx` from same reg |
| Memory (load) | `mov_r64_mem` | `MOV r64, [base + disp32]` and `[RIP + disp32]` |
| Memory (store) | `mov_mem_r64` | `MOV [base + disp32], r64` |
| Memory (8-bit store) | `mov_mem8_imm8` | For `io_out` newline emission |
| Stack | `push`, `pop` | 64-bit, for prologue/epilogue |
| RIP-relative | `lea_rip_rel(Reg, Reloc)` | Emits `LEA r64, [RIP+disp32]` with external reloc |
| Branch (cond) | `jcc(Cond, LabelId)` | `Jcc rel32` |
| Branch (uncond) | `jmp(LabelId)` | `JMP rel32` |
| Call (direct) | `call_rel32(Reloc)` | `CALL rel32` with external reloc |
| Call (indirect) | `call_r64`, `ret` | For function-pointer dispatch and return |
| Misc | `nop`, `ud2`, `int3` | Trap (`ud2`) for `type_assert` deopt |

Everything outside this list is deferred.  The backend returns
`BackendRefused` for opcodes the encoder cannot emit, exactly like the
AArch64 path.

### Truth tables — small but worth fixing in the spec

**Calling-convention argument registers (System V AMD64):**

| Arg # | Register |
|---|---|
| 1 | RDI |
| 2 | RSI |
| 3 | RDX |
| 4 | RCX |
| 5 | R8 |
| 6 | R9 |
| 7+ | Stack (right-to-left, 8-byte slots) |

**Calling-convention argument registers (Microsoft x64):**

| Arg # | Register |
|---|---|
| 1 | RCX |
| 2 | RDX |
| 3 | R8 |
| 4 | R9 |
| 5+ | Stack (with 32-byte "home" shadow space) |

The encoder does not care about the ABI — these tables are
documented here for the backend's reference.

---

## Encoding strategy

x86-64 instructions are composed of up to five logical pieces:

```
[ legacy prefixes (0-4) ] [ REX (0-1) ] [ opcode (1-3) ] [ ModR/M (0-1) ] [ SIB (0-1) ] [ displacement (0/1/2/4) ] [ immediate (0/1/2/4/8) ]
```

Internal helpers (private):

```rust
fn rex(w: bool, r: bool, x: bool, b: bool) -> u8;
fn modrm(mode: u8, reg: u8, rm: u8) -> u8;
fn sib(scale: u8, index: u8, base: u8) -> u8;
```

These are pure functions; encoding remains testable in isolation.

### REX prefix

For 64-bit GPR ops we always emit `REX.W = 1` (`0x48` base).  Bits:

- `W` = 1: 64-bit operand size
- `R`: extension of ModR/M.reg (set when reg ≥ R8)
- `X`: extension of SIB.index (set when index ≥ R8)
- `B`: extension of ModR/M.r/m or SIB.base or opcode reg field (set when reg/base ≥ R8)

The encoder omits REX when no extension is needed *and* the operand
size matches the default (rare; for V1 we just always emit the REX).

### ModR/M `mod` field summary

| `mod` | Meaning |
|---|---|
| `00` | `[base]` (no displacement) — but `r/m=101` means `[RIP + disp32]`; `r/m=100` requires SIB |
| `01` | `[base + disp8]` |
| `10` | `[base + disp32]` |
| `11` | Pure register operand |

### Variable-length tradeoffs

Two simple rules keep us safe:

1. **Always use the long form for forward branches** — `Jcc rel32`,
   `JMP rel32`, `CALL rel32`.  Never `Jcc rel8`.  Removes the need
   for backpatching width.
2. **Always use disp32 for stack-frame accesses** — even when disp8
   would suffice.  Costs 3 extra bytes per access, eliminates a
   width-selection branch in the encoder.

These two rules together mean every instruction's byte length is
known the moment its first byte is emitted, so label resolution is a
straight "patch the rel32 field at the recorded offset" pass.

---

## Bit layouts (worked examples)

These examples are sized to give an implementer immediate traction.
Reference: Intel SDM Vol. 2.

### `MOV r/m64, r64`  — `mov_r64_r64(dst, src)`

Opcode `0x89 /r`.

```
[REX.W=1, R=src.high, B=dst.high] [0x89] [ModR/M(mod=11, reg=src.low3, rm=dst.low3)]
```

For `MOV RAX, RDI`: `48 89 F8`.

### `ADD r/m64, r64`  — `add(dst, src)`

Opcode `0x01 /r`.  Same encoding shape as MOV.

For `ADD RAX, RCX`: `48 01 C8`.

### `ADD r/m64, imm32` (sign-extended)  — `add_imm32(dst, imm)`

Opcode `0x81 /0`.  ModR/M reg field is the opcode extension `0`.

```
[REX.W=1, B=dst.high] [0x81] [ModR/M(mod=11, reg=0, rm=dst.low3)] [imm32 LE]
```

For `ADD RAX, 1`: `48 83 C0 01` (note: `0x83` is the 8-bit-imm sign-extended
variant, which we *also* use when the immediate fits in `i8`).  V1
always emits the `0x81` form to keep encoding deterministic.

### `MOV r64, imm64`  — `mov_r64_imm64(dst, imm)`

Opcode `B8+rd`.  The destination register is encoded in the opcode
byte's low 3 bits, with the high bit going into `REX.B`.

```
[REX.W=1, B=dst.high] [B8 + dst.low3] [imm64 LE]
```

For `MOV RAX, 0x1234567890ABCDEF`: `48 B8 EF CD AB 90 78 56 34 12`.

### `CMP r/m64, r64`  — `cmp(lhs, rhs)`

Opcode `0x39 /r`.  Like `SUB` but discards the result, sets flags.

For `CMP RAX, RCX`: `48 39 C8`.

### `Jcc rel32`  — `jcc(cond, label)`

Two-byte opcode `0F 80+cc`.

```
[0F] [80 + cond_code] [rel32 LE — patched at finish()]
```

For `JE label`: `0F 84 ?? ?? ?? ??` with the `??` bytes recorded as a
fixup.

### `CALL rel32`  — `call_rel32(reloc)`

Opcode `0xE8`.

```
[E8] [rel32 LE — patched by packager with PLT-relative offset]
```

The encoder emits `E8 00 00 00 00` and records an `ExternalReloc {
patch_offset = current+1, kind = PltRel32, symbol, addend = -4 }`.
The `-4` addend accounts for x86-64 PC-relative being relative to
the *end* of the instruction.

### `LEA r64, [RIP + disp32]`  — `lea_rip_rel(dst, reloc)`

Opcode `0x8D /r` with `mod=00, r/m=101` (RIP-relative form).

```
[REX.W=1, R=dst.high] [8D] [ModR/M(mod=00, reg=dst.low3, rm=101)] [disp32 LE — patched]
```

Emits `48 8D 05 00 00 00 00` for `LEA RAX, [rel]` and records a
`PcRel32` reloc.

### `PUSH r64` / `POP r64`

`50 + rd` / `58 + rd`.  REX.B extends the register.  No ModR/M needed.

For `PUSH RBP`: `55`.  For `PUSH R15`: `41 57`.

### `RET`

Opcode `0xC3`.  Single byte.

### `UD2` (deopt trap)

Opcode `0x0F 0x0B`.  Two bytes.  Used to lower `type_assert` (no
deopt in AOT — just trap; matches what AArch64 does with `UDF`).

---

## Test plan

Mirror `aarch64-encoder`'s suite.  Coverage target ≥ 95%.

1. **Per-instruction byte-exact tests** — every public method has at
   least one test that asserts the encoded byte string against a
   hand-verified reference (cross-checked with `objdump -d` /
   `llvm-mc` output during development).
2. **Register-extension tests** — emit the same instruction with
   `RAX` and with `R15`; assert REX.B flips correctly and the low 3
   bits stay the same.
3. **Label resolution** — emit `JMP fwd; ... ; bind(fwd); RET`,
   `finish()`, decode the rel32 field, assert it equals the byte
   delta from the end of the JMP to the RET.
4. **Unbound-label error** — `finish()` returns `UnboundLabel`.
5. **External relocation recording** — `call_rel32(reloc)` records
   `patch_offset` pointing at the `rel32` slot, not at the opcode.
6. **Negative-immediate sign-extension** — `add_imm32(rax, -1)`
   encodes as `48 81 C0 FF FF FF FF` and the value sign-extends to
   `-1` when executed.
7. **Round-trip with `iced-x86`** — for a subset of instructions,
   decode the emitted bytes with the `iced-x86` crate (dev-only
   dependency) and assert the decoded form matches the expected
   mnemonic + operands.  Pure validation tool; not shipped in the
   crate's regular dependency tree.

`iced-x86` lives behind `#[cfg(test)]` only — production builds keep
the zero-dep guarantee.

---

## Out of scope (deferred to follow-up specs)

- **SSE/AVX float ops** — needed eventually for `f32`/`f64`; mirrors
  the "floats NOT YET" line in `aarch64-encoder`.
- **Short branch relaxation** (rel8 forms) — pure size optimisation.
- **AVX-512** — not on the V1 path.
- **Atomic ops (LOCK prefix)** — needed for `gc-core` interactions
  later.
- **Implicit-CL handling** — caller pre-loads `CL` before
  `shl_cl`/`shr_cl`/`sar_cl`; the encoder does not insert it.
- **REX prefix omission optimisation** — V1 always emits REX for 64-bit
  ops, even when redundant; minor size cost.
