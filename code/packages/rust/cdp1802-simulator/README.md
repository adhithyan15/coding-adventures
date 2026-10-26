# RCA CDP1802 simulator

This package is the checked instruction-level model of the RCA COSMAC
CDP1802AC/3 defined by [Spec 07bc](../../../specs/07bc-rca-cdp1802-simulator.md).
It owns the complete sixteen-register matrix, selectors, D/DF/IE/Q/T, external
inputs, installed-image metadata, reset interrupt-inhibit latch, IDL state, and
all 65,536 memory bytes.

```rust
use cdp1802_simulator::Cdp1802Simulator;

// LDI 42h; PHI R1; IDL
let mut cpu = Cdp1802Simulator::new();
cpu.load(&[0xf8, 0x42, 0xb1, 0x00], 0x0100)?;
let result = cpu.run(3)?;
assert!(result.stopped);
assert_eq!(result.final_state.registers[1], 0x4200);
# Ok::<(), cdp1802_simulator::Cdp1802Error>(())
```

All FN1441 Table 4 encodings are implemented except the documented unassigned
byte `0x68`. Trace names use the canonical aliases `SHRC`, `SHLC`, `SKP`, and
`LSKP`. Instruction operands are constrained to the installed image; ordinary
data, DMA, and the real but CPU-trace-abstracted long-skip read cycles use the
full wrapping address space.

Loads, restore, instruction steps, bounded runs, and load-and-run execution are
typed and transactional. Retained complete-state traces are capped at 64 and
checked execution preflights all fallible storage. DMA input/output and
interrupt acknowledge are explicit operations. The first-instruction reset
inhibit has precedence over IE, remains set through DMA and failed steps, and
clears only after a successful instruction, including IDL.

## Composition boundary

This crate is a CPU and bus-request oracle. It does not invent a VIP/ELF board,
ports, peripherals, arbitration, WAIT/CLEAR pin timing, clock rate, radiation
behavior, or transistor topology. A later SoC cell can consume the explicit
port, DMA, interrupt, and memory transactions; a separate educational gate
cell can differential-test against this functional authority.

## Verification

The completion suite independently classifies all 256 first bytes, executes
every assigned first byte, exhaustively checks every 256x256x2 add/subtract
tuple, covers every short/long condition and selector-alias partition, all
ports, DMA/interrupt/IDL lifecycle, typed fault rollback, and injected
allocation failure. Release tests, strict Clippy, rustdoc, BUILD integration,
lessons validation, and greater than 80% line coverage are publication gates.
The local completion baseline covers **87.76% of source lines (832/948)**
under the LLVM coverage engine.
