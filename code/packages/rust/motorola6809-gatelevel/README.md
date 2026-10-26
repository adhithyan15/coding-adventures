# Motorola MC6809 gate-level simulator

This package is the gate-level partner to `motorola6809-simulator`. It stores
the complete 64 KiB memory, A/B, X/Y, U/S, PC, DP, CC, wait state, and loaded
image metadata in exactly 524,434 simulated D flip-flops.

Its independent transition engine covers all documented base, page-2, and
page-3 opcodes and all 217 legal indexed postbytes. One-hot decoders, Boolean
networks, ripple adders, gated partial-product multiplication, shifts,
selectors, branch predicates, address generation, and stack controls produce
the candidate transition. The checked functional simulator supplies only the
trace and fault oracle; state is clocked after complete agreement.

```rust
use motorola6809_gatelevel::Motorola6809GateLevel;

let mut cpu = Motorola6809GateLevel::new();
cpu.load(&[0x86, 0x15, 0xc6, 0x03, 0x3d, 0x13], 0)?;
let result = cpu.run(8)?;
assert_eq!(result.final_state.d(), 0x003f);
# Ok::<(), motorola6809_gatelevel::Motorola6809Error>(())
```

This is an educational architecture-level topology, not a transistor-accurate
reconstruction of Motorola mask artwork or undocumented internal control.
Pins, buses, peripherals, and timing belong to the later system/SoC layer. See
[`../../../specs/07ah2-motorola6809-gatelevel.md`](../../../specs/07ah2-motorola6809-gatelevel.md).

## Development

```bash
bash BUILD
```
