# Layer 07al2 — Arm Cortex-M3 Gate-Level Simulator

## 1. Purpose and evidence boundary

`cortex-m3-gatelevel` is the educational architectural gate companion to the
checked `cortex-m3-simulator`. The normative public references remain Arm's
*Cortex-M3 Devices Generic User Guide* (DUI 0552A), the *Armv7-M Architecture
Reference Manual*, and the *Cortex-M3 Technical Reference Manual*.

This package models the functional cell's complete bounded instruction surface
while making every persistent architectural bit a repository D flip-flop and
deriving decode, arithmetic, Boolean, shift, comparison, address, condition,
and write-enable decisions through repository gate networks. It is not a
transistor layout, pipeline timing model, or claim about a particular MCU.
NVIC arbitration, exception entry/return, SysTick, MPU, debug, AHB timing,
bit-band bus translation, and vendor peripherals remain later SoC cells.

The functional package supplies public value types and a test oracle only.
Production gate transitions never instantiate or call the functional
simulator.

## 2. Fixed persistent topology

Every selected persistent bit is clocked through
`logic_gates::sequential::register`. A host-side set of nonzero memory byte
addresses may accelerate reset, but it is derived cache state and contributes
no architectural storage.

| Component | D flip-flops |
|---|---:|
| 65,536-byte memory | 524,288 |
| R0-R12 | 416 |
| MSP, PSP, LR, PC, APSR | 160 |
| IPSR and ITSTATE | 16 |
| PRIMASK, BASEPRI, FAULTMASK, CONTROL, Handler mode | 19 |
| Halt, sleep, WFE cause, and event latches | 4 |
| Exclusive monitor address and valid latch | 33 |
| Installed-image origin | 32 |
| Installed-image length, including 65,536 | 17 |
| **Total** | **524,985** |

The reset state, two stack pointers, complete status surface, power/event
state, exclusive monitor, full 64 KiB memory, and exact installed-image
metadata therefore remain observable DFF-backed state.

## 3. Gate transition engine

- Thumb width and primary-family classification are reconstructed locally from
  gate equality and masked-decode lines. Reserved neighbours fail atomically.
- Ripple add/subtract networks produce arithmetic values, carry, and overflow;
  Boolean gates produce logical results; mux stages implement shifts and
  rotates; and gate comparisons select branches, IT members, and address
  writeback.
- Little-endian fetch and data assembly, byte/halfword/word selection, stack
  and multiple-transfer addresses, wide immediate formation, table branches,
  and exclusive-monitor overlap are gate-derived before any storage clock.
- The package implements the complete functional cell surface: Cortex-M0
  compatible Thumb integer instructions plus IT, CBZ/CBNZ, MOVW/MOVT,
  SDIV/UDIV, bounded wide scalar memory, TBB/TBH, word LDREX/STREX, CLREX,
  BASEPRI, FAULTMASK, barriers, and status-register views.
- `BKPT #0` clocks the deterministic halt latch. Other BKPT and SVC encodings
  retain state and return typed boundary errors. WFE/WFI and signal methods
  update their fixed event/power latches through the same storage path.

The engine may use host integers for bounded control flow, indexing, and trace
bookkeeping. Architectural data-path results and selections must pass through
the fixed-width gate helpers before the candidate state can commit.

## 4. Checked lifecycle and allocation

Load and restore validate the complete input before clocking a DFF. `step`
executes against a fallibly allocated candidate snapshot, produces exact raw
bytes, width, conventional mnemonic (or `it-skip`), and complete before/after
states, then clocks the settled candidate. Every error is atomic.

`run` and `execute` preflight the functional cell's 64-trace retention limit,
reserve all outer and nested snapshot storage before the first architectural
transition, and roll back the whole operation on faults or step-limit
exhaustion. Direct register, memory, event, interrupt, reset, load, restore,
step, run, and execute APIs preserve the functional contract.

## 5. Completion gate

The cell is complete only when it:

1. pins the topology to exactly 524,985 DFFs;
2. proves production independence from `cortex-m3-simulator`;
3. exercises every primary 16-bit and 32-bit decode family and implemented
   arithmetic, flag, memory, control, IT, special-register, power, and
   exclusive subfamily through gate witnesses;
4. compares exact result/error, raw bytes, width, mnemonic, and complete
   before/after state with the checked functional oracle;
5. proves invalid restore/load/fetch/data/control/decode operations and
   post-halt/sleep transitions are typed and atomic, and proves bounded
   allocation/rollback behavior; and
6. passes release tests, package-scoped formatting, strict Clippy, rustdoc,
   package BUILD, lessons validation, affected-package checks, independent
   security review, and more than 80% line coverage with a target above 95%.
