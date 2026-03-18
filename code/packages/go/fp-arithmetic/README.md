# fp-arithmetic (Go)

IEEE 754 floating-point arithmetic built from logic gates -- a Go port of the Python
`fp-arithmetic` package. This package implements floating-point encoding/decoding,
addition, subtraction, multiplication, fused multiply-add (FMA), format conversion,
and clock-driven pipelined execution units.

## Where it fits in the stack

```
Logic Gates (AND, OR, NOT, XOR)
    +-- Clock (system clock, listeners)
        +-- FP Arithmetic (this package)
            +-- formats: FloatFormat, FloatBits, FP32/FP16/BF16
            +-- ieee754: FloatToBits, BitsToFloat, IsNaN/IsInf/IsZero
            +-- fp_adder: FPAdd, FPSub, FPNeg, FPAbs, FPCompare
            +-- fp_multiplier: FPMul
            +-- fma: FMA, FPConvert
            +-- pipeline: PipelinedFPAdder, PipelinedFPMultiplier, PipelinedFMA, FPUnit
```

## Supported formats

| Format | Total | Exp | Mantissa | Bias | Used by |
|--------|-------|-----|----------|------|---------|
| FP32   | 32    | 8   | 23       | 127  | CPU, GPU (default precision) |
| FP16   | 16    | 5   | 10       | 15   | GPU training (mixed precision) |
| BF16   | 16    | 8   | 7        | 127  | TPU (native), ML training |

## Usage

```go
package main

import (
    fp "github.com/adhithyan15/coding-adventures/code/packages/go/fp-arithmetic"
    "github.com/adhithyan15/coding-adventures/code/packages/go/clock"
    "fmt"
)

func main() {
    // Basic arithmetic
    a := fp.FloatToBits(1.5, fp.FP32)
    b := fp.FloatToBits(2.5, fp.FP32)
    sum := fp.FPAdd(a, b)
    fmt.Println(fp.BitsToFloat(sum)) // 4.0

    // Multiplication
    product := fp.FPMul(a, b)
    fmt.Println(fp.BitsToFloat(product)) // 3.75

    // Fused multiply-add: a * b + c (single rounding)
    c := fp.FloatToBits(0.25, fp.FP32)
    fmaResult := fp.FMA(a, b, c)
    fmt.Println(fp.BitsToFloat(fmaResult)) // 4.0

    // Format conversion
    bf16 := fp.FPConvert(a, fp.BF16)
    fmt.Println(fp.BitsToFloat(bf16)) // ~1.5

    // Pipelined execution (like a GPU core)
    clk := clock.New(1_000_000)
    unit := fp.NewFPUnit(clk, fp.FP32)
    unit.Adder.Submit(a, b)
    unit.Tick(5) // 5 cycles for adder latency
    fmt.Println(fp.BitsToFloat(*unit.Adder.Results[0])) // 4.0
}
```

## Pipeline architecture

The pipelined units model real GPU floating-point hardware:

- **PipelinedFPAdder** (5 stages): Unpack -> Align -> Add/Sub -> Normalize -> Round/Pack
- **PipelinedFPMultiplier** (4 stages): Unpack+Exp -> Multiply -> Normalize -> Round/Pack
- **PipelinedFMA** (6 stages): Unpack -> Multiply -> Align -> Add -> Normalize -> Round/Pack
- **FPUnit**: All three pipelines sharing a single clock

After the initial fill-up latency, each pipeline produces one result per clock cycle
(throughput = 1 result/cycle).

## Testing

```bash
cd code/packages/go/fp-arithmetic
go test ./... -v -cover
```

77 tests, 85.6% statement coverage.

## Dependencies

- `logic-gates` -- AND, OR, NOT, XOR gates for special-value detection
- `clock` -- Clock and ClockEdge for pipeline timing
