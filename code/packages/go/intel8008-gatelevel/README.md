# Intel 8008 Gate-Level Simulator (Go)

**Layer 7-f2 of the computing stack** — simulates the 1972 Intel 8008 microprocessor at the hardware level, where every arithmetic operation routes through real logic gate functions.

## Overview

This package implements the Intel 8008 instruction set by composing gates from the `logic-gates` and `arithmetic` packages — the same gate chain the real chip used. Every add routes through 8 full adders (40 gates total). Every register write clocks through 8 D flip-flops. The instruction decoder is a combinational AND/OR/NOT gate tree.

This is **not** the behavioral simulator at `intel8008-simulator`. Where the behavioral simulator executes instructions directly with host-language arithmetic, this package routes everything through the gate abstractions built in the lower layers.

## Why Gate-Level?

The real Intel 8008 had approximately 3,500 transistors — 52% more than the 4004's 2,300. Those extra transistors paid for:

1. **8-bit datapath** instead of 4-bit — doubles the adder and register width
2. **7 registers** instead of accumulator-only — expands the register file
3. **14-bit PC** instead of 12-bit — larger address counter chain
4. **8-level stack** instead of 3-level — more push-down registers
5. **Richer decoder** — parity computation added, 3-byte instruction formats

By simulating at gate level, we can count exactly how each transistor contributes:

```
Component               Gates   Transistors (×4/gate)
---------------------   -----   ---------------------
ALU (8-bit)             96      384
Register file (7×8)     304     1,216
Flag register (4-bit)   16      64
Push-down stack (8×14)  522     2,088
Decoder                 ~80     320
Control + wiring        ~100    400
---------------------   -----   ---------------------
Total                   ~1,118  ~4,472
```

## Architecture

Every computation flows through this chain:

```
NOT/AND/OR/XOR → half_adder → full_adder → ripple_carry_adder → ALU
D flip-flop → register → register file / push-down stack
```

### Components

| File | Description |
|------|-------------|
| `bits.go` | `IntToBits`, `BitsToInt`, `ComputeParity` (7-gate XOR tree) |
| `alu.go` | 8-bit `GateALU`: add, subtract, bitwise ops, rotates, flag computation |
| `registers.go` | 7×8-bit `RegisterFile` and 4-bit `FlagRegister` built from D flip-flops |
| `stack.go` | 8-level 14-bit `PushDownStack` (entry 0 is always the PC) |
| `decoder.go` | Combinational `Decode()` function: opcode bits → control signals |
| `cpu.go` | `Intel8008GateLevel`: top-level wiring, `Step()`, `Run()` |

### The Push-Down Stack

The 8008's stack is unique: entry 0 IS the current program counter. There is no separate PC register. CALL rotates the stack down (saving the return address), RET rotates up:

```
CALL target:            RET:
  entry[7] ← entry[6]    entry[0] ← entry[1]
  entry[6] ← entry[5]    entry[1] ← entry[2]
  ...                     ...
  entry[1] ← entry[0]    entry[6] ← entry[7]
  entry[0] ← target       entry[7] ← 0
```

### The Decoder

The instruction decoder is pure combinational logic — no state, no clock. It uses AND/OR/NOT gate trees to pattern-match opcode bits into control signals. Example:

```
Decode ADD B (opcode = 0x80 = 10 000 000):
  b7=1, b6=0 → group_10 = AND(b7, NOT(b6)) = 1   ✓ ALU group
  b5=0, b4=0, b3=0 → ALUOp = 000 = ADD           ✓
  b2=0, b1=0, b0=0 → RegSrc = 000 = B            ✓
```

## Usage

```go
import intel8008gatelevel "github.com/adhithyan15/coding-adventures/code/packages/go/intel8008-gatelevel"

cpu := intel8008gatelevel.NewIntel8008GateLevel()

// Compute 1 + 2 through real gates:
//   MVI B, 1      (B = 1)
//   MVI A, 2      (A = 2)
//   ADD B          (8 full adders: A = 2+1 = 3)
//   HLT
program := []byte{
    0x06, 1,   // MVI B, 1
    0x3E, 2,   // MVI A, 2
    0x80,      // ADD B
    0x00,      // HLT
}

traces := cpu.Run(program, 1000)
// cpu.Registers()[7] == 3  (register 7 = A)
// cpu.GateCount() == 1118  (estimated)
```

### Multiply 4 × 5 via repeated addition

```go
program := []byte{
    0x06, 4,              // MVI B, 4
    0x0E, 5,              // MVI C, 5
    0x3E, 0,              // MVI A, 0
    0x80,                 // ADD B     ← loop start (addr 6)
    0x09,                 // DCR C
    0x48, 0x06, 0x00,     // JFZ 6     (jump if zero false = C != 0)
    0x00,                 // HLT
}
// After Run: cpu.Registers()[7] == 20
```

### Input/Output Ports

```go
cpu.SetInputPort(0, 0xAB) // set external input port 0

program := []byte{
    0x41,      // IN 0   (read port 0 into A)
    0x22,      // OUT 17 (write A to output port 17)
    0x00,      // HLT
}
cpu.Run(program, 100)
// cpu.GetOutputPort(17) == 0xAB
```

## Gate Count

```go
cpu := intel8008gatelevel.NewIntel8008GateLevel()
fmt.Println(cpu.GateCount()) // ~1118
```

## Testing

```bash
go test -v -cover ./...
```

Coverage: 89.7% of statements.

## Cross-Validation

The gate-level CPU produces identical results to the behavioral simulator for any program. The `TestCPU_*` integration tests validate this against known-correct expected values. Both simulators implement the same MCS-8 encoding conflicts:

- `0x76` → HLT (not MOV M,M)
- `0x7E` → CAL unconditional (not MOV A,M)
- `0x7C` → JMP unconditional (not MOV A,H)
- SSS=001 in group 01 → IN (not MOV D,C)
