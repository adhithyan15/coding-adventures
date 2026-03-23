# fp-arithmetic

IEEE 754 floating-point arithmetic from logic gates -- FP32, FP16, BF16 formats.

## Layer 9

This package is part of Layer 9 of the coding-adventures computing stack. It implements the complete floating-point arithmetic pipeline: encoding/decoding, arithmetic operations, fused multiply-add, format conversion, and pipelined hardware simulation -- all built on top of logic gates, just like real hardware.

## Supported Formats

| Format | Total Bits | Exponent | Mantissa | Bias | Used by |
|--------|-----------|----------|----------|------|---------|
| FP32   | 32        | 8        | 23       | 127  | CPU, GPU (default precision) |
| FP16   | 16        | 5        | 10       | 15   | GPU mixed-precision training |
| BF16   | 16        | 8        | 7        | 127  | TPU (native), ML training |

## Dependencies

- **logic-gates** (Layer 10): AND, OR, XOR gates for bit-level operations
- **clock** (Layer 8): Clock signal for pipeline simulation

## Module Structure

| Module | Purpose |
|--------|---------|
| `formats.lua` | FloatFormat, FloatBits types; FP32/FP16/BF16 constants; bit utilities |
| `ieee754.lua` | Encoding (float->bits), decoding (bits->float), special value detection |
| `fp_adder.lua` | Addition, subtraction, negation, absolute value, comparison |
| `fp_multiplier.lua` | Multiplication |
| `fma.lua` | Fused multiply-add, format conversion |
| `pipeline.lua` | Pipelined adder (5-stage), multiplier (4-stage), FMA (6-stage), FPUnit |

## Usage

```lua
local fp = require("coding_adventures.fp_arithmetic")

-- Encode a Lua number as FP32
local bits = fp.float_to_bits(3.14, fp.FP32)

-- Decode back to Lua number
local value = fp.bits_to_float(bits)  -- 3.14

-- Arithmetic
local a = fp.float_to_bits(1.5, fp.FP32)
local b = fp.float_to_bits(2.5, fp.FP32)
local sum = fp.fp_add(a, b)           -- 4.0
local product = fp.fp_mul(a, b)       -- 3.75

-- Fused multiply-add: a*b + c with single rounding
local c = fp.float_to_bits(0.25, fp.FP32)
local result = fp.fma(a, b, c)        -- 1.5*2.5 + 0.25 = 4.0

-- Format conversion (e.g., FP32 -> BF16 for TPU)
local bf16_bits = fp.fp_convert(bits, fp.BF16)

-- Pipelined simulation with clock
local clock = require("coding_adventures.clock")
local clk = clock.Clock.new(1000000)
local unit = fp.FPUnit.new(clk, fp.FP32)
unit.adder:submit(a, b)
unit:tick(6)  -- 5 pipeline stages + 1
local pipelined_result = unit.adder.results[1]
```

## Development

```bash
# Run tests
bash BUILD
```
