# D00 — Deep CPU Architecture Overview

## Overview

This document describes the **deep CPU internals** — the micro-architectural
components that live inside a modern processor core. Where the existing
`cpu-simulator` (Layer 8) models a simple fetch-decode-execute cycle, these
packages model what really happens inside that cycle: pipelined execution,
branch prediction, hazard detection, data forwarding, cache hierarchies, and
the composition of all these into a configurable core.

The key design principle: **ISA-independent micro-architecture.** Just as ARM
Holdings licenses the ARM instruction set and then Apple, Qualcomm, and Samsung
each build their own wildly different core designs around it, our packages
separate the "what to execute" (ISA decoder) from the "how to execute it"
(core micro-architecture). You can plug any ISA decoder into any core
configuration.

## Why this matters

A CPU instruction set (ARM, RISC-V, x86) defines the *contract* between
software and hardware — what instructions exist, what they do, how memory is
addressed. But the instruction set says nothing about *how fast* those
instructions execute. That is the job of the **micro-architecture**.

Two processors can implement the exact same ISA yet perform vastly differently:

```
Same ISA (ARMv9), different micro-architectures:

ARM Cortex-A510 (efficiency core)    ARM Cortex-X4 (performance core)
├── 5-stage in-order pipeline         ├── 10+ stage out-of-order pipeline
├── Simple branch predictor           ├── TAGE branch predictor
├── 32KB L1 cache                     ├── 64KB L1 cache
├── No rename registers               ├── 192 physical registers
└── ~1 IPC                            └── ~4+ IPC (superscalar)
```

The Cortex-A510 executes one instruction at a time, in order, with minimal
speculation. The Cortex-X4 executes multiple instructions per cycle, out of
order, with aggressive branch prediction. Same instruction set. Completely
different performance characteristics.

Our packages let you build both — and anything in between.

## Package Composition

```
Core (configurable micro-architecture)
│
├── Pipeline (D04)              — configurable N-stage pipeline
│   ├── Stage 1: Instruction Fetch (IF)
│   ├── Stage 2: Instruction Decode (ID)
│   ├── Stage 3: Execute (EX)
│   ├── Stage 4: Memory Access (MEM)
│   ├── Stage 5: Write Back (WB)
│   └── ... (configurable: 5 to 20+ stages)
│
├── Branch Predictor (D02)      — pluggable prediction algorithm
│   ├── Static (always taken / always not-taken)
│   ├── 1-bit predictor
│   ├── 2-bit saturating counter
│   ├── Branch Target Buffer (BTB)
│   └── TAGE (future: what modern CPUs use)
│
├── Hazard Detection Unit (D03) — detects data/control/structural hazards
│
├── Forwarding Unit (D03)       — bypasses data to avoid stalls
│
├── Register File               — configurable width + count
│   └── (from existing cpu-simulator, extended)
│
├── FP Unit                     — floating-point execution
│   └── (from fp-arithmetic package)
│
├── L1I Cache (D01) ──┐
├── L1D Cache (D01)   ├── all from cache package, different configs
└── L2 Cache  (D01) ──┘

ISA Decoder (pluggable — not part of core)
├── ARM decoder   (from arm-simulator)
├── RISC-V decoder (from riscv-simulator)
└── Custom decoder (user-defined)
```

### Multi-Core Composition

```
Multi-Core CPU (D05)
│
├── Core 0 ─── L1I + L1D ─── Private L2
├── Core 1 ─── L1I + L1D ─── Private L2
├── Core 2 ─── L1I + L1D ─── Private L2
├── Core 3 ─── L1I + L1D ─── Private L2
│
├── Shared L3 Cache
├── Memory Controller ─── DRAM (main memory)
└── Interrupt Controller
```

## The ARM analogy

This design directly mirrors the real semiconductor industry:

1. **ARM Holdings** designs the **instruction set** (ARMv9). This is a
   specification — a document describing every instruction, its encoding,
   its semantics. ARM does not build chips.

2. **Apple** takes the ARM ISA and builds a custom core around it (M4's
   "Everest" performance core). Apple chooses the pipeline depth, predictor
   algorithm, cache sizes, execution width — everything. The result is a
   core that runs ARM instructions but is completely Apple's design.

3. **Qualcomm** takes the same ARM ISA and builds a different core (Oryon).
   Different pipeline, different predictor, different caches. Same instructions,
   different performance.

4. **Samsung** does the same (Cortex-X series), and so does **MediaTek**.

In our system:

| Real world             | Our packages                                    |
|------------------------|-------------------------------------------------|
| ARM ISA specification  | `arm-simulator` decoder (or `riscv-simulator`)  |
| Apple M4 core design   | `Core(pipeline=13, predictor=TAGE, l1=64KB)`    |
| Qualcomm Oryon design  | `Core(pipeline=10, predictor=TwoBit, l1=32KB)`  |
| A simple teaching core | `Core(pipeline=5, predictor=Static, l1=4KB)`    |

## Layer Position

```
Existing stack:

Logic Gates (10) → Arithmetic (9) → FP Arithmetic (FP01) → CPU Simulator (8) → ISA Simulators (7)

Deep CPU packages slot between FP Arithmetic and the ISA Simulators:

Logic Gates (10) → Arithmetic (9) → FP Arithmetic (FP01)
                                         │
                                    Deep CPU Internals
                                    ├── Cache (D01)
                                    ├── Branch Predictor (D02)
                                    ├── Hazard Detection (D03)
                                    ├── Pipeline (D04)
                                    └── Core (D05) ← composes everything
                                         │
                                    ISA Simulators (7)
                                    ├── ARM
                                    ├── RISC-V
                                    └── Custom
```

## Dependencies Between Deep CPU Packages

```
D01 (Cache)              — depends on: clock
D02 (Branch Predictor)   — depends on: clock
D03 (Hazard Detection)   — depends on: nothing (pure logic)
D04 (Pipeline)           — depends on: clock, D02, D03
D05 (Core)               — depends on: D01, D02, D03, D04, fp-arithmetic, clock
```

All packages depend on the `clock` package for cycle-accurate simulation.

## Spec Numbering

| Spec | Package | Description |
|------|---------|-------------|
| D00 | — | This architecture overview |
| D01 | `cache` | Cache hierarchy (L1/L2/L3, set-associative, LRU) |
| D02 | `branch-predictor` | Branch prediction (static, 1-bit, 2-bit, BTB) |
| D03 | `hazard-detection` | Hazard detection + data forwarding |
| D04 | `pipeline` | Configurable N-stage instruction pipeline |
| D05 | `core` | Core composition + multi-core CPU |

## Implementation Languages

Each package will be implemented in Python, Ruby, and Go — matching the
existing pattern in the repo. Python is the primary implementation, Ruby and Go
follow with equivalent functionality.

## Future Extensions

- **Out-of-order execution**: reorder buffer, reservation stations, register renaming
- **Superscalar execution**: multiple instructions per cycle
- **Speculative execution**: execute past branches before knowing the outcome
- **Memory ordering**: store buffer, load-store queue
- **NUMA**: non-uniform memory access for multi-socket systems
- **Power modeling**: estimate power consumption based on utilization
