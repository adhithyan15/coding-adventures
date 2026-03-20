# G02 — Parallel Execution Engine

## Overview

This package implements **Layer 8 of the accelerator computing stack** — the
parallel execution engine that sits between individual processing elements
(Layer 9, `gpu-core`) and the compute unit (Layer 7, future `sm-simulator`).

This is where parallelism happens. Layer 9 gave us a single core that executes
one instruction at a time. Layer 8 takes many of those cores and orchestrates
them to execute in parallel — but the *way* they're orchestrated differs
fundamentally across architectures.

This is NOT a "SIMT engine." It's a **parallel execution engine** with pluggable
execution models. The same protocol supports:

- **SIMT** (NVIDIA CUDA, ARM Mali) — threads with independent PCs, grouped into warps
- **SIMD** (AMD GCN/RDNA, Intel Arc) — one instruction over wide vector lanes
- **Systolic Dataflow** (Google TPU) — data flowing through a grid, no PCs at all
- **Scheduled MAC** (Apple ANE, Qualcomm Hexagon) — compiler-scheduled MAC arrays

## Layer position

```
Layer 11: Logic Gates (AND, OR, XOR, NAND)
    │
Layer 10: FP Arithmetic (IEEE 754 add/mul/fma)
    │
Layer 9:  Accelerator Core (gpu-core) — one core, one instruction at a time
    │
Layer 8:  Parallel Execution Engine ← YOU ARE HERE
    │
    ├──→ GPU (NVIDIA): WarpEngine     — 32 threads, SIMT, divergence masks
    ├──→ GPU (AMD):    WavefrontEngine — 32/64 lanes, SIMD, lane masking
    ├──→ GPU (Intel):  SubsliceEngine  — SIMD8 × EU threads, thread arbitration
    ├──→ GPU (ARM):    WarpEngine      — 16 threads, SIMT (similar to NVIDIA)
    ├──→ TPU (Google): SystolicArray   — NxN PE grid, dataflow, no instruction fetch
    └──→ NPU (Apple):  MACArrayEngine  — parallel MACs, scheduled by compiler
    │
Layer 7:  Compute Unit (SM / MXU / Neural Engine Core) — future
```

## Why this layer matters — Flynn's Taxonomy

In 1966, Michael Flynn classified computer architectures by how many
instruction streams and data streams they process simultaneously:

```
                    │  Single Data   │  Multiple Data
────────────────────┼────────────────┼─────────────────
Single Instruction  │  SISD          │  SIMD
                    │  (classic CPU) │  (vector processors)
────────────────────┼────────────────┼─────────────────
Multiple Instruction│  MISD          │  MIMD
                    │  (rare)        │  (multi-core CPUs)
```

But modern accelerators don't fit neatly into Flynn's boxes. NVIDIA invented
the term SIMT (Single Instruction, Multiple Threads) because their hardware
is neither pure SIMD nor pure MIMD — it's something in between. And TPU
systolic arrays don't really fit any of Flynn's categories.

Here's the full taxonomy of execution models we need to support:

### SISD — Single Instruction, Single Data
**What:** One instruction, one datum. This is a classic single-core CPU.
**Who:** Our `gpu-core` package at Layer 9. Also the CPU simulator.
**Example:** `R2 = R0 + R1` — one add, one result.

Not parallel at all. This is our starting point.

### SIMD — Single Instruction, Multiple Data
**What:** One instruction operates on a **vector** of data simultaneously.
The hardware has a physically wide ALU — think of it as 32 (or 64) ALUs
bolted together that all do the same operation at the same time.

**Who:** AMD GCN/RDNA wavefronts, Intel AVX-512, Intel Arc Xe, ARM NEON.

**Key property:** There are NO threads. There's one instruction stream and
a wide vector register file. Each "lane" of the vector is just a position
in the hardware — lane 3 doesn't have its own program counter or stack.

```
SIMD (AMD wavefront of 64 lanes):

Instruction: v_add_f32 v0, v1, v2

Lane 0:   v0[0]  = v1[0]  + v2[0]     ─┐
Lane 1:   v0[1]  = v1[1]  + v2[1]      │
Lane 2:   v0[2]  = v1[2]  + v2[2]      │  One instruction,
...                                      │  one wide ALU,
Lane 62:  v0[62] = v1[62] + v2[62]     │  64 results
Lane 63:  v0[63] = v1[63] + v2[63]    ─┘
```

**Divergence handling:** When lanes need to take different paths (e.g.,
`if (lane_id < 32)`), SIMD uses an **execution mask**. Masked-off lanes
are disabled — the ALU still runs but results are discarded. This is
pure waste. The hardware doesn't "know" about individual threads; it
just has a bitmask saying which lanes are active.

```
AMD execution mask for: if (lane_id < 4)

EXEC mask:  1 1 1 1 0 0 0 0 ... 0 0 0 0  (lanes 0-3 active)
            │ │ │ │ │ │ │ │     │ │ │ │
            ▼ ▼ ▼ ▼ ▼ ▼ ▼ ▼     ▼ ▼ ▼ ▼
Result:     ✓ ✓ ✓ ✓ ✗ ✗ ✗ ✗ ... ✗ ✗ ✗ ✗  (disabled lanes waste cycles)
```

### SIMT — Single Instruction, Multiple Threads
**What:** Multiple threads, each with its own registers and (logically) its
own program counter, that are *grouped* and usually execute in lockstep.
But — and this is the key — threads CAN diverge. When they do, the
hardware serializes the divergent paths.

**Who:** NVIDIA CUDA (warps of 32), ARM Mali (warps of 16).

**Key property:** Each thread is a real, independent entity. Thread 7 has
its own R0, its own stack pointer, and conceptually its own PC. The warp
scheduler just *happens* to issue the same instruction to all threads
when they agree. When they don't agree, it handles the divergence.

```
SIMT (NVIDIA warp of 32 threads):

Warp instruction: FADD R2, R0, R1

Thread 0:   R2[t0]  = R0[t0]  + R1[t0]    ─┐
Thread 1:   R2[t1]  = R0[t1]  + R1[t1]     │
Thread 2:   R2[t2]  = R0[t2]  + R1[t2]     │  Same instruction,
...                                          │  but each thread has
Thread 30:  R2[t30] = R0[t30] + R1[t30]    │  its own registers
Thread 31:  R2[t31] = R0[t31] + R1[t31]   ─┘
```

**Divergence handling:** When threads take different branches, the warp
hardware creates an **active mask** (similar to SIMD) but with a key
difference: the hardware tracks a **reconvergence point** and
automatically rejoins threads when they reach it. Pre-Volta NVIDIA
used a hardware stack for this; Volta+ uses **Independent Thread
Scheduling** where each thread truly has its own PC.

```
NVIDIA divergence (pre-Volta stack-based):

if (threadIdx.x < 16):           // Threads 0-15 go here
    path_A()
else:                            // Threads 16-31 go here
    path_B()
// reconvergence point            // All 32 threads rejoin

Execution timeline:
Cycle 1-N:   Threads 0-15 execute path_A  (16-31 masked off)
Cycle N+1-M: Threads 16-31 execute path_B (0-15 masked off)
Cycle M+1:   All 32 threads active again

NVIDIA divergence (Volta+ independent thread scheduling):

Each thread has its own PC. The scheduler picks subsets of
threads that happen to be at the same PC and issues them together.
Threads at different PCs naturally execute in different sub-warps.
```

### SIMD vs SIMT — The Critical Difference

The difference is subtle but architecturally profound:

```
                    SIMD (AMD)                 SIMT (NVIDIA)
─────────────────────────────────────────────────────────────
Unit of work:       Vector lane                Thread
Has own registers:  No (one wide VRF)          Yes (per-thread RF)
Has own PC:         No (one PC)                Yes (logically)
Can diverge:        Via exec mask only         Via thread scheduling
Reconvergence:      Programmer manages mask    Hardware manages
Memory model:       Lanes share address        Each thread has address
Wasted work:        Masked lanes still burn    Inactive threads masked
```

In practice, the result is similar — both process ~32 elements in parallel
and both lose efficiency on divergent code. But the programming models and
hardware implementations are quite different.

**Why it matters for us:** An `simt-engine` would force AMD/Intel into
NVIDIA's worldview. Our engine must support both models natively.

### Systolic Dataflow — No Instructions At All
**What:** Data flows through a grid of processing elements. Each PE does
one multiply-accumulate and passes the result to its neighbor. There's
no instruction fetch, no program counter, no branches. The "program" is
the physical layout of the array.

**Who:** Google TPU (MXU), Intel AMX (on CPU die), some NPU designs.

```
Systolic Array (4×4 for illustration, real TPUs use 128×128 or 256×256):

Weights pre-loaded into each PE:

         col 0    col 1    col 2    col 3
        ┌────────┌────────┌────────┌────────┐
row 0   │ w00    │ w01    │ w02    │ w03    │  ← data flows left→right
        │ MAC    │→MAC    │→MAC    │→MAC    │
        └────────└────────└────────└────────┘
           │↓       │↓       │↓       │↓
        ┌────────┌────────┌────────┌────────┐
row 1   │ w10    │ w11    │ w12    │ w13    │  partial sums flow
        │ MAC    │→MAC    │→MAC    │→MAC    │  top→bottom
        └────────└────────└────────└────────┘
           │↓       │↓       │↓       │↓
        ┌────────┌────────┌────────┌────────┐
row 2   │ w20    │ w21    │ w22    │ w23    │
        │ MAC    │→MAC    │→MAC    │→MAC    │
        └────────└────────└────────└────────┘
           │↓       │↓       │↓       │↓
        ┌────────┌────────┌────────┌────────┐
row 3   │ w30    │ w31    │ w32    │ w33    │
        │ MAC    │→MAC    │→MAC    │→MAC    │
        └────────└────────└────────└────────┘
                                      │↓
                                   results

Each PE computes: accumulator += input_from_left × local_weight
Then passes input_from_left to the right, and accumulator down.

After N cycles, the matrix product C = A × W emerges at the bottom.
```

**Key property:** No instruction stream. Each PE is a dumb accumulator.
The "intelligence" is in how data is fed to the edges of the array.
This is why TPUs are incredibly efficient for matrix multiplication
but terrible at irregular computation.

### Scheduled MAC Array — Compiler-Orchestrated
**What:** An array of multiply-accumulate units that execute operations
scheduled by the compiler at compile time. No hardware scheduler, no
warp management — the compiler decides exactly which MAC does what,
when.

**Who:** Apple Neural Engine (ANE), Qualcomm Hexagon NPU, some
custom AI accelerators.

```
MAC Array (simplified Apple ANE-style):

Input Buffer:  [a0, a1, a2, a3, a4, a5, a6, a7]
Weight Buffer: [w0, w1, w2, w3, w4, w5, w6, w7]

        ┌──────┐  ┌──────┐  ┌──────┐  ┌──────┐
        │ MAC0 │  │ MAC1 │  │ MAC2 │  │ MAC3 │   ← parallel MACs
        │a0×w0 │  │a1×w1 │  │a2×w2 │  │a3×w3 │
        └──┬───┘  └──┬───┘  └──┬───┘  └──┬───┘
           │         │         │         │
           └─────────┴─────────┴─────────┘
                          │
                    ┌─────┴─────┐
                    │  Adder    │  ← reduction tree
                    │  Tree     │
                    └─────┬─────┘
                          │
                    ┌─────┴─────┐
                    │ Activation│  ← optional: ReLU, sigmoid, etc.
                    └─────┬─────┘
                          │
                    Output Buffer

The compiler emits a schedule: "cycle 1: load tile A[0:4],
cycle 2: MAC all, cycle 3: store result, cycle 4: load tile A[4:8]..."
```

**Key property:** The hardware is simple — just MACs, buffers, and an
adder tree. All complexity is in the compiler. This is why NPUs are
power-efficient: no branch predictor, no warp scheduler, no speculation.

## Architecture: Protocol-Based Design

### The ParallelExecutionEngine Protocol

Every execution engine at Layer 8 implements this protocol:

```python
class ParallelExecutionEngine(Protocol):
    """Any parallel execution engine: warp, wavefront, systolic array, MAC array.

    This is the common interface that Layer 7 (compute units) uses to drive
    parallel computation. It doesn't assume threads, lanes, or PEs — it just
    says "here's work, do it in parallel, tell me what happened."
    """

    @property
    def name(self) -> str:
        """Engine name: 'WarpEngine', 'WavefrontEngine', 'SystolicArray', etc."""
        ...

    @property
    def width(self) -> int:
        """Parallelism width: 32 threads (SIMT), 64 lanes (SIMD),
        NxN PEs (systolic), M MACs (MAC array)."""
        ...

    @property
    def execution_model(self) -> ExecutionModel:
        """Which parallel execution model this engine uses."""
        ...

    def step(self, clock_edge: ClockEdge) -> EngineTrace:
        """Advance one clock cycle. All parallel work for this cycle happens here.

        The clock drives everything — each step is one cycle of the engine.
        For SIMT/SIMD, this means issuing one instruction to all threads/lanes.
        For systolic, this means data advances one PE in the array.
        For MAC, this means one multiply-accumulate across all units.
        """
        ...

    @property
    def halted(self) -> bool:
        """True if all work is complete."""
        ...

    def reset(self) -> None:
        """Reset to initial state."""
        ...
```

### ExecutionModel Enum

Explicitly names the five parallel execution models:

```python
class ExecutionModel(Enum):
    """The five parallel execution models supported by this package.

    Each model represents a fundamentally different way to organize
    parallel computation. They are NOT interchangeable — each has
    different properties around divergence, synchronization, and
    data movement.
    """

    SIMT = "simt"
    """Single Instruction, Multiple Threads.
    Threads with own registers/PC, grouped into warps.
    Hardware manages divergence via masks + reconvergence.
    Used by: NVIDIA CUDA, ARM Mali."""

    SIMD = "simd"
    """Single Instruction, Multiple Data.
    One instruction stream, wide vector ALU.
    Divergence via execution mask (programmer or compiler managed).
    Used by: AMD GCN/RDNA, Intel Arc Xe, Intel AVX."""

    SYSTOLIC = "systolic"
    """Systolic Dataflow.
    NxN grid of PEs, data flows through the array.
    No instruction fetch, no PC, no branches.
    Used by: Google TPU MXU, Intel AMX."""

    SCHEDULED_MAC = "scheduled_mac"
    """Compiler-Scheduled MAC Array.
    Parallel MACs driven by a static schedule.
    No hardware scheduler — compiler determines all timing.
    Used by: Apple ANE, Qualcomm Hexagon NPU."""

    VLIW = "vliw"
    """Very Long Instruction Word.
    Multiple operations packed into one wide instruction.
    Compiler decides what runs in parallel.
    Used by: Some DSPs, Qualcomm Adreno (older)."""
```

### The EngineTrace

Every engine produces traces for educational visibility:

```python
@dataclass(frozen=True)
class EngineTrace:
    """Record of one parallel execution step.

    This captures what happened across ALL parallel units in one cycle.
    The trace format adapts to the execution model — SIMT traces show
    per-thread state, systolic traces show data flow through PEs, etc.
    """

    cycle: int
    engine_name: str
    execution_model: ExecutionModel

    # What work was performed (interpretation depends on model)
    description: str

    # Per-unit state (thread/lane/PE/MAC index → description)
    unit_traces: dict[int, str]

    # Which units were active vs masked/idle
    active_mask: list[bool]

    # Aggregate stats
    active_count: int       # How many units did useful work
    total_count: int        # Total units available
    utilization: float      # active_count / total_count (0.0 to 1.0)

    # Optional: divergence info (SIMT/SIMD only)
    divergence_info: DivergenceInfo | None = None

    # Optional: data flow info (systolic only)
    dataflow_info: DataflowInfo | None = None
```

## Engine 1: WarpEngine (SIMT)

The SIMT execution engine. Used by NVIDIA CUDA and ARM Mali.

### Core concept

A **warp** is a group of threads (typically 32 for NVIDIA, 16 for ARM Mali)
that execute the same instruction simultaneously. Each thread has its own
register file but they share the instruction stream (when not diverged).

### Architecture

```
WarpEngine (NVIDIA-style, 32 threads)
┌─────────────────────────────────────────────────────┐
│                                                     │
│  Instruction Buffer: [FADD, FMUL, STORE, ...]       │
│  Warp PC: 0x004                                     │
│                                                     │
│  ┌─────────────────────────────────────────────┐    │
│  │  Active Mask: 1111 1111 1111 1111 ... 1111   │    │
│  └─────────────────────────────────────────────┘    │
│                                                     │
│  ┌───────┐ ┌───────┐ ┌───────┐     ┌───────┐       │
│  │Core 0 │ │Core 1 │ │Core 2 │ ... │Core 31│       │
│  │R0..R31│ │R0..R31│ │R0..R31│     │R0..R31│       │
│  │Mem    │ │Mem    │ │Mem    │     │Mem    │       │
│  └───────┘ └───────┘ └───────┘     └───────┘       │
│                                                     │
│  Divergence Stack:                                  │
│  ┌─────────────────────────────────────────────┐    │
│  │ (reconvergence_pc, saved_active_mask)        │    │
│  └─────────────────────────────────────────────┘    │
│                                                     │
│  Thread→Lane Mapping:                               │
│  Thread 0 → Core 0, Thread 1 → Core 1, ...         │
│                                                     │
└─────────────────────────────────────────────────────┘
```

### Configuration

```python
@dataclass
class WarpConfig:
    """Configuration for a SIMT warp engine."""

    warp_width: int = 32            # 32 (NVIDIA), 16 (ARM Mali)
    num_registers: int = 32         # per-thread register count
    memory_per_thread: int = 1024   # per-thread local memory (bytes)
    float_format: FloatFormat = FP32
    max_divergence_depth: int = 32  # max nesting of divergent branches
    isa: InstructionSet = GenericISA()

    # NVIDIA Volta+ independent thread scheduling
    independent_thread_scheduling: bool = False
```

### Divergence handling (pre-Volta stack-based)

When threads take different branches, the warp doesn't split — it
serializes the paths using a divergence stack:

```
                          Step-by-step divergence:

Program:                   Active mask:         Divergence stack:
─────────                  ────────────         ─────────────────
0: LIMM R0, threadId       1111...1111          (empty)
1: LIMM R1, 16             1111...1111          (empty)
2: BLT R0, R1, +2          ──── branch! ────
   │                       1111...0000          push(PC=5, mask=0000...1111)
   │ (threads 0-15)        │
3: path_A_instr_1          1111...0000          (PC=5, mask=0000...1111)
4: path_A_instr_2          1111...0000          (PC=5, mask=0000...1111)
   │                       pop! ────────
   │ (threads 16-31)       │
3: path_B_instr_1          0000...1111          (empty)
4: path_B_instr_2          0000...1111          (empty)
   │                       reconverge ──
5: common_code             1111...1111          (empty)
```

The divergence stack stores:
- The **reconvergence PC** — where threads rejoin
- The **saved mask** — which threads took the other path

When the first path completes, the stack is popped, the mask is swapped,
and the second path executes. At the reconvergence point, all threads
are active again.

### Divergence handling (Volta+ independent thread scheduling)

Each thread maintains its own PC. The scheduler examines all 32 PCs,
groups threads with the same PC into **sub-warps**, and issues them
together:

```
Thread PCs after branch:   Scheduler groups:

T0:  0x010  ─┐
T1:  0x010   ├─ Sub-warp A (issue together)
T2:  0x010   │
T3:  0x010  ─┘
T4:  0x020  ─┐
T5:  0x020   ├─ Sub-warp B (issue together)
T6:  0x020  ─┘
T7:  0x010  ─── joins Sub-warp A next cycle
```

This allows threads to truly diverge and reconverge at any point, not just
at structured branch boundaries. It also enables **warp-level cooperation**
like producer-consumer patterns that were impossible before.

### Thread identity

Each thread knows its position within the warp:

```python
@dataclass
class ThreadContext:
    """Per-thread execution context in a SIMT warp."""

    thread_id: int          # 0 to warp_width-1
    core: GPUCore           # this thread's processing element
    active: bool = True     # whether this thread is currently active
    pc: int = 0             # per-thread PC (for independent scheduling)
```

### WarpEngine API

```python
class WarpEngine:
    """SIMT warp execution engine.

    Manages N threads executing in lockstep with divergence support.
    Each thread is backed by a GPUCore from the gpu-core package.
    """

    def __init__(self, config: WarpConfig, clock: Clock) -> None: ...

    def load_program(self, program: list[Instruction]) -> None:
        """Load the same program into all threads.
        Each thread gets its own register state but shares the code."""
        ...

    def set_thread_register(self, thread_id: int, reg: int, value: float) -> None:
        """Set a per-thread register value.
        This is how you give each thread different data to work on."""
        ...

    def step(self, clock_edge: ClockEdge) -> EngineTrace:
        """Execute one cycle: issue one instruction to all active threads."""
        ...

    def run(self, max_cycles: int = 10000) -> list[EngineTrace]:
        """Run until all threads halt or max_cycles reached."""
        ...

    @property
    def active_mask(self) -> list[bool]:
        """Which threads are currently active."""
        ...

    @property
    def halted(self) -> bool:
        """True if ALL threads have halted."""
        ...

    def reset(self) -> None: ...
```

## Engine 2: WavefrontEngine (SIMD)

The SIMD execution engine. Used by AMD GCN/RDNA.

### Core concept

A **wavefront** is a set of vector lanes (64 on GCN, 32 on RDNA) that
execute one instruction per cycle. Unlike SIMT, there are no "threads" —
just one wide vector register file and one instruction stream.

### Architecture

```
WavefrontEngine (AMD RDNA-style, 32 lanes)
┌─────────────────────────────────────────────────────┐
│                                                     │
│  Instruction Buffer: [V_ADD_F32, V_MUL_F32, ...]    │
│  PC: 0x004 (one PC for the whole wavefront)         │
│                                                     │
│  ┌─────────────────────────────────────────────┐    │
│  │  EXEC Mask: 1111 1111 1111 1111 ... 1111     │    │
│  └─────────────────────────────────────────────┘    │
│                                                     │
│  Vector Register File (VGPRs):                      │
│  ┌─────────────────────────────────────────────┐    │
│  │ v0:  [lane0][lane1][lane2]...[lane31]        │    │
│  │ v1:  [lane0][lane1][lane2]...[lane31]        │    │
│  │ ...                                          │    │
│  │ v255:[lane0][lane1][lane2]...[lane31]        │    │
│  └─────────────────────────────────────────────┘    │
│                                                     │
│  Scalar Register File (SGPRs):                      │
│  ┌──────────────────────────────────┐               │
│  │ s0, s1, s2, ... s103             │               │
│  │ (shared across all lanes)        │               │
│  └──────────────────────────────────┘               │
│                                                     │
│  Shared Local Data Store (LDS): 64 KB               │
│                                                     │
└─────────────────────────────────────────────────────┘
```

### How AMD differs from NVIDIA

AMD's architecture has two types of registers:

1. **VGPRs (Vector General Purpose Registers)**: One value per lane.
   `v0` isn't one register — it's 32 registers (one per lane).
2. **SGPRs (Scalar General Purpose Registers)**: One value shared by all lanes.
   Used for constants, addresses, loop counters — anything that's the same
   across all lanes.

This is architecturally different from NVIDIA where every thread has the
same register set. AMD's split means scalar work (like computing an address)
happens once, not 32 times.

### EXEC mask

AMD uses the `EXEC` register (a 32-bit or 64-bit bitmask) to control
which lanes execute each instruction. All branching is done by manipulating
this mask:

```
AMD branch handling (GCN/RDNA):

s_cmp_lt_u32 s0, 16        // scalar compare: s0 < 16?
s_cbranch_scc1 else_label   // if true, jump to else

// Instead of branching, AMD does:
v_cmp_lt_u32 vcc, v_tid, 16 // compare per-lane: which lanes have tid < 16?
s_and_saveexec s[10:11], vcc // EXEC = EXEC & vcc, save old EXEC

// Now only lanes with tid < 16 are active
v_add_f32 v0, v1, v2        // only active lanes execute

s_or_b64 exec, exec, s[10:11] // restore EXEC for the else path
s_xor_b64 exec, exec, s[10:11] // flip: now only tid >= 16 active

// Now only lanes with tid >= 16 are active
v_mul_f32 v0, v1, v2        // only these lanes execute

s_or_b64 exec, exec, s[10:11] // restore full EXEC (reconverge)
```

Notice: the programmer (or compiler) explicitly manages the mask. In NVIDIA's
SIMT model, the hardware does this automatically.

### Configuration

```python
@dataclass
class WavefrontConfig:
    """Configuration for an AMD-style SIMD wavefront engine."""

    wave_width: int = 32            # 64 (GCN), 32 (RDNA)
    num_vgprs: int = 256            # vector registers per lane
    num_sgprs: int = 104            # scalar registers (shared)
    lds_size: int = 65536           # Local Data Store in bytes (64 KB)
    float_format: FloatFormat = FP32
    isa: InstructionSet = GenericISA()
```

### WavefrontEngine API

```python
class WavefrontEngine:
    """SIMD wavefront execution engine.

    One instruction stream, one wide vector ALU, explicit EXEC mask.
    Lane state is stored in a vector register file (not per-lane cores).
    """

    def __init__(self, config: WavefrontConfig, clock: Clock) -> None: ...

    def load_program(self, program: list[Instruction]) -> None: ...

    def set_lane_register(self, lane: int, vreg: int, value: float) -> None:
        """Set a per-lane vector register value."""
        ...

    def set_scalar_register(self, sreg: int, value: float) -> None:
        """Set a scalar register (shared across all lanes)."""
        ...

    def set_exec_mask(self, mask: list[bool]) -> None:
        """Explicitly set the execution mask."""
        ...

    def step(self, clock_edge: ClockEdge) -> EngineTrace: ...
    def run(self, max_cycles: int = 10000) -> list[EngineTrace]: ...

    @property
    def exec_mask(self) -> list[bool]: ...

    @property
    def halted(self) -> bool: ...

    def reset(self) -> None: ...
```

## Engine 3: SystolicArray (Dataflow)

The systolic dataflow engine. Used by Google TPU MXU.

### Core concept

A **systolic array** is an NxN grid of processing elements where data
flows through in a wave-like pattern. "Systolic" comes from the Greek
word for heart — like blood pumping through the body, data pulses through
the array on each clock cycle.

There are NO instructions. Each PE does exactly one thing: multiply its
input by its weight, add to the accumulator, and pass data to its neighbor.

### Architecture

```
SystolicArray (4×4 example, TPU uses 128×128 or 256×256)

        ┌─── Input activations flow left → right ───┐
        │                                            │
        ▼                                            ▼
    a[0]──→ PE(0,0) ──→ PE(0,1) ──→ PE(0,2) ──→ PE(0,3) ──→
             │↓           │↓           │↓           │↓
    a[1]──→ PE(1,0) ──→ PE(1,1) ──→ PE(1,2) ──→ PE(1,3) ──→
             │↓           │↓           │↓           │↓
    a[2]──→ PE(2,0) ──→ PE(2,1) ──→ PE(2,2) ──→ PE(2,3) ──→
             │↓           │↓           │↓           │↓
    a[3]──→ PE(3,0) ──→ PE(3,1) ──→ PE(3,2) ──→ PE(3,3) ──→
             │↓           │↓           │↓           │↓
             ▼            ▼            ▼            ▼
        Output partial sums flow top → bottom

Each PE:
┌─────────────────────┐
│ weight: w            │  (pre-loaded before computation)
│ acc: accumulator     │
│                      │
│ On each cycle:       │
│   acc += in × weight │  (one MAC operation)
│   pass in → right    │  (data flows to next PE)
│   pass acc → down    │  (partial sum to next row)
└─────────────────────┘
```

### Data flow timing

The systolic array has a beautiful property: data enters the array with
**staggered timing** so that all the right values meet at the right PEs
at the right time. Here's how a 3×3 matmul works:

```
Computing C = A × W where:

A = [a00 a01 a02]    W = [w00 w01 w02]    (W is pre-loaded into PEs)
    [a10 a11 a12]        [w10 w11 w12]
    [a20 a21 a22]        [w20 w21 w22]

Cycle 1:  a00 enters PE(0,0)
          a00 × w00 → acc(0,0)

Cycle 2:  a01 enters PE(0,0), a00 flows to PE(0,1)
          a10 enters PE(1,0) (staggered by 1 cycle)
          a01 × w00 + acc → acc(0,0)
          a00 × w01 → acc(0,1)
          a10 × w10 → acc(1,0)

Cycle 3:  all 3 rows feeding, data flowing right and down
          ...

Cycle 2N-1:  last values emerge at bottom. C is complete.
```

The staggering is what makes it work — row i starts feeding data i cycles
late. This ensures each PE receives the right input at the right time
without any addressing logic. Pure clockwork.

### Configuration

```python
@dataclass
class SystolicConfig:
    """Configuration for a systolic array engine."""

    rows: int = 4                   # 4 (teaching), 128 (TPU v2/v3), 256 (TPU v4)
    cols: int = 4                   # usually square
    float_format: FloatFormat = FP32  # BF16 for real TPUs
    accumulator_format: FloatFormat = FP32  # always higher precision for accum
```

### SystolicArray API

```python
class SystolicArray:
    """Systolic dataflow execution engine.

    NxN grid of processing elements. Data flows through the array —
    activations left-to-right, partial sums top-to-bottom.
    No instruction stream. Just data in, results out.
    """

    def __init__(self, config: SystolicConfig, clock: Clock) -> None: ...

    def load_weights(self, weights: list[list[float]]) -> None:
        """Pre-load the weight matrix into the PE array.
        weights[row][col] goes to PE(row, col)."""
        ...

    def feed_input(self, row: int, value: float) -> None:
        """Feed one activation value into the left edge of the array.
        Row i receives the i-th element of the input vector."""
        ...

    def feed_input_vector(self, values: list[float]) -> None:
        """Feed a full vector to all rows (with staggered timing)."""
        ...

    def step(self, clock_edge: ClockEdge) -> EngineTrace:
        """Advance one cycle: data moves one PE to the right/down."""
        ...

    def run_matmul(
        self, activations: list[list[float]], weights: list[list[float]]
    ) -> list[list[float]]:
        """Convenience: run a complete matrix multiplication.
        Handles weight loading, input feeding, staggering, and draining."""
        ...

    def drain_outputs(self) -> list[list[float]]:
        """Read the accumulated results from the bottom of the array."""
        ...

    @property
    def halted(self) -> bool:
        """True if all data has flowed through and results are available."""
        ...

    def reset(self) -> None: ...
```

### Systolic PE

Each processing element in the array is simple — just a MAC unit:

```python
@dataclass
class SystolicPE:
    """One processing element in the systolic array.

    It's just a multiply-accumulate unit with two data ports:
    - Horizontal: input data flows left → right
    - Vertical: partial sums flow top → bottom
    """

    row: int
    col: int
    weight: FloatBits          # pre-loaded, stays fixed during computation
    accumulator: FloatBits     # running sum, flows down
    input_buffer: FloatBits | None = None  # data to be consumed this cycle

    def step(self) -> tuple[FloatBits | None, FloatBits]:
        """One clock cycle:
        1. acc += input_buffer × weight  (MAC)
        2. Return (input to pass right, acc to pass down)
        """
        ...
```

## Engine 4: MACArrayEngine (Scheduled MAC)

The compiler-scheduled MAC array engine. Used by NPUs.

### Core concept

A **MAC array** is a bank of multiply-accumulate units driven by a static
schedule computed at compile time. Unlike SIMT/SIMD (where hardware
schedules threads at runtime), the NPU's compiler determines exactly
which MAC unit processes which data on which cycle.

### Architecture

```
MACArrayEngine (simplified NPU)
┌─────────────────────────────────────────────────────┐
│                                                     │
│  Schedule: [(cycle, operation, src, dst), ...]      │
│  Schedule PC: 0                                     │
│                                                     │
│  Input Buffer:                                      │
│  ┌──────────────────────────────────────────┐       │
│  │ [tile_0] [tile_1] [tile_2] ...           │       │
│  └──────────────────────────────────────────┘       │
│                                                     │
│  ┌──────┐ ┌──────┐ ┌──────┐ ┌──────┐               │
│  │ MAC0 │ │ MAC1 │ │ MAC2 │ │ MAC3 │  ← all MACs   │
│  │ a×w  │ │ a×w  │ │ a×w  │ │ a×w  │    in parallel │
│  └──┬───┘ └──┬───┘ └──┬───┘ └──┬───┘               │
│     │        │        │        │                    │
│     └────────┴────────┴────────┘                    │
│                  │                                  │
│           ┌──────┴──────┐                           │
│           │ Adder Tree  │                           │
│           └──────┬──────┘                           │
│                  │                                  │
│           ┌──────┴──────┐                           │
│           │ Activation  │  (ReLU, sigmoid, etc.)    │
│           └──────┬──────┘                           │
│                  │                                  │
│  Output Buffer:                                     │
│  ┌──────────────────────────────────────────┐       │
│  │ [result_0] [result_1] ...                │       │
│  └──────────────────────────────────────────┘       │
│                                                     │
└─────────────────────────────────────────────────────┘
```

### Schedule format

The NPU's "program" is a schedule, not an instruction stream:

```python
@dataclass(frozen=True)
class MACScheduleEntry:
    """One entry in the MAC array schedule.

    The compiler generates these at compile time. Each entry describes
    exactly what happens on one cycle.
    """

    cycle: int                  # which cycle to execute
    operation: MACOperation     # LOAD_INPUT, MAC, REDUCE, ACTIVATE, STORE_OUTPUT
    input_indices: list[int]    # which input buffer slots to read
    weight_indices: list[int]   # which weight buffer slots to use
    output_index: int           # where to write the result
    activation: str = "none"    # "none", "relu", "sigmoid", "tanh"
```

### Configuration

```python
@dataclass
class MACArrayConfig:
    """Configuration for a scheduled MAC array engine."""

    num_macs: int = 8               # number of parallel MAC units
    input_buffer_size: int = 1024   # input buffer in elements
    weight_buffer_size: int = 4096  # weight buffer in elements
    output_buffer_size: int = 1024  # output buffer in elements
    float_format: FloatFormat = FP16  # NPUs often use FP16/INT8
    accumulator_format: FloatFormat = FP32  # higher precision accumulation
    has_activation_unit: bool = True  # hardware activation function
```

### MACArrayEngine API

```python
class MACArrayEngine:
    """Compiler-scheduled MAC array execution engine.

    No hardware scheduler. The compiler generates a static schedule
    that says exactly what each MAC does on each cycle.
    """

    def __init__(self, config: MACArrayConfig, clock: Clock) -> None: ...

    def load_schedule(self, schedule: list[MACScheduleEntry]) -> None:
        """Load a compiler-generated execution schedule."""
        ...

    def load_inputs(self, data: list[float]) -> None:
        """Load input activations into the input buffer."""
        ...

    def load_weights(self, data: list[float]) -> None:
        """Load weights into the weight buffer."""
        ...

    def step(self, clock_edge: ClockEdge) -> EngineTrace:
        """Execute one scheduled cycle."""
        ...

    def run(self, max_cycles: int = 10000) -> list[EngineTrace]:
        """Run the full schedule."""
        ...

    def read_outputs(self) -> list[float]:
        """Read results from the output buffer."""
        ...

    @property
    def halted(self) -> bool:
        """True if the schedule is complete."""
        ...

    def reset(self) -> None: ...
```

## Engine 5: SubsliceEngine (Intel Xe SIMD)

Intel's GPU architecture uses yet another model: SIMD8 execution units
organized into subslices. Each EU runs multiple hardware threads, each
thread processing SIMD8 vectors.

### Architecture

```
SubsliceEngine (Intel Xe-style)
┌──────────────────────────────────────────────────────┐
│  Subslice                                            │
│  ┌────────────────────────────────┐                  │
│  │  EU 0 (Execution Unit)         │                  │
│  │  ┌──────────────────────────┐  │                  │
│  │  │ Thread 0: SIMD8 ALU      │  │  ← 7 threads    │
│  │  │ Thread 1: SIMD8 ALU      │  │     per EU       │
│  │  │ ...                      │  │                  │
│  │  │ Thread 6: SIMD8 ALU      │  │                  │
│  │  └──────────────────────────┘  │                  │
│  │  GRF: 128 × 256-bit registers  │                  │
│  │  Thread Arbiter                │                  │
│  └────────────────────────────────┘                  │
│                                                      │
│  ┌────────────────────────────────┐                  │
│  │  EU 1 ... EU 7 (same structure)│                  │
│  └────────────────────────────────┘                  │
│                                                      │
│  Shared Local Memory (SLM): 64 KB                    │
│  Instruction Cache                                   │
│  Thread Dispatcher                                   │
└──────────────────────────────────────────────────────┘

Total parallelism per subslice: 8 EUs × 7 threads × 8 SIMD = 448 operations
```

This is a hybrid model: SIMD within each thread, multiple threads per EU
(to hide latency), multiple EUs per subslice. We model this with a
configuration:

```python
@dataclass
class SubsliceConfig:
    """Configuration for an Intel Xe-style SIMD subslice."""

    num_eus: int = 8                # EUs per subslice
    threads_per_eu: int = 7         # hardware threads per EU
    simd_width: int = 8             # SIMD8 (can be 16 or 32 for wider ops)
    grf_size: int = 128             # general register file entries per EU
    slm_size: int = 65536           # shared local memory (64 KB)
    float_format: FloatFormat = FP32
    isa: InstructionSet = GenericISA()
```

## Divergence Comparison

A key educational goal is showing how different architectures handle the
same divergent code. Here's the same branch across all models:

```python
# Pseudocode: if (id < 16): a = x + y  else: a = x * y

# SIMT (NVIDIA):
# Hardware detects divergence, pushes reconvergence point onto stack,
# executes both paths serially with different active masks,
# then reconverges automatically. Programmer writes normal if/else.

# SIMD (AMD):
# Compiler emits: v_cmp, s_and_saveexec, <path A>, s_or, s_xor,
# <path B>, s_or. The EXEC mask is explicitly manipulated by
# instructions. Hardware just obeys the mask.

# Systolic (TPU):
# NOT APPLICABLE. There are no branches. The compiler must restructure
# the computation to avoid conditionals, or use padding/masking at the
# data level. The array just flows data through unconditionally.

# Scheduled MAC (NPU):
# The compiler evaluates the branch at compile time (for static shapes)
# or generates two schedules (for dynamic shapes) and picks at runtime.
# The MAC array itself has no branch hardware.

# Subslice (Intel):
# Similar to SIMD — the channel enable mask controls which SIMD lanes
# are active. The compiler generates predicated instructions.
```

## Clock Integration

All engines are driven by the `Clock` package from the shared foundation.
The clock provides synchronization:

```python
clock = Clock(frequency_hz=1_000_000_000)  # 1 GHz

# SIMT/SIMD: one instruction issued per rising edge
warp = WarpEngine(WarpConfig(warp_width=32), clock)
clock.register_listener(lambda edge: warp.step(edge) if edge.is_rising else None)

# Systolic: data advances one PE per rising edge
systolic = SystolicArray(SystolicConfig(rows=4, cols=4), clock)
clock.register_listener(lambda edge: systolic.step(edge) if edge.is_rising else None)

# Run 100 cycles
clock.run(100)
```

This mirrors real hardware: everything happens on clock edges.

## Example Programs

### SAXPY across all engines

`y[i] = a * x[i] + y[i]` for 32 elements — the same computation,
four different execution models:

**SIMT (NVIDIA):**
```python
# Each thread handles one element
warp = WarpEngine(WarpConfig(warp_width=32), clock)
warp.load_program([
    limm(0, 2.0),        # R0 = a (same for all threads)
    # R1 = x[threadId], R2 = y[threadId] (loaded per-thread)
    ffma(3, 0, 1, 2),    # R3 = a * x + y
    halt(),
])
# Give each thread its own x and y values
for t in range(32):
    warp.set_thread_register(t, 1, x[t])
    warp.set_thread_register(t, 2, y[t])
traces = warp.run()
```

**SIMD (AMD):**
```python
# One instruction operates on all 32 lanes
wave = WavefrontEngine(WavefrontConfig(wave_width=32), clock)
wave.load_program([
    limm(0, 2.0),        # s0 = a (scalar, shared)
    # v1 = x[lane], v2 = y[lane] (vector registers, per-lane)
    ffma(3, 0, 1, 2),    # v3 = s0 * v1 + v2
    halt(),
])
for lane in range(32):
    wave.set_lane_register(lane, 1, x[lane])
    wave.set_lane_register(lane, 2, y[lane])
traces = wave.run()
```

**Systolic (TPU):**
```python
# SAXPY is a vector op, not a matmul — systolic arrays aren't ideal.
# But we can express it as: y = [a] × [x] + [y] (1×1 "matrix" per element)
# In practice, TPUs batch these into matrix ops.
array = SystolicArray(SystolicConfig(rows=1, cols=32), clock)
array.load_weights([[a] * 32])  # weight = a for each PE
# Feed x values, accumulate into y
# (This is contrived — TPUs shine on real matrix ops, not element-wise)
```

**Scheduled MAC (NPU):**
```python
# Compiler generates the schedule at compile time
schedule = [
    MACScheduleEntry(cycle=0, operation=LOAD, ...),
    MACScheduleEntry(cycle=1, operation=MAC, ...),  # a * x for all elements
    MACScheduleEntry(cycle=2, operation=REDUCE, ...), # + y
    MACScheduleEntry(cycle=3, operation=STORE, ...),
]
mac = MACArrayEngine(MACArrayConfig(num_macs=8), clock)
mac.load_schedule(schedule)
mac.load_inputs(x)
mac.load_weights([a] * 32)
traces = mac.run()
```

### Matrix Multiply across all engines

`C = A × B` where A is 4×4 and B is 4×4. This is where the systolic
array truly shines:

**Systolic:** 7 cycles (2N-1 for N=4). Data flows through. Each PE does
one MAC per cycle. No instruction overhead. Maximum efficiency.

**SIMT:** Each thread computes one output element. Thread t computes
C[t/4][t%4] = dot(A[t/4], B_col[t%4]). Inner loop of 4 FMA instructions.
Total: ~20 cycles (setup + 4 FMAs + sync per thread).

**SIMD:** Similar to SIMT but with vector ops. 16 lanes compute 16 output
elements simultaneously. Two passes for 16 elements. Total: ~12 cycles.

**MAC Array:** Compiler tiles the matmul into chunks that fit the MAC array.
Schedule is generated statically. Total: depends on MAC count and tiling.

## Dependencies

- **gpu-core**: `GPUCore`, `FPRegisterFile`, `LocalMemory`, `Instruction`,
  `InstructionSet`, `GenericISA`, all opcodes and helpers
- **fp-arithmetic**: `FloatBits`, `FloatFormat`, `FP32`/`FP16`/`BF16`,
  `fp_add`/`fp_mul`/`fp_fma`/`fp_neg`/`fp_abs`/`fp_compare`
- **clock**: `Clock`, `ClockEdge` (for synchronized stepping)

## Package name

`parallel-execution-engine` across all languages (Ruby: `parallel_execution_engine`).

## Implementation order

Since this is a large package with five engines, implement incrementally:

1. **Protocols + ExecutionModel enum** — the common interface
2. **WarpEngine (SIMT)** — most common, most educational (NVIDIA)
3. **WavefrontEngine (SIMD)** — shows contrast with SIMT (AMD)
4. **SystolicArray (Dataflow)** — completely different paradigm (TPU)
5. **MACArrayEngine (Scheduled)** — compiler-driven approach (NPU)
6. **SubsliceEngine (Hybrid)** — Intel's unique mix

Each engine is independently testable. The protocol ensures they're
interchangeable from Layer 7's perspective.

## Implementation languages

Python, Ruby, Go, TypeScript, Rust — all five languages in the repo.

## Test strategy

Each engine needs:
1. **Unit tests**: Construction, configuration, reset
2. **Execution tests**: Run a known program, verify results
3. **Trace tests**: Verify traces contain expected information
4. **Divergence tests** (SIMT/SIMD only): Branch with different paths,
   verify correct serialization and reconvergence
5. **Cross-engine tests**: Same computation (SAXPY, matmul) on all engines,
   verify same numerical results but different traces/cycle counts
6. **Clock integration tests**: Verify engines advance on clock edges
7. **Utilization tests**: Measure and verify utilization metrics

Coverage target: 95%+ for the core protocols, 90%+ for each engine.
