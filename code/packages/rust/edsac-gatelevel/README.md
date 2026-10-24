# EDSAC gate-level simulator

`edsac-gatelevel` is the digital-logic partner for the repository's functional
1949 EDSAC simulator. It models the complete order code with repository CMOS
logic gates, ripple-carry full adders, and master/slave D flip-flops.

The model has exactly **18,095 persistent DFFs**:

| State | DFFs |
|---|---:|
| 512 × 18-bit even storage cells | 9,216 |
| 512 × 17-bit odd storage cells | 8,704 |
| SCR and order tank | 27 |
| 71-bit accumulator | 71 |
| 35-bit multiplier and multiplicand | 70 |
| Length, last-output, and halt latches | 7 |

The paper-tape input, teleprinter transcript, their cursors, and installed
program range are host peripheral/lifecycle metadata. They still participate
in complete snapshots and transactional rollback, but they do not invent
fixed hardware capacity.

## Fidelity boundary

The crate exposes stable instruction-boundary behavior, not analogue mercury
delay-line propagation or valve timing. The independent order engine decodes
through one-hot gate predicates; performs arithmetic with full-adder chains;
forms multiplication from AND partial products; and clocks every fixed-width
state update through repository DFFs. A decode, fetch, input, halt, or step-limit
fault occurs before any rising edge and therefore preserves the entire machine.
Runs retain at most 65,536 traces/output events; larger limits and failed trace
reservations become typed, atomic errors instead of unbounded allocation.

## Example

```rust
use edsac_gatelevel::{encode_order, EdsacGateLevel, Length, Opcode};

let program = [
    encode_order(Opcode::X, 0, Length::Short),
    encode_order(Opcode::Z, 0, Length::Short),
];
let mut machine = EdsacGateLevel::new();
machine.load_words(&program, 0)?;
let result = machine.run(2)?;
assert!(result.halted);
assert_eq!(result.steps, 2);
# Ok::<(), edsac_gatelevel::EdsacError>(())
```

The checked API also provides little-endian transport, direct short/long store
access, paper-tape input, accumulator setup, complete state/restore, atomic
single stepping, and transactional execute helpers. Public traces, states, and
errors are shared with `edsac-simulator`, making full-state lockstep comparison
straightforward.

See [Spec 07ab2](../../../specs/07ab2-edsac-gatelevel.md) for the normative
topology and acceptance boundary.
