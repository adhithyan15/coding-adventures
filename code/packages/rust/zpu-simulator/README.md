# Zylin ZPU4-small simulator

This package implements the checked `Zpu4Small32x15V1` instruction-boundary
profile defined by [Spec 07bf](../../../specs/07bf-zylin-zpu4-small-simulator.md).
It owns complete 15-bit PC/SP, IDIM, interrupt/rearm, sticky BREAK, 32 KiB
internal RAM, deterministic external-word-bus adapter, and installed-image
state.

```rust
use zpu_simulator::ZpuSimulator;

let mut cpu = ZpuSimulator::new();
cpu.load(&[0x81, 0x82, 0x05, 0x00], 0)?; // IM 1; IM continuation; ADD; BREAK
let result = cpu.run(4)?;
assert!(result.break_asserted);
# Ok::<(), zpu_simulator::ZpuError>(())
```

All 256 bytes are legal in the pinned small core. Bytes `0x20..=0x3f` enter
the software EMULATE vectors, while `0x01`, `0x03`, `0x0e`, and `0x0f` are
configuration-specific NOP aliases. Programs and words are big-endian. ISA
LOAD/STORE discard alignment and high address bits exactly at the VHDL port;
checked host helpers reject malformed addresses instead.

The external adapter and every transaction are explicit so a later SoC cell
can replace them with decoded devices, arbitration, wait states, DMA, and an
interrupt controller. This package does not claim cycle timing, an FPGA
layout, a physical netlist, transistor topology, or SPICE behavior.

## Verification

Release tests exhaustively classify all 256 bytes and cover each parameter
family, short operation, stack wrap/alias boundary, internal/external access,
interrupt/IDIM/rearm transition, lifecycle fault, and deterministic allocation
rollback. The checked-in GHDL corpus contains 256 isolated full-memory
instruction boundaries plus external LOAD/STORE, continued-IM, and deferred
interrupt programs. Rust consumes every row against the production simulator;
an independent Python witness separately checks the schema and semantics.

Ordinary tests are offline. To regenerate the corpus, install GHDL 6.0.0 or a
compatible VHDL-2008 build and run from `code/packages/rust`:

```sh
python3 zpu-simulator/tools/regenerate_oracle_vectors.py \
  --ghdl /absolute/path/to/ghdl --check
```

The command downloads all six files listed in `oracle/sources.json` into a
disposable directory, refuses a checksum mismatch, makes only the pinned
`DontCareValue='0'` substitution, compiles the exact core with the checked-in
dual-port RAM/adapter harness, uses an explicit non-stopping assertion level
for the source's intentional BREAK diagnostic, and requires the final PASS
sentinel and zero process status. Harness failures use `std.env.stop(1)` or a
failure diagnostic that the runner rejects. No upstream source is checked in.

`ZpuSimulator::try_new` is the checked construction path; `new`/`Default` are
the ordinary infallible convenience boundary. Construction and every later
allocation in checked load, snapshot, step, run, and execute paths use
deterministic failure seams, including snapshot, trace reservation, and
post-progress rollback tests.

Measured release coverage is 88.82% of package source lines (548/617) and
89.16% of regions (790/886).
