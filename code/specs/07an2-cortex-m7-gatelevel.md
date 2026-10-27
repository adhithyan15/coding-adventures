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

Before this package can be implemented, `cortex-m3-gatelevel` must expose a
narrow gate-to-gate integration module. Its physical-storage inventory entry
point must derive named bit counts from the lengths of the production DFF
containers. Returning `CORTEX_M3_DFF_COUNT`, or summing constants that are not
checked against those containers, is not admissible evidence. The integration
module is for repository gate companions, not an additional application-level
CPU API, and it must not expose mutable access to the authoritative DFFs.

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

The inherited package's gate-to-gate integration module must provide all of
the following before M7 implementation starts:

- fallible snapshot into caller-preallocated storage, without an intermediate
  infallible full-memory clone;
- validated restore into a candidate and an explicit fallible commit
  operation, so the M7 wrapper can settle inherited and GE candidates before
  either clocks; because compatibility keeps candidate value mutation public,
  commit must revalidate the complete inherited state immediately before the
  first DFF clock;
- a transactional delegated-step operation that writes an exact trace into
  caller-preallocated storage and leaves the inherited DFFs untouched on every
  error;
- preflight sizing for every nested snapshot/trace buffer used by step, run,
  execute, and public read/write helpers; and
- the same deterministic allocation-failure controller used by the production
  fallible allocation adapter, plus the derived storage-inventory hook defined
  above.

These capabilities may be grouped into opaque scratch/candidate types, but
they must be callable across the crate boundary. Copying the M3 engine,
maintaining a second M3 state, or relying on its current infallible public
`get_state` path is forbidden.

## 4. Status transport and exact compatibility

APSR-family `MRS` views read GE from the added DFF bank while preserving the
inherited N/Z/C/V/Q behavior and privilege checks. APSR-family `MSR` honours
the Armv7-M mask field (bits 11:10) as Spec 07an1 requires of the functional
core: `APSR_nzcvq` (`10`) runs through the inherited engine and preserves GE;
`APSR_g` (`01`) and `APSR_nzcvqg` (`11`) are refused by the non-DSP inherited
engine and therefore execute in this crate, where gate-decoded `nzcvq` and
`g` wires, each ANDed with the IT write enable, select the NZCVQ and GE muxes
independently. Mask `00`, GE masks on non-APSR `SYSm`, and SP/LR/PC sources
fault atomically with the oracle's error. Reset and a
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
the settled candidate only after every check succeeds. Final inherited commit
revalidates even a previously settled candidate, closing the public mutable
candidate seam. Unknown encodings, reserved DSP operands, invalid
restore/load/fetch/data/control operations, hostile post-settle mutations,
halted/sleeping state, SVC, and non-sentinel BKPT preserve every DFF.

`run` and `execute` retain the functional cell's maximum of 64 complete-state
traces. They preflight all outer and nested snapshot allocations before the
first transition and roll back the whole invocation on a fault or exhausted
step budget. Deterministic allocation-failure seams must cover inherited and
new state, traces, differential conversion, and rollback without adding
production-only shortcuts.

The DSP network must also have a compiled-definition-bound structural test
surface. Production decode, add/subtract, saturation, lane, GE, Q, IT, and
write-enable helpers run through one gate-backend interface. Normal builds bind
that interface to repository gates. Tests bind the identical compiled helpers
to a recording/hostile backend that can invert one selected primitive output.
For every named path, at least one hostile fixture must change the architectural
result or write enable. A gate-call counter, source-text search, identity gate
anchor, or an oracle that duplicates the production predicate is insufficient.

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
3. binds production DSP helpers to the repository gate backend and proves with
   recording/hostile-backend fixtures that decode, QADD/QSUB result,
   saturation, sticky Q, SADD8/UADD8 lane result, carry/sign-to-GE, IT write
   enable, destination write enable, and GE transport depend on those gates;
4. exercises every inherited primary and subfamily through the reused M3 gate
   engine and compares the complete Spec 07an result/error/trace/state contract
   with the checked Cortex-M7 oracle;
5. independently exhausts all 65,536 ordered byte-operand pairs for signed and
   unsigned lane sum/result/GE, all 4,096 `Rd/Rn/Rm` field triples for each DSP
   encoding including every reserved-PC neighbor, and reduced-width exhaustive
   add/sub carry, sign, overflow, saturation, and sticky-Q truth quadrants;
6. proves atomic invalid restore/load/fetch/data/control/decode, post-halt and
   sleep behavior, retention, allocation failure, whole-run rollback, and
   exact pre-write fetched bytes; and
7. proves inherited lifecycle composition through the narrow M3 integration
   module, including preallocated snapshots, derived physical inventory,
   candidate commit, delegated traces, and deterministic failure at every
   outer and nested allocation site; and
8. passes release tests, package-scoped formatting, strict Clippy, rustdoc,
   package BUILD, lessons validation, affected-package checks, latest-main
   ancestry, independent exact-head review, and more than 80% line coverage.
