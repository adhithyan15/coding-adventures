# Microwatt / Power ISA simulator

This crate implements the bounded scalar fixed-point profile in
[`07bd-microwatt-powerisa-simulator.md`](../../../specs/07bd-microwatt-powerisa-simulator.md).
It is a 64-bit, little-endian, supervisor-context functional model—not a
pipeline, MMU, cache, SoC, timing, or gate model.

The architecture lock is OpenPOWER Foundation *Power ISA Version 3.0C*
(download SHA-256
`0c0064eda27161dcddf52f69a5d26799aaadc5ec73a2de9d90ed09a6ce015010`).
The implementation lock is Microwatt commit
`f341bed306a9a7281bf65200df8cc1f0a002617d`.

```rust
use microwatt_powerisa_simulator::encoding::{assemble, attn, d_form};
use microwatt_powerisa_simulator::MicrowattPowerIsaSimulator;

let program = assemble(&[d_form(14, 3, 0, 42), attn()]);
let mut cpu = MicrowattPowerIsaSimulator::new();
let result = cpu.execute(&program, 0x100, 4)?;
assert!(result.halted);
assert_eq!(result.final_state.gpr[3], 42);
# Ok::<(), microwatt_powerisa_simulator::Error>(())
```

`State` owns all 32 GPRs, CIA/LR/CTR, CR, the five modeled XER bits, exact
64 KiB memory, halt, and installed-image metadata. Loads, steps, bounded runs,
restore, and direct memory/register access are checked. Every failure is typed
and atomic; failed runs restore the whole pre-call state. Successful traces own
exact fetched little-endian bytes, raw word, canonical mnemonic, effects, and
complete before/after state.

`sync`, `lwsync`, `eieio`, and `isync` are trace-visible no-ops in this
single-agent model. Valid `ptesync` is recognized but returns
`UnsupportedInstruction` because the MMU/page-table facility is excluded.
Microwatt's exact `0x0000_0200` `attn` convention halts deterministically.

## Verification

```sh
cargo fmt --package microwatt-powerisa-simulator -- --check
cargo clippy --package microwatt-powerisa-simulator --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --package microwatt-powerisa-simulator --no-deps
cargo test --package microwatt-powerisa-simulator --release
```

The checked-in tests exhaust primary selectors and relevant extended-selector
spaces, cover reserved and recognized-but-unsupported neighbors, and exercise
the complete transactional lifecycle. Source-locked Microwatt semantic witness
rows cover the reviewed SO, word multiply/divide shaping/Rc, branch, memory,
and synchronization boundaries. A separately generated 173-row corpus was
executed against the complete pinned Microwatt VHDL core with official,
checksum-verified GHDL 6.0.0. Independent Rust consumers validate its manifest
and compare every complete-state/memory result with this simulator. Network and
GHDL access are not needed for normal tests. The exact tool/source pins,
reproduction command, and evidence boundary are in
[`tests/README.md`](tests/README.md).
