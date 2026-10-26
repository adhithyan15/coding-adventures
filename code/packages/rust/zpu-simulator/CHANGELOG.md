# Changelog

## 0.1.0 — Unreleased

- Add the complete pinned ZPU4-small 256-byte decode and instruction-boundary
  semantics.
- Add complete stack, IDIM, interrupt/rearm, BREAK, internal-memory, external
  adapter, installed-image, trace, and transaction state.
- Add checked atomic load, restore, step, bounded run, execute, helper, and
  allocation-failure behavior.
- Add 256 isolated full-memory instruction boundaries and four multistep,
  external-bus, and interrupt scenarios produced by checksum-pinned upstream
  VHDL under GHDL, plus independent Rust and Python semantic consumers.
- Keep SoC, physical-netlist, transistor, timing, and SPICE claims outside the
  functional CPU boundary.
