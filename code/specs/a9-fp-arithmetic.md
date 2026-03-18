# A9 — Floating-Point Arithmetic

## Overview

This package implements IEEE 754 floating-point arithmetic — the
mathematical foundation of GPU computing. Every GPU core contains a
floating-point unit (FPU) that performs these operations in hardware.
We simulate it in software, built conceptually from logic gates.

## Why floating-point matters for GPUs

CPUs primarily do integer arithmetic (array indexing, pointer math,
control flow). GPUs primarily do floating-point arithmetic:

- 3D graphics: every pixel color is 3 floats (R, G, B)
- ML training: every weight, gradient, and activation is a float
- Physics simulation: every position, velocity, force is a float
- Signal processing: every FFT coefficient is a float

The FPU is the GPU's ALU equivalent — the fundamental compute unit.

## IEEE 754 format

A floating-point number is stored as three fields:

```
| sign (1 bit) | exponent (E bits) | mantissa (M bits) |

Value = (-1)^sign × 2^(exponent - bias) × 1.mantissa

Example: FP32 representation of 3.14
  sign = 0 (positive)
  exponent = 10000000 (128, biased by 127 → actual exponent = 1)
  mantissa = 10010001111010111000011 (1.5700000524...)
  Value = (-1)^0 × 2^1 × 1.57 ≈ 3.14
```

## Supported formats

| Format | Total bits | Exponent | Mantissa | Bias | Use case |
|--------|-----------|----------|----------|------|----------|
| FP32   | 32        | 8        | 23       | 127  | Standard GPU, training |
| FP16   | 16        | 5        | 10       | 15   | Inference, 2x throughput |
| BF16   | 16        | 8        | 7        | 127  | Training, same range as FP32 |

## Operations

### FP Addition (fp_add)
The most complex FP operation. Pipeline:
1. Unpack sign, exponent, mantissa
2. Handle special cases (NaN, Inf, zero)
3. Align mantissas (shift smaller number right)
4. Add or subtract mantissas (based on signs)
5. Normalize result (shift to get leading 1)
6. Round (round-to-nearest-even)
7. Handle overflow (→ Inf) and underflow (→ denorm or zero)
8. Pack result

### FP Multiplication (fp_mul)
Simpler than addition:
1. XOR signs
2. Add exponents (subtract bias)
3. Multiply mantissas (integer multiply)
4. Normalize and round
5. Handle overflow/underflow

### Fused Multiply-Add (fp_fma)
`result = a × b + c` with only ONE rounding step.

This is the GPU's fundamental operation. A typical GPU core does
one FMA per clock cycle. NVIDIA's H100 has 16,896 FP32 cores,
each doing one FMA/clock at ~2 GHz = ~67 TFLOPS.

FMA is more accurate than separate mul + add because the intermediate
product is kept at full precision (no rounding between mul and add).

### Format conversion (fp_convert)
Convert between FP32, FP16, BF16. Required for:
- Mixed-precision training: compute in FP16, accumulate in FP32
- Loading weights: stored as BF16, computed as FP32

## Special values

| Exponent | Mantissa | Meaning |
|----------|----------|---------|
| All 0    | 0        | Zero (+0 or -0) |
| All 0    | Non-zero | Denormalized (very small, gradual underflow) |
| All 1    | 0        | Infinity (+∞ or -∞) |
| All 1    | Non-zero | NaN (Not a Number) |

## Testing requirements

IEEE 754 has many edge cases. Tests must cover:
- All special values: ±0, ±Inf, NaN, denormalized
- All operations with special values
- Overflow and underflow boundaries
- Rounding: round-to-nearest-even (banker's rounding)
- Catastrophic cancellation
- Format conversion precision loss
- FMA single-rounding accuracy vs separate mul+add

Target: 95%+ code coverage.
