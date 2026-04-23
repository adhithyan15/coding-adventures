# Intel 8008 Simulator (Go)

**Layer 7-f of the computing stack** — simulates the complete 1972 Intel 8008 microprocessor, the first 8-bit microprocessor and the direct ancestor of the x86 architecture.

## Overview

The Intel 8008 was released in April 1972 with approximately 3,500 transistors running at 0.5–0.8 MHz. Originally designed as the CPU for the Datapoint 2200 terminal, it became the foundation for the 8080, 8086, and ultimately every modern Intel processor. It is the first chip in history to hold an 8-bit accumulator alongside a full complement of 8-bit general-purpose registers.

This simulator implements all Intel 8008 instructions from the MCS-8 User Manual. It is standalone — it does not depend on a generic VM framework. The 8008's push-down stack, 14-bit program counter, and unique opcode encoding are sufficiently distinct to warrant a dedicated implementation.

## Architecture

| Component       | Details                                                      |
|-----------------|--------------------------------------------------------------|
| Data width      | 8 bits                                                       |
| Program counter | 14 bits (addresses up to 16,384 bytes)                       |
| Registers       | 7 × 8-bit: A (accumulator), B, C, D, E, H, L                |
| Pseudo-register | M — reads/writes memory at address formed by H:L             |
| Flags           | Carry, Zero, Sign, Parity (even parity → P=1)               |
| Stack           | 8-level hardware push-down stack (entry 0 is the PC)        |
| Memory          | Up to 16,384 bytes (14-bit address space)                    |

### Register Encoding

The MCS-8 manual uses a 3-bit DDD/SSS field to identify registers:

| Code | Register | Notes                                    |
|------|----------|------------------------------------------|
| 0    | B        |                                          |
| 1    | C        | SSS=001 in group-01 encodes IN, not MOV  |
| 2    | D        |                                          |
| 3    | E        |                                          |
| 4    | H        |                                          |
| 5    | L        |                                          |
| 6    | M        | Pseudo-register: memory[((H&0x3F)<<8)\|L] |
| 7    | A        |                                          |

### Opcode Structure

Instructions are grouped by the top 2 bits:

| Group | Bits 7-6 | Primary use                                     |
|-------|----------|-------------------------------------------------|
| 0     | `00`     | HLT, MVI, rotates, INR, DCR, OUT               |
| 1     | `01`     | HLT, MOV, IN, unconditional JMP/CAL, cond. JMP/CAL |
| 2     | `10`     | ALU register operations (ADD, ADC, SUB, SBB, ANA, XRA, ORA, CMP) |
| 3     | `11`     | ALU immediate (ADI, SUI, ANI, XRI, ORI, CPI), RST, RET |

### Encoding Conflicts (by design, per MCS-8 manual)

The 8008's dense encoding creates intentional aliasing:

| Opcode | Appears to be | Actually is |
|--------|---------------|-------------|
| `0x76` | MOV M, M      | HLT         |
| `0xFF` | RST 7         | HLT         |
| `0x7E` | MOV A, M      | CAL (unconditional call) |
| `0x7C` | MOV A, H      | JMP (unconditional jump) |
| `0x40`–`0x5E` even | MOV D,B/D/E/H/L/M with SSS=0/2/4 | Conditional JMP/CAL |
| `0x09`, `0x0B`, etc. | MOV — | Conditional RET        |
| SSS=001 in group 01 | MOV D, C | IN port          |

### Push-Down Stack

The 8008 uses an 8-level hardware stack where **entry 0 is always the current PC**. Unlike a conventional stack:
- A CALL rotates existing entries down one position and sets entry 0 to the target address
- A RET rotates entries up one position (entry 0 takes the value from entry 1)
- This means calls cannot exceed 7 levels deep without losing the oldest return address

### Flags

| Flag   | Set to 1 when…                                   |
|--------|--------------------------------------------------|
| Carry  | Arithmetic overflowed (carry out of bit 7) or borrow occurred in subtraction |
| Zero   | Result equals 0x00                               |
| Sign   | Bit 7 of result is 1                             |
| Parity | XOR of all 8 result bits equals 0 (even parity)  |

## Instruction Set

### Group 0 (00xxxxxx): Data Movement and Rotates

| Mnemonic       | Opcode pattern        | Bytes | Description                     |
|----------------|-----------------------|-------|---------------------------------|
| HLT            | `00 000 000` = 0x00   | 1     | Halt                            |
| HLT            | `11 111 111` = 0xFF   | 1     | Halt (alternate encoding)       |
| MVI D, imm     | `00 DDD 110`          | 2     | Load immediate into register D  |
| INR D          | `00 DDD 000`          | 1     | Increment register D            |
| DCR D          | `00 DDD 001`          | 1     | Decrement register D            |
| RLC            | `00 000 010`          | 1     | Rotate A left through carry     |
| RRC            | `00 001 010`          | 1     | Rotate A right through carry    |
| RAL            | `00 010 010`          | 1     | Rotate A left (A7 → CY)        |
| RAR            | `00 011 010`          | 1     | Rotate A right (A0 → CY)       |
| OUT port       | `00 PPP 010`, DDD≥4   | 1     | Output A to port (port = opcode>>1 & 0x1F) |

### Group 1 (01xxxxxx): MOV, IN, Jumps, Calls

| Mnemonic       | Opcode                | Bytes | Description                     |
|----------------|-----------------------|-------|---------------------------------|
| HLT            | `0x76`                | 1     | Halt (alias of MOV M, M)        |
| MOV D, S       | `01 DDD SSS`          | 1     | Copy register S into D          |
| IN port        | `01 DDD 001`, SSS=001 | 1     | Input from port into A          |
| JMP addr       | `0x7C` + lo + hi      | 3     | Unconditional jump              |
| CAL addr       | `0x7E` + lo + hi      | 3     | Unconditional call              |
| Jcc addr       | `01 CCC 000` + lo + hi| 3     | Conditional jump on condition C |
| Ccc addr       | `01 CCC 010` + lo + hi| 3     | Conditional call on condition C |
| Rcc            | `00 CCC 011`          | 1     | Conditional return on C         |

Condition codes (CCC field):
| CCC | Mnemonic suffix | Condition       |
|-----|-----------------|-----------------|
| 000 | FC              | Carry false     |
| 001 | FZ              | Zero false      |
| 010 | FS              | Sign false      |
| 011 | FP              | Parity false    |
| 100 | TC              | Carry true      |
| 101 | TZ              | Zero true       |
| 110 | TS              | Sign true       |
| 111 | TP              | Parity true     |

### Group 2 (10xxxxxx): ALU Register Operations

| Mnemonic | Opcode pattern | Description                     |
|----------|----------------|---------------------------------|
| ADD S    | `10 000 SSS`   | A = A + S                       |
| ADC S    | `10 001 SSS`   | A = A + S + CY                  |
| SUB S    | `10 010 SSS`   | A = A - S (CY=1 means borrow)   |
| SBB S    | `10 011 SSS`   | A = A - S - CY                  |
| ANA S    | `10 100 SSS`   | A = A AND S                     |
| XRA S    | `10 101 SSS`   | A = A XOR S                     |
| ORA S    | `10 110 SSS`   | A = A OR S                      |
| CMP S    | `10 111 SSS`   | Flags only: A - S (A unchanged) |

### Group 3 (11xxxxxx): Immediate ALU, RST, RET

| Mnemonic | Opcode    | Bytes | Description                         |
|----------|-----------|-------|-------------------------------------|
| ADI imm  | `11 000 100` | 2  | A = A + imm                         |
| SUI imm  | `11 001 100` | 2  | A = A - imm                         |
| ANI imm  | `11 100 100` | 2  | A = A AND imm                       |
| XRI imm  | `11 101 100` | 2  | A = A XOR imm                       |
| ORI imm  | `11 110 100` | 2  | A = A OR imm                        |
| CPI imm  | `11 111 100` | 2  | Flags only: A - imm (A unchanged)   |
| RST n    | `11 NNN 101` | 1  | Call to address 8×N                 |
| RET      | `11 111 111` is HLT; `11 000 111` | 1 | Unconditional return |

## Usage

```go
import intel8008simulator "github.com/adhithyan15/coding-adventures/code/packages/go/intel8008-simulator"

sim := intel8008simulator.NewSimulator(16384)

// Compute 1 + 2:
//   MVI B, 1    ; B = 1
//   MVI A, 2    ; A = 2
//   ADD B       ; A = A + B = 3
//   HLT
program := []byte{
    0x06, 0x01, // MVI B, 1   (00 000 110, 0x01)
    0x3E, 0x02, // MVI A, 2   (00 111 110, 0x02)
    0x80,       // ADD B       (10 000 000)
    0x00,       // HLT
}

traces := sim.Run(program, 1000)
// sim.Registers[7] == 2+1 = 3  (A is register index 7)
```

### Multiply Example (4 × 5 using repeated addition)

```go
// Compute 4 × 5 by adding 4 five times:
//   MVI B, 4    ; multiplier
//   MVI C, 5    ; counter
//   MVI A, 0    ; accumulator = 0
// loop:
//   ADD B        ; A += B
//   DCR C        ; C--
//   JFZ loop     ; jump if Zero false (C != 0)
//   HLT
program := []byte{
    0x06, 0x04,  // MVI B, 4
    0x0E, 0x05,  // MVI C, 5
    0x3E, 0x00,  // MVI A, 0
    0x80,        // ADD B          (offset 6)
    0x09,        // DCR C
    0x40, 0x06, 0x00,  // JFZ 6   (jump back to ADD B)
    0x00,        // HLT
}
// After Run: A == 20
```

### Call/Return Example

```go
// CAL subroutine at address 5, return to caller
//   CAL 5        ; call subroutine
//   HLT          ; stop after return
//   (padding)
//   MVI A, 42   ; subroutine body
//   RET          ; return to caller
program := []byte{
    0x7E, 0x05, 0x00,  // CAL 5
    0x00,              // HLT
    0x00,              // padding
    0x3E, 0x2A,        // MVI A, 42
    0xC7,              // RET
}
// After Run: A == 42
```

## Key Design Decisions

- **Standalone**: Does not use a generic VM framework. The 8008's push-down stack, 14-bit address space, and opcode aliasing make it unique enough to implement directly.
- **Encoding conflicts honored**: `0x76` (MOV M,M → HLT), `0x7E` (MOV A,M → CAL), `0x7C` (MOV A,H → JMP) all follow the MCS-8 manual exactly. These are not bugs; they are intentional hardware design choices.
- **SUB/SBB borrow convention**: CY=1 means a borrow occurred (result underflowed). This matches the 8008 convention where borrow and carry use the same flag with opposite polarity compared to later Intel chips.
- **Parity**: P=1 means even parity. Computed as `NOT(XOR of all 8 result bits)`.
- **M address**: `((H & 0x3F) << 8) | L` — only the lower 6 bits of H contribute to the 14-bit address.
- **Stack depth tracking**: `stackDepth` tracks how many valid return addresses are stored. A fresh simulator starts with depth 0; each CAL increments it (up to 7 active return addresses).

## Testing

```bash
go test -v -cover ./...
```

Coverage: 86.2% of statements.
