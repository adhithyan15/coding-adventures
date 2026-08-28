# CDC 6600 gate-level simulator

This Rust crate is the gate-level companion to `cdc6600-simulator`. It models
the repository's complete Spec 07t subset: 22 short and 14 long instructions,
60-bit X registers, 18-bit A/B registers, parcel-addressed execution, and a
4,096-word 60-bit memory.

Every persistent architectural bit is stored in a simulated master-slave D
flip-flop, except hardwired B0. Opcode selection is one-hot. Architectural
addition, subtraction, signed comparison, variable shifts, multiplication,
address generation, branch decisions, and P updates flow through gates. Host
integers are confined to checked memory selection, instruction sequencing,
transport conversion, and owned observations.

See [Spec 07t2](../../../specs/07t2-cdc6600-gatelevel.md) for the model boundary,
topology counts, and conformance requirements.

```bash
cargo test -p cdc6600-gatelevel
cargo clippy -p cdc6600-gatelevel --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p cdc6600-gatelevel --no-deps
```
