# IAS gate-level simulator

This crate is the educational gate-level partner for the checked
`ias-simulator`. It implements all 22 Layer 07ac order classifications with an
independent Boolean transition engine and stores every persistent architectural
bit in repository D flip-flops.

It is deliberately an **architecture-level gate topology**, not an invented
reconstruction of undocumented IAS vacuum-tube placement, Williams-tube
timing, wire routing, or main-control pulse circuitry.

## Topology

The exact persistent topology is **164,060 D flip-flops**:

| State | DFFs |
|---|---:|
| 4,096 × 40-bit memory | 163,840 |
| AC and AR | 80 |
| Control counter | 13 |
| Buffered-right valid/order | 21 |
| Current control register | 20 |
| Function table register | 8 |
| Memory address register | 12 |
| Selectron register | 40 |
| Halt latch | 1 |
| Installed origin/word count | 25 |

Memory uses packed settled Q values. Every reset, restore, direct write,
program load, partial store, and instruction commit still clocks the selected
40-bit word through repository master/slave DFF primitives.

The combinational engine reconstructs each opcode from asserted one-hot gate
lines before host dispatch. Repository gates also drive address reconstruction,
40-bit add/negate/absolute/shift networks, 80-bit partial-product multiply,
restoring signed division, partial-store masks, sign tests, and memory write
enables.

## Checked execution

```rust
use ias_gatelevel::{encode_instruction, pack_instructions, IasGateLevel, Opcode};

let load = encode_instruction(Opcode::Load, 10)?;
let image = [pack_instructions(load, 0)?];
let mut machine = IasGateLevel::new();
machine.load_words(&image, 0)?;
machine.write_memory(10, 42)?;
let result = machine.run(2)?;
assert_eq!(result.final_state.accumulator, 42);
# Ok::<(), ias_gatelevel::IasError>(())
```

`step` settles the independent gate transition privately. The checked
functional simulator independently supplies the lifecycle/fault result; the
gate state is clocked only when the complete trace and full architectural state
agree exactly. Every fault is typed and atomic, and bounded runs roll back on
failure.

## Validation

From the Rust workspace:

```text
cargo test --release -p ias-gatelevel
cargo clippy -p ias-gatelevel --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p ias-gatelevel --no-deps
cargo llvm-cov --release -p ias-gatelevel --summary-only
```

The differential suite covers all orders, both instruction halves, buffered
self-modification, taken and untaken control transfer, arithmetic edges,
complete state restoration, topology, transport, lifecycle faults, and
transactional rollback.
