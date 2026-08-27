# ibm704-simulator

Instruction-level simulator for the IBM 704 mainframe (1954), implemented in
Rust from the architecture contract in `07h-ibm704-simulator.md`.

The simulator models 32K words of 36-bit core memory, the 38-bit
sign-magnitude accumulator (S, Q, P, and 35 magnitude bits), MQ, three 15-bit
index registers, the PC, overflow trigger, and divide-check trigger. It
implements the v1 load/store, integer arithmetic, multiply/divide, transfer,
index, and floating-point instruction families—42 instructions in total.

## Lifecycle

`IBM704Simulator` provides deterministic `reset`, strict `load`, `step`,
bounded `run`/`execute`, and owned `get_state` operations. Programs are
canonical five-byte big-endian words from `ibm704-encoder`; malformed words,
unknown opcodes, and out-of-range accesses return typed errors and halt the
machine rather than panicking or continuing with partial state.

```rust
use ibm704_encoder::{encode_cla, encode_htr, pack_word};
use ibm704_simulator::IBM704Simulator;

let words = [encode_cla(2), encode_htr(1), 123];
let program: Vec<u8> = words.into_iter().flat_map(pack_word).collect();
let result = IBM704Simulator::new().execute(&program, 10)?;
assert_eq!(result.final_state.accumulator_magnitude, 123);
# Ok::<(), ibm704_simulator::IBM704Error>(())
```

The integration suite includes per-family edge cases plus FORTRAN-style sum
and factorial loops, LISP `car`/`cdr` extraction, and a floating-point
polynomial.

## Scope

This v1 intentionally defers I/O devices, sense switches/lights, programmed
interrupts, BCD/boolean operations, shifts, sign-control operations, and the
unnormalized/rounding floating-point variants listed as v2 in the spec.

## Development

```bash
bash BUILD
cargo clippy --manifest-path ../Cargo.toml -p ibm704-simulator --all-targets -- -D warnings
```
