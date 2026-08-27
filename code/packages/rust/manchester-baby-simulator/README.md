# Manchester Baby simulator

A deterministic Rust functional simulator for the Manchester Small-Scale
Experimental Machine (SSEM), which ran the world's first stored program in
1948.

The Baby has a 32-word store, a 32-bit accumulator, a five-bit control
instruction counter, and seven operations. This crate implements both SUB
encodings and the machine's unusual pre-increment-before-fetch cycle. All
arithmetic wraps explicitly at 32 bits.

## Usage

```rust
use manchester_baby_simulator::{
    encode_instruction, BabySimulator, Function,
};

let stop = encode_instruction(Function::Stop, 0).to_le_bytes();
let result = BabySimulator::new().execute(&stop, 1)?;

assert!(result.halted);
assert_eq!(result.steps, 1);
# Ok::<(), manchester_baby_simulator::BabyError>(())
```

For debugging, call `reset`, `load`, and `step` separately and inspect the
returned trace records. `run` and `execute` always require a maximum step count
so malformed or intentionally looping guest programs cannot run unbounded.

The architecture contract is in
[`../../../specs/07l-manchester-baby-simulator.md`](../../../specs/07l-manchester-baby-simulator.md).

## Development

```bash
bash BUILD
cargo clippy --all-targets -- -D warnings
```
