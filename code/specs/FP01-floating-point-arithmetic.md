# FP01 — Floating-Point Arithmetic

## Overview

This package implements IEEE 754 floating-point arithmetic from logic gates.
It is the **shared foundation** for both the CPU stack (FPU) and all three
accelerator stacks (GPU/TPU/NPU). Every floating-point operation in the repo
— from `3.14 + 2.71` in Python to matrix multiplication on a GPU — ultimately
passes through this layer.

## Layer position

```
Layer 11: Logic Gates (AND, OR, XOR, NAND)
    │
Layer 10: FP Arithmetic ← YOU ARE HERE
    │
    ├──→ CPU: FPU in cpu-simulator
    ├──→ GPU: CUDA core (FP32 ALU)
    ├──→ TPU: Processing element (MAC unit)
    └──→ NPU: MAC unit
```

## What is floating-point?

Integers can only represent whole numbers: 0, 1, 42, -7. But most real-world
computation needs fractions: 3.14159, 0.001, 6.022e23. Floating-point is a
way to represent these numbers in binary, using a format similar to scientific
notation:

```
Scientific notation:  -6.022 × 10^23
IEEE 754 (binary):    (-1)^sign × 1.mantissa × 2^(exponent - bias)
```

The number is stored as three bit fields:

```
FP32 (32 bits):  [sign(1)] [exponent(8)] [mantissa(23)]
FP16 (16 bits):  [sign(1)] [exponent(5)] [mantissa(10)]
BF16 (16 bits):  [sign(1)] [exponent(8)] [mantissa(7)]
```

## Formats

### FP32 (single precision) — the standard
- 32 bits total: 1 sign + 8 exponent + 23 mantissa
- Bias: 127 (exponent stored as unsigned, subtract 127 to get real value)
- Range: ±1.18e-38 to ±3.40e38
- Precision: ~7 decimal digits
- Used by: CPU FPU, GPU CUDA cores, default for most computation

### FP16 (half precision) — GPU training
- 16 bits total: 1 sign + 5 exponent + 10 mantissa
- Bias: 15
- Range: ±5.96e-8 to ±65504
- Precision: ~3-4 decimal digits
- Used by: GPU training (mixed precision), inference

### BF16 (brain float) — ML training
- 16 bits total: 1 sign + 8 exponent + 7 mantissa
- Bias: 127 (same exponent range as FP32!)
- Range: same as FP32 (±1.18e-38 to ±3.40e38)
- Precision: ~2-3 decimal digits
- Used by: TPU (native format), GPU training
- Key insight: BF16 keeps FP32's range but sacrifices precision. For ML
  training, range matters more than precision because gradients can be
  very large or very small.

## Public API

### Data types

```python
@dataclass(frozen=True)
class FloatFormat:
    name: str            # "fp32", "fp16", "bf16"
    total_bits: int
    exponent_bits: int
    mantissa_bits: int   # explicit bits (without the implicit leading 1)
    bias: int

FP32 = FloatFormat("fp32", 32, 8, 23, 127)
FP16 = FloatFormat("fp16", 16, 5, 10, 15)
BF16 = FloatFormat("bf16", 16, 8, 7, 127)

@dataclass(frozen=True)
class FloatBits:
    sign: int           # 0 or 1
    exponent: list[int] # exponent bits, MSB first
    mantissa: list[int] # mantissa bits, MSB first
    fmt: FloatFormat
```

### Encoding / decoding

```python
def float_to_bits(value: float, fmt: FloatFormat = FP32) -> FloatBits
def bits_to_float(bits: FloatBits) -> float
def is_nan(bits: FloatBits) -> bool
def is_inf(bits: FloatBits) -> bool
def is_zero(bits: FloatBits) -> bool
def is_denormalized(bits: FloatBits) -> bool
```

### Arithmetic (all built from logic gates)

```python
def fp_add(a: FloatBits, b: FloatBits) -> FloatBits
def fp_sub(a: FloatBits, b: FloatBits) -> FloatBits
def fp_mul(a: FloatBits, b: FloatBits) -> FloatBits
def fp_fma(a: FloatBits, b: FloatBits, c: FloatBits) -> FloatBits
def fp_neg(a: FloatBits) -> FloatBits
def fp_abs(a: FloatBits) -> FloatBits
def fp_compare(a: FloatBits, b: FloatBits) -> int  # -1, 0, 1
```

### Format conversion

```python
def fp_convert(bits: FloatBits, target_fmt: FloatFormat) -> FloatBits
```

## Algorithms (built from gates)

### FP Addition (fp_add)

```
Step 1: Compare exponents
        Use ripple_carry_adder to subtract exponents
        Determine which operand has the larger exponent

Step 2: Align mantissas
        Shift the smaller mantissa right by the exponent difference
        (barrel shifter built from MUX gates)

Step 3: Add mantissas
        Use ripple_carry_adder on the aligned mantissas
        Handle sign: if signs differ, subtract instead of add

Step 4: Normalize
        If result mantissa overflows (> 1.xxx), shift right, increment exponent
        If result mantissa is too small (0.0xxx), shift left, decrement exponent
        (leading-one detector built from gates)

Step 5: Round
        Round the mantissa to fit the target format's bit width
        (round-to-nearest-even is the default IEEE 754 mode)
```

### FP Multiplication (fp_mul)

```
Step 1: XOR signs → result sign

Step 2: Add exponents (ripple_carry_adder), subtract bias

Step 3: Multiply mantissas
        Shift-and-add multiplier built from gates:
        For each bit of multiplier B:
            if bit is 1: add shifted multiplicand A to accumulator
        Uses AND gates for partial products, ripple_carry_adder to sum

Step 4: Normalize and round (same as addition)
```

### Fused Multiply-Add (fp_fma)

```
a * b + c  with single rounding

Step 1: Multiply a × b with FULL precision (no rounding yet)
        Mantissa product is 2× width (46 bits for FP32)

Step 2: Align c's mantissa to the product's exponent

Step 3: Add the full-precision product and aligned c

Step 4: Normalize and round ONCE

Key insight: FMA gives better accuracy than separate mul + add because
it avoids the intermediate rounding. This matters for numerical stability
in ML training (gradient computation).
```

## Dependencies

- `coding-adventures-logic-gates` — AND, OR, XOR, NOT for all bit operations
- `coding-adventures-arithmetic` — ripple_carry_adder for exponent/mantissa arithmetic

## Test strategy

- Encode/decode roundtrip for known values (1.0, -1.0, 0.0, 3.14, smallest denormal)
- Special values: NaN, +Inf, -Inf, +0, -0
- FP add: test against Python's native float for many value pairs
- FP mul: same roundtrip testing
- FP FMA: verify it produces different (more accurate) results than separate mul+add
- Edge cases: overflow to Inf, underflow to zero, NaN propagation
- Format conversion: FP32 → FP16, FP32 → BF16, round-trip accuracy loss
- Target: 95%+ coverage
