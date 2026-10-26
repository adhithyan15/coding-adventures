# Xtensa LX6 simulator

Checked Rust functional model of the bounded ESP32-class Xtensa LX6 integer
profile defined by [Spec 07az](../../../specs/07az-xtensa-lx6-simulator.md).

The model owns all 64 physical address registers, the logical register-window
mapping, `PC`, `SAR`, `WindowBase`, `WindowStart`, `CALLINC`, zero-overhead loop
registers, 64 KiB of little-endian memory, halt state, and exact installed-image
metadata. Loads, restore, step, bounded run, and execute are typed and
transactional. Successful traces retain exact fetched bytes, instruction
width, conventional mnemonic, optional data access, and complete before/after
snapshots.

The fixed `Esp32Lx6IntegerV1` profile covers the public Core Architecture,
Code Density, Loop, and application-visible Windowed Register instructions
admitted by Spec 07az. Deferred standard options are reported as typed
unsupported instructions, privileged handler instructions are recognized at
their boundary, and proprietary/configuration-unknown extensions are never
invented. This package is neither a cycle-accurate ESP32 nor a claim about the
proprietary LX6 gate topology.

## Verification

The completion suite generates classification cases across every admitted
major family and exercises mixed-width transport, arithmetic, shifts, memory,
branches, calls, windows, loops, special registers, narrow encodings, exact
trace bytes, typed fault precedence, and whole-state rollback. All 29 release
tests, strict Clippy, and rustdoc warnings pass. The completion baseline is
89.96% line coverage (1,380/1,534).
