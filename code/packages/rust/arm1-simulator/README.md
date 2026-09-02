# arm1-simulator

Complete Rust behavioral simulator for ARM1 / ARMv1 (1985), the first ARM
processor.

The implementation covers all sixteen data-processing operations, every
condition code, immediate and register barrel shifts, single and block data
transfer modes, branches and links, software/undefined traps, force-user block
transfers, four processor modes with all 27 physical banked registers, and
external IRQ/FIQ entry.

## Machine model

- 32-bit fixed-width little-endian instructions
- 26-bit, 64 MiB architectural address space via `ARM1::architectural()`
- 16 visible registers and 27 physical registers across USR/FIQ/IRQ/SVC
- combined PC, NZCV/IF status, and mode in R15
- ARM1 pipeline-visible PC+8 register reads
- repository HALT convention: `SWI 0x123456`

`ARM1::new(memory_size)` remains available for bounded tests and compatibility.

## Checked lifecycle

```rust
use arm1_simulator::{encode_halt, encode_mov_imm, ARM1, COND_AL};

let words = [encode_mov_imm(COND_AL, 0, 42), encode_halt()];
let code: Vec<u8> = words.iter().flat_map(|word| word.to_le_bytes()).collect();
let mut cpu = ARM1::new(4096);
let result = cpu.run_checked(&code, 100)?;
assert!(result.halted);
assert_eq!(result.final_state.regs[0], 42);
# Ok::<(), arm1_simulator::Arm1Error>(())
```

`Arm1State` owns all 27 physical registers, every memory byte, halt state, and
the installed-program range. `load_checked`, `restore`, `step_checked`,
`run_loaded_checked`, and `run_checked` reject failures without partial state
changes and return complete before/after snapshots. Checked register and memory
helpers replace the legacy panic/silent-boundary behavior for new callers.

## Conformance

The Python oracle generator covers 599 deterministic one-step vectors across
all sixteen operations, sixteen conditions, immediate/register shifts, every
single/block-transfer control combination, branches, SWI, HALT, and undefined
entry. Each Rust transition compares all 27 physical registers, every memory
byte, and halt state.

Validation includes 51 unit tests, nine lifecycle tests, one aggregate Python
full-state differential, the ARM1 gate-level consumer, strict formatting,
Clippy, and rustdoc. Rust line coverage is 92.29% (1,675/1,815). See
`code/specs/07e-arm1-simulator.md` for the normative architecture and lifecycle
contract.

## Development

```bash
bash BUILD
```
