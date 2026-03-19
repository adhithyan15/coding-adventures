# clock (Rust)

Clock signal generator — the heartbeat of every digital circuit.

## What This Is

This crate provides a system clock generator that produces a square wave alternating between 0 and 1. It includes clock edge records, the observer pattern for listeners, clock dividers for generating slower frequencies, and multi-phase clocks for pipeline stage timing.

## How It Fits in the Stack

The clock drives all sequential logic in the accelerator stack. Flip-flops, registers, counters, and pipeline stages all depend on clock edges to synchronize their operations. Without the clock, digital circuits would be chaotic — signals would arrive at different times and produce garbage.

## Types

- **`Clock`** — square-wave generator with listener support
- **`ClockEdge`** — record of a single clock transition (rising or falling)
- **`ClockDivider`** — generates slower clocks from a faster source
- **`MultiPhaseClock`** — generates non-overlapping phase signals for pipelines

## Usage

```rust
use clock::{Clock, ClockEdge, ClockDivider, MultiPhaseClock};

// Basic clock
let mut clk = Clock::new(1_000_000_000); // 1 GHz
let edge = clk.tick(); // rising edge
assert!(edge.is_rising);

// Clock divider (manual edge forwarding due to Rust ownership)
let mut divider = ClockDivider::new(1_000_000_000, 4);
// Feed edges from source clock to divider
let edge = clk.tick();
divider.on_edge(&edge);

// Multi-phase clock
let mut mpc = MultiPhaseClock::new(4);
mpc.on_edge(&edge);
```

## Rust Ownership Notes

Unlike the Python version where `ClockDivider` registers itself as a listener, in Rust we use explicit `on_edge()` calls. This is because Rust's borrow checker prevents a struct from both being stored as a listener AND holding a mutable reference to the source clock. The explicit approach makes data flow visible and guarantees memory safety at compile time.

## Running Tests

```bash
cargo test -p clock
```
