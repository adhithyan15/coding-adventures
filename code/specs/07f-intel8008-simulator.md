# 07f — Intel 8008 Simulator (Full Instruction Set)

## Overview

The Intel 8008 simulator implements the complete instruction set of the world's first
8-bit microprocessor, released by Intel in April 1972. It was designed by Ted Hoff,
Stanley Mazor, and Hal Feeney at the request of Computer Terminal Corporation (CTC),
who wanted a CPU for their Datapoint 2200 terminal. CTC ultimately rejected the chip
for being too slow, allowing Intel to sell it commercially. The 8008 went on to
inspire the 8080, which inspired the Z80 and the x86 architecture — making this
humble terminal chip the ancestor of the processors running the world's computers today.

This is a **behavioral simulator** — it executes 8008 machine code directly, producing
correct results without modeling internal hardware. For a gate-level simulation that
routes every operation through actual logic gates, see `07f2-intel8008-gatelevel.md`.

The simulator uses a **custom dispatch loop** (not GenericVM) because the 8008 has
variable-length instructions (1, 2, or 3 bytes) and its address space and stack model
differ enough from the generic VM that building on top of it would require more adapters
than the implementation itself.

## Layer Position

```
Logic Gates → Arithmetic → CPU → [YOU ARE HERE] → Assembler → Lexer → Parser → Compiler → VM
```

This is an alternative Layer 4 alongside RISC-V (07a), ARM/ARMv7 (07b), WASM (07c),
Intel 4004 (07d), and ARM1 (07e).

## Why the Intel 8008?

- **Historical** — the chip that launched 8-bit computing and the x86 lineage
- **Bridge** — connects the 4-bit accumulator world (4004, 1971) to modern computing
- **Rejected and vindicated** — CTC said no; the rest of computing history said yes
- **Architectural evolution** — 7 general-purpose registers vs 4004's accumulator-only design
- **Real constraints** — 3,500 transistors, 14-bit address space, internal push-down stack
- **Contrast with ARM1** — shows 13 years of architecture evolution (1972→1985)

## Architecture

| Feature | Value |
|---------|-------|
| Data width | 8 bits |
| Instruction width | 8 bits (instructions are 1, 2, or 3 bytes) |
| Registers | 7 × 8-bit (A, B, C, D, E, H, L) + M (memory pseudo-register) |
| Accumulator | A (register 7 — used implicitly in most ALU operations) |
| Flags | Carry (CY), Zero (Z), Sign (S), Parity (P) — 4 flags |
| Program counter | 14 bits (addresses 16 KiB) |
| Stack | 8-level internal push-down stack (14-bit entries) — 7 usable for calls |
| Memory | 16,384 bytes (14-bit address space) |
| I/O | 8 input ports (IN 0–7), 24 output ports (OUT 0–23) |
| Transistors | ~3,500 (PMOS, 10 μm process) |
| Clock | 500–800 kHz two-phase clock (≈200–500 kHz effective instruction rate) |

### Registers

The 8008 exposes 7 working registers plus a memory pseudo-register:

```
┌───┬──────────────────────────────────────────────────────────────┐
│ A │ Accumulator — primary target of all ALU operations           │
│ B │ General purpose 8-bit register                               │
│ C │ General purpose 8-bit register                               │
│ D │ General purpose 8-bit register                               │
│ E │ General purpose 8-bit register                               │
│ H │ High byte of the memory address register pair                │
│ L │ Low byte of the memory address register pair                 │
│ M │ Pseudo-register: memory at address [H:L] (H=high, L=low)     │
└───┴──────────────────────────────────────────────────────────────┘
```

M is not a physical register — it is a shorthand for an indirect memory reference.
When an instruction uses M as a source or destination, it reads or writes the byte
at the 14-bit address formed by combining H (bits 13–8) and L (bits 7–0):

```
memory_address = ((H & 0x3F) << 8) | L      ; 14-bit: H contributes top 6 bits
```

Only the low 6 bits of H are used for addressing (the top 2 bits of H are "don't care").

### Register Encoding (3-bit field)

Instructions encode register operands in 3-bit fields:

| Binary | Register | Notes |
|--------|----------|-------|
| 000 | B | |
| 001 | C | |
| 010 | D | |
| 011 | E | |
| 100 | H | High byte of address pair |
| 101 | L | Low byte of address pair |
| 110 | M | Indirect memory [H:L] |
| 111 | A | Accumulator |

### Flag Register

The 8008 maintains 4 condition flags, updated by most ALU operations:

```
Bit 7  Bit 6  Bit 5  Bit 4  Bit 3  Bit 2  Bit 1  Bit 0
  S      -      -      -      -      P      Z     CY
```

- **CY (Carry):** Set when an addition overflows 8 bits, or when a subtraction borrows.
  Also set/cleared by rotate instructions.
- **Z (Zero):** Set when the result of an operation is exactly 0x00.
- **S (Sign):** Set when bit 7 of the result is 1 (treating result as a signed byte).
- **P (Parity):** Set when the result has an even number of 1-bits (even parity).

Unlike the Intel 8080 (its successor), the 8008 does **not** have an Auxiliary Carry flag.
This means BCD arithmetic requires software emulation rather than the DAA instruction
that the 8080 provides.

### The 8-Level Push-Down Stack

The 8008's hardware stack is fundamentally different from the software stack used by
most modern CPUs. There is no stack pointer register visible to the programmer. Instead,
the chip contains 8 × 14-bit registers arranged as a circular push-down stack:

```
Stack (conceptual view — entries are 14-bit return addresses):

    ┌──────────────────┐  ← Top of stack (current PC lives here)
    │  Entry 0 (PC)    │  ← Program counter is always the top entry
    ├──────────────────┤
    │  Entry 1         │  ← First saved return address
    ├──────────────────┤
    │  Entry 2         │
    ├──────────────────┤
    │  Entry 3         │
    ├──────────────────┤
    │  Entry 4         │
    ├──────────────────┤
    │  Entry 5         │
    ├──────────────────┤
    │  Entry 6         │
    ├──────────────────┤
    │  Entry 7         │  ← Oldest saved return address
    └──────────────────┘
```

**Critical insight:** Entry 0 is always the current program counter. When a CALL
instruction executes:
1. The stack rotates down (entry 0 → entry 1, entry 1 → entry 2, ...)
2. The jump target is loaded into entry 0 (the new PC)

When a RETURN executes:
1. The stack rotates up (entry 1 → entry 0, ...)
2. Entry 0 now contains the saved return address, which becomes the new PC

Since entry 0 is always consumed by the current PC, programs can call at most 7 levels
deep before the stack wraps (overwriting the oldest return address silently).

This is a pure push-down automaton — there is no `PUSH`/`POP` for data, only for
return addresses via CALL and RETURN.

### Address Space Layout

The 8008 addresses 16 KiB of unified memory:

```
0x0000 ──────────────────────────────────────
        Program memory starts here
        (code and data share the same space)

        The 8008 has no separate I/O address space —
        I/O is handled by IN/OUT instructions with
        port numbers encoded in the opcode.

0x3FFF ──────────────────────────────────────
        End of 14-bit address space (16,384 bytes)
```

## Complete Instruction Set

The 8008 has 48 distinct operations (not counting register variants as separate
instructions). Instructions are 1, 2, or 3 bytes wide.

### Instruction Encoding Overview

```
 7   6   5   4   3   2   1   0
┌───┬───┬───┬───┬───┬───┬───┬───┐
│ b7  b6 │  DDD  │  SSS/data │
└───┴───┴───┴───┴───┴───┴───┴───┘

Bits 7–6: Major opcode group
  00 = Register operations (INR, DCR, Rotates) and 2-byte MVI
  01 = Register-to-register transfer (MOV) and HLT
  10 = ALU register operand (ADD, SUB, AND, OR, XOR, CMP, ADC, SBB)
  11 = ALU immediate and control flow (ADI, SUI, JMP, CALL, RET, IN, OUT)

Bits 5–3 (DDD): Destination register (for MOV/MVI)
                Or ALU operation select (for group 10/11)
Bits 2–0 (SSS): Source register (for MOV/ALU)
                Or sub-operation code (for group 00/11)
```

### Group 1: Index Register Instructions

These instructions load, store, increment, and decrement the working registers.

#### MOV — Register-to-Register Transfer (1 byte)

```
Encoding: 01 DDD SSS
```

Copies the value of source register SSS into destination register DDD. If either
is M, the memory at [H:L] is read or written. The instruction `01 110 110` (MOV M, M)
is the HALT instruction — an intentional design quirk.

| Mnemonic | Encoding | Operation |
|----------|----------|-----------|
| MOV D, S | `01 DDD SSS` | D ← S |
| MOV A, B | `01 111 000` (`0x78`) | A ← B |
| MOV H, L | `01 100 101` (`0x65`) | H ← L |
| MOV M, A | `01 110 111` (`0x77`) | mem[H:L] ← A |
| MOV A, M | `01 111 110` (`0x7E`) | A ← mem[H:L] |

Flags: Not affected.

#### MVI — Move Immediate (2 bytes)

```
Encoding: 00 DDD 110, data8
```

Loads the 8-bit immediate value `data8` into register DDD. If DDD = M (110), the
value is written to memory at [H:L].

| Mnemonic | Encoding | Operation |
|----------|----------|-----------|
| MVI A, d | `00 111 110, d` (`0x3E, d`) | A ← d |
| MVI B, d | `00 000 110, d` (`0x06, d`) | B ← d |
| MVI M, d | `00 110 110, d` (`0x36, d`) | mem[H:L] ← d |

Flags: Not affected.

#### INR — Increment Register (1 byte)

```
Encoding: 00 DDD 000
```

Increments register DDD by 1. Wraps from 0xFF to 0x00. Updates Z, S, P flags;
does **not** update CY (carry is preserved).

| Mnemonic | Encoding | Operation |
|----------|----------|-----------|
| INR A | `00 111 000` (`0x38`) | A ← A + 1 |
| INR B | `00 000 000` (`0x00`) | B ← B + 1 |
| INR M | `00 110 000` (`0x30`) | mem[H:L] ← mem[H:L] + 1 |

Flags: Z, S, P updated. CY unchanged.

**Note:** `INR B` encodes to `0x00`. The 8008 has no explicit NOP instruction —
`INR B` with B=0xFF wrapping is the closest equivalent (or just accepting the
side effect). Some assemblers emit `INR B` as a NOP alternative.

#### DCR — Decrement Register (1 byte)

```
Encoding: 00 DDD 001
```

Decrements register DDD by 1. Wraps from 0x00 to 0xFF. Updates Z, S, P flags;
does **not** update CY.

| Mnemonic | Encoding | Operation |
|----------|----------|-----------|
| DCR A | `00 111 001` (`0x39`) | A ← A - 1 |
| DCR B | `00 000 001` (`0x01`) | B ← B - 1 |
| DCR M | `00 110 001` (`0x31`) | mem[H:L] ← mem[H:L] - 1 |

Flags: Z, S, P updated. CY unchanged.

### Group 2: Accumulator ALU Instructions

All arithmetic and logical operations target the accumulator A. The source can be
any register (including M for memory) or an immediate byte.

#### ALU Register Instructions (1 byte)

```
Encoding: 10 OOO SSS
  OOO = operation (3 bits)
  SSS = source register (3 bits)
```

| OOO | Mnemonic | Operation | Flags |
|-----|----------|-----------|-------|
| 000 | ADD S | A ← A + S | Z, S, P, CY |
| 001 | ADC S | A ← A + S + CY | Z, S, P, CY |
| 010 | SUB S | A ← A − S | Z, S, P, CY |
| 011 | SBB S | A ← A − S − CY | Z, S, P, CY |
| 100 | ANA S | A ← A & S | Z, S, P; CY=0 |
| 101 | XRA S | A ← A ^ S | Z, S, P; CY=0 |
| 110 | ORA S | A ← A \| S | Z, S, P; CY=0 |
| 111 | CMP S | Set flags for A − S, A unchanged | Z, S, P, CY |

Examples:
- `ADD B` = `10 000 000` (`0x80`) — A ← A + B
- `SUB M` = `10 010 110` (`0x96`) — A ← A − mem[H:L]
- `ANA A` = `10 100 111` (`0xA7`) — A ← A & A (zeros CY, useful to clear carry)
- `CMP A` = `10 111 111` (`0xBF`) — compare A with itself (sets Z, clears CY, sets P)

**Subtraction note:** SUB uses two's complement. CY is set if a borrow occurred
(i.e., when the unsigned subtraction would go negative). This is the inverse of
the carry convention used on some other architectures — on the 8008, CY=1 after
SUB means "the result required a borrow."

**ANA clears carry:** AND, OR, XOR always clear the carry flag. This differs from
the 8080 successor, which clears carry for AND but not consistently for OR/XOR.

#### ALU Immediate Instructions (2 bytes)

```
Encoding: 11 OOO 100, data8
  OOO = same operation codes as above
```

| OOO | Mnemonic | Operation |
|-----|----------|-----------|
| 000 | ADI d | A ← A + d |
| 001 | ACI d | A ← A + d + CY |
| 010 | SUI d | A ← A − d |
| 011 | SBI d | A ← A − d − CY |
| 100 | ANI d | A ← A & d |
| 101 | XRI d | A ← A ^ d |
| 110 | ORI d | A ← A \| d |
| 111 | CPI d | Set flags for A − d, A unchanged |

Examples:
- `ADI 5` = `11 000 100, 0x05` (`0xC4, 0x05`) — A ← A + 5
- `CPI 0x0A` = `11 111 100, 0x0A` (`0xFC, 0x0A`) — compare A with 10

### Group 3: Rotate Instructions (1 byte)

Rotate the accumulator left or right, either circular or through the carry flag.

```
Encoding: 00 0RR 010  (R=rotate type, using bits 4–3)
```

| Binary | Mnemonic | Operation | Description |
|--------|----------|-----------|-------------|
| `00 000 010` (`0x02`) | RLC | CY ← A[7]; A ← (A << 1) \| A[7] | Rotate left circular (bit 7 → CY and bit 0) |
| `00 001 010` (`0x0A`) | RRC | CY ← A[0]; A ← (A >> 1) \| (A[0] << 7) | Rotate right circular (bit 0 → CY and bit 7) |
| `00 010 010` (`0x12`) | RAL | new_CY ← A[7]; A ← (A << 1) \| old_CY | Rotate left through carry (9-bit rotation) |
| `00 011 010` (`0x1A`) | RAR | new_CY ← A[0]; A ← (old_CY << 7) \| (A >> 1) | Rotate right through carry (9-bit rotation) |

```
RLC (Rotate Left Circular):         RAL (Rotate Left through Carry):
  ┌───────────────────────┐           ┌──────────────────────────────┐
  │ A[7] A[6] ... A[1] A[0] │           │ CY  A[7] A[6] ... A[1] A[0] │
  └──┬──────────────────┬──┘           └─┬──────────────────────────┬─┘
     │     rotate left  │                │        rotate left         │
     └──────────────────┘                └───────────────────────────┘
  CY ← A[7] (old)                     CY ← A[7] (old)
  A[0] ← A[7] (old)                   A[0] ← CY (old)
```

Flags: CY updated. Z, S, P not affected by rotate instructions.

### Group 4: Jump Instructions (3 bytes)

Jumps use a 14-bit address spread across two bytes following the opcode. The address
is stored **low byte first** (little-endian): `addr_lo` (bits 7–0) then `addr_hi`
(bits 13–8, in the low 6 bits of the second address byte).

```
Address reconstruction: address = (addr_hi & 0x3F) << 8 | addr_lo
```

#### JMP — Unconditional Jump (3 bytes)

```
Encoding: 01 111 100, addr_lo, addr_hi
          0x7C, low, high
```

PC ← (addr_hi[5:0] << 8) | addr_lo

#### Conditional Jumps (3 bytes)

```
Encoding: 01 CCC T00, addr_lo, addr_hi
  CCC = condition code (3 bits)
  T = sense (0 = jump if false/not-set, 1 = jump if true/set)
```

Condition codes CCC:
| CCC | Condition tested |
|-----|-----------------|
| 000 | CY (Carry) |
| 001 | Z (Zero) |
| 010 | S (Sign) |
| 011 | P (Parity) |
| 100–111 | (reserved) |

| Mnemonic | Encoding | Jump if... |
|----------|----------|------------|
| JFC addr | `01 000 000, lo, hi` (`0x40`) | Carry false (CY=0) |
| JTC addr | `01 000 100, lo, hi` (`0x44`) | Carry true (CY=1) |
| JFZ addr | `01 001 000, lo, hi` (`0x48`) | Zero false (Z=0) |
| JTZ addr | `01 001 100, lo, hi` (`0x4C`) | Zero true (Z=1) |
| JFS addr | `01 010 000, lo, hi` (`0x50`) | Sign false (S=0, positive) |
| JTS addr | `01 010 100, lo, hi` (`0x54`) | Sign true (S=1, negative) |
| JFP addr | `01 011 000, lo, hi` (`0x58`) | Parity false (P=0, odd) |
| JTP addr | `01 011 100, lo, hi` (`0x5C`) | Parity true (P=1, even) |
| JMP addr | `01 111 100, lo, hi` (`0x7C`) | Always (unconditional) |

### Group 5: Call Instructions (3 bytes)

```
Encoding: 01 CCC T10, addr_lo, addr_hi
```

Call instructions push the current PC onto the hardware stack and jump to the target.
The condition encoding is identical to jumps (T=0 for "if false", T=1 for "if true").

| Mnemonic | Encoding | Call if... |
|----------|----------|------------|
| CFC addr | `01 000 010, lo, hi` (`0x42`) | Carry false |
| CTC addr | `01 000 110, lo, hi` (`0x46`) | Carry true |
| CFZ addr | `01 001 010, lo, hi` (`0x4A`) | Zero false |
| CTZ addr | `01 001 110, lo, hi` (`0x4E`) | Zero true |
| CFS addr | `01 010 010, lo, hi` (`0x52`) | Sign false |
| CTS addr | `01 010 110, lo, hi` (`0x56`) | Sign true |
| CFP addr | `01 011 010, lo, hi` (`0x5A`) | Parity false |
| CTP addr | `01 011 110, lo, hi` (`0x5E`) | Parity true |
| CAL addr | `01 111 110, lo, hi` (`0x7E`) | Always (unconditional) |

### Group 6: Return Instructions (1 byte)

```
Encoding: 00 CCC T11
```

Pop the return address from the hardware stack and jump to it. Same condition encoding.

| Mnemonic | Encoding | Return if... |
|----------|----------|-------------|
| RFC | `00 000 011` (`0x03`) | Carry false |
| RTC | `00 000 111` (`0x07`) | Carry true |
| RFZ | `00 001 011` (`0x0B`) | Zero false |
| RTZ | `00 001 111` (`0x0F`) | Zero true |
| RFS | `00 010 011` (`0x13`) | Sign false |
| RTS | `00 010 111` (`0x17`) | Sign true |
| RFP | `00 011 011` (`0x1B`) | Parity false |
| RTP | `00 011 111` (`0x1F`) | Parity true |
| RET | `00 111 111` (`0x3F`) | Always (unconditional) |

### Group 7: Restart Instructions (1 byte)

```
Encoding: 00 AAA 101
```

Restart is a 1-byte CALL to a fixed low-memory address. The 3-bit field AAA encodes
the target address as `AAA << 3` (multiples of 8: 0, 8, 16, 24, 32, 40, 48, 56).
This provides 8 fast interrupt-service entry points in the first 64 bytes of memory.

| Mnemonic | Encoding | Target |
|----------|----------|--------|
| RST 0 | `00 000 101` (`0x05`) | 0x0000 |
| RST 1 | `00 001 101` (`0x0D`) | 0x0008 |
| RST 2 | `00 010 101` (`0x15`) | 0x0010 |
| RST 3 | `00 011 101` (`0x1D`) | 0x0018 |
| RST 4 | `00 100 101` (`0x25`) | 0x0020 |
| RST 5 | `00 101 101` (`0x2D`) | 0x0028 |
| RST 6 | `00 110 101` (`0x35`) | 0x0030 |
| RST 7 | `00 111 101` (`0x3D`) | 0x0038 |

RST is equivalent to `CAL target` but encoded in a single byte.

### Group 8: Input/Output Instructions (1 byte)

The 8008 has separate IN and OUT instructions. Port numbers are encoded in the opcode.

#### IN — Read from Input Port

```
Encoding: 01 PPP P01  (port number split across bits 4–1)
```

Reads 8 bits from input port `P` (0–7) into the accumulator. The port number is
encoded in bits 4–1 of the opcode byte (4-bit field, supporting 8 distinct ports
with the MSB always 0).

| Mnemonic | Encoding | Operation |
|----------|----------|-----------|
| IN 0 | `01 000 001` (`0x41`) | A ← port[0] |
| IN 1 | `01 000 011`... wait, let me clarify | A ← port[1] |

Actually, the IN instruction encoding in the 8008 is:
```
IN P: 01 PP0 001  where PP is 2-bit port number (ports 0–3 directly accessible)
```

The actual encoding puts the port number in bits [4:3]:
- IN 0: `0x41` (`01 000 001`)
- IN 1: `0x49` (`01 001 001`)
- IN 2: `0x51` (`01 010 001`)
- IN 3: `0x59` (`01 011 001`)
- IN 4: `0x61` (`01 100 001`)
- IN 5: `0x69` (`01 101 001`)
- IN 6: `0x71` (`01 110 001`)
- IN 7: `0x79` (`01 111 001`)

Flags: Not affected.

#### OUT — Write to Output Port

```
Encoding: 00 PPP P10  (port number in bits 4–1, 3 LSB = 010)
```

Writes the accumulator to output port `P` (0–23). The 8008 supports 24 output
ports via a 5-bit port field in the opcode.

- OUT 0: `0x02` — OUT 7: `0x3A` (first bank, port 8 via bit)
- Ports 8–23 use the high bits of the port field

Flags: Not affected.

### Group 9: Machine Instructions (1 byte)

#### HLT — Halt

The 8008 has two halt encodings:

| Encoding | Notes |
|----------|-------|
| `01 110 110` (`0x76`) | MOV M, M — the standard HLT opcode |
| `11 111 111` (`0xFF`) | Also halts the processor |

When HLT executes, the processor stops fetching instructions and enters a halted
state. External hardware can resume execution via the interrupt mechanism.
In the simulator, HLT terminates the `run()` loop.

## Execution Engine

The simulator uses a custom fetch-decode-execute loop rather than GenericVM, because:

1. Instructions are variable-length (1, 2, or 3 bytes) in a way that doesn't fit
   GenericVM's fixed opcode model
2. The 14-bit PC and 8-level push-down stack need custom logic
3. The IN/OUT port model has no equivalent in GenericVM

### Fetch

```python
opcode = memory[pc]
pc += 1

# Detect instruction length from opcode
if needs_data_byte(opcode):
    data = memory[pc]; pc += 1
if needs_addr_bytes(opcode):
    addr_lo = memory[pc]; pc += 1
    addr_hi = memory[pc]; pc += 1
```

Variable-length detection rules:
- MVI (00DDD110): 2 bytes total (opcode + data)
- ALU immediate (11OOO100): 2 bytes total
- JMP/JFC/etc. (01CCC_00, 01CCC_10): 3 bytes total (opcode + addr_lo + addr_hi)
- All other instructions: 1 byte

### Decode

The opcode byte is decomposed into bit fields:

```python
group   = (opcode >> 6) & 0x03    # bits 7–6
ddd     = (opcode >> 3) & 0x07    # bits 5–3 (destination or operation)
sss     = opcode & 0x07           # bits 2–0 (source or sub-op)
```

### Execute

Handlers are dispatched based on group and sub-fields. The dispatch tree follows
the instruction encoding structure exactly, mirroring the real chip's decoder.

### State

The `Intel8008Simulator` instance holds all CPU state:

```python
registers: list[int]     # [B, C, D, E, H, L, unused, A] — indexed by 3-bit reg code
flags: int               # 4-bit: bits [S, -, -, -, -, P, Z, CY]
memory: bytearray        # 16,384 bytes
pc: int                  # 14-bit program counter
stack: list[int]         # 8-entry push-down stack (stack[0] = return addrs)
stack_depth: int         # 0–7 — how many entries are in use beyond current PC
input_ports: list[int]   # 8 input port values (set externally)
output_ports: list[int]  # 24 output port values (written by OUT instructions)
```

## Public API

```python
class Intel8008Simulator:
    def __init__(self) -> None: ...

    # --- CPU State ---
    @property
    def a(self) -> int: ...                  # Accumulator (0–255)
    @property
    def b(self) -> int: ...                  # Register B
    @property
    def c(self) -> int: ...                  # Register C
    @property
    def d(self) -> int: ...                  # Register D
    @property
    def e(self) -> int: ...                  # Register E
    @property
    def h(self) -> int: ...                  # Register H
    @property
    def l(self) -> int: ...                  # Register L
    @property
    def hl_address(self) -> int: ...         # 14-bit: (H & 0x3F) << 8 | L
    @property
    def pc(self) -> int: ...                 # 14-bit program counter (0–16383)
    @property
    def flags(self) -> Intel8008Flags: ...   # Named flag fields
    @property
    def stack(self) -> list[int]: ...        # Current stack contents (up to 7 entries)
    @property
    def stack_depth(self) -> int: ...        # 0–7

    # --- Memory ---
    @property
    def memory(self) -> bytearray: ...       # 16,384 bytes

    # --- I/O ---
    def set_input_port(self, port: int, value: int) -> None: ...   # port 0–7
    def get_output_port(self, port: int) -> int: ...               # port 0–23

    # --- Execution ---
    def load_program(self, program: bytes, start_address: int = 0) -> None: ...
    def step(self) -> Intel8008Trace: ...
    def run(
        self,
        program: bytes,
        max_steps: int = 100_000,
        start_address: int = 0,
    ) -> list[Intel8008Trace]: ...
    def reset(self) -> None: ...

@dataclass
class Intel8008Flags:
    carry: bool     # CY
    zero: bool      # Z
    sign: bool      # S — True when result bit 7 is 1
    parity: bool    # P — True when result has even parity

@dataclass
class Intel8008Trace:
    address: int                   # PC where this instruction was fetched
    raw: bytes                     # Raw instruction bytes (1, 2, or 3 bytes)
    mnemonic: str                  # "MOV A, B", "ADI 0x05", "JMP 0x0100"
    a_before: int
    a_after: int
    flags_before: Intel8008Flags
    flags_after: Intel8008Flags
    memory_address: int | None     # Set if instruction accessed memory (M register)
    memory_value: int | None       # Value read/written (if applicable)
```

## Example Programs

### x = 1 + 2 (Basic Arithmetic)

```asm
; Load 1 into B, 2 into A, then compute A = A + B
        MVI B, 0x01     ; B ← 1               (0x06 0x01)
        MVI A, 0x02     ; A ← 2               (0x3E 0x02)
        ADD B           ; A ← A + B = 3       (0x80)
        HLT             ; stop                (0x76)
; Result: A = 0x03, Z=0, S=0, CY=0, P=1 (even parity of 0b00000011)
```

### x = 1 + 2 Using Memory

```asm
; Store 1 at [H:L]=0x0010, load it back, add 2
        MVI H, 0x00     ; H ← 0               (0x26 0x00)
        MVI L, 0x10     ; L ← 16 = address    (0x2E 0x10)
        MVI M, 0x01     ; mem[0x0010] ← 1     (0x36 0x01)
        MOV A, M        ; A ← mem[0x0010] = 1 (0x7E)
        ADI 0x02        ; A ← A + 2 = 3       (0xC4 0x02)
        HLT             ;                      (0x76)
```

### Multiply 4 × 5 (Loop with DCR / JFZ)

```asm
; A = 4 × 5 using repeated addition
; B = multiplicand (5), C = counter (4), A = accumulator (running total)
        MVI B, 0x05     ; B ← 5               (0x06 0x05)
        MVI C, 0x04     ; C ← 4               (0x0E 0x04)
        MVI A, 0x00     ; A ← 0               (0x3E 0x00)
LOOP:   ADD B           ; A ← A + B           (0x80)
        DCR C           ; C ← C - 1           (0x09)
        JFZ LOOP        ; if Z=0 goto LOOP    (0x48 <lo> <hi>)
        HLT             ; A = 20 = 0x14       (0x76)
```

### Subroutine: Absolute Value

```asm
; Call a subroutine that computes |A| (absolute value of signed byte)
        MVI A, 0xF6     ; A ← -10 (signed: 0xF6 = -10)  (0x3E 0xF6)
        CAL ABS_VAL     ; call subroutine                 (0x7E lo hi)
        HLT             ; A = 10 = 0x0A                   (0x76)

ABS_VAL:
; If S=0 (positive), return immediately
        JFS DONE        ; if Sign=0, skip negate          (0x50 lo hi)
; Negate: A ← ~A + 1 (two's complement)
        XRI 0xFF        ; A ← A ^ 0xFF (bitwise NOT)      (0xED 0xFF)
        ADI 0x01        ; A ← A + 1                       (0xC4 0x01)
DONE:   RET             ; return to caller                 (0x3F)
```

### Parity Check (Using P Flag)

```asm
; Check if the value in A has even or odd parity
; The 8008 P flag makes this trivial — just load the value and check P
        MVI A, 0b10110101   ; A ← 0xB5 (5 ones = odd parity)  (0x3E 0xB5)
        ORI 0x00            ; A unchanged, flags updated        (0xF4 0x00)
; After ORI 0x00: P=0 (odd parity), Z=0, S=1, CY=0
        HLT                                                     (0x76)
```

`ORI 0x00` is the canonical 8008 idiom for "set flags from A without changing A."

### H:L Pointer — Walking Memory

```asm
; Copy 4 bytes from address 0x0020 to 0x0030
; Uses H:L as a source pointer and H:D... wait, 8008 only has one address pair.
; Strategy: copy bytes one at a time using both H:L for source and fixed dest.
        MVI H, 0x00     ; H ← 0               (0x26 0x00)
        MVI L, 0x20     ; L ← 0x20 (source)   (0x2E 0x20)
        MOV A, M        ; A ← mem[0x0020]      (0x7E)
        MVI L, 0x30     ; L ← 0x30 (dest)      (0x2E 0x30)
        MOV M, A        ; mem[0x0030] ← A       (0x77)
        HLT             ;                       (0x76)
; (For a loop, maintain the offset in B/C and use ADI to advance L)
```

## Test Strategy

### Individual Instruction Tests

Every instruction must be tested in isolation with:
- Correct result value
- Correct flag updates after the instruction
- Flags that should not change are confirmed unchanged
- PC advances by the correct number of bytes (1, 2, or 3)

**ALU edge cases:**
- ADD: 0xFF + 0x01 = 0x00 with CY=1, Z=1
- SUB: 0x00 - 0x01 = 0xFF with CY=1 (borrow), S=1, Z=0, P=1
- ANA: 0xFF & 0x00 = 0x00 with CY=0 (always cleared)
- ADC with CY=1: 0xFE + 0x01 + 1 = 0x00 with CY=1, Z=1

**Rotate edge cases:**
- RLC with 0x80: result=0x01, CY=1 (bit 7 wraps to bit 0 and carry)
- RAL with 0xFF and CY=0: result=0xFE, CY=1
- RAR with 0x01 and CY=1: result=0x80, CY=1

**INR/DCR:**
- INR 0xFF → 0x00: Z=1, S=0, P=1, CY unchanged
- DCR 0x00 → 0xFF: Z=0, S=1, P=1, CY unchanged

### Flag Tests

- Z: test ADD producing 0, subtract producing 0, compare equal
- S: test results with bit 7 set (values 0x80–0xFF)
- P: test values with even (0x00, 0x03, 0xFF) and odd (0x01, 0x80, 0x7F) parity
- CY: test addition overflow, subtraction borrow, rotate into/out of carry
- Flag independence: INR/DCR do not touch CY; ANA/XRA/ORA clear CY

### Control Flow Tests

- JMP: unconditional jump to any 14-bit address
- JFC/JTC: all 8 conditions (4 flags × true/false)
- Conditional jump not taken: PC advances past 3-byte instruction
- CAL/RET: subroutine call saves return address, return restores it
- Nested calls: 2, 3, 4, 7 levels deep
- Stack overflow (8th call wraps): oldest return address silently overwritten
- RST: jump to fixed low-memory address, verify stack push

### Memory (M pseudo-register) Tests

- MOV A, M: read from current [H:L]
- MOV M, A: write to current [H:L]
- MVI M, d: write immediate to current [H:L]
- INR M / DCR M: increment/decrement memory
- ADD M / SUB M: ALU with memory operand
- H:L boundary: address 0x0000, 0x3FFF, and wraparound

### Stack Tests

- 1 level: CAL + RET restores correct PC
- 7 levels: maximum usable depth before wrap
- 8th call overwrites entry (verify with specific return addresses)
- Return without call (underflow): simulator must handle gracefully

### I/O Tests

- IN 0–7: verify A loaded from input port value
- OUT 0–23: verify output port updated with A value
- Input ports can be set externally and read correctly

### End-to-End Programs

- x = 1 + 2 (basic)
- Multiply 4 × 5 via loop
- Absolute value subroutine (tests CAL/RET + conditional branch)
- Parity check (flag-based branch)
- Bubble sort over 8 bytes in memory (tests H:L addressing, INR L, compare)
- Fibonacci sequence (tests nested register operations)

### Cross-Language Consistency

Same programs must produce identical results across all language implementations
(Python, Go, Rust, Ruby, TypeScript). Each language's simulator is run with
the same byte sequences and the final state (A, B, C, D, E, H, L, PC, flags)
is compared.

## Dependencies

```
intel8008-simulator
└── (no runtime dependencies)
    (test dependencies: pytest, hypothesis for property-based testing)
```

Unlike the Intel 4004 simulator, this package does not depend on `virtual-machine`.
The 8008's instruction format and stack model make a custom loop cleaner.

## Future Extensions

- **Assembler** — Accept 8008 assembly mnemonics and produce machine code. Use the
  `assembler` package (spec 06) as the foundation.
- **Disassembler** — Decode raw bytes back into mnemonic form (useful for trace output).
- **Interrupt simulation** — Model the INT line, which causes a RST instruction to be
  injected into the instruction stream by external hardware.
- **Timing model** — The 8008 takes 5–11 clock cycles per instruction. A cycle-accurate
  model would let us measure execution time for real programs.
- **Gate-level simulator** — See `07f2-intel8008-gatelevel.md`.
