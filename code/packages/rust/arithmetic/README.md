# arithmetic (Rust)

Adder circuits and ALU — the computational heart of a CPU.

## What This Is

This crate implements arithmetic building blocks from logic gates: half adders, full adders, ripple-carry adders, and a complete Arithmetic Logic Unit (ALU) with status flags. Everything is built on top of the `logic-gates` crate.

## How It Fits in the Stack

The arithmetic crate sits on Layer 2, directly above logic-gates. The ALU is the core "calculator" inside every processor — every ADD, SUB, AND, OR instruction passes through it. Higher layers (floating-point arithmetic, pipeline stages) build on these primitives.

## Modules

- **`adders`** — half_adder, full_adder, ripple_carry_adder
- **`alu`** — ALU with ADD, SUB, AND, OR, XOR, NOT operations and status flags (zero, carry, negative, overflow)

## Usage

```rust
use arithmetic::adders::{half_adder, full_adder, ripple_carry_adder};
use arithmetic::alu::{alu, AluOp};

// Half adder: 1 + 1 = 0, carry 1
assert_eq!(half_adder(1, 1), (0, 1));

// Ripple-carry: 5 + 3 = 8 (LSB-first bit vectors)
let result = ripple_carry_adder(&[1,0,1,0], &[1,1,0,0]);
assert_eq!(result.sum, vec![0,0,0,1]); // 8

// ALU: 3 + 5 = 8 with flags
let a = vec![1,1,0,0,0,0,0,0]; // 3 LSB-first
let b = vec![1,0,1,0,0,0,0,0]; // 5 LSB-first
let result = alu(&a, &b, AluOp::Add);
// result.result = 8, result.zero = false, result.carry = false
```

## Running Tests

```bash
cargo test -p arithmetic
```
