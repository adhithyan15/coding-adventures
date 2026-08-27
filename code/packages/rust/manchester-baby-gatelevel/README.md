# manchester-baby-gatelevel

Gate-level Manchester Baby (SSEM) simulator in Rust.

The Baby ran the first stored program in 1948. Its entire architectural state
is a 32-word store, a 32-bit accumulator, a five-bit control-instruction
counter, and a halt latch. This package stores all 1,062 state bits in simulated
D flip-flops and routes instruction decoding and arithmetic through the
repository's logic-gate and ripple-carry-adder crates.

```rust
use manchester_baby_gatelevel::ManchesterBabyGateLevel;

// STP has function code 111 in instruction bits 13..15.
let stop = (0b111_u32 << 13).to_le_bytes();
let result = ManchesterBabyGateLevel::new().execute(&stop, 10)?;
assert!(result.halted);
assert_eq!(result.steps, 1);
# Ok::<(), manchester_baby_gatelevel::BabyError>(())
```

## Model boundary

- Store, accumulator, CI, and halt state are D flip-flop registers.
- CI increment, JRP, LDN, SUB, and negative-CMP skip use ripple-carry paths.
- Function bits are decoded into eight one-hot lines with NOT and AND gates;
  the SUB lines are combined with OR.
- Host control flow sequences a complete instruction cycle and selects a store
  line. It does not calculate architectural arithmetic results.
- This is an educational ISA-level circuit model, not a transistor-accurate
  reconstruction of Williams-tube electronics.

The public lifecycle is `reset`, `load`, `step`, `run`, `execute`, and
`get_state`. All execution is caller-bounded, invalid load origins return typed
errors, and state snapshots are owned. `flip_flop_count()` reports the exact
storage count; `gate_count()` reports the stable topology estimate documented
by Spec 07l2.

Tests cover every function encoding, arithmetic and CI wraparound, jumps,
self-modifying storage, bounded loops, and seeded differential execution
against `manchester-baby-simulator`.

## Dependencies

- logic-gates
- arithmetic
- manchester-baby-simulator (differential tests only)

## Development

```bash
# Run tests
bash BUILD
```

See `code/specs/07l2-manchester-baby-gatelevel.md` for the circuit contract.
