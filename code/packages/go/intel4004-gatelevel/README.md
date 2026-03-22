# Intel 4004 Gate-Level Simulator (Go)

A gate-level simulation of the Intel 4004 CPU in Go. Every computation routes through real logic gates (AND, OR, NOT, XOR) and D flip-flops — no behavioral shortcuts.

## How it works

All data paths flow through the same gate chain the real Intel 4004 used:

```
NOT/AND/OR/XOR -> half_adder -> full_adder -> ripple_carry_adder -> ALU
D flip-flop -> register -> register file / program counter / stack
```

When you execute `ADD R3`, the register value is read from flip-flops, the accumulator is read from flip-flops, both are fed into the ALU (which uses full adders built from gates), and the result is clocked back into the accumulator's flip-flops.

## Architecture

| Component | File | Gates | Description |
|---|---|---|---|
| Bit helpers | `bits.go` | — | IntToBits / BitsToInt conversion (LSB-first) |
| ALU | `alu.go` | 32 | 4-bit arithmetic via ripple-carry adders |
| Registers | `registers.go` | 510 | 16x4-bit register file + accumulator + carry flag |
| Program Counter | `pc.go` | 96 | 12-bit register with half-adder increment chain |
| Stack | `stack.go` | 226 | 3-level x 12-bit hardware call stack |
| RAM | `ram.go` | 7,880 | 4 banks x 4 registers x 20 nibbles |
| Decoder | `decoder.go` | ~50 | Combinational AND/OR/NOT opcode detection |
| CPU | `cpu.go` | ~1,014 total | Full fetch-decode-execute pipeline |

## Dependencies

- `logic-gates` — AND, OR, NOT, XOR, D flip-flop, Register
- `arithmetic` — HalfAdder, FullAdder, ALU (ADD/SUB/AND/OR/XOR/NOT)

## Usage

```go
package main

import (
    cpu "github.com/adhithyan15/coding-adventures/code/packages/go/intel4004-gatelevel"
)

func main() {
    c := cpu.NewIntel4004GateLevel()

    // x = 1 + 2
    traces := c.Run([]byte{0xD1, 0xB0, 0xD2, 0x80, 0xB1, 0x01}, 10000)

    // c.Registers()[1] == 3
    // c.Halted() == true
}
```

## Supported instructions

All 46 Intel 4004 instructions are implemented:

- **Data movement**: NOP, HLT, LDM, LD, XCH, FIM, SRC, FIN, JIN
- **Arithmetic**: ADD, SUB, INC
- **Control flow**: JUN, JCN, ISZ, JMS, BBL
- **Accumulator ops**: CLB, CLC, IAC, CMC, CMA, RAL, RAR, TCC, DAC, TCS, STC, DAA, KBP, DCL
- **I/O**: WRM, WMP, WRR, WPM, WR0-3, SBM, RDM, RDR, ADM, RD0-3

## Testing

```bash
mise exec -- go test ./... -v -cover
mise exec -- go vet ./...
```
