# fp-arithmetic

IEEE 754 floating-point formats and arithmetic, built from first principles in Rust.

## What this crate does

This is an educational implementation of IEEE 754 floating-point arithmetic. It teaches how floating-point numbers actually work at the hardware level -- from bit-level encoding through pipelined FMA units like those found in modern GPUs.

## How it fits in the stack

```
Layer 6: Floating-Point Arithmetic  <-- this crate
Layer 5: Hazard Detection
Layer 4: Cache, Branch Predictor
Layer 3: Arithmetic (integer)
Layer 2: Clock, Sequential Circuits
Layer 1: Logic Gates
Layer 0: NAND gate (the atom of computing)
```

## Modules

| Module | What it does |
|--------|-------------|
| `formats` | `FloatFormat` and `FloatBits` types, FP32/FP16/BF16 constants |
| `ieee754` | Encoding (`float_to_bits`) and decoding (`bits_to_float`), special value detection |
| `fp_adder` | Addition, subtraction, negation, absolute value, comparison |
| `fp_multiplier` | Multiplication via shift-and-add |
| `fma` | Fused multiply-add (single rounding!), format conversion |
| `pipeline` | Pipelined adder (5-stage), multiplier (4-stage), FMA (6-stage), FPUnit |

## Supported formats

| Format | Bits | Exponent | Mantissa | Bias | Used by |
|--------|------|----------|----------|------|---------|
| FP32 | 32 | 8 | 23 | 127 | CPU, GPU (default) |
| FP16 | 16 | 5 | 10 | 15 | GPU mixed precision |
| BF16 | 16 | 8 | 7 | 127 | Google TPU, ML training |

## Usage examples

```rust
use fp_arithmetic::*;

// Encode a float into its IEEE 754 bit representation
let bits = float_to_bits(3.14, FP32);
assert_eq!(bits.sign, 0); // positive

// Decode back to a Rust float
let value = bits_to_float(&bits);
assert_eq!(value as f32, 3.14_f32);

// Arithmetic operations
let a = float_to_bits(1.5, FP32);
let b = float_to_bits(2.5, FP32);
let sum = fp_add(&a, &b);
assert_eq!(bits_to_float(&sum) as f32, 4.0);

// Fused multiply-add: a * b + c with single rounding
let c = float_to_bits(0.25, FP32);
let result = fp_fma(&a, &b, &c);
assert_eq!(bits_to_float(&result) as f32, 4.0);

// Pipelined FP unit (GPU-style throughput)
let mut unit = FPUnit::new(FP32);
unit.adder.submit(float_to_bits(1.0, FP32), float_to_bits(2.0, FP32));
unit.tick(5);
assert_eq!(bits_to_float(&unit.adder.results[0]) as f32, 3.0);
```

## Design notes

- Uses `u64` for all bit manipulation (Rust's native 64-bit integers)
- Uses `f32::to_bits()` / `f32::from_bits()` for hardware-exact FP32 encoding
- Uses `Vec<u8>` for bit arrays (MSB-first) to stay faithful to the hardware model
- All code uses Knuth-style literate programming with extensive inline explanations
- No external dependencies
