# OCT01 — Intel 8008 Backend Specification

## Overview

This document specifies the Intel 8008 backend for the Oct compiler toolchain: the
code generator that translates the repository's common IR (`compiler_ir.IrProgram`)
into Intel 8008 assembly text, and the two-pass assembler that converts that assembly
text into a 16 KB binary ROM image encoded in Intel HEX format.

The backend has four stages:

```
Oct IR  (compiler_ir.IrProgram)
    ↓  [Pre-flight validator]     intel-8008-ir-validator
    ↓  [Code Generator]           ir-to-intel-8008-compiler
8008 Assembly Text (.asm)
    ↓  [Two-Pass Assembler]       intel-8008-assembler
Binary ROM Image (16 384 bytes)
    ↓  [Intel HEX Packager]       intel-8008-packager
Intel HEX File (.hex)
    ↓  [intel8008-simulator] / [intel8008-gatelevel]
8008 Hardware / Emulator
```

### Relationship to the Simulator

The 8008 behavioral simulator (`code/packages/*/intel8008-simulator`) executes the
binary output produced by this backend.  The full end-to-end pipeline is therefore:

```
Write Oct source (.oct)
  → compile with octcc
  → validate with intel-8008-ir-validator
  → code-generate with ir-to-intel-8008-compiler
  → assemble with intel-8008-assembler
  → package with intel-8008-packager
  → run on intel8008-simulator (behavioral, fast)
  → run on intel8008-gatelevel (gate-level, slow but true to silicon)
```

---

## Intel 8008 ISA Overview

The Intel 8008 (1972) is an 8-bit CPU with a 14-bit address space (16 KB).  Its
instruction set is compact but complete enough to build a structured language backend
on top of it.

### Architecture Snapshot

| Property | Value |
|----------|-------|
| Data width | 8 bits |
| Address width | 14 bits (16 384 bytes) |
| Registers | A (accumulator), B, C, D, E, H, L (7 total) |
| Pseudo-register | M — memory byte at address H:L |
| Call stack | 8-level hardware push-down; level 0 = current PC |
| Available call levels | 7 (one level always in use for current PC) |
| Flags | CY (carry/borrow), Z (zero), S (sign), P (even parity) |
| Input ports | 8 (ports 0–7), each 8-bit |
| Output ports | 24 (ports 0–23), each 8-bit |
| Instruction width | 1, 2, or 3 bytes |

### Instruction Groups

Instructions are decoded by the upper 2 bits of the opcode byte.

#### Group 00 — Register Immediate, Rotate, Conditional Return

| Mnemonic | Bytes | Description |
|----------|-------|-------------|
| `MVI r, d8` | 2 | Move immediate byte `d8` into register `r` |
| `INR r` | 1 | Increment register `r`; does **not** affect CY |
| `DCR r` | 1 | Decrement register `r`; does **not** affect CY |
| `RLC` | 1 | Rotate A left circular: CY ← bit7, bit0 ← old bit7 |
| `RRC` | 1 | Rotate A right circular: CY ← bit0, bit7 ← old bit0 |
| `RAL` | 1 | Rotate A left through carry (9-bit): CY ← bit7, bit0 ← old CY |
| `RAR` | 1 | Rotate A right through carry (9-bit): CY ← bit0, bit7 ← old CY |
| `RFC` | 1 | Return if Carry False (CY = 0) |
| `RTC` | 1 | Return if Carry True (CY = 1) |
| `RFZ` | 1 | Return if Zero False (Z = 0, i.e. result ≠ 0) |
| `RTZ` | 1 | Return if Zero True (Z = 1, i.e. result = 0) |
| `RFS` | 1 | Return if Sign False (S = 0, bit 7 = 0) |
| `RTS` | 1 | Return if Sign True (S = 1, bit 7 = 1) |
| `RFP` | 1 | Return if Parity False (P = 0, odd parity) |
| `RTP` | 1 | Return if Parity True (P = 1, even parity) |
| `RST n` | 1 | Restart: push PC, jump to page-0 address `n × 8` (n = 0–7) |
| `HLT` | 1 | Halt the processor |

`r` encodes: B=0, C=1, D=2, E=3, H=4, L=5, M=6 (memory at H:L), A=7.

#### Group 01 — Move, Port Input, Unconditional Jump/Call

| Mnemonic | Bytes | Description |
|----------|-------|-------------|
| `MOV dst, src` | 1 | Copy `src` into `dst` (both are register codes 0–7) |
| `IN p` | 1 | Read input port `p` (0–7) into A |
| `JMP a14` | 3 | Unconditional jump to 14-bit address |
| `CAL a14` | 3 | Call subroutine at 14-bit address; push return PC |
| `JFC a14` | 3 | Jump if Carry False |
| `JTC a14` | 3 | Jump if Carry True |
| `JFZ a14` | 3 | Jump if Zero False (Z=0, i.e. result ≠ 0) |
| `JTZ a14` | 3 | Jump if Zero True (Z=1, i.e. result = 0) |
| `JFS a14` | 3 | Jump if Sign False |
| `JTS a14` | 3 | Jump if Sign True |
| `JFP a14` | 3 | Jump if Parity False (P=0, odd parity) |
| `JTP a14` | 3 | Jump if Parity True (P=1, even parity) |
| `CFC a14` | 3 | Call if Carry False |
| `CTC a14` | 3 | Call if Carry True |
| `CFZ a14` | 3 | Call if Zero False |
| `CTZ a14` | 3 | Call if Zero True |

`MOV M, M` (src=6, dst=6) is an alternative encoding of `HLT`.

#### Group 10 — ALU Register Operations

All ALU ops use A as one implicit operand; the other is a register `r`.  Result is
placed in A.  All flags are updated.

| Mnemonic | Description |
|----------|-------------|
| `ADD r` | A ← A + r |
| `ADC r` | A ← A + r + CY (add with carry) |
| `SUB r` | A ← A − r (CY = borrow) |
| `SBB r` | A ← A − r − CY (subtract with borrow) |
| `ANA r` | A ← A AND r (CY cleared) |
| `XRA r` | A ← A XOR r (CY cleared) |
| `ORA r` | A ← A OR r (CY cleared) |
| `CMP r` | A − r: flags updated, **A unchanged** |

#### Group 11 — ALU Immediate, Port Output

| Mnemonic | Bytes | Description |
|----------|-------|-------------|
| `ADI d8` | 2 | A ← A + d8 |
| `ACI d8` | 2 | A ← A + d8 + CY |
| `SUI d8` | 2 | A ← A − d8 |
| `SBI d8` | 2 | A ← A − d8 − CY |
| `ANI d8` | 2 | A ← A AND d8 (CY cleared) |
| `XRI d8` | 2 | A ← A XOR d8 (CY cleared) |
| `ORI d8` | 2 | A ← A OR d8 (CY cleared) |
| `CPI d8` | 2 | A − d8: flags set, A unchanged |
| `OUT p` | 1 | Write A to output port `p` (0–23) |

### Flag Behaviour

| Flag | Set when | Cleared when |
|------|----------|--------------|
| CY | arithmetic borrow/overflow; bit out of rotation | AND, OR, XOR ops |
| Z | result = 0 | result ≠ 0 |
| S | result bit 7 = 1 (negative in two's complement) | result bit 7 = 0 |
| P | result has even number of 1-bits | result has odd number of 1-bits |

---

## Registers and Calling Convention

### Physical Register Map

| Register | Role in Oct backend |
|----------|---------------------|
| A | Accumulator — scratch, not preserved across calls |
| B | 1st local / 1st parameter slot |
| C | 2nd local / 2nd parameter slot |
| D | 3rd local / 3rd parameter slot |
| E | 4th local / 4th parameter slot |
| H | High byte of memory address — scratch, managed by code generator |
| L | Low byte of memory address — scratch, managed by code generator |
| M | Virtual register: memory byte at address H:L |

### Calling Convention

**Parameters** are passed in B, C, D, E (left-to-right, 1st param → B).
**Return value** is placed in A by the callee immediately before `RET`.
**A, H, L** are caller-saved (any callee may destroy them).
**B, C, D, E** are callee-saved — the callee must restore them if it modifies them.

However, because Oct functions have a maximum of 4 locals/params (hard limit enforced
by the type checker), callee-saved spilling is rarely needed.  Functions that use all 4
slots must treat B–E as their private local store and not overlap them with args they
receive (params and locals share the same 4 slots).

```
Caller sets up args:  B ← arg0, C ← arg1, D ← arg2, E ← arg3
                      CAL target
                      ; on return, A holds the result (if any)
Callee saves on entry: (save B–E if it needs extra scratch — rare in v1)
Callee places result:  MOV A, Rresult  ; before every RET
```

### Static Variable Memory Layout

Static variables are placed in the **RAM region**, beginning at base address
`0x2000` (8 KiB mark).  Each `static u8` variable occupies exactly 1 byte.  The
code generator keeps a monotonically increasing address counter:

```
0x2000  static[0]   (first declared static variable)
0x2001  static[1]
0x2002  static[2]
…
0x3FFF  (end of RAM — at most 8191 static bytes)
```

ROM occupies `0x0000–0x1FFF` and holds the program binary (code + entry stub).
ROM and RAM are distinct address spaces on the 8008 — the CPU never writes to ROM.

The code generator emits a **static initializer block** at the very start of the ROM
(before `CAL main`) that writes each static variable's initial value to its address
via H:L/M.

### Call Stack

The 8008 push-down stack holds 8 return addresses internally (not in RAM).  Level 0
is always the current PC, so 7 levels are available for `CAL` instructions.  The Oct
compiler enforces this at compile time via static call-graph analysis.

---

## IR → 8008 Assembly Mapping

The code generator walks the IR instruction list and emits assembly.  Each IR
instruction maps to a small, fixed sequence of 8008 mnemonics.

The pattern for arithmetic is always:

> **Load operand(s) into A → apply ALU op → store A into destination register**

IR register *n* maps to physical register according to the 4-slot table in §Calling
Convention.  IR register index 0 → B, 1 → C, 2 → D, 3 → E.  The accumulator A is
never an IR register; it is scratch.

### Data Movement

| IR Instruction | 8008 Assembly Emitted | Notes |
|----------------|-----------------------|-------|
| `LOAD_IMM Rdst, imm` | `MVI Rdst, imm` | 2 bytes; imm ∈ 0–255 |
| `LOAD_BYTE Rdst, addr` | `MVI H, hi(addr)` / `MVI L, lo(addr)` / `MOV A, M` / `MOV Rdst, A` | 8 bytes total; loads from static address |
| `STORE_BYTE Rsrc, addr` | `MVI H, hi(addr)` / `MVI L, lo(addr)` / `MOV A, Rsrc` / `MOV M, A` | 8 bytes total; writes to static address |

`hi(addr)` = `(addr >> 8) & 0x3F`  (high 6 bits of 14-bit address).
`lo(addr)` = `addr & 0xFF`          (low 8 bits).

### Arithmetic

| IR Instruction | 8008 Assembly Emitted |
|----------------|-----------------------|
| `ADD Rdst, Ra, Rb` | `MOV A, Ra` / `ADD Rb` / `MOV Rdst, A` |
| `SUB Rdst, Ra, Rb` | `MOV A, Ra` / `SUB Rb` / `MOV Rdst, A` |
| `ADD_IMM Rdst, Ra, imm` | `MOV A, Ra` / `ADI imm` / `MOV Rdst, A` |

The carry flag is set naturally by `ADD`/`SUB` and is available immediately after
for `carry()` intrinsic reads (see SYSCALL 15 below).

### Bitwise

| IR Instruction | 8008 Assembly Emitted | Notes |
|----------------|-----------------------|-------|
| `AND Rdst, Ra, Rb` | `MOV A, Ra` / `ANA Rb` / `MOV Rdst, A` | CY cleared by ANA |
| `OR Rdst, Ra, Rb` | `MOV A, Ra` / `ORA Rb` / `MOV Rdst, A` | CY cleared by ORA |
| `XOR Rdst, Ra, Rb` | `MOV A, Ra` / `XRA Rb` / `MOV Rdst, A` | CY cleared by XRA |
| `NOT Rdst, Ra` | `MOV A, Ra` / `XRI 0xFF` / `MOV Rdst, A` | XOR with all-ones flips all bits |

### Comparisons

Comparisons use the `CMP r` instruction (A − r, flags set, A unchanged).  The result
(0 or 1) is materialised into the destination register with a conditional branch pair:

**`CMP_EQ Rdst, Ra, Rb`** — set Rdst = 1 if Ra == Rb, else 0:
```asm
    MOV  A, Ra
    CMP  Rb          ; Z=1 iff Ra == Rb
    MVI  Rdst, 1     ; optimistic: assume equal
    JTZ  .eq_done    ; if Z set (equal), keep 1
    MVI  Rdst, 0     ; not equal: overwrite with 0
.eq_done:
```

**`CMP_NE Rdst, Ra, Rb`** — set Rdst = 1 if Ra ≠ Rb, else 0:
```asm
    MOV  A, Ra
    CMP  Rb
    MVI  Rdst, 0
    JTZ  .ne_done    ; if equal (Z=1), keep 0
    MVI  Rdst, 1
.ne_done:
```

**`CMP_LT Rdst, Ra, Rb`** — set Rdst = 1 if Ra < Rb (unsigned), else 0.
`CMP Rb` computes A − Rb; borrow (CY=1) means A < Rb:
```asm
    MOV  A, Ra
    CMP  Rb          ; CY=1 iff Ra < Rb (borrow = unsigned less-than)
    MVI  Rdst, 1
    JTC  .lt_done    ; if carry (Ra < Rb), keep 1
    MVI  Rdst, 0
.lt_done:
```

**`CMP_GT Rdst, Ra, Rb`** — set Rdst = 1 if Ra > Rb (unsigned), else 0.
Swap operands: Rb < Ra means Ra > Rb:
```asm
    MOV  A, Rb       ; note: B in accumulator
    CMP  Ra          ; CY=1 iff Rb < Ra, i.e. Ra > Rb
    MVI  Rdst, 1
    JTC  .gt_done
    MVI  Rdst, 0
.gt_done:
```

### Control Flow

| IR Instruction | 8008 Assembly Emitted | Notes |
|----------------|-----------------------|-------|
| `BRANCH_Z Rcond, label` | `MOV A, Rcond` / `CPI 0` / `JTZ label` | Jump if Rcond == 0 |
| `BRANCH_NZ Rcond, label` | `MOV A, Rcond` / `CPI 0` / `JFZ label` | Jump if Rcond ≠ 0 |
| `JUMP label` | `JMP label` | Unconditional, 3 bytes |
| `LABEL name` | `name:` | Defines a label at current address |
| `CALL name` | `CAL name` | Pushes PC+3, jumps to name |
| `RET` (void) | `RET` | Pops return address, continues |
| `RET Rval` | `MOV A, Rval` / `RET` | Places return value in A |
| `HALT` | `HLT` | Stops the processor |

`RET` (unconditional return) is a pseudo-mnemonic for `RFC` — return if carry false.
Since the code generator never generates code that calls `RET` with carry set
intentionally, using `RFC` as the unconditional return is the standard 8008 practice.
The assembler accepts both `RET` and `RFC`; they emit the same byte.

### SYSCALLs — 8008 Intrinsics

The 8008 SYSCALL numbers 3–4, 11–16, 20–27, and 40–63 are handled directly in the
code generator; no operating-system trampoline is involved.  Each SYSCALL lowers to
a short inline sequence.

#### SYSCALL 3 — `adc(a, b)` — Add with Carry

```asm
    MOV  A, Ra
    ADC  Rb          ; A ← Ra + Rb + CY
    MOV  Rdst, A
```

#### SYSCALL 4 — `sbb(a, b)` — Subtract with Borrow

```asm
    MOV  A, Ra
    SBB  Rb          ; A ← Ra − Rb − CY
    MOV  Rdst, A
```

#### SYSCALL 11–14 — Rotations

| SYSCALL | Oct intrinsic | Assembly emitted |
|---------|---------------|------------------|
| 11 | `rlc(a)` | `MOV A, Ra` / `RLC` / `MOV Rdst, A` |
| 12 | `rrc(a)` | `MOV A, Ra` / `RRC` / `MOV Rdst, A` |
| 13 | `ral(a)` | `MOV A, Ra` / `RAL` / `MOV Rdst, A` |
| 14 | `rar(a)` | `MOV A, Ra` / `RAR` / `MOV Rdst, A` |

Rotations update CY, so a `carry()` call immediately after a rotation read the
bit that was rotated out.

#### SYSCALL 15 — `carry()` — Read Carry Flag

The 8008 has no "read flag to register" instruction.  The trick: `ACI 0` computes
`A ← A + 0 + CY`, so priming A with 0 before the `ACI` materialises CY into A:

```asm
    MVI  A, 0
    ACI  0           ; A = 0 + 0 + CY = CY (0 or 1)
                     ; note: ACI sets CY=0 (no overflow), but that is fine
                     ;       because carry() is only valid once immediately
                     ;       after the arithmetic that produced it
    MOV  Rdst, A
```

#### SYSCALL 16 — `parity(a)` — Read Parity Flag

The P flag is set by any ALU result.  `ORA A` (OR A with itself) is a zero-cost
way to refresh all flags from the current value of A without changing A:

```asm
    MOV  A, Ra
    ORA  A           ; flags updated from A; P=1 iff popcount(A) is even
    MVI  Rdst, 0     ; assume odd parity
    JFP  .par_done   ; JFP = jump if parity false (odd); keep 0
    MVI  Rdst, 1     ; even parity: set 1
.par_done:
```

#### SYSCALL 20 + p — `in(p)` — Read Input Port

```asm
    IN   p           ; A ← value on input port p (p ∈ 0–7)
    MOV  Rdst, A
```

`IN p` is a 1-byte instruction.  Port number `p` is encoded directly in the opcode.

#### SYSCALL 40 + p — `out(p, val)` — Write Output Port

```asm
    MOV  A, Rval
    OUT  p           ; output port p ← A (p ∈ 0–23)
```

`OUT p` is a 1-byte instruction.  Port number `p` is encoded directly in the opcode.

---

## Assembly Text Format

The 8008 assembler accepts the following text format.

### Structure

```asm
; Oct-generated 8008 assembly
; Source: program.oct
; Generated: 2026-04-20

    ORG  0x0000          ; ROM starts at address 0

; --------------- static initializers ---------------
; Initialize static variable 'counter' at RAM address 0x2000
    MVI  H, 0x20         ; H ← high byte of 0x2000
    MVI  L, 0x00         ; L ← low byte of 0x2000
    MVI  A, 0            ; initial value = 0
    MOV  M, A            ; write to RAM[0x2000]

; --------------- entry point ---------------
    CAL  main            ; call main
    HLT                  ; halt after main returns

; --------------- function: main ---------------
main:
    ; ... function body ...
    RET                  ; return void (RFC)
```

### ORG Directive

`ORG address` sets the assembler's location counter.  Programs begin with
`ORG 0x0000`.  There is no page-relative constraint on the 8008 (unlike the 4004) —
all jump instructions use full 14-bit addresses, so branch targets may be anywhere in
the 16 KB ROM without page alignment concerns.

### Memory Map

```
0x0000–0x1FFF   ROM: program code + static initializer preamble (8 KB)
0x2000–0x3FFF   RAM: static variable data (8 KB)
```

The total ROM image packaged into the Intel HEX file is 16 384 bytes (0x0000–0x3FFF),
covering both regions.  Unused ROM bytes are padded to `0xFF`.

### Label Syntax

Labels are identifiers followed by a colon on their own line (labels may also share
a line with an instruction, but the code generator always places them alone):

```asm
loop_0_start:
    MOV  A, B
```

Labels are resolved to 14-bit addresses in the second assembler pass.  The code
generator names labels with a function-scoped numbering scheme:

| Pattern | Meaning |
|---------|---------|
| `func_name` | Function entry point |
| `loop_N_start` | Top of while/loop number N |
| `loop_N_end` | Exit of while/loop number N |
| `if_N_else` | Else branch of if number N |
| `if_N_end` | End of if/else number N |
| `cmp_N_done` | Post-materialisation label for comparison N |

### Comment Syntax

Line comments start with `;`:

```asm
    MVI  A, 0x3A    ; load ASCII ':' into accumulator
```

Block comments are not supported. Use multiple `;` lines.

### Instruction Format

```asm
    MNEMONIC  [operands]          ; optional comment
```

- Indentation: 4 spaces for instructions, 0 for labels
- Operand separator: `, ` (comma-space)
- Register names: `A`, `B`, `C`, `D`, `E`, `H`, `L`, `M`
- Immediate values: decimal (`42`) or hex with `0x` prefix (`0xFF`)
- Addresses: 14-bit, hex preferred (`0x1A00`)

---

## Two-Pass Assembler Design

The assembler processes the assembly text in two passes to resolve forward label
references.

### Instruction Sizes

| Instruction form | Bytes |
|------------------|-------|
| Single-register ops (MOV, ADD, SUB, ANA, XRA, ORA, CMP, INR, DCR, rotations, RET/RFC/RTZ/etc., IN, OUT, HLT) | 1 |
| Register + immediate (MVI, ADI, ACI, SUI, SBI, ANI, XRI, ORI, CPI) | 2 |
| Address instructions (JMP, CAL, JFC/JTC/JFZ/JTZ/JFS/JTS/JFP/JTP, CFC/CTC/etc.) | 3 |

### Pass 1 — Symbol Collection

Scan the assembly text from top to bottom.  For each line:

1. Strip leading/trailing whitespace; remove comments (`;` through end of line).
2. If blank after stripping, skip.
3. If the line is a label definition (`identifier:`), record `label → current_address`
   in the symbol table.  A label definition does not advance the address counter.
4. If the line starts with `ORG`, set the address counter to the operand value.
5. Otherwise, determine the instruction size (1, 2, or 3 bytes from the table above)
   and advance the address counter accordingly.

At the end of Pass 1 the **symbol table** maps every label to a 14-bit ROM address.

Example:
```
symbol_table = {
    "main":          0x0008,
    "loop_0_start":  0x0012,
    "loop_0_end":    0x001E,
    "cmp_0_done":    0x0024,
}
```

### Pass 2 — Code Emission

Scan the assembly text again.  For each instruction:

1. Parse the mnemonic and operands.
2. If an operand is a label, substitute the 14-bit address from the symbol table.
3. Encode the instruction to bytes using the encoding tables below.
4. Append bytes to the output buffer.

At the end of Pass 2, the output buffer contains the binary ROM image.

### Instruction Encoding

The exact byte values come from the Intel 8008 User's Manual (MCS-8, Intel, 1972) and
are authoritative in `intel8008_simulator/simulator.py`.  Key encodings used by the
code generator:

**Register code** (used in all register operands):
```
B=0, C=1, D=2, E=3, H=4, L=5, M=6, A=7
```

**Group 00 — 1-byte register operations:**
```
MVI r, d8:   (0x06 | r<<3),  d8        (2 bytes)
INR r:        0x00 | r<<3               (0x00, 0x08, 0x10, 0x18, 0x20, 0x28, 0x38 for B–A)
DCR r:        0x01 | r<<3               (0x01, 0x09, 0x11, 0x19, 0x21, 0x29, 0x39)
RLC:          0x02
RRC:          0x0A
RAL:          0x12
RAR:          0x1A
RFC / RET:    0x03
RFZ:          0x0B
RFS:          0x13
RFP:          0x1B
RTC:          0x23
RTZ:          0x2B
RTS:          0x33
RTP:          0x3B
HLT:          0xFF  (also 0x00 = HLT variant; use 0xFF for clarity)
RST n:        0x05 | n<<3               (n = 0–7)
```

**Group 01 — 1-byte MOV, IN; 3-byte jumps/calls:**
```
MOV dst, src: 0x40 | (dst<<3) | src    (1 byte)
IN p:         0x41 | p<<3               (ports 0–7: 0x41, 0x49, 0x51, 0x59, 0x61, 0x69, 0x71, 0x79)
JMP a14:      0x44, lo(a14), hi6(a14)  (3 bytes; hi6 = a14>>8, in low 6 bits of byte)
CAL a14:      0x46, lo(a14), hi6(a14)
JFC a14:      0x40, lo(a14), hi6(a14)  (bits 3-1 select condition: carry)
JTC a14:      0x60, lo(a14), hi6(a14)
JFZ a14:      0x48, lo(a14), hi6(a14)
JTZ a14:      0x68, lo(a14), hi6(a14)
JFS a14:      0x50, lo(a14), hi6(a14)
JTS a14:      0x70, lo(a14), hi6(a14)
JFP a14:      0x58, lo(a14), hi6(a14)
JTP a14:      0x78, lo(a14), hi6(a14)
```

Where `lo(a14) = a14 & 0xFF` and `hi6(a14) = (a14 >> 8) & 0x3F`.

**Group 10 — ALU register (1 byte each):**
```
ADD r:   0x80 | r
ADC r:   0x88 | r
SUB r:   0x90 | r
SBB r:   0x98 | r
ANA r:   0xA0 | r
XRA r:   0xA8 | r
ORA r:   0xB0 | r
CMP r:   0xB8 | r
```

**Group 11 — ALU immediate (2 bytes) + OUT (1 byte):**
```
ADI d8:  0x04, d8
ACI d8:  0x0C, d8
SUI d8:  0x14, d8
SBI d8:  0x1C, d8
ANI d8:  0x24, d8
XRI d8:  0x2C, d8
ORI d8:  0x34, d8
CPI d8:  0x3C, d8
OUT p:   0x41 | (p<<1) | 1   — ports 0–23
         (0x41, 0x43, 0x45, …)  — exact encoding: see Intel 8008 manual §4.5
```

> **Note:** The OUT instruction encoding uses a different slot arrangement from IN.
> The implementation must reference the MCS-8 User's Manual table for ports 0–23.
> For port 0: `0x41`; port 1: `0x43`; incrementing by 2 for each port.

### Error Handling

The assembler halts with a descriptive error on:

- **Undefined label** — a branch/call target not defined in the assembly text
- **ROM overflow** — the assembled binary exceeds 8 191 bytes (ROM region limit)
- **Static overflow** — more than 8 191 static variables declared
- **Immediate overflow** — an immediate value does not fit in 8 bits (0–255)
- **Port out of range** — IN port ≥ 8 or OUT port ≥ 24
- **Unknown mnemonic** — not in the supported instruction set

---

## Intel HEX Format

The packager produces an **Intel HEX** file from the binary ROM image.  This is the
same format used by the Intel 4004 backend; only the image size and address range differ.

### Record Format

Each line is one record:

```
:LLAAAATT[DD...]CC
```

| Field | Meaning |
|-------|---------|
| `:` | Start code |
| `LL` | Byte count (2 hex digits) |
| `AAAA` | 16-bit load address of first data byte in record (4 hex digits) |
| `TT` | Record type: `00` = data, `01` = end-of-file |
| `DD...` | Data bytes (2 hex digits each) |
| `CC` | Checksum |

### Checksum

```python
def checksum(record_bytes_without_colon: bytes) -> int:
    total = sum(record_bytes_without_colon) & 0xFF
    return (~total + 1) & 0xFF   # two's complement
```

### Packaging Constraints

1. Emit data records of **16 bytes** each (standard HEX line length).
2. The full ROM image is **16 384 bytes** (0x0000–0x3FFF), covering both ROM (code)
   and RAM (data) address regions.
3. Pad unused bytes in ROM region to `0xFF` (erased flash state).
4. Leave RAM region bytes as `0x00` (zeroed RAM at power-on).
5. Emit the End-of-File record (`TT=01`, `LL=00`, `AAAA=0000`) as the last line.

### Example Output

```
:100000003E00263020003E003620310000000000B4
:10001000...
...
:00000001FF
```

---

## Summary of Backend Passes

```
Oct IR (IrProgram)
  │
  ▼  Pass: Pre-flight validation  (intel-8008-ir-validator)
  │  - Verify all opcodes are in the 8008-supported set
  │  - Verify LOAD_IMM / ADD_IMM immediates ∈ 0–255
  │  - Verify SYSCALL numbers are in the 8008 whitelist (3–4, 11–16, 20–27, 40–63)
  │  - Return list of human-readable errors; abort if non-empty
  │
  ▼  Pass: Static allocation  (ir-to-intel-8008-compiler)
  │  - Walk IR for STORE_BYTE / LOAD_BYTE to identify static variables
  │  - Assign each static an address starting at 0x2000
  │
  ▼  Pass: Code generation  (ir-to-intel-8008-compiler)
  │  - Emit the static-initializer preamble (MVI H / MVI L / MVI A / MOV M, A per static)
  │  - Emit the entry stub (CAL main; HLT)
  │  - Walk IR instructions, emit 8008 assembly per the IR → ASM mapping table
  │  - Assign IR register indices (0–3) to physical registers (B–E)
  │  - Generate unique local labels for comparisons, branches, loops
  │  → Assembly text (.asm)
  │
  ▼  Pass: Assembler Pass 1 — Symbol Collection  (intel-8008-assembler)
  │  - Scan assembly text; count instruction sizes (1, 2, or 3 bytes)
  │  - Build symbol table: label → 14-bit address
  │
  ▼  Pass: Assembler Pass 2 — Code Emission  (intel-8008-assembler)
  │  - Encode each instruction; substitute labels with addresses
  │  - Validate immediates, port numbers, ROM overflow
  │  → Flat binary buffer (≤ 8 192 bytes of code)
  │
  ▼  Pass: Intel HEX Packaging  (intel-8008-packager)
  │  - Construct full 16 384-byte image (ROM code + RAM zeros)
  │  - Pad ROM with 0xFF; emit 16-byte records; compute checksums
  │  - Emit End-of-File record
  → .hex file ready for ROM programmer or simulator
```

---

## Divergence from NIB01

This backend is structurally parallel to the Nib/4004 backend (NIB01) but differs in
the following ways:

| Aspect | NIB01 (4004) | OCT01 (8008) |
|--------|-------------|-------------|
| Word width | 4 bits (nibble) | 8 bits (byte) |
| Address space | 12 bits (4 KB ROM) | 14 bits (16 KB) |
| RAM model | External RAM chips; SRC/RDM/WRM | External RAM; H:L/MOV M |
| Jump instruction size | 2 bytes (page-relative JCN) | 3 bytes (full-address JTZ etc.) |
| Page alignment | Required for JCN (256-byte page) | Not required (14-bit absolute) |
| Call instruction | `JMS` (2 bytes) | `CAL` (3 bytes) |
| Return instruction | `BBL d4` (returns value in A) | `RFC` / `RET` (value in A separately) |
| Stack depth | 4 levels (3 usable) | 8 levels (7 usable) |
| Port I/O | ROM port via `WRR`/`RDR` | Dedicated `IN p` / `OUT p` |
| ROM image size | 4 096 bytes | 16 384 bytes |

---

## Version History

| Version | Date | Description |
|---------|------|-------------|
| 0.1.0 | 2026-04-20 | Initial backend specification |
