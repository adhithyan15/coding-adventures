# Spec 07m — Intel 8086 Behavioral Simulator

## Overview

The **Intel 8086** (1978) is a 16-bit microprocessor and the direct ancestor of
every x86 CPU from the IBM PC era through today's 64-bit Intel and AMD chips.
Designed by a team led by Stephen Morse at Intel, it was the successor to the
8-bit Intel 8080 and targeted the emerging minicomputer market with a 16-bit
data bus and a 20-bit segmented address bus giving 1 MB of addressable memory.

Historical milestones powered by the 8086 family:
- **IBM PC** (1981) — used the 8088 (8-bit external bus variant), spawning the
  entire IBM-compatible PC ecosystem that still dominates computing today
- **MS-DOS** — the dominant personal computer OS for over a decade
- **Lotus 1-2-3**, **WordPerfect**, **dBASE** — the killer apps of the 1980s
- The 8086's architecture evolved into **286 → 386 → 486 → Pentium → Core** in
  an unbroken line of backwards-compatible ISA revisions

The 8086 is fundamentally different from its 8-bit predecessors:
- **16-bit internal registers** (not just byte-wide like the 8080/Z80)
- **Segmented memory model**: physical address = segment × 16 + offset
- **Variable-length instructions**: 1–6 bytes depending on addressing mode
- **ModRM byte**: a rich encoding for register/register and register/memory ops
- **String operations**: MOVS, CMPS, SCAS, LODS, STOS with REP prefixes
- **Separate multiply and divide**: MUL, IMUL, DIV, IDIV

This spec defines Layer **07m** — a Python behavioral simulator for the 8086
following the SIM00 `Simulator[X86State]` protocol.

---

## Architecture

### Registers

#### General-purpose registers (16-bit, with 8-bit halves)

| 16-bit | High byte | Low byte | Conventional use |
|--------|-----------|----------|-----------------|
| AX     | AH        | AL       | Accumulator; result of MUL/DIV |
| BX     | BH        | BL       | Base; memory addressing |
| CX     | CH        | CL       | Counter; LOOP, REP, shifts |
| DX     | DH        | DL       | Data; high word of MUL/DIV; I/O port |

Each 16-bit register can be accessed as a pair of 8-bit halves:
- AX = (AH << 8) | AL
- Write AL: AX = (AX & 0xFF00) | (value & 0xFF)
- Write AH: AX = (AX & 0x00FF) | ((value & 0xFF) << 8)

#### Index and pointer registers (16-bit only, no byte access)

| Name | Description |
|------|-------------|
| SI   | Source Index — used by LODS, MOVS, CMPS, SCAS |
| DI   | Destination Index — used by STOS, MOVS, CMPS |
| SP   | Stack Pointer — points to top of stack in SS segment |
| BP   | Base Pointer — base for stack frame addressing |

#### Segment registers (16-bit)

| Name | Segment | Default for |
|------|---------|-------------|
| CS   | Code    | Instruction fetch: `CS:IP` |
| DS   | Data    | Data references (default) |
| SS   | Stack   | Stack ops: `SS:SP`, `SS:BP` |
| ES   | Extra   | Destination of string ops |

#### Instruction pointer

| Name | Description |
|------|-------------|
| IP   | 16-bit instruction pointer (program counter within CS segment) |

Physical address of next instruction = `CS × 16 + IP`.

#### FLAGS register (16-bit)

```
Bit: 15  14  13  12  11  10   9   8   7   6   5   4   3   2   1   0
      —   —   —   —  OF  DF  IF  TF  SF  ZF   —  AF   —  PF   1  CF
```

| Bit | Name | Description |
|-----|------|-------------|
| 11  | OF   | Overflow — signed result out of range |
| 10  | DF   | Direction — 0=forward (inc SI/DI), 1=backward (dec SI/DI) |
|  9  | IF   | Interrupt enable — 1=maskable interrupts enabled |
|  8  | TF   | Trap — 1=single-step mode |
|  7  | SF   | Sign — bit 15 (or 7 for byte ops) of result |
|  6  | ZF   | Zero — result is zero |
|  4  | AF   | Auxiliary carry — carry from bit 3 to 4 (BCD arithmetic) |
|  2  | PF   | Parity — 1 if low byte of result has even number of 1-bits |
|  1  | —    | Always 1 (reserved) |
|  0  | CF   | Carry — carry/borrow out of MSB |

**For this simulator:** CF, PF, AF, ZF, SF, OF are computed accurately for
all arithmetic and logical instructions. DF affects string operation direction.
IF and TF are stored but interrupts are not modeled (no interrupt controller).

---

## Memory model

The 8086 uses a **segmented** 20-bit address space:

```
Physical address = (segment register × 16) + offset
```

- Segment registers are 16-bit (values 0x0000–0xFFFF)
- Offsets are 16-bit (0x0000–0xFFFF)
- Physical addresses range from 0x00000 to 0xFFFFF (1 MB = 1,048,576 bytes)
- Wrapping: if segment×16 + offset > 0xFFFFF, the address wraps modulo 2²⁰

**Default segment registers:**
- Instruction fetch: CS
- Stack (PUSH/POP): SS
- Data reads (default): DS
- String destinations (MOVS, STOS, CMPS, SCAS): ES
- String sources and general data: DS

**Segment override prefixes** (modify the default data segment for one instruction):
- `0x26` — ES:
- `0x2E` — CS:
- `0x36` — SS:
- `0x3E` — DS:

### load() convention

`load(program, origin=0)` writes raw bytes to physical memory starting at
address `origin` (default 0). This is a flat byte offset into the 1 MB space.

After reset, CS=0 and IP=0, so execution starts at physical address 0x00000.
Calling `sim.execute(program_bytes)` loads at address 0 and runs from CS:IP=0:0.

---

## Instruction encoding

### Opcode space

The 8086 uses a mostly one-byte opcode with a rich secondary encoding in a
**ModRM byte** that follows many opcodes.

Most instructions come in two variants:
- **d=0**: reg is the source, r/m is the destination (`OP r/m, reg`)
- **d=1**: reg is the destination, r/m is the source (`OP reg, r/m`)
- **w=0**: 8-bit operands; **w=1**: 16-bit operands

Common opcode pattern: `[op6][d][w]` where op6 is the 6-bit operation code.

### ModRM byte

Many opcodes are followed by a **ModRM** byte (mod-reg-r/m):

```
Bit:  7   6   5   4   3   2   1   0
    |  mod  |   reg   |    r/m   |
```

**mod field:**
| mod | Meaning |
|-----|---------|
| 00  | Register-indirect: `[EA]` (except r/m=110: direct `[disp16]`) |
| 01  | Register-indirect + 8-bit signed displacement: `[EA + disp8]` |
| 10  | Register-indirect + 16-bit displacement: `[EA + disp16]` |
| 11  | Register-to-register: `r/m` is a register, not memory |

**reg field (16-bit, w=1):**
| reg | 16-bit | 8-bit |
|-----|--------|-------|
| 000 | AX     | AL    |
| 001 | CX     | CL    |
| 010 | DX     | DL    |
| 011 | BX     | BL    |
| 100 | SP     | AH    |
| 101 | BP     | CH    |
| 110 | SI     | DH    |
| 111 | DI     | BH    |

**r/m field effective address (mod ≠ 11):**
| r/m | Effective address base |
|-----|----------------------|
| 000 | BX + SI |
| 001 | BX + DI |
| 010 | BP + SI |
| 011 | BP + DI |
| 100 | SI |
| 101 | DI |
| 110 | BP  (or direct [disp16] when mod=00) |
| 111 | BX |

Effective address is computed modulo 2¹⁶ (16-bit offset), then combined with
the active segment register to form the physical address.

### Instruction prefixes

| Byte | Prefix | Effect |
|------|--------|--------|
| 0x26 | ES:    | Override data segment to ES for next instruction |
| 0x2E | CS:    | Override data segment to CS |
| 0x36 | SS:    | Override data segment to SS |
| 0x3E | DS:    | Override data segment to DS (explicit; DS is default) |
| 0xF0 | LOCK   | Bus lock (ignored in this simulator) |
| 0xF2 | REPNE  | Repeat while CX≠0 and ZF=0 (string ops) |
| 0xF3 | REP/REPE | Repeat while CX≠0 (and ZF=1 for CMPS/SCAS) |

---

## Instruction set

### Data transfer

| Mnemonic | Encoding | Description |
|----------|----------|-------------|
| `MOV r/m, reg` | 88/89 | Move reg → r/m (byte/word) |
| `MOV reg, r/m` | 8A/8B | Move r/m → reg (byte/word) |
| `MOV r/m, imm` | C6/C7 /0 | Move immediate → r/m (byte/word) |
| `MOV reg, imm` | B0+r/B8+r | Move immediate → register (byte/word) |
| `MOV acc, [addr]` | A0/A1 | Move `[addr]` → AL or AX |
| `MOV [addr], acc` | A2/A3 | Move AL or AX → `[addr]` |
| `MOV r, sreg` | 8C | Move segment register → r/m16 |
| `MOV sreg, r` | 8E | Move r/m16 → segment register |
| `XCHG r/m, reg` | 86/87 | Exchange byte/word |
| `XCHG AX, reg` | 90+r | Exchange AX with register (90 = NOP) |
| `PUSH r/m` | FF /6 | Push word onto stack |
| `PUSH reg` | 50+r | Push register |
| `PUSH sreg` | 06/0E/16/1E | Push segment register |
| `POP r/m` | 8F /0 | Pop word from stack |
| `POP reg` | 58+r | Pop register |
| `POP sreg` | 07/0F/17/1F | Pop segment register |
| `PUSHF` | 9C | Push FLAGS |
| `POPF` | 9D | Pop FLAGS |
| `LEA reg, r/m` | 8D | Load effective address (offset only) |
| `LDS reg, r/m` | C5 | Load DS:reg from memory pointer |
| `LES reg, r/m` | C4 | Load ES:reg from memory pointer |
| `LAHF` | 9F | Load AH from FLAGS low byte |
| `SAHF` | 9E | Store AH into FLAGS low byte |
| `CBW` | 98 | Sign-extend AL into AX |
| `CWD` | 99 | Sign-extend AX into DX:AX |
| `XLAT` | D7 | AL ← [DS:BX+AL] (table lookup) |

### Arithmetic

| Mnemonic | Encoding | Description |
|----------|----------|-------------|
| `ADD r/m, reg` | 00/01 | r/m ← r/m + reg |
| `ADD reg, r/m` | 02/03 | reg ← reg + r/m |
| `ADD acc, imm` | 04/05 | AL/AX ← AL/AX + imm |
| `ADD r/m, imm` | 80/81 /0 | r/m ← r/m + imm (byte/word) |
| `ADC r/m, reg` | 10/11 | r/m ← r/m + reg + CF |
| `ADC reg, r/m` | 12/13 | reg ← reg + r/m + CF |
| `ADC acc, imm` | 14/15 | AL/AX ← AL/AX + imm + CF |
| `ADC r/m, imm` | 80/81 /2 | r/m ← r/m + imm + CF |
| `SUB r/m, reg` | 28/29 | r/m ← r/m − reg |
| `SUB reg, r/m` | 2A/2B | reg ← reg − r/m |
| `SUB acc, imm` | 2C/2D | AL/AX ← AL/AX − imm |
| `SUB r/m, imm` | 80/81 /5 | r/m ← r/m − imm |
| `SBB r/m, reg` | 18/19 | r/m ← r/m − reg − CF |
| `SBB reg, r/m` | 1A/1B | reg ← reg − r/m − CF |
| `SBB acc, imm` | 1C/1D | AL/AX ← AL/AX − imm − CF |
| `SBB r/m, imm` | 80/81 /3 | r/m ← r/m − imm − CF |
| `INC r/m` | FE/FF /0 | r/m ← r/m + 1 (CF unaffected) |
| `INC reg` | 40+r | reg ← reg + 1 (word only; CF unaffected) |
| `DEC r/m` | FE/FF /1 | r/m ← r/m − 1 (CF unaffected) |
| `DEC reg` | 48+r | reg ← reg − 1 (word only; CF unaffected) |
| `NEG r/m` | F6/F7 /3 | r/m ← 0 − r/m; CF = (r/m ≠ 0) |
| `CMP r/m, reg` | 38/39 | Subtract, set flags, discard result |
| `CMP reg, r/m` | 3A/3B | Subtract, set flags, discard result |
| `CMP acc, imm` | 3C/3D | Subtract imm, set flags, discard |
| `CMP r/m, imm` | 80/81 /7 | Subtract imm, set flags, discard |
| `MUL r/m` | F6/F7 /4 | AX ← AL × r/m8; DX:AX ← AX × r/m16 (unsigned) |
| `IMUL r/m` | F6/F7 /5 | Signed multiply (same result registers) |
| `DIV r/m` | F6/F7 /6 | AL,AH ← AX ÷ r/m8; AX,DX ← DX:AX ÷ r/m16 (unsigned) |
| `IDIV r/m` | F6/F7 /7 | Signed divide (same result registers) |
| `DAA` | 27 | Decimal adjust AL after BCD addition |
| `DAS` | 2F | Decimal adjust AL after BCD subtraction |
| `AAA` | 37 | ASCII adjust AL after addition |
| `AAS` | 3F | ASCII adjust AL after subtraction |
| `AAM` | D4 0A | ASCII adjust AX after multiply |
| `AAD` | D5 0A | ASCII adjust AX before divide |

**Signed immediate shorthand (80-group):**
Opcode `0x83` (like `0x81` but sign-extends an 8-bit immediate to 16 bits)
applies to ADD, ADC, SUB, SBB, CMP on word r/m.

### Logical

| Mnemonic | Encoding | Description |
|----------|----------|-------------|
| `AND r/m, reg` | 20/21 | Bitwise AND |
| `AND reg, r/m` | 22/23 | Bitwise AND |
| `AND acc, imm` | 24/25 | AL/AX ← AL/AX & imm |
| `AND r/m, imm` | 80/81 /4 | r/m ← r/m & imm |
| `OR r/m, reg` | 08/09 | Bitwise OR |
| `OR reg, r/m` | 0A/0B | Bitwise OR |
| `OR acc, imm` | 0C/0D | AL/AX ← AL/AX \| imm |
| `OR r/m, imm` | 80/81 /1 | r/m ← r/m \| imm |
| `XOR r/m, reg` | 30/31 | Bitwise XOR |
| `XOR reg, r/m` | 32/33 | Bitwise XOR |
| `XOR acc, imm` | 34/35 | AL/AX ← AL/AX ^ imm |
| `XOR r/m, imm` | 80/81 /6 | r/m ← r/m ^ imm |
| `NOT r/m` | F6/F7 /2 | r/m ← ~r/m (flags unaffected) |
| `TEST r/m, reg` | 84/85 | AND, set flags, discard result |
| `TEST acc, imm` | A8/A9 | AL/AX & imm, set flags, discard |
| `TEST r/m, imm` | F6/F7 /0 | r/m & imm, set flags, discard |

### Shifts and rotates

| Mnemonic | Encoding | Description |
|----------|----------|-------------|
| `ROL r/m, 1` | D0/D1 /0 | Rotate left 1 |
| `ROL r/m, CL` | D2/D3 /0 | Rotate left CL times |
| `ROR r/m, 1` | D0/D1 /1 | Rotate right 1 |
| `ROR r/m, CL` | D2/D3 /1 | Rotate right CL |
| `RCL r/m, 1` | D0/D1 /2 | Rotate left through CF 1 |
| `RCL r/m, CL` | D2/D3 /2 | Rotate left through CF CL times |
| `RCR r/m, 1` | D0/D1 /3 | Rotate right through CF 1 |
| `RCR r/m, CL` | D2/D3 /3 | Rotate right through CF CL |
| `SHL/SAL r/m, 1` | D0/D1 /4 | Shift left 1 (CF = evicted bit) |
| `SHL/SAL r/m, CL` | D2/D3 /4 | Shift left CL |
| `SHR r/m, 1` | D0/D1 /5 | Logical shift right 1 |
| `SHR r/m, CL` | D2/D3 /5 | Logical shift right CL |
| `SAR r/m, 1` | D0/D1 /7 | Arithmetic shift right 1 (sign-fill) |
| `SAR r/m, CL` | D2/D3 /7 | Arithmetic shift right CL |

### Control flow

| Mnemonic | Encoding | Description |
|----------|----------|-------------|
| `JMP short` | EB cb | IP ← IP + sign-extend(cb) |
| `JMP near` | E9 cw | IP ← IP + cw (signed 16-bit) |
| `JMP far` | EA seg:off | CS ← seg; IP ← off |
| `JMP r/m16` | FF /4 | IP ← r/m16 (indirect near) |
| `JMP m32` | FF /5 | CS:IP ← [m32] (indirect far) |
| `CALL near` | E8 cw | Push IP; IP ← IP + cw |
| `CALL far` | 9A seg:off | Push CS; push IP; CS:IP ← seg:off |
| `CALL r/m16` | FF /2 | Push IP; IP ← r/m16 |
| `CALL m32` | FF /3 | Push CS; push IP; CS:IP ← [m32] |
| `RET` | C3 | IP ← pop() |
| `RET n` | C2 iw | IP ← pop(); SP ← SP + n |
| `RETF` | CB | IP ← pop(); CS ← pop() |
| `RETF n` | CA iw | IP ← pop(); CS ← pop(); SP ← SP + n |
| `INT n` | CD n | Software interrupt (not modeled; treated as HLT) |
| `INT 3` | CC | Breakpoint (not modeled; treated as HLT) |
| `INTO` | CE | Interrupt on overflow (not modeled) |
| `IRET` | CF | IP ← pop(); CS ← pop(); FLAGS ← pop() |

**Conditional jumps** (all use short signed 8-bit displacement relative to next IP):

| Mnemonic | Opcode | Condition |
|----------|--------|-----------|
| `JO`     | 70     | OF=1 |
| `JNO`    | 71     | OF=0 |
| `JB/JC/JNAE` | 72 | CF=1 |
| `JNB/JNC/JAE` | 73 | CF=0 |
| `JZ/JE`  | 74     | ZF=1 |
| `JNZ/JNE`| 75     | ZF=0 |
| `JBE/JNA`| 76     | CF=1 or ZF=1 |
| `JA/JNBE`| 77     | CF=0 and ZF=0 |
| `JS`     | 78     | SF=1 |
| `JNS`    | 79     | SF=0 |
| `JP/JPE` | 7A     | PF=1 |
| `JNP/JPO`| 7B     | PF=0 |
| `JL/JNGE`| 7C     | SF≠OF |
| `JGE/JNL`| 7D     | SF=OF |
| `JLE/JNG`| 7E     | ZF=1 or SF≠OF |
| `JG/JNLE`| 7F     | ZF=0 and SF=OF |

**Loop instructions** (short signed displacement, decrement CX):

| Mnemonic | Opcode | Description |
|----------|--------|-------------|
| `LOOP`   | E2     | CX ← CX−1; jump if CX≠0 |
| `LOOPE/LOOPZ` | E1 | CX ← CX−1; jump if CX≠0 and ZF=1 |
| `LOOPNE/LOOPNZ` | E0 | CX ← CX−1; jump if CX≠0 and ZF=0 |
| `JCXZ`  | E3     | Jump if CX=0 (no CX decrement) |

### String operations

All string ops use SI (source in DS by default) and/or DI (destination in ES).
After each operation, SI/DI increment (DF=0) or decrement (DF=1) by 1 or 2.

| Mnemonic | Encoding | Description |
|----------|----------|-------------|
| `MOVS`/`MOVSB`/`MOVSW` | A4/A5 | `[ES:DI] ← [DS:SI]`; advance SI, DI |
| `CMPS`/`CMPSB`/`CMPSW` | A6/A7 | Compare `[DS:SI]` with `[ES:DI]`; advance |
| `SCAS`/`SCASB`/`SCASW` | AE/AF | Compare AL/AX with `[ES:DI]`; advance DI |
| `LODS`/`LODSB`/`LODSW` | AC/AD | AL/AX ← `[DS:SI]`; advance SI |
| `STOS`/`STOSB`/`STOSW` | AA/AB | `[ES:DI]` ← AL/AX; advance DI |
| `REP` + MOVS/STOS/LODS | F3+op | Repeat while CX≠0; CX ← CX−1 |
| `REPE` + CMPS/SCAS | F3+op | Repeat while CX≠0 and ZF=1 |
| `REPNE` + CMPS/SCAS | F2+op | Repeat while CX≠0 and ZF=0 |

### Miscellaneous

| Mnemonic | Encoding | Description |
|----------|----------|-------------|
| `NOP` | 90 | No operation (XCHG AX, AX) |
| `HLT` | F4 | Halt; sets `halted=True` |
| `CLC` | F8 | CF ← 0 |
| `STC` | F9 | CF ← 1 |
| `CMC` | F5 | CF ← ~CF |
| `CLD` | FC | DF ← 0 (forward direction) |
| `STD` | FD | DF ← 1 (backward direction) |
| `CLI` | FA | IF ← 0 (disable interrupts) |
| `STI` | FB | IF ← 1 (enable interrupts) |
| `IN AL, n` | E4 n | AL ← input_ports[n] |
| `IN AX, n` | E5 n | AX ← input_ports[n] (low) | input_ports[n+1]<<8 |
| `IN AL, DX` | EC | AL ← input_ports[DX & 0xFF] |
| `IN AX, DX` | ED | AX ← input_ports[DX & 0xFF] |
| `OUT n, AL` | E6 n | output_ports[n] ← AL |
| `OUT n, AX` | E7 n | output_ports[n] ← AX |
| `OUT DX, AL` | EE | output_ports[DX & 0xFF] ← AL |
| `OUT DX, AX` | EF | output_ports[DX & 0xFF] ← AX |
| `LOCK` | F0 | Bus lock prefix (ignored) |
| `WAIT` | 9B | Wait for FPU (ignored) |

---

## Flag behavior

### CF (Carry Flag)

- **ADD/ADC**: 1 if result > 0xFFFF (word) or 0xFF (byte); unsigned overflow.
- **SUB/SBB/CMP/NEG**: 1 if borrow (result < 0 unsigned; i.e. subtrahend > minuend).
- **SHL**: last bit shifted out.
- **SHR/SAR**: last bit shifted out.
- **ROL/ROR/RCL/RCR**: the bit shifted into CF.
- **INC/DEC**: CF **unaffected**.
- **AND/OR/XOR/TEST/NOT**: CF ← 0.
- **MUL/IMUL**: 1 if upper half of result (AH or DX) is non-zero.

### OF (Overflow Flag)

- Set if signed result overflows (ADD: (+)+(+)=(−) or (−)+(−)=(+); SUB: (−)−(+)=(+) etc.)
- 8-bit: overflow if result outside −128…127.
- 16-bit: overflow if result outside −32768…32767.
- AND/OR/XOR/TEST: OF ← 0.
- INC: OF=1 if result=0x80 (byte) or 0x8000 (word).
- DEC: OF=1 if result=0x7F (byte) or 0x7FFF (word).

### ZF (Zero Flag)

Set if the result (after masking to size) is zero.

### SF (Sign Flag)

Set to the MSB of the result: bit 7 for byte ops, bit 15 for word ops.

### PF (Parity Flag)

Set if the **low byte** of the result has an even number of 1-bits.

### AF (Auxiliary Carry Flag)

- **ADD/ADC**: carry out of bit 3 into bit 4.
- **SUB/SBB/CMP**: borrow from bit 4 (i.e. low nibble of minuend < low nibble of subtrahend + borrow).
- **INC**: carry from bit 3.
- **DEC**: borrow into bit 3.
- **AND/OR/XOR/NOT**: AF ← 0.

---

## Reset state

After `reset()`:
- AX, BX, CX, DX = 0
- SI, DI, SP, BP = 0
- CS, DS, SS, ES = 0
- IP = 0
- All flags = 0 (CF=PF=AF=ZF=SF=TF=IF=DF=OF = False)
- All 1,048,576 memory bytes = 0
- halted = False
- input_ports, output_ports = all 0 (256 ports each)

---

## SIM00 Protocol

`X86Simulator` implements `Simulator[X86State]`:

### `reset() → None`

Restore machine to the state above.

### `load(program: bytes, origin: int = 0) → None`

Write `program` bytes into memory starting at physical address `origin`.
Bytes beyond 0xFFFFF are silently ignored.
Does **not** reset registers or clear other memory.

### `step() → StepTrace`

Execute one complete fetch-decode-execute cycle:
1. Compute physical fetch address: `(CS * 16 + IP) & 0xFFFFF`
2. Decode any prefix bytes (segment override, REP/REPNE, LOCK)
3. Fetch opcode byte; advance IP
4. Decode operands (ModRM, displacement, immediate)
5. Execute instruction; update registers, flags, memory
6. Return `StepTrace(pc_before, pc_after, mnemonic, description)`

Where:
- `pc_before` = IP before the step (the offset within CS, **not** the physical address)
- `pc_after` = IP after the step

Raises `RuntimeError` if called on a halted simulator.

### `execute(program: bytes, max_steps: int = 10_000) → ExecutionResult[X86State]`

Reset → load → run until HLT or max_steps. Returns `ExecutionResult`:
- `halted`: True if HLT was reached
- `steps`: instructions executed
- `final_state`: frozen `X86State` snapshot
- `error`: None if halted cleanly; `"max_steps (N) exceeded"` otherwise
- `traces`: list of `StepTrace` (one per instruction)

### `get_state() → X86State`

Return a frozen `X86State` snapshot. The `memory` tuple contains all
1,048,576 bytes. Mutations to the simulator after this call do not affect the
snapshot.

---

## X86State

```python
@dataclass(frozen=True)
class X86State:
    # General-purpose registers (unsigned 16-bit: 0–65535)
    ax: int
    bx: int
    cx: int
    dx: int

    # Index / pointer registers (unsigned 16-bit)
    si: int
    di: int
    sp: int
    bp: int

    # Segment registers (unsigned 16-bit)
    cs: int
    ds: int
    ss: int
    es: int

    # Instruction pointer (unsigned 16-bit)
    ip: int

    # Flags
    cf: bool   # carry
    pf: bool   # parity
    af: bool   # auxiliary carry
    zf: bool   # zero
    sf: bool   # sign
    tf: bool   # trap
    if_: bool  # interrupt enable (trailing underscore avoids Python keyword)
    df: bool   # direction
    of: bool   # overflow

    halted: bool

    # I/O ports
    input_ports: tuple[int, ...]   # 256 bytes
    output_ports: tuple[int, ...]  # 256 bytes

    # Memory (1 MB)
    memory: tuple[int, ...]        # 1,048,576 bytes

    # Derived register accessors
    @property
    def al(self) -> int: ...   # AX & 0xFF
    @property
    def ah(self) -> int: ...   # (AX >> 8) & 0xFF
    @property
    def bl(self) -> int: ...
    @property
    def bh(self) -> int: ...
    @property
    def cl(self) -> int: ...
    @property
    def ch(self) -> int: ...
    @property
    def dl(self) -> int: ...
    @property
    def dh(self) -> int: ...

    @property
    def flags(self) -> int:
        """Pack all flags into a 16-bit FLAGS register value."""
        ...

    @property
    def ax_signed(self) -> int:
        """AX interpreted as signed 16-bit (-32768 … 32767)."""
        ...

    @property
    def al_signed(self) -> int:
        """AL interpreted as signed 8-bit (-128 … 127)."""
        ...
```

---

## Package layout

```
code/packages/python/intel-8086-simulator/
├── BUILD
├── pyproject.toml
├── README.md
├── CHANGELOG.md
└── src/
    └── intel_8086_simulator/
        ├── __init__.py         (exports X86Simulator, X86State)
        ├── py.typed
        ├── state.py            (X86State frozen dataclass)
        ├── flags.py            (flag computation helpers)
        └── simulator.py        (X86Simulator class)
tests/
├── __init__.py
├── test_protocol.py            (SIM00 contract)
├── test_instructions.py        (per-opcode unit tests)
└── test_programs.py            (multi-instruction programs)
```

---

## Test coverage targets

- `test_protocol.py`: construction, reset, load, step, execute (halt + max_steps),
  get_state snapshot isolation — mirrors the SIM00 test pattern from previous layers.
- `test_instructions.py`: at least one test per instruction group (MOV, ADD, SUB, ADC,
  SBB, INC, DEC, NEG, CMP, MUL, IMUL, DIV, IDIV, AND, OR, XOR, NOT, TEST, SHL, SHR,
  SAR, ROL, ROR, RCL, RCR, PUSH, POP, CALL, RET, JMP, all Jcc, LOOP, JCXZ, string ops,
  NOP, HLT, CLC/STC/CMC, CLD/STD, CBW, CWD, XCHG, LAHF, SAHF, LEA, IN, OUT).
- `test_programs.py`: multi-instruction programs (sum loop, factorial, string copy,
  GCD via Euclidean algorithm, bubble sort, BCD arithmetic, I/O port roundtrip).
- **Target**: 100% line coverage.
