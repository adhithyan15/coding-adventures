# Cortex-M3 simulator

Checked Rust functional model of one Arm Cortex-M3 core, implementing the
Armv7-M Thumb instruction boundary defined by
[Spec 07al](../../../specs/07al-cortex-m3-simulator.md).

The model owns all integer and special-register state, IT state, two stack
pointers, the local exclusive monitor, 64 KiB of little-endian memory,
power/event state, and exact installed-image metadata. Loads, restore, step,
bounded run, and execute are typed and transactional. Successful traces retain
exact raw bytes, instruction width, mnemonic, and complete before/after
snapshots. Executed instructions use conventional mnemonics; a condition-
failed IT member uses the explicit synthetic `it-skip` marker.
Composed cores can use `try_get_state` to obtain the same complete snapshot
with typed allocation failure.

APSR writes honor the architectural two-bit MSR mask. `APSR_nzcvq` updates
N/Z/C/V/Q. Because Cortex-M3 does not have the DSP extension, `APSR_g` and
`APSR_nzcvqg` are rejected atomically along with mask zero and non-APSR mask
aliases. The source-compatible `msr` encoder emits `APSR_nzcvq`; the typed
`ApsrMask` and `msr_masked` helpers expose exact forms for tests and downstream
DSP cores without silently truncating integers.

The core is intentionally separate from NVIC, SysTick, debug, AHB-Lite timing,
and vendor peripherals. Those compose around this package in later gate and SoC
cells.

## Verification

The completion suite covers every primary 16-bit family plus Armv7-M IT,
CBZ/CBNZ, MOVW/MOVT, division, bounded immediate-offset wide scalar memory
over R0-R12/SP bases (with PC forms rejected), exclusive monitor, table
branch, barrier, and special-register families. It also covers flags, scalar
and multiple memory, branches, stack ordering, dual stack selection, privilege,
power hints, typed faults, and transactional lifecycle. Release tests, strict
Clippy, and rustdoc warnings pass. The current completion baseline is 92.22%
line coverage (1,457/1,580).
