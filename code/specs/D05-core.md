# D05 — Core (Integration Point)

## Overview

The Core package is the integration point — it composes a pipeline, branch
predictor, hazard detection unit, forwarding unit, register file, FP unit, and
L1/L2 caches into a single working processor core. It also provides
`MultiCoreCPU`, which connects multiple cores to a shared L3 cache, memory
controller, and interrupt controller.

A Core does not define any new micro-architectural behavior. Instead, it wires
together the components defined in D01-D04 and provides the configuration
system that makes them pluggable. The Core is to micro-architecture what a
motherboard is to a desktop PC — it connects the parts.

The ISA decoder is injected into the Core from the outside. The Core knows how
to move instructions through a pipeline, predict branches, detect hazards, and
access caches — but it does not know what any instruction *means*. That
separation is what makes the same core usable with ARM, RISC-V, or any custom
instruction set.

## Layer Position

```
ISA Simulators (Layer 7)
├── ARM decoder
├── RISC-V decoder
└── Custom decoder
        │
        │ (ISA decoder injected here)
        ▼
Core (D05) ← YOU ARE HERE
├── Pipeline (D04)
├── Branch Predictor (D02)
├── Hazard Detection + Forwarding (D03)
├── Cache Hierarchy (D01)
├── Register File (from cpu-simulator, extended)
├── FP Unit (from fp-arithmetic)
└── Clock (from clock package)

Multi-Core CPU (also D05)
├── Core 0 ... Core N
├── Shared L3 Cache (D01)
├── Memory Controller
└── Interrupt Controller
```

**Depends on:** `clock`, `cache` (D01), `branch-predictor` (D02),
`hazard-detection` (D03), `pipeline` (D04), `fp-arithmetic`, `cpu-simulator`
(register file)
**Used by:** ISA simulators (ARM, RISC-V, etc.)

## Key Concepts

### Core = Composition, Not New Logic

Think of the Core like a car. The engine, transmission, brakes, and steering
are all separate systems with their own specs. The "car" is the specific
combination: a V6 engine + 8-speed automatic + 4-wheel disc brakes. The Core
is the same idea:

```
Core "SimpleRISC":
├── Pipeline:         5-stage (IF, ID, EX, MEM, WB)
├── Branch Predictor: StaticPredictor(strategy="btfn")
├── Hazard Detection: enabled (with forwarding)
├── Register File:    16 registers, 32-bit
├── FP Unit:          None (integer only)
├── L1I Cache:        4 KB, direct-mapped, 1-cycle
├── L1D Cache:        4 KB, direct-mapped, 1-cycle
├── L2 Cache:         None
└── ISA Decoder:      (provided externally)
```

Every parameter above is configurable. Change the branch predictor from Static
to TwoBit and you get measurably different performance on branch-heavy code.
Double the L1 cache and you get fewer misses on data-heavy code. Deepen the
pipeline to 10 stages and you get higher clock speed but worse misprediction
penalty. These are exactly the tradeoffs real CPU designers navigate.

### ISA Injection: The Decoder + Executor Protocol

The Core does not know what instructions look like or what they do. It
delegates that to an **ISA decoder** that implements a simple protocol:

```python
class ISADecoder(Protocol):
    """Protocol that any ISA decoder must implement."""

    def decode(self, raw_instruction: int, token: PipelineToken) -> PipelineToken:
        """
        Decode a raw instruction into a PipelineToken.

        Fill in: opcode, rs1, rs2, rd, immediate, control signals
        (reg_write, mem_read, mem_write, is_branch, is_halt).
        """
        ...

    def execute(self, token: PipelineToken, registers: RegisterFile) -> PipelineToken:
        """
        Execute the decoded instruction.

        Compute: alu_result, branch_taken, branch_target.
        For memory instructions: compute effective address.
        """
        ...

    @property
    def instruction_size(self) -> int:
        """Size of one instruction in bytes (4 for ARM/RISC-V, 2 for Thumb)."""
        ...
```

This is the same pattern as real CPU design. ARM defines the decoder semantics.
Apple builds the pipeline and caches. The decoder plugs into the pipeline. Our
packages work identically:

```
arm-simulator provides:     riscv-simulator provides:
  ARMDecoder.decode()         RISCVDecoder.decode()
  ARMDecoder.execute()        RISCVDecoder.execute()
  instruction_size = 4        instruction_size = 4

Both plug into the same Core:
  core = Core(config, decoder=ARMDecoder())
  core = Core(config, decoder=RISCVDecoder())
```

### Example Configurations

#### "Simple" — Teaching Core

The minimal core for learning. No branch prediction, no caches, tiny pipeline.
Equivalent to a 1980s microcontroller:

```python
SIMPLE_CONFIG = CoreConfig(
    name="Simple",
    pipeline=PipelineConfig.classic_5_stage(),
    branch_predictor=StaticPredictor(strategy="always_not_taken"),
    hazard_detection=True,
    forwarding=True,
    register_file=RegisterFileConfig(count=16, width=32),
    fp_unit=None,
    l1i_cache=CacheConfig(size_bytes=4096, associativity=1, access_latency=1),
    l1d_cache=CacheConfig(size_bytes=4096, associativity=1, access_latency=1),
    l2_cache=None,
)
```

Expected characteristics:
- IPC: ~0.7-0.9 (stalls from load-use hazards and branch mispredictions)
- Branch accuracy: ~65% (static predictor)
- Cache miss rate: high (only 4KB, direct-mapped)

#### "Cortex-A78-like" — Modern Performance Core

Inspired by ARM Cortex-A78 (used in Snapdragon 888, Dimensity 9000):

```python
CORTEX_A78_LIKE = CoreConfig(
    name="CortexA78Like",
    pipeline=PipelineConfig.deep_13_stage(),
    branch_predictor=TwoBitPredictor(table_size=4096),  # Simplified vs real TAGE
    hazard_detection=True,
    forwarding=True,
    register_file=RegisterFileConfig(count=31, width=64),  # ARMv8 = 31 GP regs
    fp_unit=FPUnitConfig(formats=["fp32", "fp64"]),
    l1i_cache=CacheConfig(size_bytes=65536, associativity=4, access_latency=1),
    l1d_cache=CacheConfig(size_bytes=65536, associativity=4, access_latency=1),
    l2_cache=CacheConfig(size_bytes=262144, associativity=8, access_latency=12),
)
```

Expected characteristics:
- IPC: ~0.85-0.95 (in-order; real A78 is out-of-order with higher IPC)
- Branch accuracy: ~90-95% (2-bit predictor)
- L1 miss rate: ~3-5% (64KB 4-way)
- L2 miss rate: ~10-20%

#### "Apple M4-like" — Wide Performance Core

Inspired by Apple M4 "Everest" core (aggressive, wide pipeline):

```python
APPLE_M4_LIKE = CoreConfig(
    name="AppleM4Like",
    pipeline=PipelineConfig(
        stages=[...],  # ~14 stages
        execution_width=1,  # We model single-issue for now; real M4 is 8-wide
    ),
    branch_predictor=TwoBitPredictor(table_size=8192),  # Real M4 uses TAGE
    hazard_detection=True,
    forwarding=True,
    register_file=RegisterFileConfig(count=31, width=64),
    fp_unit=FPUnitConfig(formats=["fp32", "fp64", "fp16"]),
    l1i_cache=CacheConfig(size_bytes=196608, associativity=6, access_latency=1),  # 192KB
    l1d_cache=CacheConfig(size_bytes=131072, associativity=8, access_latency=1),  # 128KB
    l2_cache=CacheConfig(size_bytes=16777216, associativity=16, access_latency=15),  # 16MB
)
```

### Multi-Core CPU

Real processors contain multiple cores sharing a last-level cache and memory
controller. Our MultiCoreCPU models this:

```
MultiCoreCPU
│
├── Core 0 ←─── L1I + L1D ←─── Private L2
│     │
├── Core 1 ←─── L1I + L1D ←─── Private L2
│     │
├── Core 2 ←─── L1I + L1D ←─── Private L2
│     │
├── Core 3 ←─── L1I + L1D ←─── Private L2
│     │
├── ══════════════════════════════════════
│   Shared L3 Cache (all cores access this)
├── ══════════════════════════════════════
│
├── Memory Controller
│   ├── Address mapping (which DRAM bank to access)
│   └── Request queuing (handle concurrent core requests)
│
├── Interrupt Controller
│   ├── Route external interrupts to specific cores
│   └── Inter-processor interrupts (core-to-core signals)
│
└── Shared Memory (DRAM simulation)
    └── bytearray with configurable latency
```

Each core runs independently on the same clock. They share the L3 cache and
main memory. The memory controller serializes requests from multiple cores.

### Clock Integration

A single clock drives all components. The Core registers itself and all its
sub-components as clock listeners:

```
Clock.tick()
  │
  ├── Core 0
  │   ├── Pipeline.step()
  │   │   ├── IF: fetch from L1I cache
  │   │   ├── ID: decode + register read
  │   │   ├── EX: ALU + branch resolution
  │   │   ├── MEM: L1D cache access
  │   │   └── WB: register write
  │   ├── BranchPredictor (update if branch resolved)
  │   ├── HazardUnit (detect, produce stall/flush signals)
  │   └── Caches.tick() (advance pending misses)
  │
  ├── Core 1 (same as above)
  │
  ├── L3 Cache.tick()
  │
  └── Memory Controller.tick()
```

### Performance Statistics

The Core collects statistics from all sub-components and computes aggregate
metrics:

```
Core Statistics:
─────────────────────────────────────────────
Instructions completed:    10,000
Total cycles:              12,347
IPC (instructions/cycle):  0.81
CPI (cycles/instruction):  1.23

Pipeline:
  Stall cycles:            1,203 (9.7%)
  Flush cycles:            892   (7.2%)
  Useful cycles:           10,252 (83.0%)

Branch Prediction:
  Total branches:          2,150
  Correct predictions:     1,935 (90.0%)
  Mispredictions:          215   (10.0%)
  Misprediction penalty:   2 cycles each → 430 cycles wasted

Cache Performance:
  L1I hit rate:            98.2%
  L1D hit rate:            94.5%
  L2 hit rate:             87.3%
  Average memory latency:  2.3 cycles

Hazard Breakdown:
  EX→EX forwards:         3,421
  MEM→EX forwards:        892
  Load-use stalls:         1,203
  Structural stalls:       0
```

## Public API

```python
from dataclasses import dataclass
from typing import Optional, Protocol

class ISADecoder(Protocol):
    """Protocol for ISA decoders (ARM, RISC-V, etc.)."""

    def decode(self, raw_instruction: int, token: PipelineToken) -> PipelineToken: ...
    def execute(self, token: PipelineToken, registers: 'RegisterFile') -> PipelineToken: ...

    @property
    def instruction_size(self) -> int: ...


@dataclass
class RegisterFileConfig:
    """Configuration for the register file."""
    count: int = 16               # Number of general-purpose registers
    width: int = 32               # Bits per register (32 or 64)
    zero_register: bool = True    # Is register 0 hardwired to zero? (RISC-V: yes, ARM: no)


@dataclass
class FPUnitConfig:
    """Configuration for the floating-point unit."""
    formats: list[str] = None     # Supported formats: ["fp32"], ["fp32", "fp64"], etc.
    pipeline_depth: int = 3       # FP operations take this many cycles


@dataclass
class CoreConfig:
    """
    Complete configuration for a processor core.

    Every knob that a real CPU architect would tune is represented here.
    """
    name: str = "Default"

    # Pipeline
    pipeline: PipelineConfig = None          # Defaults to classic 5-stage

    # Branch prediction
    branch_predictor_type: str = "two_bit"   # "static", "one_bit", "two_bit"
    branch_predictor_size: int = 1024        # Table size for dynamic predictors
    btb_size: int = 256                      # Branch Target Buffer entries

    # Hazard handling
    hazard_detection: bool = True            # Enable hazard detection unit
    forwarding: bool = True                  # Enable data forwarding paths

    # Register file
    register_file: RegisterFileConfig = None  # Defaults to 16 regs, 32-bit

    # Floating point
    fp_unit: Optional[FPUnitConfig] = None   # None = no FP support

    # Cache hierarchy
    l1i_cache: CacheConfig = None            # L1 instruction cache config
    l1d_cache: CacheConfig = None            # L1 data cache config
    l2_cache: Optional[CacheConfig] = None   # L2 unified cache (per-core)

    # Memory
    memory_size: int = 65536                 # Main memory size in bytes
    memory_latency: int = 100                # DRAM access latency in cycles


@dataclass
class CoreStats:
    """Aggregate statistics from all core sub-components."""
    instructions_completed: int
    total_cycles: int
    ipc: float
    cpi: float

    pipeline_stats: 'PipelineStats'
    predictor_stats: 'PredictorStats'
    hazard_stats: 'HazardStats'
    cache_stats: dict[str, 'CacheStats']     # {'l1i': ..., 'l1d': ..., 'l2': ...}


class Core:
    """
    A configurable processor core.

    Composes pipeline, branch predictor, hazard unit, forwarding unit,
    register file, FP unit, and caches into a working processor.
    The ISA decoder is injected to provide instruction semantics.
    """

    def __init__(self, config: CoreConfig, decoder: ISADecoder) -> None:
        """
        Create a processor core with the given configuration and ISA decoder.

        Initializes all sub-components and wires them together:
        - Pipeline stages call back to the decoder for instruction semantics
        - Pipeline consults branch predictor for speculative fetch
        - Pipeline consults hazard unit for stall/flush signals
        - Pipeline uses forwarding unit for data bypass
        - IF stage reads from L1I cache
        - MEM stage reads/writes L1D cache
        - L1 misses go to L2, L2 misses go to memory
        """
        ...

    def load_program(self, program: bytes, start_address: int = 0) -> None:
        """Load machine code into memory."""
        ...

    def step(self) -> PipelineSnapshot:
        """
        Execute one clock cycle.

        Advances the pipeline, updates caches, records statistics.
        """
        ...

    def run(self, max_cycles: int = 100000) -> CoreStats:
        """Run until halt or max_cycles. Returns aggregate statistics."""
        ...

    @property
    def state(self) -> 'CPUState':
        """Current register file and memory state."""
        ...

    @property
    def stats(self) -> CoreStats:
        """Aggregate statistics from all sub-components."""
        ...

    @property
    def is_halted(self) -> bool:
        """True if a halt instruction has completed."""
        ...

    def read_register(self, index: int) -> int:
        """Read a general-purpose register."""
        ...

    def write_register(self, index: int, value: int) -> None:
        """Write a general-purpose register."""
        ...


@dataclass
class MultiCoreConfig:
    """Configuration for a multi-core CPU."""
    num_cores: int = 4
    core_config: CoreConfig = None           # All cores use same config (for now)
    l3_cache: Optional[CacheConfig] = None   # Shared L3 cache
    memory_size: int = 1048576               # 1 MB shared memory
    memory_latency: int = 100                # DRAM latency in cycles


class MultiCoreCPU:
    """
    A multi-core CPU with shared L3 cache and memory.

    Each core runs independently on the same clock. They share
    the L3 cache and main memory via a memory controller.
    """

    def __init__(self, config: MultiCoreConfig, decoders: list[ISADecoder]) -> None:
        """
        Create a multi-core CPU.

        Args:
            config: Multi-core configuration
            decoders: One ISA decoder per core (can be the same decoder instance
                      or different decoders for heterogeneous multi-core)
        """
        ...

    def load_program(self, core_id: int, program: bytes, start_address: int = 0) -> None:
        """Load a program into a specific core's memory view."""
        ...

    def step(self) -> list[PipelineSnapshot]:
        """Advance all cores by one clock cycle. Returns per-core snapshots."""
        ...

    def run(self, max_cycles: int = 100000) -> list[CoreStats]:
        """Run all cores until all halt or max_cycles. Returns per-core stats."""
        ...

    @property
    def cores(self) -> list[Core]:
        """Access individual cores."""
        ...

    @property
    def stats(self) -> list[CoreStats]:
        """Per-core statistics."""
        ...
```

## Data Structures

### RegisterFile

```python
class RegisterFile:
    """
    General-purpose register file.

    Configurable number of registers and bit width.
    Optionally hardwires register 0 to zero (RISC-V convention).
    """

    def __init__(self, config: RegisterFileConfig) -> None: ...

    def read(self, index: int) -> int:
        """Read register value. Register 0 always returns 0 if zero_register=True."""
        ...

    def write(self, index: int, value: int) -> None:
        """Write register value. Writes to register 0 are silently ignored."""
        ...

    @property
    def values(self) -> list[int]:
        """All register values (for inspection/debugging)."""
        ...
```

### Memory Controller

```python
class MemoryController:
    """
    Serializes memory requests from multiple cores.

    In a multi-core system, multiple cores may request memory access
    in the same cycle. The memory controller queues these requests
    and services them in order (FIFO or priority-based).
    """

    def __init__(self, memory: bytearray, latency: int = 100) -> None: ...

    def request_read(self, address: int, num_bytes: int, requester_id: int) -> None:
        """Submit a read request. Result available after `latency` cycles."""
        ...

    def request_write(self, address: int, data: bytes, requester_id: int) -> None:
        """Submit a write request. Completes after `latency` cycles."""
        ...

    def tick(self) -> list[tuple[int, bytes]]:
        """
        Advance one cycle. Returns list of (requester_id, data) for completed reads.
        """
        ...
```

### Interrupt Controller (Shell)

```python
class InterruptController:
    """
    Routes interrupts to cores.

    Shell implementation — future extension.
    """

    def raise_interrupt(self, interrupt_id: int, target_core: int = -1) -> None:
        """
        Raise an interrupt.

        target_core=-1 means "route to any available core."
        """
        ...

    def acknowledge(self, core_id: int, interrupt_id: int) -> None:
        """Core acknowledges and begins handling the interrupt."""
        ...
```

## Test Strategy

### Core Assembly Tests

- **Construction**: Core initializes all sub-components from config
- **Simple config**: 5-stage, static predictor, 4KB cache → runs without error
- **Complex config**: 13-stage, 2-bit predictor, 64KB cache → runs without error
- **Missing optional**: no L2 cache, no FP unit → still runs correctly

### Single-Instruction Tests

- **NOP**: runs through pipeline in 5 cycles, no side effects
- **ADD**: result appears in destination register after pipeline completes
- **LOAD**: data moves from memory to register via L1D cache
- **STORE**: data moves from register to memory via L1D cache
- **BRANCH (taken)**: PC changes to target, pipeline flushed correctly
- **BRANCH (not taken)**: PC continues sequentially, no flush

### Program Execution Tests

- **Simple sequence**: LOAD, ADD, STORE → verify final memory/register state
- **Loop**: 10-iteration loop → verify correct final state and cycle count
- **Fibonacci**: compute fib(10) → verify result = 55
- **Bubble sort**: sort a small array → verify sorted output

### Performance Comparison Tests

- **Predictor impact**: run same program with Static vs TwoBit predictor,
  verify TwoBit has higher IPC on branch-heavy code
- **Cache impact**: run same program with 4KB vs 64KB L1, verify larger cache
  has higher hit rate on data-heavy code
- **Pipeline depth impact**: run same program with 5-stage vs 13-stage,
  verify deeper pipeline has more flush cycles but potentially higher throughput
- **Forwarding impact**: run with forwarding disabled vs enabled, verify
  enabled has fewer stall cycles

### Statistics Tests

- **IPC calculation**: verify IPC = instructions / cycles
- **Aggregate stats**: verify CoreStats correctly aggregates sub-component stats
- **Cache stats propagation**: verify L1I, L1D, L2 stats accessible through Core
- **Predictor stats propagation**: verify prediction accuracy accessible through Core

### Multi-Core Tests

- **Independent programs**: two cores run separate programs simultaneously,
  both produce correct results
- **Shared memory**: core 0 writes to address X, core 1 reads address X
  (after sufficient cycles) → sees the written value
- **L3 cache sharing**: both cores miss L1/L2, both hit L3 for same address
- **Memory controller serialization**: two cores request memory in same cycle,
  both eventually get served
- **Core count scaling**: 1-core, 2-core, 4-core — verify all work

### ISA Decoder Injection Tests

- **ARM decoder**: Core + ARMDecoder → runs ARM binary correctly
- **RISC-V decoder**: Core + RISCVDecoder → runs RISC-V binary correctly
- **Same program, different ISA**: same algorithm compiled for ARM and RISC-V,
  same final state, different cycle counts (due to different instruction counts)
- **Mock decoder**: Core + mock decoder → verifies protocol is respected

### Configuration Preset Tests

- **SIMPLE_CONFIG**: verify all fields have expected values
- **CORTEX_A78_LIKE**: verify cache sizes, pipeline depth, predictor type
- **APPLE_M4_LIKE**: verify large caches, deep pipeline

## Future Extensions

- **Out-of-order core**: reorder buffer, reservation stations, register renaming,
  speculative execution — the full "big core" treatment
- **Heterogeneous multi-core (big.LITTLE)**: mix performance and efficiency cores
  in one MultiCoreCPU, like ARM big.LITTLE or Apple's P/E cores
- **Cache coherence**: MESI protocol for multi-core shared memory consistency
- **Power modeling**: estimate dynamic + static power based on utilization,
  cache activity, and clock frequency
- **Thermal modeling**: track heat per core, implement thermal throttling
- **Performance counters**: model hardware performance counters (like Linux perf)
  that count events (cache misses, branch mispredictions, stall cycles)
- **Dynamic voltage/frequency scaling (DVFS)**: change clock speed at runtime
  based on workload
- **Checkpoint/restore**: save complete core state and resume later
- **GDB-like debugger interface**: breakpoints, watchpoints, single-step
  through instructions with full pipeline visibility
