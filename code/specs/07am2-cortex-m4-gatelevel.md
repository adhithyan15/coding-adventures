# Layer 07am2 — Arm Cortex-M4 Gate-Level Simulator

## 1. Purpose and evidence boundary

`cortex-m4-gatelevel` is the educational architectural gate companion to the
checked `cortex-m4-simulator`. The public evidence remains Arm's *Cortex-M4
Devices Generic User Guide* (DUI 0553), *Cortex-M4 Technical Reference Manual*
(DDI 0439), and the *Armv7-M Architecture Reference Manual*.

The package covers the complete bounded functional cell: the inherited
Cortex-M3 Thumb/Armv7-M surface, Cortex-M4 DSP state and instructions, and the
configuration-pinned FPv4-SP surface. It is not a transistor layout, pipeline
timing model, or model of a vendor microcontroller. Exception entry/return,
NVIC arbitration, SysTick, MPU, debug/trace, cache and bus timing, vendor
peripherals, and implementation timing remain later SoC cells.

The production package may reuse `cortex-m3-gatelevel` as its physical base,
but it must not depend on or call `cortex-m4-simulator`. The functional package
is a dev-only differential oracle. Cortex-M4 extension decode is reconstructed
independently from instruction bits; production code may not route an oracle
classification or transition into the gate machine.

## 2. Fixed persistent topology

Every architecturally persistent bit is clocked through
`logic_gates::sequential::register`. The inherited Cortex-M3 storage remains
owned by its accepted 524,952-DFF gate machine. Cortex-M4 adds only the state
that is absent from that base.

| Component | D flip-flops |
|---|---:|
| Inherited Cortex-M3 architectural machine | 524,952 |
| APSR GE[3:0] | 4 |
| FPU-enabled configuration latch | 1 |
| S0-S31 binary32 registers | 1,024 |
| FPSCR | 32 |
| **Total** | **526,013** |

The complete reset state, 64 KiB memory, integer/DSP/FPU registers, status,
privilege, power/event state, exclusive monitor, halt, and exact installed
image metadata are therefore observable stable-Q state. Host collections may
cache derived indexes, but they are not architectural authority and are
excluded from the inventory.

## 3. Gate transition engine

- The inherited instruction surface is executed by `cortex-m3-gatelevel`, not
  by a functional adapter. Extension-versus-inherited routing comes from an
  independent masked gate decoder, including reserved-neighbour refusals.
- Ripple add/subtract, sign/zero extension, saturation comparators, muxes, and
  Boolean reduction networks produce QADD/QSUB, SSAT/USAT, SADD8/UADD8,
  SMUAD, and SMLAD results plus APSR.Q and GE updates.
- FPv4-SP sign, exponent, significand, normalization, rounding, exception, and
  comparison paths are fixed-width gate networks. Host floating-point
  arithmetic is forbidden in the production engine. Canonical NaN behavior,
  signaling-versus-quiet comparison, round-to-nearest ties-to-even, cumulative
  IOC/DZC/OFC/UFC/IXC/IDC, and FPSCR.NZCV match the functional boundary.
- VMOV/VABS/VNEG, VADD/VSUB/VMUL/VDIV/VSQRT, VCMP, VMRS, and VMSR preserve the
  exact functional raw bytes, width, conventional mnemonic, and state effects.
  Unsupported FPSCR control modes and disabled-FPU execution are typed and
  atomic.
- Host integers are permitted only for bounded indexing, loop counters, and
  trace bookkeeping. An architecturally visible result, flag, predicate,
  register select, or write-enable must be derived by a gate/mux network before
  any storage clocks.

## 4. Checked lifecycle and allocation

Reset, load, restore, direct integer/FPU register and memory access, event and
interrupt signals, `step`, `run`, and `execute` preserve the functional API and
checked state boundary. Inputs are validated before any DFF clocks. `step`
settles a candidate, constructs a complete trace fallibly, and commits only
after every validation and allocation succeeds.

`run` and `execute` enforce the functional cell's 64-trace and 8 MiB retention
limits. They preflight outer and nested snapshot storage before the first
transition and restore the whole entry state on faults, step-limit exhaustion,
or deterministic allocation-seam failure. Fetch, data, decode, control,
disabled-FPU, FPSCR, lifecycle, and allocation errors are typed and preserve
every architectural bit.

## 5. Completion gate

The cell is complete only when it:

1. pins the topology to exactly 526,013 DFFs and rejects hidden host-authority
   storage with compiler-checked, full-crate parsed-syntax hostile fixtures;
2. source-locks production independence from `cortex-m4-simulator` and the
   accepted `cortex-m3-gatelevel` machine as the only inherited executor. The
   structural gate must compile and reject hostile mutations that introduce
   oracle/functional routing, an alternate or duplicated inherited executor,
   raw host decode or dispatch, host floating arithmetic/conversion, host DSP
   result/flag/predicate/write-enable decisions, hidden or post-commit
   architectural mirrors, and dead or duplicate shadow helpers;
3. exercises every inherited primary decode family and every bounded DSP/FPU
   family, reserved neighbour, flag path, and configuration state;
4. differentially compares exact result/error, fetched bytes, width, mnemonic,
   and complete before/after state with `cortex-m4-simulator`, including
   inherited integer/privilege/IT/memory/exclusive/branch/power behavior;
5. proves invalid restore/load/fetch/data/control/decode/FPU operations,
   post-halt/sleep behavior, retention bounds, and injected allocation failures
   are typed and atomic; and
6. passes release tests, package-scoped formatting, strict Clippy, rustdoc,
   package BUILD, lessons validation, affected-package checks, independent
   specification and implementation review, and more than 80% line coverage
   with a target above 95%.

## 6. Local implementation evidence

The bounded package is implemented locally with the accepted Cortex-M3 gate
machine as its sole inherited executor and an exact 526,013-DFF inventory.
Twelve behavioral tests cover all inherited primary classifications, complete
inherited/extension state preservation, every bounded DSP/FPU family, reserved
neighbours, binary32 edge cases, and typed atomic lifecycle rollback. Two unit
allocation seams prove candidate and final-snapshot failures roll back. Two
full-source parsed-syntax tests exact-lock production and reject compile-valid
hostile executor, oracle, raw-decode, host-DSP, host-float, mirror, and shadow
mutations. The measured baseline is 90.86% line coverage (1,590/1,750); final
strict checks and independent implementation review remain publication gates.
