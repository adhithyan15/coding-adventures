# Intel 4004 Simulator (Go)

**Layer 4-d of the computing stack** — simulates the complete 1971 Intel 4004 microprocessor, the world's first commercial single-chip processor.

## Overview

The Intel 4004 was released by Intel in 1971 with just 2,300 transistors and a 740 kHz clock. It was designed for the Busicom 141-PF calculator but proved that a general-purpose processor could fit on a single chip.

This simulator implements all 46 real 4004 instructions plus a simulator-only HLT instruction. It is standalone — it does not depend on a generic VM framework, since the 4004's 4-bit data path, 3-level hardware stack, and BCD-oriented design are sufficiently unique.

## Architecture

| Component       | Details                                              |
|-----------------|------------------------------------------------------|
| Data width      | 4 bits (values 0-15)                                 |
| Instructions    | 8 bits (some are 2 bytes)                            |
| Registers       | 16 x 4-bit (R0-R15), organized as 8 pairs           |
| Accumulator     | 4-bit — all arithmetic flows through here            |
| Carry flag      | 1 bit — overflow/borrow indicator                    |
| Program counter | 12 bits (addresses 4096 bytes of ROM)                |
| Call stack      | 3-level hardware stack (wraps silently on overflow)  |
| RAM             | 4 banks x 4 registers x (16 main + 4 status) nibbles|
| ROM             | Up to 4096 bytes of program storage                  |

## Instruction Set (46 instructions)

```
0x00       NOP          No operation
0x01       HLT          Halt (simulator-only)
0x1_       JCN c,a  *   Conditional jump
0x2_ even  FIM Pp,d *   Fetch immediate to register pair
0x2_ odd   SRC Pp       Send register control (set RAM address)
0x3_ even  FIN Pp       Fetch indirect from ROM via P0
0x3_ odd   JIN Pp       Jump indirect via register pair
0x4_       JUN a    *   Unconditional jump (12-bit)
0x5_       JMS a    *   Jump to subroutine
0x6_       INC Rn       Increment register
0x7_       ISZ Rn,a *   Increment and skip if zero
0x8_       ADD Rn       Add register to accumulator with carry
0x9_       SUB Rn       Subtract register (complement-add)
0xA_       LD Rn        Load register into accumulator
0xB_       XCH Rn       Exchange accumulator and register
0xC_       BBL n        Branch back and load (return)
0xD_       LDM n        Load immediate into accumulator
0xE0-EF    I/O ops      RAM/ROM read/write (WRM, RDM, etc.)
0xF0-FD    Accum ops    CLB, CLC, IAC, CMC, CMA, RAL, RAR, etc.

* = 2-byte instruction
```

## Usage

```go
import intel4004simulator "github.com/adhithyan15/coding-adventures/code/packages/go/intel4004-simulator"

sim := intel4004simulator.NewIntel4004Simulator(4096)

// Compute 1 + 2 using the accumulator architecture:
// Load 1 → swap to R0 → load 2 → add R0 → swap result to R1
program := []byte{
    intel4004simulator.EncodeLdm(1), // A = 1
    intel4004simulator.EncodeXch(0), // R0 = 1, A = 0
    intel4004simulator.EncodeLdm(2), // A = 2
    intel4004simulator.EncodeAdd(0), // A = 2 + 1 = 3
    intel4004simulator.EncodeXch(1), // R1 = 3
    intel4004simulator.EncodeHlt(),  // Stop
}

traces := sim.Run(program, 1000)
// sim.Registers[1] == 3
```

### Subroutine Example

```go
b1, b2 := intel4004simulator.EncodeJms(0x004) // Call subroutine at 0x004
program := []byte{
    b1, b2,                              // 0x000: JMS 0x004
    intel4004simulator.EncodeHlt(),       // 0x002: return here
    0x00,                                 // 0x003: padding
    intel4004simulator.EncodeLdm(9),      // 0x004: subroutine
    intel4004simulator.EncodeBbl(5),      // 0x005: return with A=5
}
traces := sim.Run(program, 100)
// sim.Accumulator == 5
```

### RAM I/O Example

```go
b1, b2 := intel4004simulator.EncodeFim(0, 0x23) // Set P0 = register 2, char 3
program := []byte{
    b1, b2,
    intel4004simulator.EncodeSrc(0),  // SRC P0 — set RAM address
    intel4004simulator.EncodeLdm(9),  // A = 9
    0xE0,                              // WRM — write A to RAM
    intel4004simulator.EncodeLdm(0),  // A = 0
    0xE9,                              // RDM — read RAM into A
    intel4004simulator.EncodeHlt(),
}
traces := sim.Run(program, 100)
// sim.Accumulator == 9
```

## Key Design Decisions

- **Standalone**: Does not use a generic VM framework. The 4004's 4-bit constraints and unique RAM/stack architecture warrant a dedicated implementation.
- **Complement-add subtraction**: SUB computes `A + ~Rn + borrow_in`. Carry=true means NO borrow (matches MCS-4 manual).
- **Fixed arrays**: Registers are `[16]int`, RAM is `[4][4][16]uint8` — no heap allocation for core state.
- **Silent stack wrapping**: The 3-level hardware stack wraps mod 3 on overflow, exactly like real hardware.

## Testing

```bash
go test -v -cover ./...
```

Coverage: 98.6% of statements.
