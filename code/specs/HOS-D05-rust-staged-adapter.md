# Rust RV32I adapter for the D05 Core

## Purpose and boundary

This fixture-side test adapter connects the reusable Rust `core::Core` cycle
engine to the canonical D05 program using the existing RISC-V decoder and the
checked sequential `Rv32ISimulator`. It is not a package API for arbitrary
guest programs: Core's current memory callbacks do not return checked fetch
or data-access faults. The adapter and its Core/pipeline dependencies live
only in the package's integration-test build. This is native Rust execution
evidence, independent of the Go pipeline implementation and its recorded
cycle trace.

## Supported bounded program

Before execution, the test adapter validates that every instruction in the
canonical fixture program is one of `addi`, `sw`, `lw`, `beq`, or the exact
ECALL word `0x00000073`. Unsupported instructions and non-word-sized programs
are rejected. Decode and execute callbacks use the existing RISC-V instruction
decoder; forwarded operands are selected by source slot and register. Results
and comparisons use normalized unsigned 32-bit RV32 bit patterns.

The shared teaching fixture is the bounded evidence case: 256-byte flat RAM,
program bytes at address zero, a little-endian initial word `42` at byte 128,
and a 100-cycle ceiling. The adapter loads the program, initializes RAM using
the checked Core image API, advances the real IF/ID/EX/MEM/WB Core one cycle at
a time, and reports the cycle snapshots, forwarded operands, WB PCs, registers,
RAM bytes, and counters that actually resulted.

The test independently loads the same program and initial word into
`Rv32ISimulator`, runs within the fixture's instruction bound, and compares
architectural registers and final RAM. It asserts the expected WB PC sequence,
wrong-path PC exclusion, load-use stall, forwarding, taken redirect, and halt.

## Limits

This proves only the listed subset and fixture. The sequential oracle shares
the repository's existing RISC-V decode implementation, but uses a separate
architectural execution engine. The fixture's only data access is the aligned
word at byte 128, disjoint from the 40-byte program; it does not exercise
negative or out-of-range runtime addresses, code modification, or fetch past
the loaded image. No Go cycle equality, general RV32I pipeline support,
precise faults, store-at-retirement behavior, MMIO, guest OS boot, or browser
integration is claimed. In particular, the Core's existing store callback
updates flat RAM in MEM before WB.
