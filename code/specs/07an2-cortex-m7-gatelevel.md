# Layer 07an2 — Arm Cortex-M7 Gate-Level Simulator

## 1. Purpose and evidence boundary

`cortex-m7-gatelevel` is the educational architectural gate companion to the
checked `cortex-m7-simulator`. The normative public references remain Arm's
*Cortex-M7 Devices Generic User Guide* (DUI 0646), *Cortex-M7 Technical
Reference Manual* (DDI 0489), and the *Armv7-M Architecture Reference Manual*.

This package implements the complete bounded Spec 07an functional surface. It
reuses the completed Cortex-M3 gate engine for inherited architectural state
and instructions, and adds independently decoded Armv7E-M DSP operations and
GE storage through repository gate networks. Production transitions never
instantiate or call either functional simulator; functional packages provide
public value types and differential test oracles only.

This is an ISA-level teaching topology. It is not Arm's licensed Cortex-M7 RTL,
a transistor layout, a superscalar pipeline, a branch predictor, or a claim
about a particular MCU. Caches, TCM, AXI/AHB traffic, NVIC, MPU, FPU, debug,
timing, and vendor peripherals remain later gate/SoC cells.

## 2. Fixed persistent topology

Every selected persistent bit is clocked through
`logic_gates::sequential::register`. The inherited Cortex-M3 topology is used
without parallel host-field authority. Four new DFFs store APSR.GE[3:0].

| Component | D flip-flops |
|---|---:|
| Inherited complete Cortex-M3 gate state | 524,952 |
| APSR GE[3:0] | 4 |
| **Total** | **524,956** |

The total includes the full 64 KiB memory, all inherited registers, dual stack
pointers, status/mode/mask state, power/event latches, exclusive monitor,
halt, and installed-image metadata enumerated by Spec 07al2. GE bits are owned
only by the added register bank. Public snapshots and any compatibility fields
are observers derived from DFF Q outputs, never alternate architectural
storage.

Topology tests must derive this inventory from the production containers and
must deliberately corrupt every public observer seam to prove that reset,
restore, instruction execution, snapshots, and typed faults remain driven by
DFF Q. A hard-coded count or self-referential field comparison is insufficient.

## 3. Gate transition engine

The package delegates an instruction to the inherited Cortex-M3 gate engine
only after its own gate-derived Armv7E-M classification proves that it is not
one of the added DSP encodings. Reserved DSP neighbors and reserved operands
fail before condition/IT evaluation and never fall through as inherited
unknown instructions.

The added engine provides:

- gate equality/masking for `QADD`, `QSUB`, `SADD8`, and `UADD8` classification
  and operand-field validation;
- 32-bit ripple add/subtract, carry, and signed-overflow wires for QADD/QSUB,
  with gate muxes selecting `INT32_MIN`, `INT32_MAX`, or the ordinary result;
- a gate OR path that makes APSR.Q sticky on saturation and never clears it on
  a non-saturating operation;
- four independent eight-bit ripple lanes for SADD8/UADD8, with no cross-lane
  carry propagation;
- signed-lane GE outputs selected from each full-precision SADD8 sum's sign and
  unsigned-lane GE outputs selected from each UADD8 carry-out; and
- gate mux/write-enable paths for result register and all four GE DFFs.

All inherited decode, arithmetic, Boolean, shift, address, memory, condition,
write-enable, power, exclusive, status, and lifecycle decisions remain those
of the Cortex-M3 gate package. Architectural data results and selections must
pass through fixed-width repository gate helpers before candidate state can
commit. Host integers are limited to bounded control flow, indexes, validated
address conversion, and trace bookkeeping.

## 4. Status transport and exact compatibility

APSR-family `MRS` and `MSR` views transport GE through the added DFF bank while
preserving the inherited N/Z/C/V/Q behavior and privilege checks. Reset and a
successful load clear GE. Restore rejects values above four bits before any
DFF clocks. Ordinary inherited instructions preserve GE; QADD/QSUB preserve
GE while updating sticky Q; SADD8/UADD8 replace all four GE bits while
preserving inherited flags exactly as Spec 07an requires.

Every instruction and lifecycle operation must match `cortex-m7-simulator` in
success/error variant, exact fetched bytes, width, mnemonic, condition/IT
outcome, and complete before/after snapshot. The full inherited Cortex-M3
surface is part of that differential contract, not merely a smoke test around
the four added operations.

## 5. Checked lifecycle and allocation

Load and restore validate the complete input before clocking any DFF. `step`
executes against fallibly allocated candidate DFF state, captures instruction
bytes before any self-modifying write, produces a complete trace, and clocks
the settled candidate only after every check succeeds. Unknown encodings,
reserved DSP operands, invalid restore/load/fetch/data/control operations,
halted/sleeping state, SVC, and non-sentinel BKPT preserve every DFF.

`run` and `execute` retain the functional cell's maximum of 64 complete-state
traces. They preflight all outer and nested snapshot allocations before the
first transition and roll back the whole invocation on a fault or exhausted
step budget. Deterministic allocation-failure seams must cover inherited and
new state, traces, differential conversion, and rollback without adding
production-only shortcuts.

## 6. Explicit exclusions and follow-up backlog

The exclusions are exactly the functional cell's: the remaining Armv7E-M DSP
surface, optional single/double-precision FPU, exception entry/return, NVIC,
MPU, debug/trace, caches, predictor, ITCM/DTCM, write buffers, AXI/AHB buses,
arbitration, coherency, stalls, cycle timing, clocks, reset domains, DMA, and
peripherals. A later ISA-completion cell may expand both functional and gate
packages together. A later SoC cell may compose this fixed core with caches,
TCM, interconnect, interrupts, DMA, and devices without relabeling those
structures as current CPU state.

## 7. Completion gate

The gate cell is complete only when it:

1. derives and proves exactly 524,956 authoritative DFFs, including deliberate
   observer-divergence tests for inherited and GE state;
2. proves production independence from both functional simulators and prevents
   DSP encodings/reserved neighbors from escaping into inherited decode;
3. provides gate witnesses for positive/negative QADD/QSUB saturation,
   nonsaturation, sticky Q, every SADD8/UADD8 lane and GE outcome, write
   suppression, IT conditions, and APSR GE transport;
4. exercises every inherited primary and subfamily through the reused M3 gate
   engine and compares the complete Spec 07an result/error/trace/state contract
   with the checked Cortex-M7 oracle;
5. proves atomic invalid restore/load/fetch/data/control/decode, post-halt and
   sleep behavior, retention, allocation failure, whole-run rollback, and
   exact pre-write fetched bytes; and
6. passes release tests, package-scoped formatting, strict Clippy, rustdoc,
   package BUILD, lessons validation, affected-package checks, latest-main
   ancestry, independent exact-head review, and more than 80% line coverage.
