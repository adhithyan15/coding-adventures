# 07i — Intel 8080 Simulator (Full Instruction Set)

## Overview

The Intel 8080 simulator implements the complete instruction set of Intel's first
commercially successful 8-bit microprocessor, released in April 1974. The 8080 was
designed by Federico Faggin, Stanley Mazor, and Masatoshi Shima — building directly
on the 8008's architecture but expanding it to a true 16-bit address space, 256-port
I/O, faster clock speeds, and a richer register model. Its $360 price tag (vs $120 for
the 8008) was justified by capability: it processed 640,000 instructions per second at
2 MHz where the 8008 managed 300,000.

The 8080 became the CPU in the Altair 8800 (1975), the machine that launched the
personal computer revolution. CP/M — the first successful personal computer operating
system — targeted the 8080. Microsoft was founded to write BASIC for the Altair 8080.
The 8080's ISA was the direct ancestor of the Z80 (1976) and the Intel 8086 (1978),
making every x86 program ever written a spiritual descendant of this chip.

This is a **behavioral simulator** — it executes 8080 machine code directly, producing
correct results without modeling internal hardware. For a gate-level simulation that
routes every operation through actual logic gates, see `07i2-intel8080-gatelevel.md`.

The simulator conforms to the **SIM00 Simulator Protocol** (`simulator-protocol`),
implementing `Simulator[Intel8080State]` with full `execute`, `step`, `load`,
`get_state`, and `reset` semantics.

## Layer Position

```
Logic Gates → Arithmetic → CPU → [YOU ARE HERE] → Assembler → Lexer → Parser → Compiler → VM
```

This is Layer 4i alongside RISC-V (07a), ARM/ARMv7 (07b), WASM (07c),
Intel 4004 (07d), ARM1 (07e), Intel 8008 (07f), GE-225 (07g), and IBM 704 (07h).

## Why the Intel 8080?

- **The Altair 8800** — the chip that made personal computing real (1975)
- **CP/M target** — understanding this ISA unlocks the first mass-market OS
- **Z80/x86 ancestor** — Z80 extended it; 8086 is binary-compatible with it
- **Transition chip** — 8-bit era's most influential design before 16-bit dominance
- **Historical programming** — CP/M programs, early BASIC interpreters, games
- **6,000 transistors** — 73% more than 8008; shows architectural progress at circuit level

## Architecture

| Feature | Value |
|---------|-------|
| Data width | 8 bits |
| Instruction width | 8 bits (instructions are 1, 2, or 3 bytes) |
| Registers | 7 × 8-bit (A, B, C, D, E, H, L) + 16-bit SP + 16-bit PC |
| Register pairs | BC, DE, HL, SP (used as 16-bit in certain instructions) |
| Accumulator | A — primary target for ALU operations |
| Flags | Sign (S), Zero (Z), Auxiliary Carry (AC), Parity (P), Carry (CY) — 5 flags |
| Program counter | 16 bits (addresses 64 KiB) |
| Stack | RAM-based (pointed to by SP), grows downward |
| Memory | 65,536 bytes (16-bit address space) |
| I/O | 256 input ports (IN 0–255), 256 output ports (OUT 0–255) |
| Transistors | ~6,000 (NMOS, 6 μm process) |
| Clock | 2–5 MHz; instructions take 4–18 T-states |
| Package | 40-pin DIP |

### Registers

```
┌────┬────────────────────────────────────────────────────────────────┐
│ A  │ Accumulator — 8-bit, primary ALU target                       │
│ B  │ General purpose 8-bit register (pairs with C → BC)            │
│ C  │ General purpose 8-bit register                                 │
│ D  │ General purpose 8-bit register (pairs with E → DE)            │
│ E  │ General purpose 8-bit register                                 │
│ H  │ General purpose 8-bit register (pairs with L → HL)            │
│ L  │ General purpose 8-bit register                                 │
│ M  │ Pseudo-register: memory byte at address [H:L]                 │
│ SP │ Stack pointer — 16-bit, points to top of stack in RAM         │
│ PC │ Program counter — 16-bit, next instruction address            │
└────┴────────────────────────────────────────────────────────────────┘
```

M is not a physical register — it is shorthand for an indirect memory reference.
When any instruction uses M as source or destination, it reads or writes the byte
at the 16-bit address `(H << 8) | L`. Unlike the 8008, the full 8 bits of H are
used for addressing, giving the full 64 KiB address space.

### Register Pairs

Several instructions operate on 16-bit register pairs:

| Pair code | Pair name | Registers | Notes |
|-----------|-----------|-----------|-------|
| 00        | B         | B (high), C (low) | |
| 01        | D         | D (high), E (low) | |
| 10        | H         | H (high), L (low) | |
| 11        | SP        | Stack pointer | Only for PUSH/POP |
| 11        | PSW       | A (high), flags (low) | PUSH PSW / POP PSW |

### Register Encoding (3-bit field)

| Binary | Register | Notes |
|--------|----------|-------|
| 000    | B        | |
| 001    | C        | |
| 010    | D        | |
| 011    | E        | |
| 100    | H        | High byte of HL pair |
| 101    | L        | Low byte of HL pair |
| 110    | M        | Indirect memory [H:L] |
| 111    | A        | Accumulator |

### Flag Register

The 8080 maintains 5 condition flags in an 8-bit flags register:

```
Bit:  7   6   5   4   3   2   1   0
      S   Z   0   AC  0   P   1   CY
```

| Flag | Bit | Name | Set when |
|------|-----|------|----------|
| S    | 7   | Sign | Result bit 7 is 1 (result is "negative" in two's complement) |
| Z    | 6   | Zero | Result is 0x00 |
| AC   | 4   | Auxiliary Carry | Carry out of bit 3 into bit 4 (used by DAA) |
| P    | 2   | Parity | Result has an even number of 1-bits |
| CY   | 0   | Carry | Carry/borrow out of bit 7 |

Bits 5, 3, and 1 have fixed values (0, 0, 1 respectively) per the 8080 specification.
PUSH PSW stores these fixed bits; POP PSW restores them.

The S/Z/AC/P flags are affected by most ALU operations. CY is affected by
add/subtract/rotate/compare. AC is only meaningful for DAA (Decimal Adjust
Accumulator).

### Flag Computation Reference

```
S  = (result & 0x80) != 0
Z  = result == 0
AC = (low nibble of operand1 + low nibble of operand2) > 0x0F
P  = bin(result).count('1') % 2 == 0   (even parity)
CY = result > 0xFF   (for addition)   or   result < 0   (for subtraction)
```

## Instruction Set Architecture

The 8080 has 244 distinct opcodes arranged in a regular 8-bit encoding:

```
Bits 7–6: primary group (00=data/ALU setup, 01=MOV, 10=ALU with reg, 11=misc/control)
Bits 5–3: destination register (or operation sub-type)
Bits 2–0: source register (or operation detail)
```

### Instruction Lengths

| Length   | Examples                                        |
|----------|-------------------------------------------------|
| 1 byte   | MOV r1,r2; ADD r; INR r; NOP; HLT; most ALU    |
| 2 bytes  | MVI r,d8; ADI d8; IN port; OUT port; RST n     |
| 3 bytes  | LXI rp,d16; LDA addr; STA addr; JMP addr; CALL |

### Data Transfer Instructions

| Mnemonic         | Bytes | Description |
|-----------------|-------|-------------|
| MOV r1, r2      | 1     | r1 ← r2 (register-to-register move; either can be M) |
| MVI r, d8       | 2     | r ← immediate byte |
| LXI rp, d16     | 3     | rp ← 16-bit immediate (low byte then high byte) |
| LDA addr        | 3     | A ← memory[addr] |
| STA addr        | 3     | memory[addr] ← A |
| LHLD addr       | 3     | L ← memory[addr]; H ← memory[addr+1] |
| SHLD addr       | 3     | memory[addr] ← L; memory[addr+1] ← H |
| LDAX rp         | 1     | A ← memory[rp] (rp is BC or DE only) |
| STAX rp         | 1     | memory[rp] ← A (rp is BC or DE only) |
| XCHG            | 1     | HL ↔ DE |

### Arithmetic Instructions

| Mnemonic    | Bytes | Flags | Description |
|-------------|-------|-------|-------------|
| ADD r       | 1     | S,Z,AC,P,CY | A ← A + r |
| ADI d8      | 2     | S,Z,AC,P,CY | A ← A + immediate |
| ADC r       | 1     | S,Z,AC,P,CY | A ← A + r + CY (add with carry) |
| ACI d8      | 2     | S,Z,AC,P,CY | A ← A + immediate + CY |
| SUB r       | 1     | S,Z,AC,P,CY | A ← A - r |
| SUI d8      | 2     | S,Z,AC,P,CY | A ← A - immediate |
| SBB r       | 1     | S,Z,AC,P,CY | A ← A - r - CY (subtract with borrow) |
| SBI d8      | 2     | S,Z,AC,P,CY | A ← A - immediate - CY |
| INR r       | 1     | S,Z,AC,P    | r ← r + 1 (does NOT affect CY) |
| DCR r       | 1     | S,Z,AC,P    | r ← r - 1 (does NOT affect CY) |
| INX rp      | 1     | none        | rp ← rp + 1 (16-bit, no flags) |
| DCX rp      | 1     | none        | rp ← rp - 1 (16-bit, no flags) |
| DAD rp      | 1     | CY          | HL ← HL + rp (16-bit add, only sets CY) |
| DAA         | 1     | S,Z,AC,P,CY | Decimal Adjust Accumulator (BCD correction) |

### Logical Instructions

| Mnemonic    | Bytes | Flags | Description |
|-------------|-------|-------|-------------|
| ANA r       | 1     | S,Z,P,CY=0,AC | A ← A AND r |
| ANI d8      | 2     | S,Z,P,CY=0,AC | A ← A AND immediate |
| ORA r       | 1     | S,Z,P,CY=0,AC=0 | A ← A OR r |
| ORI d8      | 2     | S,Z,P,CY=0,AC=0 | A ← A OR immediate |
| XRA r       | 1     | S,Z,P,CY=0,AC=0 | A ← A XOR r |
| XRI d8      | 2     | S,Z,P,CY=0,AC=0 | A ← A XOR immediate |
| CMP r       | 1     | S,Z,AC,P,CY | Set flags as if A - r; A unchanged |
| CPI d8      | 2     | S,Z,AC,P,CY | Set flags as if A - immediate; A unchanged |
| RLC         | 1     | CY          | Rotate A left through carry chain (A7→CY, A7→A0) |
| RRC         | 1     | CY          | Rotate A right through carry chain (A0→CY, A0→A7) |
| RAL         | 1     | CY          | Rotate A left through carry (CY→A0, A7→CY) |
| RAR         | 1     | CY          | Rotate A right through carry (CY→A7, A0→CY) |
| CMA         | 1     | none        | A ← ~A (complement accumulator) |
| CMC         | 1     | CY          | CY ← ~CY (complement carry) |
| STC         | 1     | CY=1        | Set carry flag to 1 |

**ANA note**: The 8080 spec says ANA/ANI set AC to the OR of bit 3 of each operand.
This is the documented behavior; some documentation is inconsistent. Our simulator
follows the Intel 8080 System Reference Manual (1975) specification.

### Branch Instructions

| Mnemonic      | Bytes | Description |
|--------------|-------|-------------|
| JMP addr      | 3     | PC ← addr (unconditional) |
| JNZ addr      | 3     | PC ← addr if Z=0 |
| JZ  addr      | 3     | PC ← addr if Z=1 |
| JNC addr      | 3     | PC ← addr if CY=0 |
| JC  addr      | 3     | PC ← addr if CY=1 |
| JPO addr      | 3     | PC ← addr if P=0 (parity odd) |
| JPE addr      | 3     | PC ← addr if P=1 (parity even) |
| JP  addr      | 3     | PC ← addr if S=0 (plus/positive) |
| JM  addr      | 3     | PC ← addr if S=1 (minus/negative) |
| CALL addr     | 3     | Push PC+3; PC ← addr |
| CNZ addr      | 3     | CALL if Z=0 |
| CZ  addr      | 3     | CALL if Z=1 |
| CNC addr      | 3     | CALL if CY=0 |
| CC  addr      | 3     | CALL if CY=1 |
| CPO addr      | 3     | CALL if P=0 |
| CPE addr      | 3     | CALL if P=1 |
| CP  addr      | 3     | CALL if S=0 |
| CM  addr      | 3     | CALL if S=1 |
| RET           | 1     | Pop PC from stack |
| RNZ           | 1     | RET if Z=0 |
| RZ            | 1     | RET if Z=1 |
| RNC           | 1     | RET if CY=0 |
| RC            | 1     | RET if CY=1 |
| RPO           | 1     | RET if P=0 |
| RPE           | 1     | RET if P=1 |
| RP            | 1     | RET if S=0 |
| RM            | 1     | RET if S=1 |
| RST n (n=0–7) | 1     | Push PC; PC ← 8*n (restart vector) |
| PCHL          | 1     | PC ← HL |

### Stack Instructions

| Mnemonic    | Bytes | Description |
|-------------|-------|-------------|
| PUSH rp     | 1     | SP-=2; memory[SP+1] ← rp_high; memory[SP] ← rp_low |
| PUSH PSW    | 1     | SP-=2; memory[SP+1] ← A; memory[SP] ← flags |
| POP rp      | 1     | rp_low ← memory[SP]; rp_high ← memory[SP+1]; SP+=2 |
| POP PSW     | 1     | flags ← memory[SP]; A ← memory[SP+1]; SP+=2 |
| XTHL        | 1     | L ↔ memory[SP]; H ↔ memory[SP+1] |
| SPHL        | 1     | SP ← HL |

The stack grows **downward**: PUSH decrements SP by 2 before writing; POP reads then
increments SP by 2. This is the origin of the convention used in x86 today.

### I/O and Machine Control

| Mnemonic  | Bytes | Description |
|-----------|-------|-------------|
| IN port   | 2     | A ← input_port[port] |
| OUT port  | 2     | output_port[port] ← A |
| EI        | 1     | Enable interrupts (INTE flip-flop ← 1) |
| DI        | 1     | Disable interrupts (INTE flip-flop ← 0) |
| NOP       | 1     | No operation |
| HLT       | 1     | Halt — stop execution |

## Opcode Encoding

The 8080 instruction encoding is beautifully regular. The MOV group alone accounts
for 64 opcodes (8×8), arranged so the decoder can be a simple shifter + look-up.

```
Group 00 — Data manipulation / arithmetic setup:
  00DDDSSS with DDD=110 and SSS=110 is invalid (MOV M,M)
  00DDD110 = MVI r,d8    (immediate to register)
  00RP0001 = LXI rp,d16  (immediate to register pair)
  00RP1001 = DAD rp      (HL += rp)
  00RP0011 = INX rp
  00RP1011 = DCX rp
  ...

Group 01 — MOV:
  01DDDSSS = MOV DDD,SSS  (64 opcodes; 01110110 = HLT)

Group 10 — ALU register:
  10OPRSSS = ALU_OP(A, r_SSS)
  OPCODE 000=ADD, 001=ADC, 010=SUB, 011=SBB, 100=ANA, 101=XRA, 110=ORA, 111=CMP

Group 11 — Immediate/control/stack/branch:
  11RP0001 = POP rp
  11RP0101 = PUSH rp
  11CCC010 = conditional JMP
  11CCC100 = conditional CALL
  11CCC000 = conditional RET
  11NNN111 = RST n
  ...
```

### Full Opcode Table (Compact Reference)

```
  00: NOP     01: LXI B   02: STAX B  03: INX B   04: INR B   05: DCR B   06: MVI B   07: RLC
  08: ---     09: DAD B   0A: LDAX B  0B: DCX B   0C: INR C   0D: DCR C   0E: MVI C   0F: RRC
  10: ---     11: LXI D   12: STAX D  13: INX D   14: INR D   15: DCR D   16: MVI D   17: RAL
  18: ---     19: DAD D   1A: LDAX D  1B: DCX D   1C: INR E   1D: DCR E   1E: MVI E   1F: RAR
  20: ---     21: LXI H   22: SHLD    23: INX H   24: INR H   25: DCR H   26: MVI H   27: DAA
  28: ---     29: DAD H   2A: LHLD    2B: DCX H   2C: INR L   2D: DCR L   2E: MVI L   2F: CMA
  30: ---     31: LXI SP  32: STA     33: INX SP  34: INR M   35: DCR M   36: MVI M   37: STC
  38: ---     39: DAD SP  3A: LDA     3B: DCX SP  3C: INR A   3D: DCR A   3E: MVI A   3F: CMC
  40: MOV B,B 41: MOV B,C ... 76: HLT  77: MOV M,A
  ... (64 MOV opcodes: 40–7F, 76=HLT)
  80: ADD B   81: ADD C   82: ADD D   83: ADD E   84: ADD H   85: ADD L   86: ADD M   87: ADD A
  88: ADC B   89: ADC C   8A: ADC D   8B: ADC E   8C: ADC H   8D: ADC L   8E: ADC M   8F: ADC A
  90: SUB B   91: SUB C   92: SUB D   93: SUB E   94: SUB H   95: SUB L   96: SUB M   97: SUB A
  98: SBB B   99: SBB C   9A: SBB D   9B: SBB E   9C: SBB H   9D: SBB L   9E: SBB M   9F: SBB A
  A0: ANA B   A1: ANA C   A2: ANA D   A3: ANA E   A4: ANA H   A5: ANA L   A6: ANA M   A7: ANA A
  A8: XRA B   A9: XRA C   AA: XRA D   AB: XRA E   AC: XRA H   AD: XRA L   AE: XRA M   AF: XRA A
  B0: ORA B   B1: ORA C   B2: ORA D   B3: ORA E   B4: ORA H   B5: ORA L   B6: ORA M   B7: ORA A
  B8: CMP B   B9: CMP C   BA: CMP D   BB: CMP E   BC: CMP H   BD: CMP L   BE: CMP M   BF: CMP A
  C0: RNZ     C1: POP B   C2: JNZ     C3: JMP     C4: CNZ     C5: PUSH B  C6: ADI     C7: RST 0
  C8: RZ      C9: RET     CA: JZ      CB: ---     CC: CZ      CD: CALL    CE: ACI     CF: RST 1
  D0: RNC     D1: POP D   D2: JNC     D3: OUT     D4: CNC     D5: PUSH D  D6: SUI     D7: RST 2
  D8: RC      D9: ---     DA: JC      DB: IN      DC: CC      DD: ---     DE: SBI     DF: RST 3
  E0: RPO     E1: POP H   E2: JPO     E3: XTHL    E4: CPO     E5: PUSH H  E6: ANI     E7: RST 4
  E8: RPE     E9: PCHL    EA: JPE     EB: XCHG    EC: CPE     ED: ---     EE: XRI     EF: RST 5
  F0: RP      F1: POP PSW F2: JP      F3: DI      F4: CP      F5: PUSH PSW F6: ORI    F7: RST 6
  F8: RM      F9: SPHL    FA: JM      FB: EI      FC: CM      FD: ---     FE: CPI     FF: RST 7
```

Opcodes 08, 10, 18, 20, 28, 30, 38, CB, D9, DD, ED, FD are **undefined** on the
stock 8080. Our simulator raises a `ValueError` for these.

## Decimal Adjust Accumulator (DAA)

DAA is the most complex single instruction on the 8080. It corrects A after a BCD
addition, allowing two-digit BCD arithmetic:

```
Step 1: If (low nibble of A > 9) or AC=1: A += 6
Step 2: If (high nibble of result > 9) or CY=1: A += 0x60; CY ← 1
```

This requires saving the pre-adjustment AC flag. The flags S, Z, P are set based on
the final adjusted value.

## SIM00 Protocol Conformance

The package exports `Intel8080Simulator` which structurally satisfies
`Simulator[Intel8080State]` from `coding-adventures-simulator-protocol`.

### State Snapshot: `Intel8080State`

```python
@dataclass(frozen=True)
class Intel8080State:
    a: int                    # Accumulator (0–255)
    b: int                    # Register B (0–255)
    c: int                    # Register C (0–255)
    d: int                    # Register D (0–255)
    e: int                    # Register E (0–255)
    h: int                    # Register H (0–255)
    l: int                    # Register L (0–255)
    sp: int                   # Stack pointer (0–65535)
    pc: int                   # Program counter (0–65535)
    flag_s: bool              # Sign flag
    flag_z: bool              # Zero flag
    flag_ac: bool             # Auxiliary carry flag
    flag_p: bool              # Parity flag
    flag_cy: bool             # Carry flag
    interrupts_enabled: bool  # INTE flip-flop
    halted: bool              # HLT has been executed
    memory: tuple[int, ...]   # 65,536 bytes, immutable snapshot
    input_ports: tuple[int, ...]   # 256 input port values
    output_ports: tuple[int, ...]  # 256 output port values
```

### Simulator Methods

| Method | Description |
|--------|-------------|
| `load(program: bytes)` | Write bytes to memory starting at address 0; reset PC and registers |
| `step() -> StepTrace` | Execute one instruction; return StepTrace |
| `execute(program: bytes) -> ExecutionResult` | Load, run until HLT; return result with full traces |
| `get_state() -> Intel8080State` | Return frozen snapshot of current state |
| `reset()` | Clear all registers, flags, and memory |

### StepTrace Fields

Each `StepTrace` produced by `step()` includes:

```python
StepTrace(
    pc_before=...,         # PC before this instruction
    pc_after=...,          # PC after (PC + instr_length, or branch target)
    mnemonic=...,          # E.g. "MOV B,C", "ADD M", "CALL 0x0100"
    description=...,       # Human-readable explanation
    state_before=...,      # Intel8080State snapshot before execution
    state_after=...,       # Intel8080State snapshot after execution
)
```

### `execute` Behavior

- Resets state before loading the program (fresh start each call)
- Steps until `halted=True` or 1,000,000 steps (safety limit)
- Returns `ExecutionResult(halted=True, error=None, ...)` on clean HLT
- Returns `ExecutionResult(halted=False, error="...", ...)` on cycle limit
- Undefined opcode raises `ValueError`, which is caught and stored in `error`

## I/O Ports

The simulator maintains separate `_input_ports` and `_output_ports` arrays (256 bytes
each). Callers can pre-load input ports before `execute`:

```python
sim = Intel8080Simulator()
sim.set_input_port(0, 0xFF)   # port 0 returns 0xFF when IN 0 executes
result = sim.execute(program)
# Check output_ports in result.final_state for any OUT instructions
```

## Interrupts

The 8080 has a maskable interrupt system controlled by the INTE flip-flop. The
simulator models INTE (`interrupts_enabled`) but does **not** model external
interrupt delivery — interrupts cannot arrive between steps in this behavioral
model. `EI` and `DI` update the flag; the flip-flop is reset on HLT.

## Example Programs

### Hello (sum registers)

```
; Sum of 1 + 2 + 3 + 4 = 10
MVI A, 0x00    ; A = 0
MVI B, 0x01    ; B = 1
ADD B          ; A = 1
MVI B, 0x02    ; B = 2
ADD B          ; A = 3
MVI B, 0x03    ; B = 3
ADD B          ; A = 6
MVI B, 0x04    ; B = 4
ADD B          ; A = 10 (0x0A)
HLT
; Encoding: 3E 00 06 01 80 06 02 80 06 03 80 06 04 80 76
```

### Fibonacci (first 8 values in memory)

```
; Store Fibonacci sequence in memory starting at 0x100
LXI H, 0x0100  ; HL points to start of result buffer
MVI A, 0x00    ; F(0) = 0
MOV M, A       ; mem[0x100] = 0
INX H
MVI B, 0x01    ; F(1) = 1
MOV M, B       ; mem[0x101] = 1
INX H
MVI C, 0x06    ; counter: 6 more values
LOOP:
  MOV D, A     ; D = prev-prev (A)
  MOV A, B     ; A = prev (B)
  ADD D        ; A = F(n) = prev + prev-prev
  MOV M, A     ; store result
  INX H
  MOV D, A     ; save new value
  MOV A, B     ; old prev becomes prev-prev for next iter
  MOV B, D     ; B = new prev = D = new F(n)
  DCR C
  JNZ LOOP
HLT
```

### Stack demonstration

```
; Push 3 values, pop them in LIFO order
LXI SP, 0xFF00  ; initialize SP to 0xFF00
MVI A, 0x11     ; A = 0x11
PUSH PSW        ; push A + flags onto stack
MVI A, 0x22     ; A = 0x22
PUSH PSW
MVI A, 0x33     ; A = 0x33
PUSH PSW
POP PSW         ; A = 0x33 (LIFO)
POP PSW         ; A = 0x22
POP PSW         ; A = 0x11
HLT
```

## Historical Context

The 8080's development was driven by a simple need: the 8008 was good, but
assembler programmers needed more. Federico Faggin's team delivered:

1. **64 KiB address space** (vs 16 KiB) — enough for real operating systems
2. **RAM-based stack** (vs internal 8-level) — unlimited call depth
3. **Two-phase clock externally** — external clock oscillator eliminated a chip
4. **Separate 8-bit data bus and 16-bit address bus** — no multiplexing
5. **NMOS process** — faster and cheaper than PMOS

Gary Kildall saw the 8080's specs in 1973 and designed CP/M around it. When the
Altair 8800 shipped in January 1975, CP/M was ready. Paul Allen saw the Altair
on the cover of Popular Electronics and called Bill Gates. They wrote Altair BASIC
in 8080 assembly. The personal computer era began.

## Package Layout

```
code/packages/python/intel8080-simulator/
├── pyproject.toml
├── BUILD
├── README.md
├── CHANGELOG.md
└── src/
    └── intel8080_simulator/
        ├── __init__.py       # Public API
        ├── state.py          # Intel8080State frozen dataclass
        ├── flags.py          # Flag computation helpers
        └── simulator.py      # Intel8080Simulator main class
tests/
    ├── test_state.py         # State immutability and snapshot tests
    ├── test_flags.py         # Flag computation edge cases
    ├── test_data_transfer.py # MOV, MVI, LXI, LDA/STA, LDAX/STAX, XCHG
    ├── test_arithmetic.py    # ADD/ADC/SUB/SBB/INR/DCR/INX/DCX/DAD/DAA
    ├── test_logical.py       # ANA/ORA/XRA/CMP/RLC/RRC/RAL/RAR/CMA/CMC/STC
    ├── test_branch.py        # JMP, conditional jumps, CALL, RET, RST, PCHL
    ├── test_stack.py         # PUSH, POP, XTHL, SPHL
    ├── test_io.py            # IN, OUT, port state
    ├── test_programs.py      # End-to-end multi-instruction programs
    └── test_protocol.py      # SIM00 protocol conformance
```

## Divergences from Real Hardware

1. **Undefined opcodes**: The real 8080 has deterministic (but undocumented) behavior
   for undefined opcodes. We raise `ValueError` instead.
2. **No external interrupts**: The behavioral simulator cannot receive interrupts
   between steps.
3. **No T-state timing**: We count steps (instructions), not T-states.
4. **HALT behavior**: After HLT, `step()` is a no-op (real CPU halts clock).
5. **I/O isolation**: I/O ports are simulator-internal arrays, not connected to
   actual hardware.
6. **ANA/ANI auxiliary carry**: We implement per Intel 8080 System Reference Manual
   (OR of bit 3 of operands); some emulators differ.
