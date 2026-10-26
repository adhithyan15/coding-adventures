# Cortex-M4 simulator

Checked Rust functional model of one Arm Cortex-M4 core, implementing the
bounded Armv7E-M boundary in
[Spec 07am](../../../specs/07am-cortex-m4-simulator.md).

The package layers Cortex-M4 DSP and optional FPv4-SP behavior over the
complete Cortex-M3 architectural oracle. State snapshots therefore include
the whole Cortex-M3 core, APSR GE flags, optional-FPU enablement, S0-S31, and
FPSCR. Loads, restore, step, bounded run, and execute are typed and
transactional. Successful traces retain raw bytes, width, mnemonic, and
complete before/after state.

The DSP boundary includes QADD/QSUB, SSAT/USAT, SADD8/UADD8, SMUAD, and SMLAD.
The explicitly optional FPU boundary includes scalar binary32 arithmetic,
move/absolute/negate/square-root, compare, and FPSCR transport. The unit is
disabled after reset until the composing platform enables it. NVIC, SysTick,
MPU, exception stacking, caches, bus timing, and vendor peripherals compose
around this package in later gate and SoC cells.

Binary32 exception status is derived from exact dyadic significands and
exponents, so wide exponent gaps and gradual underflow cannot disappear in an
intermediate host `f64`. Quiet NaN comparisons produce unordered NZCV without
invalid; signaling NaNs additionally set the cumulative invalid flag.

## Verification

The 27-test completion suite covers inherited Cortex-M3 execution, every
bounded DSP and FPU family, configuration and status transport, complete state
restore, typed atomic faults including final-snapshot allocation rollback,
inherited-candidate allocation rollback, exact binary32 status, bounded
rollback, APSR.GE system-register round trips, pre-IT operand validation, and
exact traces. Release tests, strict Clippy, and rustdoc warnings pass. The
completion baseline is updated to 92.33 percent line coverage (987/1069) from
the measured release run.
