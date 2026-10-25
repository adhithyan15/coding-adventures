# DEC VAX-11/780 gate-level simulator

This crate is the educational gate-level partner for the checked
`vax11-780-simulator`. It implements the bounded Layer 07ag integer ISA with
an independent Boolean transition engine and stores every persistent
architectural bit in repository D flip-flops.

It is deliberately an **ISA-level gate topology**, not a reconstruction of
DEC's VAX-11/780 boards, microcode, propagation delays, or semiconductor
devices. The public architecture is documented; a complete transistor netlist
for the historical machine is not being invented here.

## Topology

The exact persistent topology is **524,854 D flip-flops**:

| State | DFFs |
|---|---:|
| 64 KiB byte-addressed memory | 524,288 |
| R0–R15 | 512 |
| N, Z, V, C | 4 |
| Halt latch | 1 |
| Installed origin | 32 |
| Installed length | 17 |

Memory uses packed settled Q values; each changed byte is still clocked through
eight master/slave DFF primitives. Registers, flags, halt, origin, and length
are explicit stable-bit arrays.

The combinational engine uses repository gates for one-hot opcode and full
operand-specifier decode. Host orchestration dispatches only the byte
reconstructed from those asserted gate lines. Ripple-carry
addition/subtraction, bitwise operations, masks, condition codes, fixed
partial-product multiplication, restoring signed division, address
generation, branch selection, and packed-memory write enables are also
gate-derived.

## Checked execution

```rust
use vax11_780_gatelevel::{VaxGateLevel, encoding};

let program = encoding::assemble(&[
    encoding::movl(encoding::literal(7), encoding::register(0)),
    encoding::instruction(
        0xc0, // ADDL2
        &[encoding::literal(5), encoding::register(0)],
    ),
    vec![encoding::HALT],
]);

let result = VaxGateLevel::new().run_checked(&program, 8)?;
assert_eq!(result.final_state.regs[0], 12);
# Ok::<(), vax11_780_gatelevel::VaxError>(())
```

`step_checked` settles the independent gate transition privately. The checked
functional simulator independently supplies the lifecycle/fault result; the
gate state is clocked only when raw bytes, mnemonic, scalar trace state, memory,
registers, flags, halt, and load metadata agree exactly. Every fault is typed
and atomic. Bounded runs are transactional and retain at most the functional
model's explicit safe trace limit.

The implemented surface includes the complete Layer 07ag one-byte integer
core: byte/word/long arithmetic and data families, all documented general
operand modes, branches, subroutine control, address operations, stack pushes,
zero-extension, CASE, and HALT.

## Validation

From the Rust workspace:

```text
cargo test --release -p vax11-780-gatelevel
cargo clippy -p vax11-780-gatelevel --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p vax11-780-gatelevel --no-deps
cargo llvm-cov --release -p vax11-780-gatelevel --summary-only
```

The differential suite covers every arithmetic/data opcode family, the full
addressing grammar, all branch conditions, CASE, address and stack operations,
complete state transitions, topology, direct checked access, lifecycle faults,
and rollback.
