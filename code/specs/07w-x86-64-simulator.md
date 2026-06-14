# Layer 07w — x86-64 (AMD64, 2003) Behavioral Simulator

## Overview

x86-64 (also called AMD64 or Intel 64 / EM64T) is the 64-bit extension of the
x86 instruction set architecture.  It is the dominant architecture for servers,
desktops, and laptops as of 2026.  Virtually every cloud VM, CI runner, and
gaming PC runs x86-64.

**How it came to be.**  Intel owned the x86 architecture through the 32-bit era
(8086 → 80286 → 80386 → Pentium series).  When the industry needed to move to
64-bit, Intel's initial answer was the Itanium (IA-64), a radical new VLIW ISA
that required recompilation and offered poor 32-bit compatibility.  AMD took a
different approach: extend x86-32 to 64-bit in a way that was fully backwards
compatible and could run existing 32-bit binaries natively.  The result was
AMD64, first shipped in the Opteron server chip in April 2003.  The design was
so successful that Intel adopted it under the name EM64T (Extended Memory 64
Technology) in the Pentium 4 Prescott in 2004.  Today the ISA is simply called
"x86-64" or "AMD64" in the industry.

**Historical significance:**
- AMD64 Opteron (2003) — first 64-bit x86 processor; gave AMD a temporary
  competitive lead over Intel in the server market
- Intel EM64T / Core 2 (2006) — Intel's x86-64 take-over; AMD64 became the
  industry baseline
- x86-64 is the target ISA for Linux, Windows, macOS on conventional hardware
- Dominant ISA for compiler back-ends: GCC, Clang, MSVC all default to x86-64
- LLVM's `x86_64-unknown-linux-gnu` triple is the most-tested in the world
- Critical to understand because most compiled code ultimately runs on it

**Key design choices:**
- **64-bit long mode only**: the full x86 ISA also includes real mode (DOS era),
  protected mode (32-bit), and compatibility mode (32-bit programs in a 64-bit
  OS).  This simulator implements only 64-bit long mode — the mode that operating
  systems use for 64-bit applications.
- **REX prefix for register extension**: the original x86 had 8 GPRs.  AMD64
  adds 8 more (R8–R15) encoded via a new REX prefix byte.
- **RIP-relative addressing**: a powerful new mode that lets code reference
  data via `[RIP + disp32]`, enabling position-independent code without
  needing a base register.
- **Flat 64-bit virtual address space**: segments are mostly vestigial.
  FS and GS are used for thread-local storage but otherwise segments are NOP.
- **64-bit immediates**: `MOV r64, imm64` can load a full 64-bit constant —
  useful for loading absolute addresses.

---

## Architecture

### Register File

```
Name      Width  Index  Traditional role
──────────────────────────────────────────────────────────────────
RAX       64-bit   0    Accumulator; function return value
RCX       64-bit   1    Counter (LOOP, REP string ops)
RDX       64-bit   2    Data; I/O port address; IDIV/MUL high half
RBX       64-bit   3    Base; callee-saved
RSP       64-bit   4    Stack pointer (grows downward)
RBP       64-bit   5    Frame pointer; callee-saved
RSI       64-bit   6    Source index (string ops)
RDI       64-bit   7    Destination index (string ops)
R8        64-bit   8    Extra GPR; needs REX.R / REX.B
R9        64-bit   9    Extra GPR; needs REX.R / REX.B
R10       64-bit  10    Extra GPR; needs REX.R / REX.B
R11       64-bit  11    Extra GPR; needs REX.R / REX.B
R12       64-bit  12    Extra GPR; callee-saved; needs REX.R / REX.B
R13       64-bit  13    Extra GPR; callee-saved; needs REX.R / REX.B
R14       64-bit  14    Extra GPR; callee-saved; needs REX.R / REX.B
R15       64-bit  15    Extra GPR; callee-saved; needs REX.R / REX.B
──────────────────────────────────────────────────────────────────
RIP       64-bit   —    Instruction pointer (not a GPR)
RFLAGS    64-bit   —    Condition flags (OF SF ZF PF CF tracked)
```

#### Sub-register views

Each 64-bit GPR has narrower aliases.  Writing a 32-bit alias zeros the upper
32 bits of the full 64-bit register.  Writing an 8-bit or 16-bit alias leaves
the other bits unchanged — except writing an 8-bit register when a REX prefix
is present changes the accessible set.

```
64-bit   32-bit   16-bit   8-bit (REX)   8-bit (no REX)
───────────────────────────────────────────────────────
RAX      EAX      AX       AL            AL
RCX      ECX      CX       CL            CL
RDX      EDX      DX       DL            DL
RBX      EBX      BX       BL            BL
RSP      ESP      SP       SPL           AH  (high byte of AX)
RBP      EBP      BP       BPL           CH  (high byte of CX)
RSI      ESI      SI       SIL           DH  (high byte of DX)
RDI      EDI      DI       DIL           BH  (high byte of BX)
R8       R8D      R8W      R8B           —
R9       R9D      R9W      R9B           —
R10      R10D     R10W     R10B          —
R11      R11D     R11W     R11B          —
R12      R12D     R12W     R12B          —
R13      R13D     R13W     R13B          —
R14      R14D     R14W     R14B          —
R15      R15D     R15W     R15B          —
```

**Key rule — 32-bit zero-extension**: when a 32-bit operation (REX.W = 0)
writes to EAX, the processor silently zeroes bits 63:32 of RAX.  This means
`MOV EAX, 0` is equivalent to `MOV RAX, 0`.  This is a common optimization
trick: 32-bit operations on zero-extended registers are often shorter encodings.

**Why AH/BH/CH/DH disappear with REX**: The 8-bit "high byte" registers AH, BH,
CH, DH are encoded as register numbers 4–7 in the ModRM reg or rm field when
no REX prefix is present.  With a REX prefix, those same encoding numbers 4–7
refer to SPL, BPL, SIL, DIL.  It is therefore impossible to encode AH/BH/CH/DH
in an instruction that also uses a REX prefix.

---

### RFLAGS (condition flags)

The full RFLAGS register is 64 bits.  This simulator tracks only the five flags
relevant to the integer instruction set:

```
Bit   Abbr   Name       Meaning
──────────────────────────────────────────────────────────────────────
  0   CF     Carry      Set when an unsigned arithmetic operation produces a
                        carry-out (addition) or borrow (subtraction).  Also
                        set/cleared by shift/rotate operations.
  2   PF     Parity     Set when the low 8 bits of the result contain an even
                        number of 1-bits (even parity).  PF=1 means "even parity".
  6   ZF     Zero       Set when the result of an operation is zero.
  7   SF     Sign       Copy of the most-significant bit of the result.
                        SF=1 means the result is negative when interpreted as
                        a signed (two's complement) integer.
 11   OF     Overflow   Set when a signed arithmetic operation produces a result
                        that cannot be represented in the destination width.
                        For addition: overflow when both operands have the same
                        sign and the result has the opposite sign.
                        For subtraction: overflow when the operands have opposite
                        signs and the result has the same sign as the subtrahend.
```

Untracked flags (treated as always 0): AF (auxiliary carry / BCD nibble carry),
DF (direction flag for string ops — strings use DF=0/forward direction),
IF (interrupt enable), TF (trap), RF (resume), VM (virtual-8086), AC, etc.

**PF in detail**: count the number of 1-bits in `result & 0xFF`.  If that count
is even, PF = 1.  If odd, PF = 0.  Example: result byte = 0b00000011 has two
1-bits (even) → PF = 1.  Result byte = 0b00000001 has one 1-bit (odd) → PF = 0.

```python
def compute_pf(result: int) -> int:
    byte = result & 0xFF
    popcount = bin(byte).count('1')
    return 1 if (popcount % 2 == 0) else 0
```

---

### Instruction Encoding

x86-64 is a variable-length instruction set.  Instructions range from 1 to 15
bytes.  The encoding reads left to right in memory order:

```
[Legacy Prefixes] [REX] [Opcode] [ModRM] [SIB] [Displacement] [Immediate]
 0–4 bytes         0–1   1–3      0–1     0–1    0,1,2,4        0,1,2,4,8
```

#### 1. Legacy prefixes (0–4 bytes, one per group, optional)

Each prefix is one byte.  Only one prefix from each group may appear per
instruction (undefined behaviour if repeated).

```
Group 1 — LOCK and repeat:
  F0  LOCK      atomic memory access (treated as NOP here)
  F2  REPNE/REPNZ  repeat while CX≠0 and ZF=0  (used with CMPS/SCAS)
  F3  REP/REPE   repeat while CX≠0 (and ZF=1 for REPE)

Group 2 — segment overrides (all treated as NOP in this simulator):
  26  ES override
  2E  CS override
  36  SS override
  3E  DS override
  64  FS override
  65  GS override

Group 3 — operand-size override:
  66  Switch to 16-bit operand size (this simulator does NOT support 16-bit
      arithmetic; prefix is accepted but ignored — effective size remains 32
      unless REX.W=1 makes it 64)

Group 4 — address-size override:
  67  Switch to 32-bit address size (ignored; this simulator always uses 64-bit
      effective addresses)
```

#### 2. REX prefix (0 or 1 byte, range 0x40–0x4F)

The REX prefix was invented by AMD to give instructions access to R8–R15 and
to select 64-bit operand size.  Any byte in the range 0x40–0x4F is a REX prefix;
the low nibble encodes four single-bit fields:

```
 7   6   5   4   3   2   1   0
┌───┬───┬───┬───┬───┬───┬───┬───┐
│ 0 │ 1 │ 0 │ 0 │ W │ R │ X │ B │
└───┴───┴───┴───┴───┴───┴───┴───┘

W (bit 3): 1 = 64-bit operand size; 0 = 32-bit operand size (default when
           no 66h prefix).  REX.W overrides the 66h prefix.
R (bit 2): extends the ModRM.reg field from 3 bits to 4 bits.
           The 4th bit is REX.R; the lower 3 bits come from ModRM.reg.
           Used when the reg operand is R8–R15.
X (bit 1): extends the SIB.index field.  Used when the index register in a
           SIB-encoded address is R8–R15.
B (bit 0): extends ModRM.rm OR SIB.base OR the opcode register field.
           Used when the rm/base/opcode register is R8–R15.
```

Example: `REX.W + REX.R` = 0x4C (0b0100_1100).  This selects 64-bit operand
size and extends ModRM.reg, allowing the instruction to use R8–R15 as its
register operand.

Only one REX byte may appear per instruction.  A REX byte must immediately
precede the opcode; it cannot precede a legacy prefix.

#### 3. Opcode (1–3 bytes)

Most instructions have a single-byte opcode.  Extended opcodes start with
the escape byte `0F`; then the second byte identifies the specific instruction.
A few three-byte opcodes exist (0F 38 xx, 0F 3A xx) but are not used in this
simulator.

#### 4. ModRM byte (0 or 1 byte)

When present, ModRM encodes the addressing mode and the first operand register.

```
  7   6   5   4   3   2   1   0
┌───┬───┬───┬───┬───┬───┬───┬───┐
│  mod  │    reg    │    rm     │
└───────┴───────────┴───────────┘

mod[7:6]: addressing mode
  11  → rm is a register (no memory access)
  00  → rm is a memory reference [reg], with two special cases:
         rm = 100 (4) → SIB byte follows (scaled-index-base addressing)
         rm = 101 (5) → [RIP + disp32] (RIP-relative addressing)
  01  → [reg + disp8] (8-bit signed displacement sign-extended to 64 bits)
         rm = 100 (4) → SIB byte + disp8
  10  → [reg + disp32] (32-bit signed displacement sign-extended to 64 bits)
         rm = 100 (4) → SIB byte + disp32

reg[5:3]: register field.  Identifies a register operand OR
          an opcode extension (/0 through /7).
          When REX.R = 1, the effective register is reg | 8 (i.e. R8–R15).

rm[2:0]:  identifies the second register/memory operand.
          When REX.B = 1 and mod != 11, extends the base register.
          When REX.B = 1 and mod == 11, the effective register is rm | 8.
```

Register encoding for 64-bit GPRs (REX.B / REX.R extend the 3-bit field):

```
Encoding  Without REX extension  With REX extension (bit = 1)
  000     RAX                    R8
  001     RCX                    R9
  010     RDX                    R10
  011     RBX                    R11
  100     RSP  (or SIB)          R12  (or SIB with REX.B)
  101     RBP  (or RIP+disp32)   R13  (or RIP+disp32 with REX.B in mod=00)
  110     RSI                    R14
  111     RDI                    R15
```

#### 5. SIB byte (0 or 1 byte)

The Scale-Index-Base byte is present when `ModRM.rm = 100` (RSP encoding) and
`mod != 11`.  It encodes a compound effective address:

```
  7   6   5   4   3   2   1   0
┌───┬───┬───┬───┬───┬───┬───┬───┐
│  ss   │   index   │   base    │
└───────┴───────────┴───────────┘

ss[7:6]:    scale factor for index register
  00 → index × 1
  01 → index × 2
  10 → index × 4
  11 → index × 8

index[5:3]: index register.  REX.X extends to 4 bits.
  If index = 100 (RSP encoding) AND REX.X = 0 → no index register (index = 0).

base[2:0]:  base register.  REX.B extends to 4 bits.
  Special case: base = 101 (RBP encoding) when mod = 00 → no base, use disp32.

Effective address = base + index * (2^ss) + displacement
```

#### 6. Displacement (0, 1, or 4 bytes)

- **disp8**: 1 byte, sign-extended to 64 bits.  Used with `mod = 01`.
- **disp32**: 4 bytes, little-endian, sign-extended to 64 bits.  Used with
  `mod = 10` or when ModRM.rm = 101 in mod=00 (RIP-relative).

No 2-byte displacement exists in x86-64 long mode (the 16-bit mode had one,
but it is inaccessible in long mode without the 67h prefix).

#### 7. Immediate (0, 1, 2, 4, or 8 bytes)

- **imm8**: 1 byte; sign-extended to the operand width when used with `83 /x`.
- **imm16**: 2 bytes, little-endian.  Used for RET imm16.
- **imm32**: 4 bytes, little-endian.  Sign-extended to 64 bits when the
  operand is 64-bit (e.g., `MOV r/m64, imm32` with opcode C7).
- **imm64**: 8 bytes, little-endian.  Used only with `MOV r64, imm64` (B8+rd io).

**Little-endian byte order**: x86-64 (like all x86) stores multi-byte integers
with the least-significant byte at the lowest address.  When reading a 4-byte
value from address 0x1000: byte[0x1000] is bits 7:0, byte[0x1001] is bits 15:8,
byte[0x1002] is bits 23:16, byte[0x1003] is bits 31:24.

---

## Supported Instructions

The following tables list every instruction supported by this simulator.  Each
entry gives the opcode bytes (in Intel hex notation, where `/r` means a ModRM
byte with reg and rm encoding registers/memory, `/0`–`/7` means a ModRM byte
where the reg field is the listed value, `ib` = imm8, `id` = imm32, `io` = imm64,
`rd` = register number encoded in the low 3 bits of the opcode, `cb` = 1-byte
relative offset, `cd` = 4-byte relative offset, `iw` = imm16).

### Data Transfer

| Mnemonic                      | Opcode            | Notes |
|-------------------------------|-------------------|-------|
| MOV r/m64, r64                | REX.W 89 /r       | Store register to r/m |
| MOV r64, r/m64                | REX.W 8B /r       | Load r/m to register |
| MOV r64, imm64                | REX.W B8+rd io    | 64-bit immediate to register; rd = reg index 0–7 |
| MOV r/m64, imm32              | REX.W C7 /0 id    | Sign-extend imm32 → 64 bits, store |
| MOV r/m8, r8                  | 88 /r             | 8-bit store |
| MOV r8, r/m8                  | 8A /r             | 8-bit load |
| MOVSX r64, r/m8               | REX.W 0F BE /r    | Sign-extend 8-bit to 64-bit |
| MOVSX r64, r/m32              | REX.W 63 /r       | Sign-extend 32-bit to 64-bit |
| MOVZX r64, r8                 | REX.W 0F B6 /r    | Zero-extend 8-bit to 64-bit |
| MOVZX r64, r16                | REX.W 0F B7 /r    | Zero-extend 16-bit to 64-bit |
| XCHG r64, r/m64               | REX.W 87 /r       | Swap two operands atomically |
| LEA r64, m                    | REX.W 8D /r       | Load effective address (no memory read) |
| PUSH r/m64                    | FF /6             | RSP -= 8; [RSP] = r/m64 |
| PUSH imm8                     | 6A ib             | Push sign-extended 8-bit immediate |
| PUSH imm32                    | 68 id             | Push sign-extended 32-bit immediate |
| POP r/m64                     | 8F /0             | r/m64 = [RSP]; RSP += 8 |

**XCHG notes**: When one operand is RAX and the other is encoded via B8+rd
(without ModRM), this is the NOP encoding when both operands are RAX (0x90).
Our simulator handles `XCHG RAX, RAX` as NOP for that specific opcode.

**LEA notes**: LEA uses the full address computation logic (ModRM + SIB +
displacement) but does *not* read from memory.  It stores the computed
effective address in the destination register.  Useful for pointer arithmetic
without touching memory.

### Arithmetic

| Mnemonic                     | Opcode               | Notes |
|------------------------------|----------------------|-------|
| ADD r/m64, r64               | REX.W 01 /r          | Sets CF PF ZF SF OF |
| ADD r64, r/m64               | REX.W 03 /r          | |
| ADD r/m64, imm8              | REX.W 83 /0 ib       | imm8 sign-extended |
| ADD r/m64, imm32             | REX.W 81 /0 id       | imm32 sign-extended |
| ADC r/m64, r64               | REX.W 11 /r          | Add with carry-in (CF) |
| ADC r64, r/m64               | REX.W 13 /r          | |
| ADC r/m64, imm8              | REX.W 83 /2 ib       | |
| ADC r/m64, imm32             | REX.W 81 /2 id       | |
| SUB r/m64, r64               | REX.W 29 /r          | Sets CF PF ZF SF OF |
| SUB r64, r/m64               | REX.W 2B /r          | |
| SUB r/m64, imm8              | REX.W 83 /5 ib       | |
| SUB r/m64, imm32             | REX.W 81 /5 id       | |
| SBB r/m64, r64               | REX.W 19 /r          | Subtract with borrow (CF) |
| SBB r64, r/m64               | REX.W 1B /r          | |
| SBB r/m64, imm8              | REX.W 83 /3 ib       | |
| SBB r/m64, imm32             | REX.W 81 /3 id       | |
| IMUL r64, r/m64              | REX.W 0F AF /r       | Signed multiply; result in r64 |
| IMUL r64, r/m64, imm8        | REX.W 6B /r ib       | r64 = r/m64 * sign-extend(imm8) |
| IMUL r64, r/m64, imm32       | REX.W 69 /r id       | r64 = r/m64 * sign-extend(imm32) |
| MUL r/m64                    | REX.W F7 /4          | Unsigned RDX:RAX = RAX * r/m64 |
| DIV r/m64                    | REX.W F7 /6          | Unsigned RAX = RDX:RAX / r/m64; RDX = remainder |
| IDIV r/m64                   | REX.W F7 /7          | Signed RAX = RDX:RAX / r/m64; RDX = remainder |
| NEG r/m64                    | REX.W F7 /3          | r/m64 = 0 - r/m64; sets all flags |
| INC r/m64                    | REX.W FF /0          | r/m64++; sets PF ZF SF OF (not CF) |
| DEC r/m64                    | REX.W FF /1          | r/m64--; sets PF ZF SF OF (not CF) |
| CMP r/m64, r64               | REX.W 39 /r          | Subtract without store; sets all flags |
| CMP r64, r/m64               | REX.W 3B /r          | |
| CMP r/m64, imm8              | REX.W 83 /7 ib       | |
| CMP r/m64, imm32             | REX.W 81 /7 id       | |

**INC and DEC** do not affect CF.  This is deliberate: it allows INC/DEC to
serve as loop counters inside multi-precision arithmetic without disturbing the
carry chain between limbs.

**MUL / DIV**: these are unsigned and operate on the implicit RDX:RAX pair.
Before a 64-bit division, RDX must be set to 0 (for unsigned) or sign-extended
from RAX (for IDIV, use the CQO instruction — not implemented here; caller must
set RDX manually).  Division by zero raises a Python `ZeroDivisionError` in
this simulator.

### Logical

| Mnemonic                     | Opcode               | Notes |
|------------------------------|----------------------|-------|
| AND r/m64, r64               | REX.W 21 /r          | Sets PF ZF SF; clears CF OF |
| AND r64, r/m64               | REX.W 23 /r          | |
| AND r/m64, imm8              | REX.W 83 /4 ib       | |
| AND r/m64, imm32             | REX.W 81 /4 id       | |
| OR r/m64, r64                | REX.W 09 /r          | |
| OR r64, r/m64                | REX.W 0B /r          | |
| OR r/m64, imm8               | REX.W 83 /1 ib       | |
| OR r/m64, imm32              | REX.W 81 /1 id       | |
| XOR r/m64, r64               | REX.W 31 /r          | XOR reg, reg is the canonical zero idiom |
| XOR r64, r/m64               | REX.W 33 /r          | |
| XOR r/m64, imm8              | REX.W 83 /6 ib       | |
| XOR r/m64, imm32             | REX.W 81 /6 id       | |
| NOT r/m64                    | REX.W F7 /2          | Bitwise NOT; does NOT affect flags |
| TEST r/m64, r64              | REX.W 85 /r          | AND without store; sets PF ZF SF; clears CF OF |
| TEST r/m64, imm32            | REX.W F7 /0 id       | |

### Shift and Rotate

| Mnemonic                     | Opcode               | Notes |
|------------------------------|----------------------|-------|
| SHL r/m64, 1                 | REX.W D1 /4          | Logical left shift by 1 |
| SHL r/m64, CL                | REX.W D3 /4          | Logical left shift by CL mod 64 |
| SHL r/m64, imm8              | REX.W C1 /4 ib       | Logical left shift by imm8 mod 64 |
| SHR r/m64, 1                 | REX.W D1 /5          | Logical right shift by 1 (zero fill) |
| SHR r/m64, CL                | REX.W D3 /5          | |
| SHR r/m64, imm8              | REX.W C1 /5 ib       | |
| SAR r/m64, 1                 | REX.W D1 /7          | Arithmetic right shift by 1 (sign fill) |
| SAR r/m64, CL                | REX.W D3 /7          | |
| SAR r/m64, imm8              | REX.W C1 /7 ib       | |
| ROL r/m64, imm8              | REX.W C1 /0 ib       | Rotate left; CF = last bit rotated out |
| ROR r/m64, imm8              | REX.W C1 /1 ib       | Rotate right; CF = last bit rotated out |

**SHL / SAL**: SHL and SAL are identical (same opcode, /4).  SAL is an alias.

**Shift count masking**: For 64-bit operands, the shift/rotate count is masked
to 6 bits (i.e., `count & 63`).  For 32-bit operands, count is masked to 5
bits (`count & 31`).  A shift by 0 is a no-op and does not update flags.

**CF from shift**: CF is set to the last bit shifted/rotated out of the operand.
For left shifts, CF = bit `(64 - count)` of the original value.  For right shifts,
CF = bit `(count - 1)` of the original value.

**OF for shifts by 1**: OF is set when a left shift by 1 causes a sign change
(the MSB before the shift differs from the MSB after the shift), or cleared
otherwise.  OF is undefined for shifts by more than 1.

### Control Flow

| Mnemonic                     | Opcode               | Notes |
|------------------------------|----------------------|-------|
| JMP rel8                     | EB cb                | Short jump; RIP += sign_extend(cb) |
| JMP rel32                    | E9 cd                | Near jump; RIP += sign_extend(cd) |
| JMP r/m64                    | FF /4                | Indirect jump; RIP = r/m64 |
| CALL rel32                   | E8 cd                | Push RIP; RIP += sign_extend(cd) |
| CALL r/m64                   | FF /2                | Push RIP; RIP = r/m64 |
| RET                          | C3                   | Pop RIP |
| RET imm16                    | C2 iw                | Pop RIP; RSP += imm16 |
| Jcc rel8                     | 70–7F cb             | Conditional jump short |
| Jcc rel32                    | 0F 80–8F cd          | Conditional jump near |
| LOOP rel8                    | E2 cb                | RCX--; jump if RCX != 0 |
| LOOPE rel8                   | E1 cb                | RCX--; jump if RCX != 0 AND ZF = 1 |
| LOOPNE rel8                  | E0 cb                | RCX--; jump if RCX != 0 AND ZF = 0 |
| JRCXZ rel8                   | E3 cb                | Jump if RCX = 0 (no decrement) |
| NOP                          | 90                   | No operation |
| HLT                          | F4                   | Halt simulation |

**Branch target calculation**: relative jumps add the signed offset to RIP
*after* the instruction has been fetched.  So for `JMP rel8` at address A,
target = A + 2 + sign_extend(cb).  The "+2" accounts for the 2-byte instruction
length.  Similarly for `JMP rel32`: target = A + 5 + sign_extend(cd).

**CALL / RET and the stack**: CALL pushes the 8-byte return address (address
of the instruction after CALL) by decrementing RSP by 8 then writing to [RSP].
RET pops the 8-byte return address and jumps to it.  `RET imm16` additionally
adds `imm16` to RSP after popping (used to release stack arguments in the
callee before returning — stdcall convention).

**LOOP**: decrements RCX first, then checks.  Does NOT set any flags.

### String Operations (simplified)

| Mnemonic                     | Opcode               | Notes |
|------------------------------|----------------------|-------|
| REP STOSQ                    | F3 REX.W AB          | [RDI] = RAX; RDI += 8; RCX--; repeat while RCX != 0 |

**REP STOSQ detail**:
- Each iteration writes RAX as an 8-byte little-endian value to [RDI].
- RDI advances by +8 (forward direction; DF=0 assumed).
- RCX is decremented by 1 per iteration.
- Stops when RCX reaches 0.
- ZF is not checked (that is for REPE/REPNE variants).
- Memory destination wraps modulo MEM_SIZE.

This instruction is the primary way compiled code zeroes or fills a buffer.
For example, the C `memset` of a page-aligned allocation often compiles to
`REP STOSQ` with RAX=0.

### Bit Manipulation

| Mnemonic                     | Opcode               | Notes |
|------------------------------|----------------------|-------|
| BSF r64, r/m64               | REX.W 0F BC /r       | Bit scan forward: r64 = index of lowest set bit; ZF=1 if src=0 |
| BSR r64, r/m64               | REX.W 0F BD /r       | Bit scan reverse: r64 = index of highest set bit; ZF=1 if src=0 |
| BT r/m64, r64                | REX.W 0F A3 /r       | Bit test: CF = bit[r64] of r/m64 |
| BT r/m64, imm8               | REX.W 0F BA /4 ib    | Bit test with immediate |
| BSWAP r64                    | REX.W 0F C8+rd       | Reverse byte order of 64-bit register |

**BSF / BSR**: if the source is 0, ZF is set and the destination register is
undefined (we leave it unchanged).  If the source is non-zero, ZF is cleared
and the destination holds the bit index.

**BSWAP**: reverses all 8 bytes of a 64-bit register.  Used to convert between
big-endian and little-endian representations.  Example: if RBX = 0x0102030405060708,
after `BSWAP RBX`, RBX = 0x0807060504030201.

### Conditional Move

| Mnemonic                     | Opcode               | Notes |
|------------------------------|----------------------|-------|
| CMOVcc r64, r/m64            | REX.W 0F 40–4F /r    | If condition, r64 = r/m64; else no-op |

CMOVcc uses the same 16 condition codes as Jcc (see Condition Codes section).
The opcode byte in the 0F 40–4F range encodes the condition: 0F 40 = CMOVO,
0F 41 = CMOVNO, ..., 0F 4F = CMOVG.

CMOVcc always reads the source (so it cannot be used to suppress a memory fault
on the source operand).  It only conditionally writes the destination.

### SETcc

| Mnemonic                     | Opcode               | Notes |
|------------------------------|----------------------|-------|
| SETcc r/m8                   | 0F 90–9F /0          | r/m8 = 1 if condition, else 0 |

SETcc writes a 1 or 0 byte to an 8-bit register or memory location.  It does
not zero-extend to the full register; the upper bytes are unchanged.  The
condition encoding is the same as Jcc: 0F 90 = SETO, 0F 91 = SETNO, ...,
0F 9F = SETG.

---

## RFLAGS Update Rules

### Arithmetic operations: `add_with_flags` and `sub_with_flags`

These pseudocode functions are the heart of all arithmetic flag updates.
They operate on unsigned Python integers but compute the signed meaning
from the bit pattern.

```python
def add_with_flags(a: int, b: int, carry_in: int = 0, bits: int = 64) -> tuple[int, int]:
    """
    Compute a + b + carry_in and return (result, rflags_bits).

    Inputs are interpreted as unsigned integers of `bits` width.
    Returns the result truncated to `bits` bits, and the five flags
    packed as: CF=bit0, PF=bit2, ZF=bit6, SF=bit7, OF=bit11.

    The `carry_in` parameter is 0 for ADD, CF for ADC.
    The `bits` parameter is 64 for REX.W=1, 32 for REX.W=0.
    """
    mask = (1 << bits) - 1
    unsigned_sum = (a & mask) + (b & mask) + carry_in
    result = unsigned_sum & mask

    CF = 1 if unsigned_sum > mask else 0
    ZF = 1 if result == 0 else 0
    SF = (result >> (bits - 1)) & 1
    PF = compute_pf(result)
    # Overflow: both inputs have the same sign, but the result has a different sign.
    a_sign = (a >> (bits - 1)) & 1
    b_sign = (b >> (bits - 1)) & 1
    r_sign = SF
    OF = 1 if (a_sign == b_sign) and (r_sign != a_sign) else 0

    flags = (OF << 11) | (SF << 7) | (ZF << 6) | (PF << 2) | CF
    return result, flags


def sub_with_flags(a: int, b: int, borrow_in: int = 0, bits: int = 64) -> tuple[int, int]:
    """
    Compute a - b - borrow_in and return (result, rflags_bits).

    Subtraction is implemented as a + (~b) + (1 - borrow_in) to reuse
    the carry logic, then the carry out is complemented to form the borrow.
    This is the standard x86 borrow-complement convention: CF=1 means
    borrow occurred (a < b).

    The `borrow_in` parameter is 0 for SUB, CF for SBB.
    """
    mask = (1 << bits) - 1
    # Two's complement subtraction: a - b = a + (~b) + 1
    neg_b = (~b) & mask
    carry_in = 1 - borrow_in          # borrow in → complement carry in
    unsigned_sum = (a & mask) + neg_b + carry_in
    result = unsigned_sum & mask

    CF = 0 if unsigned_sum > mask else 1  # CF=1 means borrow (a < b)
    ZF = 1 if result == 0 else 0
    SF = (result >> (bits - 1)) & 1
    PF = compute_pf(result)
    # Overflow: inputs have different sign (a positive, b negative or vice versa)
    # and the result has the wrong sign for that subtraction direction.
    a_sign = (a >> (bits - 1)) & 1
    b_sign = (b >> (bits - 1)) & 1
    r_sign = SF
    OF = 1 if (a_sign != b_sign) and (r_sign != a_sign) else 0

    flags = (OF << 11) | (SF << 7) | (ZF << 6) | (PF << 2) | CF
    return result, flags
```

**Why CF=1 means borrow for SUB**: x86 uses the carry flag as a *borrow* flag
for subtraction.  When you compute `a - b` and `a < b` (unsigned), the CPU
sets CF=1 to indicate "I had to borrow".  This is the opposite of how some
architectures (notably AArch64) define carry for subtraction.  ADC and SBB use
CF directly as carry-in and borrow-in respectively.

### Logical operations (AND, OR, XOR, TEST)

```python
def logical_flags(result: int, bits: int = 64) -> int:
    """
    Compute RFLAGS for AND/OR/XOR/TEST operations.

    CF = 0 always (cleared)
    OF = 0 always (cleared)
    ZF = 1 if result == 0
    SF = MSB of result
    PF = parity of low byte

    Note: AF (auxiliary carry) is undefined for logical ops; not tracked here.
    """
    ZF = 1 if (result & ((1 << bits) - 1)) == 0 else 0
    SF = (result >> (bits - 1)) & 1
    PF = compute_pf(result)
    return (SF << 7) | (ZF << 6) | (PF << 2)  # CF=0, OF=0
```

### NEG

NEG r/m performs `0 - operand`, which is the same as `sub_with_flags(0, operand)`.
CF is set if the operand is non-zero (i.e., any non-zero value produces a borrow
when subtracting from zero).  CF is cleared when the operand is 0 (since 0 - 0 = 0
with no borrow).

### INC and DEC

INC and DEC update PF, ZF, SF, OF but do *not* touch CF.  This allows
multi-precision loops to use INC/DEC on loop counters without disturbing the
carry chain:

```python
def inc_flags(result: int, old_rflags: int, bits: int = 64) -> int:
    """INC: update OF SF ZF PF; preserve CF."""
    mask = (1 << bits) - 1
    ZF = 1 if (result & mask) == 0 else 0
    SF = (result >> (bits - 1)) & 1
    PF = compute_pf(result)
    # OF for INC: set when result == (1 << (bits-1)) — the minimum signed value,
    # indicating we wrapped from MAX_SIGNED to MIN_SIGNED.
    OF = 1 if (result & mask) == (1 << (bits - 1)) else 0
    CF = old_rflags & 1  # preserve CF
    return (OF << 11) | (SF << 7) | (ZF << 6) | (PF << 2) | CF
```

---

## Condition Codes

x86-64 defines 16 condition codes, numbered 0–15.  Each has two mnemonics
(e.g., JE and JZ are identical).  The conditions are evaluated from the current
RFLAGS.

```
Code  Opcode suffix  Mnemonics    Condition expression
──────────────────────────────────────────────────────────────────
  0   0h  / 40h      JO  / CMOVO    OF = 1
  1   1h  / 41h      JNO / CMOVNO   OF = 0
  2   2h  / 42h      JB  / JC / JNAE / CMOVB   CF = 1
  3   3h  / 43h      JNB / JNC / JAE / CMOVNB  CF = 0
  4   4h  / 44h      JE  / JZ / CMOVE   ZF = 1
  5   5h  / 45h      JNE / JNZ / CMOVNE ZF = 0
  6   6h  / 46h      JBE / JNA / CMOVBE CF = 1 OR ZF = 1
  7   7h  / 47h      JA  / JNBE / CMOVA CF = 0 AND ZF = 0
  8   8h  / 48h      JS  / CMOVS    SF = 1
  9   9h  / 49h      JNS / CMOVNS   SF = 0
 10   Ah  / 4Ah      JP  / JPE / CMOVP   PF = 1
 11   Bh  / 4Bh      JNP / JPO / CMOVNP  PF = 0
 12   Ch  / 4Ch      JL  / JNGE / CMOVL  SF ≠ OF
 13   Dh  / 4Dh      JGE / JNL / CMOVGE  SF = OF
 14   Eh  / 4Eh      JLE / JNG / CMOVLE  ZF = 1 OR SF ≠ OF
 15   Fh  / 4Fh      JG  / JNLE / CMOVG  ZF = 0 AND SF = OF
```

The condition code integer maps directly to the low nibble of the opcode.
For `Jcc rel8`, the opcode is `0x70 | cc`.  For `Jcc rel32`, the opcode pair
is `0x0F 0x80 | cc`.  For `CMOVcc`, the opcode pair is `0x0F 0x40 | cc`.
For `SETcc`, the pair is `0x0F 0x90 | cc`.

```python
def condition_holds(cc: int, rflags: int) -> bool:
    """
    Evaluate condition code `cc` (0–15) against the current `rflags`.

    Returns True if the branch/move should be taken.

    CF = rflags[0], PF = rflags[2], ZF = rflags[6], SF = rflags[7], OF = rflags[11]
    """
    CF = (rflags >> 0)  & 1
    PF = (rflags >> 2)  & 1
    ZF = (rflags >> 6)  & 1
    SF = (rflags >> 7)  & 1
    OF = (rflags >> 11) & 1

    match cc:
        case  0: return OF == 1              # JO  — overflow set
        case  1: return OF == 0              # JNO — overflow clear
        case  2: return CF == 1              # JB  — unsigned below (carry)
        case  3: return CF == 0              # JAE — unsigned above or equal
        case  4: return ZF == 1              # JE  — equal / zero
        case  5: return ZF == 0              # JNE — not equal
        case  6: return CF == 1 or ZF == 1   # JBE — unsigned below or equal
        case  7: return CF == 0 and ZF == 0  # JA  — unsigned above
        case  8: return SF == 1              # JS  — sign (negative)
        case  9: return SF == 0              # JNS — not sign (positive/zero)
        case 10: return PF == 1              # JP  — parity even
        case 11: return PF == 0              # JNP — parity odd
        case 12: return SF != OF             # JL  — signed less than
        case 13: return SF == OF             # JGE — signed greater or equal
        case 14: return ZF == 1 or SF != OF  # JLE — signed less or equal
        case 15: return ZF == 0 and SF == OF # JG  — signed greater
        case _:  raise ValueError(f"Invalid condition code: {cc}")
```

**Mnemonics decoded**: B = Below (unsigned), A = Above (unsigned), E = Equal,
G = Greater (signed), L = Less (signed), S = Sign/Negative, O = Overflow,
P = Parity.  The N prefix negates (NB = not below = above-or-equal, etc.).

---

## SIM00 Compliance

This simulator implements `Simulator[X86_64State]` from `simulator_protocol`.

### `X86_64State` dataclass

```python
@dataclass(frozen=True)
class X86_64State:
    """
    Immutable snapshot of x86-64 CPU state at a single point in time.

    Fields
    ──────
    pc      : int             — RIP, the instruction pointer.  64-bit.
    gpr     : tuple[int, ...] — 16 general-purpose registers in index order:
                                  0=RAX, 1=RCX, 2=RDX, 3=RBX,
                                  4=RSP, 5=RBP, 6=RSI, 7=RDI,
                                  8=R8,  9=R9,  10=R10, 11=R11,
                                  12=R12, 13=R13, 14=R14, 15=R15
                                All values are 64-bit unsigned (0–2^64−1).
    rflags  : int             — RFLAGS register, only CF/PF/ZF/SF/OF bits
                                defined.  Other bits are always 0.
    memory  : tuple[int, ...] — 65 536 bytes (indices 0x0000–0xFFFF).
                                Each entry is a byte value (0–255).
    halted  : bool            — True once HLT (0xF4) has been executed.
    """
    pc:     int
    gpr:    tuple[int, ...]   # 16 entries
    rflags: int
    memory: tuple[int, ...]   # MEM_SIZE = 65_536 entries
    halted: bool

    # ── Register convenience properties ──────────────────────────────────

    @property
    def rax(self) -> int: return self.gpr[0]
    @property
    def rcx(self) -> int: return self.gpr[1]
    @property
    def rdx(self) -> int: return self.gpr[2]
    @property
    def rbx(self) -> int: return self.gpr[3]
    @property
    def rsp(self) -> int: return self.gpr[4]
    @property
    def rbp(self) -> int: return self.gpr[5]
    @property
    def rsi(self) -> int: return self.gpr[6]
    @property
    def rdi(self) -> int: return self.gpr[7]
    @property
    def r8(self)  -> int: return self.gpr[8]
    @property
    def r9(self)  -> int: return self.gpr[9]
    @property
    def r10(self) -> int: return self.gpr[10]
    @property
    def r11(self) -> int: return self.gpr[11]
    @property
    def r12(self) -> int: return self.gpr[12]
    @property
    def r13(self) -> int: return self.gpr[13]
    @property
    def r14(self) -> int: return self.gpr[14]
    @property
    def r15(self) -> int: return self.gpr[15]

    # ── Flag convenience properties ───────────────────────────────────────

    @property
    def cf(self) -> bool: return bool((self.rflags >> 0)  & 1)
    @property
    def pf(self) -> bool: return bool((self.rflags >> 2)  & 1)
    @property
    def zf(self) -> bool: return bool((self.rflags >> 6)  & 1)
    @property
    def sf(self) -> bool: return bool((self.rflags >> 7)  & 1)
    @property
    def of(self) -> bool: return bool((self.rflags >> 11) & 1)
```

### `X86_64Simulator` methods

#### `reset()`

Sets all 16 GPRs to 0.  Sets RIP (pc) to 0.  Sets RFLAGS to 0.  Zeroes all
65 536 bytes of memory.  Sets `halted = False`.

Rationale for RSP = 0 on reset: in real hardware RSP is undefined after reset.
Programs in this simulator that use the stack must either set RSP explicitly
(e.g., `MOV RSP, imm64`) or load a binary that begins with RSP initialisation.
Starting RSP at 0 (which wraps to 0x10000 when first decremented by 8, since
memory is 64 KB and addresses wrap) is predictable and testable.

#### `load(program: bytes) -> None`

Copies `program` bytes into `memory[0x0000 ...]`.  Does not reset CPU state.
Programs should be loaded after `reset()`.

#### `step() -> StepTrace`

1. Check `halted`; raise `RuntimeError` if True.
2. Read up to 15 bytes starting at `memory[pc % MEM_SIZE]`.
3. Decode the instruction following the encoding described above.
4. Execute the instruction, updating registers, flags, memory, and PC.
5. Return a `StepTrace(pc_before, pc_after, mnemonic, description)`.

Encountering an unrecognised opcode raises `RuntimeError("Unknown opcode ...")`.

#### `execute(program: bytes, max_steps: int = 100_000) -> ExecutionResult[X86_64State]`

1. `reset()`
2. `load(program)`
3. Loop calling `step()` until `halted` or `max_steps` exhausted.
4. Return `ExecutionResult(halted=self.halted, steps=N, final_state=get_state(), error=..., traces=[...])`

HLT (opcode 0xF4) sets `halted = True` and stops execution.

#### `get_state() -> X86_64State`

Returns a frozen `X86_64State` snapshot.  Converts the internal mutable list
of registers to a `tuple[int, ...]` and the internal `bytearray` to a
`tuple[int, ...]`.

---

## Memory Model

```
Address range   : 0x0000–0xFFFF  (64 KB flat, byte-addressable)
Byte order      : little-endian
Wrap-around     : addresses are taken modulo MEM_SIZE (65 536)
Word size       : 8 bytes (64-bit / qword), 4 bytes (32-bit / dword),
                  2 bytes (16-bit / word), 1 byte (8-bit / byte)
```

**Reads and writes** of multi-byte values use little-endian byte order:

```python
MEM_SIZE = 65_536

def read_u8(mem: bytearray, addr: int) -> int:
    return mem[addr % MEM_SIZE]

def read_u16(mem: bytearray, addr: int) -> int:
    lo = mem[addr % MEM_SIZE]
    hi = mem[(addr + 1) % MEM_SIZE]
    return lo | (hi << 8)

def read_u32(mem: bytearray, addr: int) -> int:
    result = 0
    for i in range(4):
        result |= mem[(addr + i) % MEM_SIZE] << (8 * i)
    return result

def read_u64(mem: bytearray, addr: int) -> int:
    result = 0
    for i in range(8):
        result |= mem[(addr + i) % MEM_SIZE] << (8 * i)
    return result

def write_u8(mem: bytearray, addr: int, value: int) -> None:
    mem[addr % MEM_SIZE] = value & 0xFF

def write_u64(mem: bytearray, addr: int, value: int) -> None:
    for i in range(8):
        mem[(addr + i) % MEM_SIZE] = (value >> (8 * i)) & 0xFF
```

There is no alignment requirement in this simulator — misaligned accesses work
correctly (unlike real hardware where misaligned 64-bit accesses across cache
line boundaries may be slower or cause exceptions depending on configuration).

---

## Package Layout

```
code/packages/python/x86-64-simulator/
├── BUILD
├── CHANGELOG.md
├── README.md
├── pyproject.toml
└── src/
    └── x86_64_simulator/
        ├── __init__.py
        ├── py.typed
        ├── state.py        # X86_64State frozen dataclass
        ├── flags.py        # add_with_flags, sub_with_flags, condition_holds
        └── simulator.py    # X86_64Simulator — instruction decode + execute
tests/
├── __init__.py
├── test_data_transfer.py   # MOV, XCHG, LEA, PUSH, POP
├── test_arithmetic.py      # ADD, SUB, IMUL, IDIV, INC, DEC, CMP, NEG
├── test_logical.py         # AND, OR, XOR, NOT, TEST
├── test_shift.py           # SHL, SHR, SAR, ROL, ROR
├── test_branch.py          # JMP, CALL, RET, Jcc, LOOP, JRCXZ
├── test_bitops.py          # BSF, BSR, BT, BSWAP
├── test_cmov.py            # CMOVcc all 16 conditions
├── test_setcc.py           # SETcc all 16 conditions
├── test_string.py          # REP STOSQ
├── test_flags.py           # add_with_flags, sub_with_flags, condition_holds
├── test_protocol.py        # SIM00 protocol conformance
└── test_programs.py        # multi-instruction programs (sort, fibonacci, etc.)
```

---

## Simplifications

This simulator implements a restricted but internally consistent subset of
x86-64.  The following simplifications are made deliberately, in the spirit of
building a clean educational simulator rather than a full hardware emulator:

1. **64-bit long mode only**: real mode (DOS), protected mode (32-bit Windows/Linux),
   and compatibility mode (32-bit programs in a 64-bit OS) are not modelled.
   There is no mode switching, no GDT/LDT, no IDT, no control registers (CR0–CR4).

2. **No x87 FPU**: the 8087/387/x87 floating-point unit with its 80-bit stack
   registers (ST0–ST7) is not implemented.  Instructions: FLD, FMUL, FADD, etc.
   all raise `RuntimeError("x87 not implemented")`.

3. **No SSE/AVX/MMX**: SSE2 is technically part of the x86-64 baseline, and
   real compilers use XMM0–XMM15 heavily for floating-point and SIMD.  None of
   these register files or instructions are modelled here.  Instructions: MOVSD,
   ADDPD, VPUNPCKLQDQ, etc. raise `RuntimeError("SSE/AVX not implemented")`.

4. **No privilege levels (rings)**: there are no ring 0 / ring 3 distinctions.
   All instructions execute at the same privilege level.

5. **No segmentation**: CS, DS, ES, FS, GS, SS are accepted in prefix bytes but
   ignored.  There is no segment base, limit, or descriptor table.  All addresses
   are direct memory indices.

6. **Flat 64 KB memory**: the address space is 65 536 bytes.  Addresses wrap
   modulo 65 536.  Real x86-64 has a 48-bit virtual address space (256 TB).

7. **No paging or virtual memory**: all addresses are physical.  There is no
   page fault, no TLB, no CR3.

8. **SYSCALL / SYSRET / INT treated as NOP**: system calls are handled by the
   operating system kernel.  This simulator has no OS, so these instructions
   are silently ignored.  INT n, INT3, INTO, IRET all behave as NOP.

9. **String operations: only REP STOSQ**: the full x86 string instruction set
   includes MOVS, CMPS, SCAS, LODS, STOS in 8/16/32/64-bit widths, with REP,
   REPE, REPNE prefixes.  Only `REP STOSQ` (fill qwords) is supported here.

10. **No CPUID**: the CPUID instruction is not implemented.  Probing feature
    bits would return undefined results; the simulator raises `RuntimeError`.

11. **PF flag tracks low byte parity**: PF is computed from bits 7:0 of the
    result only (standard x86 behaviour).

12. **AF (auxiliary carry) not tracked**: the auxiliary carry / BCD nibble carry
    flag is only used by DAA/DAS/AAA/AAS (BCD instructions), none of which are
    implemented.  AF is always 0 in this simulator.

13. **32-bit writes zero the upper 32 bits**: as specified in the architecture,
    writing a 32-bit register (EAX etc.) in 64-bit mode implicitly zeroes bits
    63:32.  The simulator enforces this.  Writing a 16-bit or 8-bit register
    does not affect other bits.

---

## Design Decisions and Divergences

1. **Register index order (RAX=0, RCX=1, RDX=2, RBX=3)**: the hardware
   encoding order for the classic 8 GPRs is RAX=0, RCX=1, RDX=2, RBX=3,
   RSP=4, RBP=5, RSI=6, RDI=7.  This matches the ModRM register encoding.
   R8–R15 extend this sequence to indices 8–15 with REX prefix bits.  The
   `gpr` tuple uses this same ordering so that the hardware encoding maps
   directly to a tuple index.

2. **RSP initial value is 0**: unlike the AArch64 simulator (which starts
   with SP=0), the x86-64 spec leaves RSP undefined after reset.  A real
   OS bootloader or runtime sets RSP to the top of the allocated stack.
   Programs that use PUSH/POP/CALL/RET must set RSP before use.

3. **HLT sentinel**: opcode 0xF4 halts the simulation.  In real hardware,
   HLT enters a low-power idle state waiting for an interrupt.  Since there
   are no interrupts in this simulator, HLT is a clean program terminator —
   analogous to `exit(0)`.

4. **Unrecognised opcodes raise RuntimeError**: rather than silently advancing
   past unknown opcodes (which would produce baffling incorrect results), the
   simulator raises a descriptive `RuntimeError` identifying the opcode bytes.

5. **No multi-byte NOP**: the real x86-64 assembler emits multi-byte NOP
   sequences (e.g., 0F 1F /0) for alignment padding.  This simulator handles
   only the single-byte NOP (0x90).

6. **LOCK prefix ignored**: atomic memory operations (LOCK ADD, LOCK CMPXCHG,
   etc.) are not relevant in a single-threaded simulator.  The F0 LOCK prefix
   is consumed and ignored.

7. **No CMPXCHG, XADD, or atomic read-modify-write**: these instructions are
   used by lock-free data structures and OS synchronisation primitives.  They
   are not implemented in this simulator.
