# Intel 8086 Gate-Level Simulator (Rust)

Gate-level companion to `intel8086-simulator`. It implements the same complete
specified 8086 instruction surface and shared checked lifecycle while routing
data-path arithmetic, logic, address calculation, shifts/rotates, BCD, and
multiply/divide through fixed logic-gate networks.

## Persistent topology

The complete architectural machine is stored in exactly **8,392,922 D flip-flops**:

| State | DFFs |
|---|---:|
| 1 MiB memory | 8,388,608 |
| Thirteen 16-bit registers | 208 |
| FLAGS | 9 |
| Halt latch | 1 |
| 256 input + 256 output byte latches | 4,096 |

The memory implementation keeps a cached view of the DFF Q bus so full-state
inspection and differential hashing remain byte-linear; every write still
clocks both phases of the underlying eight flip-flops.

## Gate data paths

- 8/16/20-bit ripple-carry adders use `full_adder` stages.
- Subtraction, comparison, signed adjustment, and address increments use
  inverted operands and gate carry chains.
- AND/OR/XOR/NOT, zero, parity, sign, carry, auxiliary carry, and overflow use
  the workspace gate primitives.
- Shifts and rotates are fixed-width bit routing with gate flag outputs.
- 8×8 and 16×16 multiply use fixed shift/AND/adder partial-product networks.
- 16÷8 and 32÷16 divide use fixed restoring subtract/shift networks; signed
  variants gate-compute magnitudes and two's-complement result signs.
- Segmented physical addresses use a wired four-bit segment shift followed by
  a 20-stage ripple adder, masked to the real 20-bit bus.

## Checked API

The gate simulator shares `Intel8086State`, `StepTrace`, `ExecutionResult`, and
`Intel8086Error` with its functional oracle. Checked loads, ports,
snapshot/restore, single steps, and bounded runs are atomic and complete. The
legacy `load`, `step`, and `execute` methods remain available.

```rust
use coding_adventures_intel8086_gatelevel::Cpu8086;

let mut cpu = Cpu8086::new();
let result = cpu.run_checked(&[
    0xb8, 10, 0, // MOV AX,10
    0xbb, 5, 0,  // MOV BX,5
    0x03, 0xc3,  // ADD AX,BX
    0xf4,        // HLT
], 10)?;
assert_eq!(result.final_state.ax, 15);
# Ok::<(), coding_adventures_intel8086_gatelevel::Intel8086Error>(())
```

Direct DFF memory access is explicit:

```rust
# use coding_adventures_intel8086_gatelevel::Cpu8086;
# let mut cpu = Cpu8086::new();
cpu.write_memory(0x1234, 0xff);
assert_eq!(cpu.read_memory(0x1234), 0xff);
```

## Verification

The gate-level CPU consumes the functional package's reproducible 461-vector
full-state fixture. It covers all 256 first bytes, every dense group extension,
all effective-address forms, prefixes, strings, control flow, stack, and I/O,
and compares every register/flag plus hashes of all memory and both port banks.
The ALU also exhaustively checks every 8-bit multiply pair and runs seeded
word multiply/divide and signed-edge vectors.

See [`code/specs/07m2-intel8086-gatelevel.md`](../../../specs/07m2-intel8086-gatelevel.md).
