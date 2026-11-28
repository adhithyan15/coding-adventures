# Rust Core bounded RAM image setup and observation

## Purpose

The Rust `core` crate owns a flat byte-addressed `MemoryController`, but its
existing public `load_program`, `read_word`, and `write_word` APIs silently
ignore out-of-range accesses. They are retained for compatibility. This slice
adds a checked opt-in way to initialize data ranges and copy bounded bytes out
for test and simulator observation.

## Contract

- `MemorySegment` describes one byte range by starting address and borrowed
  contents. Addresses and lengths are byte-based; no alignment is required.
- A batch initializer validates every segment against the controller's fixed
  memory size before changing any byte. Any out-of-bounds segment rejects the
  whole batch without partial writes.
- Nonempty segments in one batch may not overlap. Overlap is rejected before
  mutation so segment order cannot silently choose a value. Empty ranges are
  no-ops, and their start address may equal the end of RAM.
- Checked byte reads use the same subtraction-based bounds rule and return a
  copy. Changing the returned vector cannot mutate simulated RAM.
- The `Core` wrapper operates only on that Core's own backing RAM. Seeding or
  reading memory does not load a program, redirect PC, reset registers, change
  pipeline state, or alter run behavior. Existing APIs retain their behavior.
- This is a flat-RAM setup/observation API only. It does not define MMIO,
  precise faults, instruction/data permissions, memory alignment, or shared
  multi-core memory.

## Verification

Tests cover valid and end-of-RAM ranges, copied reads, overlapping ranges, and
atomic rejection of a batch containing both a valid range and an invalid
range. Core tests load a small program at byte zero, seed the disjoint
little-endian word `42` at byte address `128`, and confirm program bytes and
Core execution state are unchanged by the data image operation.
