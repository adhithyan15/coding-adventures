# Cortex-M0 simulator

Checked Rust functional model of one Arm Cortex-M0 core, implementing the
Armv6-M Thumb instruction boundary defined by
[Spec 07ak](../../../specs/07ak-cortex-m0-simulator.md).

The model owns all integer and special-register state, two stack pointers,
64 KiB of little-endian memory, power/event state, and exact installed-image
metadata. Loads, restore, step, bounded run, and execute are typed and
transactional. Successful traces retain exact raw bytes, instruction width,
conventional mnemonic, and complete before/after snapshots.

The core is intentionally separate from NVIC, SysTick, debug, AHB-Lite timing,
and vendor peripherals. Those compose around this package in later gate and SoC
cells.

## Verification

The completion suite covers every primary 16-bit family, the supported 32-bit
BL/barrier/special-register families, flags, scalar and multiple memory,
branches, stack ordering, dual stack selection, privilege, power hints, typed
faults, and transactional lifecycle. Release tests, strict Clippy, and rustdoc
warnings pass. The completion baseline is 85.66% line coverage (950/1,109).
