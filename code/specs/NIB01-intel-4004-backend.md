# NIB01 — Intel 4004 Backend Specification

## Overview

This document specifies the Nib compiler's backend: the code generator that translates
the Nib compiler's intermediate representation (IR) into Intel 4004 assembly text, and
the two-pass assembler that converts that assembly text into a binary ROM image encoded
in Intel HEX format.

The backend has three stages:

```
Nib IR (from semantic checker)
    ↓  [Code Generator]
4004 Assembly Text (.asm)
    ↓  [Two-Pass Assembler]
Relocatable Object (label → address map)
    ↓  [Intel HEX Packager]
Intel HEX File (.hex)
    ↓  [ROM Programmer]
4004 ROM Chip
```

### Relationship to the Simulator

The 4004 behavioral simulator (`code/packages/*/intel4004-sim`) and gate-level
simulator (`code/packages/*/intel4004-gatelevel`) can both execute the binary output
of this backend. This means the full pipeline is:

```
Write Nib source
  → compile with nibcc
  → run on intel4004-sim (behavioral, fast)
  → run on intel4004-gatelevel (gates, slow but accurate to silicon)
```

---

## Intel 4004 ISA Overview

The Intel 4004 (1971) has 46 instructions. They fall into seven categories:

### Machine Word Format

Instructions are either **1 byte** (8 bits) or **2 bytes** (16 bits). The first byte
always contains the opcode. Some instructions pack small operands (register numbers,
condition codes) into the opcode byte itself.

```
1-byte instruction:  [opcode:4][operand:4]
2-byte instruction:  [opcode:4][operand:4]  [data:8]
```

The 4004 fetches 2 bytes at a time from ROM, so even 1-byte instructions have a
partner byte. Instruction alignment therefore matters.

### Register Operations

| Mnemonic | Bytes | Description                                          |
|----------|-------|------------------------------------------------------|
| LDM d4   | 1     | Load 4-bit immediate d4 into accumulator A           |
| LD Rr    | 1     | Load register Rr into A                              |
| XCH Rr   | 1     | Exchange A with register Rr                          |
| ADD Rr   | 1     | Add Rr to A; carry flag = overflow                   |
| SUB Rr   | 1     | Subtract Rr from A; carry = borrow                   |
| INC Rr   | 1     | Increment register Rr (does not affect carry)        |
| FIM Rp d8| 2     | Load 8-bit immediate d8 into pair Rp                 |
| SRC Rp   | 1     | Send pair Rp as address to RAM or ROM port           |
| FIN Rp   | 1     | Fetch indirect: load ROM byte at PC+R0R1 into pair Rp|

### Accumulator Operations

| Mnemonic | Bytes | Description                                          |
|----------|-------|------------------------------------------------------|
| CLB      | 1     | Clear A and carry (A=0, CY=0)                        |
| CLC      | 1     | Clear carry (CY=0)                                   |
| CMC      | 1     | Complement carry (CY=~CY)                            |
| CMA      | 1     | Complement A (A=~A, bitwise NOT of nibble)           |
| RAL      | 1     | Rotate A left through carry                          |
| RAR      | 1     | Rotate A right through carry                         |
| TCC      | 1     | Transfer carry to A, then clear carry                |
| DAC      | 1     | Decrement A                                          |
| IAC      | 1     | Increment A                                          |
| DAA      | 1     | Decimal adjust A (adds 6 if A>9 or carry is set)     |
| KBP      | 1     | Keyboard process: decode 1-hot nibble to 0–4         |
| DCL      | 1     | Designate command line: select RAM bank from A       |

### Branching

| Mnemonic   | Bytes | Description                                         |
|------------|-------|-----------------------------------------------------|
| JUN a12    | 2     | Jump unconditional to 12-bit address a12            |
| JCN c a8   | 2     | Jump conditional: condition c, 8-bit page-relative  |
| JMS a12    | 2     | Jump to subroutine at a12; push return addr to stack|
| BBL d4     | 1     | Return from subroutine; pop stack; A = d4           |
| ISZ Rr a8  | 2     | Increment Rr; jump to page-relative a8 if not zero  |

JCN condition bits (4-bit condition code):
- Bit 3: invert (1 = jump if condition NOT met)
- Bit 2: test carry flag
- Bit 1: test accumulator zero
- Bit 0: test test pin (external hardware signal)

### RAM Operations

| Mnemonic | Bytes | Description                                          |
|----------|-------|------------------------------------------------------|
| RDM      | 1     | Read RAM main character into A                       |
| RD0–RD3  | 1     | Read RAM status characters 0–3 into A                |
| WRM      | 1     | Write A to RAM main character                        |
| WR0–WR3  | 1     | Write A to RAM status characters 0–3                 |
| ADM      | 1     | Add RAM main character to A with carry               |
| SBM      | 1     | Subtract RAM main character from A with borrow       |
| WMP      | 1     | Write A to RAM output port                           |

### ROM Operations

| Mnemonic | Bytes | Description                                          |
|----------|-------|------------------------------------------------------|
| WRR      | 1     | Write A to ROM I/O port (SRC must set port address)  |
| RDR      | 1     | Read ROM I/O port into A                             |

### Input Operations

| Mnemonic | Bytes | Description                                          |
|----------|-------|------------------------------------------------------|
| WPM      | 1     | Write A to program RAM (used with extended chips)    |
| NOP      | 1     | No operation                                         |

---

## Registers and Virtual Register Mapping

The Nib compiler assigns virtual registers to physical 4004 registers according to
the following convention.

### Calling Convention

```
Caller-saved (scratch) registers:
  R0–R3   — arguments / scratch (pair P0 and P1)
  A       — accumulator (always caller-saved; every operation touches it)
  CY      — carry flag (always caller-saved)

Callee-saved registers:
  R4–R11  — local variables (pairs P2, P3, P4, P5)
  R12–R15 — reserved for the code generator (pairs P6, P7)
```

### Argument Passing

Functions take up to 4 u4 arguments, or 2 u8 arguments, or combinations:

| Argument | Physical register(s) |
|----------|---------------------|
| arg[0]: u4 | R0 |
| arg[1]: u4 | R1 |
| arg[2]: u4 | R2 |
| arg[3]: u4 | R3 |
| arg[0]: u8 | P0 (R0:R1) |
| arg[1]: u8 | P1 (R2:R3) |

Return values follow the same layout: u4 in R0, u8 in P0.

### Local Variable Allocation

The compiler performs **register allocation** for local variables (`let` declarations
and for-loop variables). Because there is no software stack, all locals must fit in
the physical registers R4–R11. If a function declares more locals than there are
available registers, the compiler emits an error.

Register allocation uses a simple linear scan approach:

1. Assign each local a virtual register number (v0, v1, v2, …)
2. Determine the live range of each virtual register (first use to last use)
3. Greedily assign virtual registers to physical registers in order
4. If no physical register is free when a virtual register becomes live, fail

### Special Registers

| Register | Purpose |
|----------|---------|
| R12:R13 (P6) | Loop counter (for for-loops) |
| R14:R15 (P7) | Temporary scratch for multi-step operations |

---

## IR Opcode to 4004 Assembly Mapping

The Nib IR is a simple 3-address code. Each IR instruction maps to one or more 4004
assembly instructions.

### Load Operations

| IR Opcode          | 4004 Assembly                    | Notes                         |
|--------------------|----------------------------------|-------------------------------|
| `LOAD_IMM dst, imm`| `LDM imm; XCH dst`               | u4: imm fits in nibble        |
| `LOAD_IMM dst, imm`| `FIM pair, imm`                  | u8: imm fits in byte          |
| `LOAD_VAR dst, src`| `LD src; XCH dst`                | Copy between registers        |
| `LOAD_STATIC dst, addr`| `SRC pair_for_addr; RDM; XCH dst` | Read from RAM                |

### Arithmetic Operations

| IR Opcode           | 4004 Assembly                    | Notes                         |
|---------------------|----------------------------------|-------------------------------|
| `ADD dst, a, b`     | `LD a; ADD b; XCH dst`           | u4 plain add                  |
| `SUB dst, a, b`     | `LD a; SUB b; XCH dst`           | u4 subtract                   |
| `WRAP_ADD dst, a, b`| `LD a; CLC; ADD b; ANL 0xF; XCH dst` | u4 wrap: mask carry away |
| `SAT_ADD dst, a, b` | `LD a; CLC; ADD b; JCN NC skip; LDM 0xF; skip: XCH dst` | u4 saturate |
| `BCD_ADD dst, a, b` | `LD a; CLC; ADD b; DAA; XCH dst` | bcd add with decimal adjust  |

Note: `ANL` is the AND logical with accumulator. `JCN NC skip` = jump if carry not set.

### Bitwise Operations

| IR Opcode         | 4004 Assembly              | Notes                      |
|-------------------|---------------------------|----------------------------|
| `AND dst, a, b`   | `LD a; ANL b; XCH dst`    | Bitwise AND                |
| `OR dst, a, b`    | `LD a; ORL b; XCH dst`    | Bitwise OR                 |
| `XOR dst, a, b`   | `LD a; XRL b; XCH dst`    | Bitwise XOR                |
| `NOT dst, a`      | `LD a; CMA; XCH dst`      | Bitwise complement nibble  |

Note: The 4004 ANL/ORL/XRL instructions are immediate (operate A with a nibble
constant). Register-to-register variants require one operand in A, the other in a
register: load into A, then combine with XCH-then-ANL sequence. The code generator
handles this.

### Comparison Operations

Comparisons use subtract-and-check-carry:

| IR Opcode        | 4004 Assembly                                         |
|------------------|-------------------------------------------------------|
| `EQ dst, a, b`   | `LD a; SUB b; JCN ZN skip; LDM 1; JUN done; skip: LDM 0; done: XCH dst` |
| `LT dst, a, b`   | `LD a; SUB b; TCC; XCH dst`  (TCC: transfer carry to A) |
| `GT dst, a, b`   | `LD b; SUB a; TCC; XCH dst`  (swap operands for a>b) |

### Branch Operations

| IR Opcode          | 4004 Assembly                    | Notes                        |
|--------------------|----------------------------------|------------------------------|
| `JMP label`        | `JUN label`                      | Unconditional 12-bit jump    |
| `JIF cond, label`  | `LD cond; JCN NZ label`          | Jump if cond != 0 (truthy)   |
| `JNIF cond, label` | `LD cond; JCN Z label`           | Jump if cond == 0 (falsy)    |

### For Loop Compilation

A for loop `for i: u4 in low..high { body }` compiles to:

```asm
    ; Load loop counter = high - low
    LDM  high
    SUB  low         ; A = count = high - low
    XCH  R12         ; R12 = loop counter

    ; Load loop variable = low
    LDM  low
    XCH  Ri          ; Ri = i (loop variable, caller's register for i)

loop_top:
    ; ... body instructions ...

    ; Increment i
    LD   Ri
    IAC
    XCH  Ri

    ; Decrement counter and loop
    ISZ  R12, loop_top   ; R12 -= 1; if R12 != 0, jump to loop_top

loop_end:
```

The `ISZ` (increment Rr, jump if not zero) instruction is used backwards here:
we pre-decrement by using the loop counter and ISZ's semantics. Actually, since ISZ
increments (not decrements), we use a DJNZ-like sequence:

```asm
    ; Decrement counter:
    LD   R12
    DAC              ; Decrement A
    XCH  R12
    JCN  NZ loop_top ; Jump if R12 != 0
```

### Function Call and Return

| IR Opcode          | 4004 Assembly                    | Notes                         |
|--------------------|----------------------------------|-------------------------------|
| `CALL label`       | `JMS label`                      | Push return address, jump     |
| `RETURN val`       | `LD val; BBL 0` OR `BBL imm`     | Return, popping stack         |

`BBL d4` returns from subroutine AND loads d4 into A. For functions returning u4,
the return value is placed in R0 (A after BBL). For void functions, `BBL 0` is used.

---

## Assembly Text Format

The Nib code generator emits 4004 assembly text in the following format.

### Structure

```asm
; Nib-generated 4004 assembly
; Source: program.nib
; Generated: 2026-04-12

    ORG 0x000         ; Program starts at ROM address 0

; --------------- static initializers ---------------
; Initialize static variable 'counter' (at RAM address 0x00)
    FIM  P0, 0x00     ; P0 = RAM address for 'counter'
    SRC  P0           ; Send address
    LDM  0            ; Initial value = 0
    WRM               ; Write to RAM

; --------------- entry point ---------------
    JMS  main         ; Call main

halt:
    JUN  halt         ; Infinite loop (halt)

; --------------- function: main ---------------
main:
    ; ... function body ...
    BBL  0            ; return void
```

### ORG Directive

`ORG address` sets the current assembly location counter to `address`. The 4004 ROM
starts at address 0x000. Programs should begin with `ORG 0x000`.

The 4004 has 4 pages of ROM (each 256 bytes = 0x000–0x0FF, 0x100–0x1FF, 0x200–0x2FF,
0x300–0x3FF). Page-relative branches (`JCN`) can only jump within the current page.
The code generator must place jump targets on the same page as their branches.

### Label Syntax

Labels are identifiers followed by a colon on their own line (or any line):

```asm
loop_top:
    LD R0
```

Labels are referenced by name in branch instructions. The assembler resolves labels
to 12-bit addresses in the second pass.

### Comment Syntax

Line comments start with `;`:

```asm
    LDM 5    ; load immediate 5 into A
```

Block comments are not supported. Use multiple `;` lines.

### Instruction Format

```asm
    MNEMONIC  [operands]   ; optional comment
```

- Indentation: 4 spaces for instructions, 0 for labels
- Operands are comma-separated with a single space after the mnemonic
- Register names: `R0`–`R15`, `P0`–`P7`, `A` (accumulator), `CY` (carry)
- Immediate values: decimal (`5`) or hex (`0xF`)
- Addresses: hex preferred for clarity (`0x3F2`)

---

## Two-Pass Assembler Design

The assembler processes the assembly text in two passes.

### Pass 1: Symbol Collection

Scan the text from top to bottom. For each line:

1. Strip leading/trailing whitespace, remove comments
2. If the line is blank after stripping, skip
3. If the line ends with `:` (after identifier), record the label → current address
4. Otherwise, parse the mnemonic and count the instruction size (1 or 2 bytes)
5. Advance the location counter by the instruction size

At the end of Pass 1, the **symbol table** maps every label to its ROM address.

Example after Pass 1:
```
symbol_table = {
    "main":     0x010,
    "loop_top": 0x018,
    "loop_end": 0x020,
    "halt":     0x008,
}
```

### Pass 2: Code Emission

Scan the text again from top to bottom. For each instruction:

1. Parse the mnemonic and operands
2. If an operand is a label, look it up in the symbol table from Pass 1
3. Verify that page-relative branches have targets on the same page
4. Encode the instruction to bytes (see encoding tables below)
5. Append bytes to the output buffer

At the end of Pass 2, the output buffer contains the complete binary ROM image.

### Instruction Encoding

Each 4004 instruction maps to one or two bytes:

```
LDM d4:       0xD_ where _ = d4          (1 byte)
LD  Rr:       0xA_ where _ = r           (1 byte)
XCH Rr:       0xB_ where _ = r           (1 byte)
ADD Rr:       0x8_ where _ = r           (1 byte)
SUB Rr:       0x9_ where _ = r           (1 byte)
INC Rr:       0x6_ where _ = r           (1 byte)
FIM Rp d8:    0x2_ 0xdd where _ = p*2, dd = d8 (2 bytes)
CLB:          0xF0                        (1 byte)
CLC:          0xF1                        (1 byte)
CMC:          0xF3                        (1 byte)
CMA:          0xF4                        (1 byte)
RAL:          0xF5                        (1 byte)
RAR:          0xF6                        (1 byte)
TCC:          0xF7                        (1 byte)
DAC:          0xF8                        (1 byte)
IAC:          0xFA                        (1 byte)
DAA:          0xFB                        (1 byte)
NOP:          0x00                        (1 byte)
JUN a12:      0x4_ 0xaa where _= a[11:8], aa = a[7:0] (2 bytes)
JMS a12:      0x5_ 0xaa  (same encoding as JUN)       (2 bytes)
JCN c a8:     0x1_ 0xaa where _ = c, aa = page-relative addr (2 bytes)
ISZ Rr a8:    0x7_ 0xaa where _ = r, aa = page-relative addr (2 bytes)
BBL d4:       0xC_ where _ = d4          (1 byte)
SRC Rp:       0x2_ where _ = p*2+1       (1 byte)
RDM:          0xE9                        (1 byte)
WRM:          0xE0                        (1 byte)
ANL:          (accumulator AND with I/O — see 4004 manual for full ANL variants)
ORL:          (accumulator OR — see manual)
XRL:          (accumulator XOR — see manual)
```

Note: `ANL`, `ORL`, `XRL` are ROM-port immediate operations in the official 4004 ISA,
not register-to-register. For bitwise AND/OR/XOR between registers, the code generator
uses the XCH/ADD/SUB trick: move one operand to a scratchpad register, XCH into A,
then use the accumulator form.

The Nib assembler supports a slightly extended pseudo-ISA that the code generator
targets, with the assembler expanding pseudo-instructions to true 4004 bytes.

### Error Handling

The assembler halts with a descriptive error on:
- Undefined label reference
- Page-relative branch out of range (target not on same 256-byte page)
- Value overflow (e.g., LDM with value > 15)
- ROM overflow (program exceeds 4096 bytes)
- Unknown mnemonic

---

## Intel HEX Format

The final output of the assembler is an **Intel HEX** file. Intel HEX is a text format
for representing binary data as ASCII hex digits. ROM programmers understand it
natively.

### Record Format

Each line of an Intel HEX file is one **record**. Records have the format:

```
:LLAAAATT[DD...]CC
```

Where:
- `:` — start code (colon)
- `LL` — byte count: number of data bytes in this record (2 hex digits)
- `AAAA` — address: 16-bit load address of the first data byte (4 hex digits)
- `TT` — record type (2 hex digits):
  - `00` — Data record
  - `01` — End of File record
- `DD...` — data bytes (2 hex digits each)
- `CC` — checksum: two's complement of the sum of all bytes from LL to last DD

### Checksum Calculation

```python
def checksum(bytes_without_colon):
    total = sum(bytes_without_colon) & 0xFF
    return (~total + 1) & 0xFF
```

### Example Output

```
:10000000D0A0F0F8F1F3F4FAFC4002580040005200AF
:10001000BBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBCC
:00000001FF
```

Line 1: 16 bytes starting at address 0x0000
Line 2: 16 bytes starting at address 0x0010
Line 3: End of File record

### ROM Packaging Constraints

The 4004 ROM is divided into **pages** of 256 bytes. The Intel HEX packager must:

1. Emit records of at most 16 bytes each (standard HEX line length)
2. Group records by page (new record address when page boundary is crossed)
3. Pad unused ROM bytes to `0xFF` (erased ROM state)
4. Emit the End of File record (`0x01`) as the last line
5. Ensure total ROM image size is exactly 4096 bytes

The packager takes the binary output buffer from the assembler and produces the HEX
file. It does not know about labels or the source program — it only handles bytes and
addresses.

---

## Summary of Backend Passes

```
Nib IR
  │
  ▼  Pass: Register Allocation
  │  - Assign virtual registers to physical R0–R15
  │  - Fail if too many locals for available registers
  │  - Record the allocation map
  │
  ▼  Pass: Code Generation
  │  - Walk IR instructions, emit 4004 pseudo-assembly lines
  │  - Use allocation map for register operands
  │  - Generate labels for: function entry points, branch targets, loop tops/ends
  │  - Emit static initializer block at ROM address 0
  │
  ▼  Pass: Assembly Pass 1 (Symbol Collection)
  │  - Scan assembly text
  │  - Count instruction sizes (1 or 2 bytes each)
  │  - Build symbol table: label → 12-bit ROM address
  │
  ▼  Pass: Assembly Pass 2 (Code Emission)
  │  - Encode each instruction to bytes
  │  - Substitute labels with addresses from symbol table
  │  - Verify page-relative branches are in-page
  │  - Produce flat binary buffer
  │
  ▼  Pass: Intel HEX Packaging
     - Pad binary buffer to 4096 bytes (0xFF fill)
     - Emit records of 16 bytes each
     - Calculate checksum per record
     - Emit End of File record
     → .hex file ready for ROM programmer
```

---

## Version History

| Version | Date       | Description                        |
|---------|------------|------------------------------------|
| 0.1.0   | 2026-04-12 | Initial backend specification      |
