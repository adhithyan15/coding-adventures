# Arithmetic (Go Port)

**Layer 2 of the computing stack** — builds number computation from logic gates.

## What is binary arithmetic?

Computers don't understand decimal numbers (0-9). They only understand binary — 0 and 1. To add two numbers, a computer breaks them into their binary digits (bits) and adds them one bit at a time, just like you add decimal numbers digit by digit with carrying.

This package builds addition from the ground up using only the logic gates from Layer 1 (`logic-gates`).

## The circuits

### Half Adder & Full Adder
A **Half Adder** adds two bits to produce a Sum and a Carry.
A **Full Adder** adds two bits PLUS a carry-in from a previous operation.

### Ripple Carry Adder
Chains `N` Full Adders together. The carry output of each adder feeds into the carry input of the next one. This enables multi-bit integer addition.

### ALU (Arithmetic Logic Unit)

The core computational brain of a CPU. It takes two binary numbers and an operator (`ADD`, `SUB`, `AND`, `OR`, `XOR`, `NOT`) and produces a result along with status flags:
- **Zero**: Is the result entirely zeros?
- **Carry**: Did the operation overflow the bit width (unsigned)?
- **Negative**: Is the Most Significant Bit 1?
- **Overflow**: Did signed arithmetic produce a mathematically impossible result?

Two's complement math allows the ALU to use the exact same adder circuit for both addition and subtraction.

## Usage

```go
import (
	"fmt"
	"github.com/adhithyan15/coding-adventures/code/packages/go/arithmetic"
)

func main() {
    // Half adder: 1 + 1 = 0 with carry 1 (binary 10)
    sum, carry := arithmetic.HalfAdder(1, 1)

    // Full adder: 1 + 1 + carryIn=1 = 1 with carry 1 (binary 11)
    sum, carry = arithmetic.FullAdder(1, 1, 1)

    // Ripple carry: 5 + 3 = 8
    a := []int{1, 0, 1, 0} // 5 in binary (LSB first)
    b := []int{1, 1, 0, 0} // 3 in binary
    result, carryOut := arithmetic.RippleCarryAdder(a, b, 0)
    // result = [0, 0, 0, 1] (8 in binary)

    // ALU: 5 - 3 = 2
    alu := arithmetic.NewALU(4)
    res := alu.Execute(arithmetic.SUB, a, b)
    fmt.Println(res.Value) // [0, 1, 0, 0] (2 in binary)
}
```

## Spec

See [09-arithmetic.md](../../../specs/09-arithmetic.md) for the full specification.
