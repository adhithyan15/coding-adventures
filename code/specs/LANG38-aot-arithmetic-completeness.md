# LANG38 — AOT Arithmetic Completeness

**Status:** Implemented — 2026-05-13

## Motivation

The `aarch64-backend` V1 only lowers `add`, `sub`, `mul`, and comparisons.
Programs that use division, modulo, negation, or any bitwise operation fail
at AOT compile time with `BackendRefused`.

This blocks a self-hosted Twig compiler, which needs integer division (for
number parsing and address arithmetic), modulo (for hash tables and character
classification), and bitwise operations (for flag manipulation).

This spec extends `aarch64-encoder` with the missing instruction families and
wires them through `aarch64-backend` as CIR opcode handlers.

---

## New ARM64 instructions in `aarch64-encoder`

All are 64-bit (sf=1) register-register forms.  Immediate forms are out of
scope — the stack-spill allocator materialises every constant in a register
anyway.

| Method | ARM64 mnemonic | Encoding base | Notes |
|--------|---------------|---------------|-------|
| `sdiv(rd, rn, rm)` | `SDIV Xd, Xn, Xm` | `0x9AC00C00` | Signed 64-bit divide |
| `udiv(rd, rn, rm)` | `UDIV Xd, Xn, Xm` | `0x9AC00800` | Unsigned 64-bit divide |
| `msub(rd, rn, rm, ra)` | `MSUB Xd, Xn, Xm, Xa` | `0x9B008000` | `Xd = Xa − Xn×Xm`; used for modulo |
| `and_(rd, rn, rm)` | `AND Xd, Xn, Xm` | `0x8A000000` | Logical AND |
| `orr(rd, rn, rm)` | `ORR Xd, Xn, Xm` | `0xAA000000` | Logical OR |
| `eor(rd, rn, rm)` | `EOR Xd, Xn, Xm` | `0xCA000000` | Logical XOR |
| `lsl_reg(rd, rn, rm)` | `LSLV Xd, Xn, Xm` | `0x9AC02000` | Shift left by variable |
| `lsr_reg(rd, rn, rm)` | `LSRV Xd, Xn, Xm` | `0x9AC02400` | Shift right logical (unsigned) |
| `asr_reg(rd, rn, rm)` | `ASRV Xd, Xn, Xm` | `0x9AC02800` | Shift right arithmetic (signed) |
| `neg_(rd, rm)` | `NEG Xd, Xm` | `0xCB0003E0` | Alias `SUB Xd, XZR, Xm` |
| `mvn(rd, rm)` | `MVN Xd, Xm` | `0xAA2003E0` | Alias `ORN Xd, XZR, Xm` (bitwise NOT) |

### Encoding derivations (ARM ARM DDI 0487)

**Data-processing (2-source) family** — base `0x9AC00000` (`sf=1, op=00,
op2=11010110`):
- Opcode field at bits [15:10]
- `UDIV` opcode = `000010` = 2 → `0x9AC00800`
- `SDIV` opcode = `000011` = 3 → `0x9AC00C00`
- `LSLV` opcode = `001000` = 8 → `0x9AC02000`
- `LSRV` opcode = `001001` = 9 → `0x9AC02400`
- `ASRV` opcode = `001010` = 10 → `0x9AC02800`

**Data-processing (3-source) family** — `MSUB` is `sf=1, op54=00, op31=000,
o0=1`:
- Base `0x9B008000`
- Layout: `| base | Rm[20:16] | Ra[14:10] | Rn[9:5] | Rd[4:0] |`

**Logical (shifted register) family** — `sf=1, shift=LSL#0`:
- `AND` opc=00, N=0: base `0x8A000000`
- `ORR` opc=01, N=0: base `0xAA000000`
- `EOR` opc=10, N=0: base `0xCA000000`
- `ORN` opc=01, N=1: base `0xAA200000`

**`NEG Xd, Xm`** = `SUB Xd, XZR, Xm` (arithmetic shifted-reg, sf=1, opc=11):
- Base `0xCB000000` with Rn=XZR (31):
  `0xCB000000 | (Rm << 16) | (31 << 5) | Rd`
  = `0xCB0003E0 | (Rm << 16) | Rd`

**`MVN Xd, Xm`** = `ORN Xd, XZR, Xm`:
- `0xAA200000 | (Rm << 16) | (31 << 5) | Rd`
  = `0xAA2003E0 | (Rm << 16) | Rd`

---

## New CIR opcode handlers in `aarch64-backend`

### Division family

`div_<ty>` lowers to `SDIV` (signed types: `i8`…`i64`) or `UDIV` (unsigned).

### Modulo family

`mod_<ty>` uses the classic two-instruction sequence:

```
SDIV X2, X0, X1      ; X2 = quotient = a / b
MSUB X0, X2, X1, X0  ; X0 = X0 - X2*X1 = a - (a/b)*b = a mod b
```

X2 is used as an intermediate scratch register (not live across instructions
in the stack-spill allocator, so reuse is safe).

### Bitwise family

| CIR opcode | Instruction |
|------------|-------------|
| `and_<ty>` | `AND Xd, Xn, Xm` |
| `or_<ty>` | `ORR Xd, Xn, Xm` |
| `xor_<ty>` | `EOR Xd, Xn, Xm` |
| `shl_<ty>` | `LSLV Xd, Xn, Xm` |
| `shr_<ty>` | `ASRV` (signed `i*`) or `LSRV` (unsigned `u*`) |

### Unary family

| CIR opcode | Instruction |
|------------|-------------|
| `neg_<ty>` | `NEG Xd, Xm` |
| `not_<ty>` | `MVN Xd, Xm` |

---

## Type suffix semantics

All new CIR handlers use the same signed/unsigned detection already used by
comparisons: `ty.starts_with('i')` = signed → arithmetic instructions; otherwise
unsigned.  Examples:
- `div_i64` → `SDIV`
- `div_u32` → `UDIV`
- `shr_i64` → `ASRV` (arithmetic / sign-extending)
- `shr_u64` → `LSRV` (logical / zero-filling)

---

## Tests

### `aarch64-encoder`

One test per new method asserting the correct 4-byte encoding:
- `sdiv_encoding`, `udiv_encoding`, `msub_encoding`
- `and_encoding`, `orr_encoding`, `eor_encoding`
- `lsl_reg_encoding`, `lsr_reg_encoding`, `asr_reg_encoding`
- `neg_encoding`, `mvn_encoding`

### `aarch64-backend`

Integration tests (hand-built CIR, compile + structural checks):
- `div_i64_lowers` — `div_i64 dest, a, b` produces a non-empty byte stream
- `mod_i64_lowers` — `mod_i64 dest, a, b` produces a non-empty byte stream
- `and_i64_lowers`, `or_i64_lowers`, `xor_i64_lowers`
- `shl_i64_lowers`, `shr_i64_lowers`, `shr_u64_lowers` (signed vs unsigned)
- `neg_i64_lowers`, `not_i64_lowers`
- `unsupported_float_still_returns_none` — regression

---

## Packages changed

| Package | Semver bump | Reason |
|---------|-------------|--------|
| `aarch64-encoder` | 0.1.0 → 0.2.0 | new public methods (additive minor) |
| `aarch64-backend` | 0.1.2 → 0.2.0 | new CIR opcodes handled (additive minor) |

---

## Out of scope

- Float arithmetic (separate PR)
- Width-masking for u8/u16/u32 results (future optimizer pass)
- Global variable opcodes (`global_load` / `global_store`) — LANG39
- Closure opcodes (`alloc_closure` / `call_closure`) — LANG40
