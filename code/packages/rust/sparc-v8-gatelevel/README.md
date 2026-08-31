# sparc-v8-gatelevel

Instruction-boundary gate-level Rust implementation of the SPARC V8
processor (1987). Persistent architectural state is clocked through D
flip-flops, and arithmetic, logic, shifts, multiplication, division, decode,
and register writes use repository gate networks. Host integers remain only
at the simulator boundary for addresses, instruction fields, lifecycle
metadata, and bit-vector conversion.

## Architecture

```text
sparc-v8-gatelevel
├── state.rs         Packed stable-Q DFF storage and 64 KiB DFF memory
├── bits.rs          Bit-vector helpers, shifts, sign extension, arithmetic
├── alu.rs           Gate ALU, fixed-round multiply/divide, MULScc, SETHI
├── register_file.rs 56 physical registers, PC/nPC, Y, PSR, CWP, depth
├── decoder.rs       SPARC formats F1, F2, F3r, and F3i
└── cpu.rs           Atomic checked lifecycle and instruction transitions
```

The exact persistent topology is **526,185 D flip-flops**: 524,288 memory
bits, 1,792 physical-register bits, 64 PC/nPC bits, 32 Y bits, four PSR bits,
two CWP bits, two window-depth bits, and one halt latch.

## Implemented SPARC V8 surface

- CALL, SETHI, all 16 Bicc predicates, NOP, JMPL, and `ta 0`
- ADD/ADDX/SUB/SUBX and AND/ANDN/OR/ORN/XOR/XNOR, with `cc` variants
- SLL, SRL, SRA, MULScc, UMUL/SMUL, UDIV/SDIV, and all `cc` variants
- LD/LDUB/LDUH/LDSB/LDSH and ST/STB/STH
- SAVE/RESTORE over three register windows, with typed overflow/underflow
- RD/WR `%y`, a 64 KiB wrapping big-endian memory, and PC/nPC control flow

The documented six-bit `UMULcc`, `SMULcc`, `UDIVcc`, and `SDIVcc` encodings
are `0x1A`, `0x1B`, `0x1E`, and `0x1F`. Divide overflow saturates and sets V;
divide by zero is an atomic typed fault.

## Checked lifecycle

```rust
use coding_adventures_sparc_v8_gatelevel::SparcCpu;

let program = [0x90_10_20_2Au32.to_be_bytes(), 0x91D0_2000u32.to_be_bytes()]
    .concat();
let mut cpu = SparcCpu::new();
let result = cpu.run_checked(&program, 8)?;
assert!(result.halted);
assert_eq!(result.final_state.regs[8], 42);
# Ok::<(), coding_adventures_sparc_v8_gatelevel::SparcError>(())
```

`load_checked` and `load_at_checked` validate before deterministic reset.
`get_state` and `restore` cover memory, all physical registers, PC/nPC, Y,
PSR, CWP, window depth, halt, and the installed-program range. Checked direct
access rejects invalid indices, alignment, and bounds. `step_checked` validates
the gate transition against the functional oracle and rolls back every fault
or divergence; bounded checked runs are transactional as a whole.

Legacy `load`, `step`, `execute`, and public register inspection remain for
existing callers. Programs halt on `ta 0` (`0x91D0_2000`).

## Validation

```sh
cargo test --package coding-adventures-sparc-v8-gatelevel
```

The suite includes the original 42 unit tests, six lifecycle suites, and a
248-vector Python/functional full-state differential. Strict formatting,
Clippy, rustdoc, and at least 80% total line coverage are completion gates.
The normative contract is Spec 07r2.
