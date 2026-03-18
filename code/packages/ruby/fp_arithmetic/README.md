# coding_adventures_fp_arithmetic

IEEE 754 floating-point arithmetic built from logic gates -- Layer 30 of the computing stack.

## Overview

This gem implements IEEE 754 floating-point operations at the bit level, using logic gates (AND, OR, NOT, XOR) and adder circuits from the lower layers. It supports three floating-point formats used in modern computing and ML:

| Format | Bits | Exponent | Mantissa | Bias | Used by |
|--------|------|----------|----------|------|---------|
| FP32   | 32   | 8        | 23       | 127  | CPU, GPU |
| FP16   | 16   | 5        | 10       | 15   | GPU mixed-precision |
| BF16   | 16   | 8        | 7        | 127  | TPU, ML training |

## Dependencies

- `coding_adventures_logic_gates` -- AND, OR, NOT, XOR gates
- `coding_adventures_arithmetic` -- half_adder, full_adder, ripple_carry_adder
- `coding_adventures_clock` -- ClockGenerator for pipelined units

## Usage

```ruby
require "coding_adventures_fp_arithmetic"

FPA = CodingAdventures::FpArithmetic

# Encode a float to bits
bits = FPA.float_to_bits(3.14, FPA::FP32)
bits.sign       # => 0
bits.exponent   # => [1, 0, 0, 0, 0, 0, 0, 0]

# Decode back
FPA.bits_to_float(bits)  # => 3.140000104904175

# Arithmetic
a = FPA.float_to_bits(1.5)
b = FPA.float_to_bits(2.5)
result = FPA.fp_add(a, b)
FPA.bits_to_float(result)  # => 4.0

# Multiplication
result = FPA.fp_mul(a, b)
FPA.bits_to_float(result)  # => 3.75

# FMA (fused multiply-add): a * b + c with single rounding
c = FPA.float_to_bits(1.0)
result = FPA.fp_fma(a, b, c)
FPA.bits_to_float(result)  # => 4.75

# Format conversion
fp16_val = FPA.fp_convert(bits, FPA::FP16)

# Pipelined FP unit
clock = CodingAdventures::Clock::ClockGenerator.new
fp_unit = FPA::FPUnit.new(clock)
fp_unit.adder.submit(a, b)
fp_unit.tick(5)
FPA.bits_to_float(fp_unit.adder.results[0])  # => 4.0
```

## How It Fits in the Stack

```
Logic Gates (AND, OR, NOT, XOR)
  +-- Arithmetic (half_adder, full_adder, ripple_carry_adder)
      +-- FP Arithmetic (this package)
          +-- Clock (for pipelined units)
```

## Testing

```bash
bundle install
bundle exec rake test
```
