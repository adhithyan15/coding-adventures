# logic-gates (Rust)

Logic gates and sequential circuits — the foundation of all digital hardware.

## What This Is

This crate implements the seven fundamental logic gates (AND, OR, NOT, XOR, NAND, NOR, XNOR), proves that all gates can be built from NAND alone (functional completeness), and provides multi-input variants. It also implements sequential logic elements (SR latch, D latch, D flip-flop, register, shift register, counter) that give circuits the ability to remember.

## How It Fits in the Stack

This is **Layer 1** — the absolute foundation. Every other crate in the accelerator stack depends on these gates. The `arithmetic` crate uses them to build adders and ALUs. The `clock` crate drives the sequential elements. Higher layers (cache, branch predictor, hazard detection) all ultimately reduce to combinations of these gates.

## Modules

- **`gates`** — 7 fundamental gates, NAND-derived gates, multi-input AND/OR
- **`sequential`** — SR latch, D latch, D flip-flop, register, shift register, counter

## Usage

```rust
use logic_gates::gates::{and_gate, or_gate, not_gate, xor_gate};
use logic_gates::sequential::{d_flip_flop, FlipFlopState};

// Basic gates
assert_eq!(and_gate(1, 1), 1);
assert_eq!(xor_gate(1, 0), 1);

// Flip-flop (edge-triggered memory)
let mut state = FlipFlopState::default();
d_flip_flop(1, 0, &mut state); // clock low: absorb
let (q, _) = d_flip_flop(1, 1, &mut state); // clock high: output
assert_eq!(q, 1);
```

## Running Tests

```bash
cargo test -p logic-gates
```
