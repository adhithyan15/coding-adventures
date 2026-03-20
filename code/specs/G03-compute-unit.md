# G03 — Compute Unit Simulator

## Overview

This package implements **Layer 7 of the accelerator computing stack** — the
compute unit that manages multiple parallel execution engines, schedules work
across them, and provides shared resources (memory, caches, register files).

Just as the CPU Core (D05) composes a pipeline, branch predictor, caches, and
register file into a working processor, the Compute Unit composes execution
engines, schedulers, shared memory, and caches into a working accelerator
compute unit. **It's composition, not new logic** — the intelligence is in how
the existing pieces are wired together.

This layer is where the real architectural diversity shows up. At Layer 8, the
execution engines were already different (SIMT vs SIMD vs systolic). At Layer 7,
the differences multiply — each architecture wraps those engines in very
different organizational structures:

- **NVIDIA SM** (Streaming Multiprocessor): 4 warp schedulers managing 48-64 warps,
  shared memory, L1 cache, register file — optimized for thread-level parallelism
- **AMD CU** (Compute Unit): 4 SIMD units sharing scalar unit, LDS, L1 cache —
  optimized for wide vector operations
- **Google TPU MXU** (Matrix Multiply Unit): systolic arrays + vector unit +
  scalar unit + accumulators — optimized for matrix math
- **Intel Xe Core**: 8-16 EUs sharing SLM, L1 cache, thread dispatcher —
  hybrid SIMD/thread model
- **Apple ANE Core** (Neural Engine): MAC arrays + activation pipeline +
  on-chip SRAM + DMA — optimized for inference

## Layer position

```
Layer 11: Logic Gates
    │
Layer 10: FP Arithmetic (shared)
    │
Layer 9:  Accelerator Core (gpu-core)
    │
Layer 8:  Parallel Execution Engine (parallel-execution-engine)
    │     ├── WarpEngine, WavefrontEngine, SystolicArray,
    │     │   MACArrayEngine, SubsliceEngine
    │     │
Layer 7:  Compute Unit ← YOU ARE HERE
    │     ├── StreamingMultiprocessor (NVIDIA SM)
    │     ├── AMDComputeUnit (AMD CU)
    │     ├── MatrixMultiplyUnit (Google TPU MXU)
    │     ├── XeCore (Intel)
    │     └── NeuralEngineCore (Apple ANE)
    │
Layer 6:  Device Simulator — future (full GPU/TPU/NPU)
```

## Why "Compute Unit" and not "SM Simulator"?

Same reason Layer 8 isn't called "SIMT Engine." Every vendor has a different name
for this level of the hierarchy:

```
NVIDIA:   Streaming Multiprocessor (SM)
AMD:      Compute Unit (CU) / Work Group Processor (WGP in RDNA)
Intel:    Xe Core (or Subslice in older gen)
Google:   Matrix Multiply Unit (MXU) + Vector/Scalar units
Apple:    Neural Engine Core
```

They all serve the same purpose: take the execution engines from Layer 8, add
scheduling and shared resources, and present a coherent compute unit to the
device layer above.

## The Assembly Line Analogy

If a single GPU core (Layer 9) is one worker at a desk, and a warp/wavefront
(Layer 8) is a team of 32 workers doing the same task on different data, then
the compute unit (Layer 7) is **the factory floor**:

- **Workers** = execution engines (warps, wavefronts, systolic arrays)
- **Floor manager** = warp/wavefront scheduler
- **Shared toolbox** = shared memory / LDS (data accessible to all teams)
- **Supply closet** = L1 cache (recent data kept nearby)
- **Filing cabinets** = register file (massive, partitioned among teams)
- **Work orders** = thread blocks / work groups queued for execution

The floor manager's job is to keep all workers busy. When one team stalls
(waiting for data from main memory), the manager immediately switches to
another team that's ready to work. This is **latency hiding through occupancy**
— the fundamental GPU performance strategy.

## Architecture: Protocol-Based Design

### The ComputeUnit Protocol

```python
class ComputeUnit(Protocol):
    """Any compute unit: SM, CU, MXU, Xe Core, ANE Core.

    A compute unit manages multiple execution engines, schedules work
    across them, and provides shared resources. It's the integration
    point between raw parallel execution and the device layer above.
    """

    @property
    def name(self) -> str:
        """Unit name: 'SM', 'CU', 'MXU', 'XeCore', 'ANECore'."""
        ...

    @property
    def architecture(self) -> Architecture:
        """Which vendor architecture this compute unit belongs to."""
        ...

    def dispatch(self, work: WorkItem) -> None:
        """Accept a work item (thread block, work group, tile) for execution.

        The compute unit queues the work and the scheduler will assign it
        to execution engines as resources become available.
        """
        ...

    def step(self, clock_edge: ClockEdge) -> ComputeUnitTrace:
        """Advance one clock cycle across all engines and the scheduler."""
        ...

    def run(self, max_cycles: int = 100000) -> list[ComputeUnitTrace]:
        """Run until all dispatched work is complete."""
        ...

    @property
    def idle(self) -> bool:
        """True if no work remains and all engines are idle."""
        ...

    def reset(self) -> None:
        """Reset all state: engines, scheduler, shared memory, caches."""
        ...
```

### Architecture Enum

```python
class Architecture(Enum):
    """Vendor architectures supported at the compute unit level."""

    NVIDIA_SM = "nvidia_sm"
    """NVIDIA Streaming Multiprocessor (Volta, Ampere, Hopper)."""

    AMD_CU = "amd_cu"
    """AMD Compute Unit (GCN) / Work Group Processor (RDNA)."""

    GOOGLE_MXU = "google_mxu"
    """Google TPU Matrix Multiply Unit."""

    INTEL_XE_CORE = "intel_xe_core"
    """Intel Xe Core (Arc, Data Center GPU)."""

    APPLE_ANE_CORE = "apple_ane_core"
    """Apple Neural Engine Core."""
```

### WorkItem — What Gets Dispatched

A `WorkItem` is the unit of work dispatched to a compute unit. Its meaning
depends on the architecture:

```python
@dataclass(frozen=True)
class WorkItem:
    """A unit of parallel work dispatched to a compute unit.

    In CUDA terms, this is a thread block (or cooperative thread array).
    In OpenCL terms, this is a work group.
    In TPU terms, this is a tile of a matrix operation.
    In NPU terms, this is an inference tile.

    The work item contains:
    - A program (instruction list) for instruction-stream architectures
    - Input data for dataflow architectures
    - Thread/lane count (how many parallel elements)
    - Per-thread initial data (register values)
    """

    work_id: int
    program: list[Instruction] | None = None      # for SIMT/SIMD/Intel
    thread_count: int = 32                         # threads in this block
    per_thread_data: dict[int, dict[int, float]] = field(default_factory=dict)
    # per_thread_data[thread_id][register_index] = value

    # For dataflow architectures (TPU/NPU):
    input_data: list[list[float]] | None = None    # activation matrix
    weight_data: list[list[float]] | None = None   # weight matrix
    schedule: list | None = None                   # MAC schedule (NPU)
```

### ComputeUnitTrace

```python
@dataclass(frozen=True)
class ComputeUnitTrace:
    """Record of one clock cycle across the entire compute unit.

    Captures scheduler decisions, engine activity, memory accesses,
    and resource utilization — everything needed to understand what
    the compute unit did in one cycle.
    """

    cycle: int
    unit_name: str
    architecture: Architecture

    # Scheduler state
    scheduler_action: str          # "issued warp 3", "stalled (memory)", etc.
    active_warps: int              # how many warps/wavefronts are active
    total_warps: int               # max warps this unit can hold

    # Per-engine traces (engine_id → EngineTrace from Layer 8)
    engine_traces: dict[int, EngineTrace]

    # Resource utilization
    shared_memory_used: int        # bytes of shared memory in use
    shared_memory_total: int       # total shared memory
    register_file_used: int        # registers allocated
    register_file_total: int       # total registers available
    occupancy: float               # active_warps / max_warps (0.0 to 1.0)

    # Cache stats (if applicable)
    l1_hits: int = 0
    l1_misses: int = 0
```

## Compute Unit 1: StreamingMultiprocessor (NVIDIA SM)

The heart of NVIDIA's GPU architecture. An SM is the most complex compute unit
because it manages the most concurrent work.

### Real-world SM configurations

```
                     Volta (V100)   Ampere (A100)   Hopper (H100)
Warp schedulers:     4              4               4
Max warps per SM:    64             64              64
Max threads per SM:  2048           2048            2048
CUDA cores (FP32):   64             64              128
Register file:       256 KB         256 KB          256 KB
Shared memory:       96 KB          164 KB          228 KB
L1 cache:            combined w/ shared mem
Tensor cores:        8              4               4 (but 4th gen)
```

### Architecture

```
StreamingMultiprocessor
┌─────────────────────────────────────────────────────────────────┐
│                                                                 │
│  Warp Scheduler 0        Warp Scheduler 1                       │
│  ┌──────────────┐       ┌──────────────┐                        │
│  │ Warp table:  │       │ Warp table:  │                        │
│  │  w0: READY   │       │  w1: STALLED │                        │
│  │  w4: READY   │       │  w5: READY   │                        │
│  │  w8: DONE    │       │  w9: RUNNING │                        │
│  │  ...         │       │  ...         │                        │
│  └──────┬───────┘       └──────┬───────┘                        │
│         │                      │                                │
│         ▼                      ▼                                │
│  ┌──────────────┐       ┌──────────────┐                        │
│  │ WarpEngine 0 │       │ WarpEngine 1 │                        │
│  │ (32 threads) │       │ (32 threads) │                        │
│  └──────────────┘       └──────────────┘                        │
│                                                                 │
│  Warp Scheduler 2        Warp Scheduler 3                       │
│  ┌──────────────┐       ┌──────────────┐                        │
│  │ ...          │       │ ...          │                        │
│  └──────┬───────┘       └──────┬───────┘                        │
│         │                      │                                │
│         ▼                      ▼                                │
│  ┌──────────────┐       ┌──────────────┐                        │
│  │ WarpEngine 2 │       │ WarpEngine 3 │                        │
│  │ (32 threads) │       │ (32 threads) │                        │
│  └──────────────┘       └──────────────┘                        │
│                                                                 │
│  Shared Resources:                                              │
│  ┌─────────────────────────────────────────────────────────┐    │
│  │ Register File: 256 KB (65,536 × 32-bit registers)       │    │
│  │ Partitioned dynamically among warps                      │    │
│  │                                                          │    │
│  │ Shared Memory: 96 KB (configurable split with L1 cache) │    │
│  │ Visible to all threads in the same thread block          │    │
│  │                                                          │    │
│  │ L1 Data Cache: shares capacity with shared memory        │    │
│  │ Hardware-managed transparent cache                       │    │
│  │                                                          │    │
│  │ Instruction Cache: 128 KB                                │    │
│  │ Shared by all warps on this SM                           │    │
│  └─────────────────────────────────────────────────────────┘    │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### Warp Scheduling — The Key Innovation

The warp scheduler is what makes GPUs fast despite each thread being slow.
Here's the insight: memory access takes ~400 cycles on a GPU. If you only
had one warp, the SM would sit idle for 400 cycles on every memory access.
But with 64 warps, when warp 0 stalls on memory, the scheduler instantly
switches to warp 1. When warp 1 stalls, switch to warp 2. By the time
you've cycled through enough warps, warp 0's memory has arrived.

This is **latency hiding through parallelism**. It's the fundamental
difference between CPU and GPU design philosophy:

```
CPU strategy:  Make one thread FAST (deep pipeline, speculation, OoO)
GPU strategy:  Have MANY threads, switch between them to hide latency
```

### Warp states

```python
class WarpState(Enum):
    """Possible states of a warp in the scheduler."""

    READY = "ready"
    """Warp has an instruction ready to issue. Can be scheduled."""

    RUNNING = "running"
    """Warp is currently executing on an engine this cycle."""

    STALLED_MEMORY = "stalled_memory"
    """Warp is waiting for a memory operation to complete."""

    STALLED_BARRIER = "stalled_barrier"
    """Warp is waiting at a __syncthreads() barrier."""

    STALLED_DEPENDENCY = "stalled_dependency"
    """Warp is waiting for a register dependency to resolve."""

    COMPLETED = "completed"
    """Warp has executed its HALT instruction. Done."""
```

### Scheduling policies

Real GPUs use variants of these scheduling policies:

```python
class SchedulingPolicy(Enum):
    """How the warp scheduler picks which warp to issue."""

    ROUND_ROBIN = "round_robin"
    """Simple rotation: warp 0, 1, 2, ..., wrap around.
    Fair but not optimal. Good for teaching."""

    GREEDY = "greedy"
    """Always pick the warp with the most ready instructions.
    Maximizes throughput but can starve some warps."""

    OLDEST_FIRST = "oldest_first"
    """Pick the warp that has been waiting longest.
    Prevents starvation. Used in some real GPUs."""

    GTO = "gto"
    """Greedy-Then-Oldest: issue from the same warp until it stalls,
    then switch to the oldest ready warp.
    Reduces context-switch overhead. NVIDIA's common choice."""

    LRR = "lrr"
    """Loose Round Robin: like round-robin but skips stalled warps.
    Simple and effective. Used in many AMD designs."""
```

### Shared Memory and Thread Blocks

In CUDA, a **thread block** (or cooperative thread array) is a group of
threads that can share data via shared memory and synchronize with barriers.
A thread block is decomposed into warps:

```
Thread Block (256 threads)
├── Warp 0:  threads 0-31
├── Warp 1:  threads 32-63
├── Warp 2:  threads 64-95
├── ...
└── Warp 7:  threads 224-255

All 8 warps share the same shared memory.
__syncthreads() waits for ALL warps in the block.
```

Multiple thread blocks can be resident on the same SM simultaneously,
but they cannot share memory or synchronize with each other. How many
blocks fit depends on their resource requirements (registers, shared memory).

### Occupancy

**Occupancy** = active warps / maximum warps. It's the key performance
metric for GPU kernels. Low occupancy means the SM can't hide memory
latency because there aren't enough warps to switch between.

What limits occupancy:
1. **Register pressure**: Each warp needs N registers × 32 threads. If a
   kernel uses 64 registers/thread, that's 64 × 32 = 2048 registers per
   warp. With 65,536 total, you can fit 32 warps max.
2. **Shared memory**: If a block uses 48 KB of shared memory and the SM
   has 96 KB, only 2 blocks can coexist.
3. **Thread block size**: If blocks have 1024 threads (32 warps) and the
   SM supports 64 warps, only 2 blocks fit.

```python
def compute_occupancy(
    registers_per_thread: int,
    shared_mem_per_block: int,
    threads_per_block: int,
    sm_config: SMConfig,
) -> float:
    """Calculate theoretical occupancy for a kernel launch configuration.

    Returns a float from 0.0 to 1.0 representing what fraction of the
    SM's warp capacity this configuration utilizes.
    """
    warps_per_block = (threads_per_block + 31) // 32

    # Limit 1: register file
    regs_per_warp = registers_per_thread * 32
    max_warps_by_regs = sm_config.register_file_size // regs_per_warp

    # Limit 2: shared memory
    if shared_mem_per_block > 0:
        max_blocks_by_smem = sm_config.shared_memory_size // shared_mem_per_block
        max_warps_by_smem = max_blocks_by_smem * warps_per_block
    else:
        max_warps_by_smem = sm_config.max_warps

    # Limit 3: hardware limit
    max_warps_by_hw = sm_config.max_warps

    # Actual occupancy is limited by the tightest constraint
    active_warps = min(max_warps_by_regs, max_warps_by_smem, max_warps_by_hw)
    return active_warps / sm_config.max_warps
```

### SM Configuration

```python
@dataclass
class SMConfig:
    """Configuration for an NVIDIA-style Streaming Multiprocessor."""

    # Schedulers and execution
    num_schedulers: int = 4           # warp schedulers
    warp_width: int = 32              # threads per warp
    max_warps: int = 48               # maximum resident warps
    max_threads: int = 1536           # max_warps × warp_width
    max_blocks: int = 16              # max resident thread blocks
    scheduling_policy: SchedulingPolicy = SchedulingPolicy.GTO

    # Register file
    register_file_size: int = 65536   # 32-bit registers (256 KB)
    max_registers_per_thread: int = 255

    # Memory
    shared_memory_size: int = 98304   # 96 KB
    l1_cache_size: int = 32768        # 32 KB (may share with shared mem)
    instruction_cache_size: int = 131072  # 128 KB

    # Float format
    float_format: FloatFormat = FP32

    # ISA
    isa: InstructionSet = GenericISA()

    # Stall simulation
    memory_latency_cycles: int = 200  # cycles for a global memory access
    barrier_enabled: bool = True      # support __syncthreads()
```

### SM API

```python
class StreamingMultiprocessor:
    """NVIDIA Streaming Multiprocessor simulator.

    Manages multiple warps executing thread blocks, with a configurable
    warp scheduler, shared memory, and register file partitioning.
    """

    def __init__(self, config: SMConfig, clock: Clock) -> None: ...

    def dispatch(self, work: WorkItem) -> None:
        """Dispatch a thread block to this SM.

        The thread block is decomposed into warps, registers are
        allocated, shared memory is reserved, and warps are added
        to the scheduler's ready queue.

        Raises:
            ResourceError: If not enough registers, shared memory,
            or warp slots to accommodate this thread block.
        """
        ...

    def step(self, clock_edge: ClockEdge) -> ComputeUnitTrace:
        """One cycle: scheduler picks warps, engines execute, memory serves."""
        ...

    def run(self, max_cycles: int = 100000) -> list[ComputeUnitTrace]:
        """Run until all work completes or max_cycles."""
        ...

    @property
    def occupancy(self) -> float:
        """Current occupancy (active warps / max warps)."""
        ...

    @property
    def idle(self) -> bool: ...
    def reset(self) -> None: ...
```

## Compute Unit 2: AMDComputeUnit (AMD CU)

AMD's Compute Unit (GCN) or Work Group Processor (RDNA).

### How AMD CUs differ from NVIDIA SMs

```
NVIDIA SM:                          AMD CU (GCN):
─────────                           ──────────────
4 warp schedulers                   4 SIMD units (16-wide each)
Each issues 1 warp (32 threads)     Each runs 1 wavefront (64 lanes)
Total: 128 threads/cycle            Total: 64 lanes × 4 = 256 lanes/cycle

Register file: unified              Register file: per-SIMD VGPR
Shared memory: explicit             LDS: explicit (similar to shared mem)
Warp scheduling: hardware           Wavefront scheduling: hardware
Scalar unit: per-thread             Scalar unit: SHARED by wavefront
```

The key difference: AMD has a **scalar unit** that executes operations common
to all lanes once (like computing an address), rather than doing it 64 times
in parallel. This is more efficient for scalar-heavy code.

### Architecture

```
AMDComputeUnit (GCN-style)
┌─────────────────────────────────────────────────────────────────┐
│                                                                 │
│  Wavefront Scheduler                                            │
│  ┌────────────────────────────────────────────────────────┐     │
│  │ wf0: READY  wf1: STALLED  wf2: READY  wf3: READY ...  │     │
│  └────────────────────────────────────────────────────────┘     │
│                                                                 │
│  ┌──────────────┐ ┌──────────────┐                              │
│  │ SIMD Unit 0  │ │ SIMD Unit 1  │                              │
│  │ 16-wide ALU  │ │ 16-wide ALU  │                              │
│  │ VGPR: 256    │ │ VGPR: 256    │                              │
│  └──────────────┘ └──────────────┘                              │
│  ┌──────────────┐ ┌──────────────┐                              │
│  │ SIMD Unit 2  │ │ SIMD Unit 3  │                              │
│  │ 16-wide ALU  │ │ 16-wide ALU  │                              │
│  └──────────────┘ └──────────────┘                              │
│                                                                 │
│  ┌──────────────┐                                               │
│  │ Scalar Unit  │  ← executes once for all lanes                │
│  │ SGPR: 104    │  (address computation, flow control)          │
│  └──────────────┘                                               │
│                                                                 │
│  Shared Resources:                                              │
│  ┌────────────────────────────────────────────────┐             │
│  │ LDS (Local Data Share): 64 KB                   │             │
│  │ L1 Vector Cache: 16 KB                          │             │
│  │ L1 Scalar Cache: 16 KB                          │             │
│  │ L1 Instruction Cache: 32 KB                     │             │
│  └────────────────────────────────────────────────┘             │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### Configuration

```python
@dataclass
class AMDCUConfig:
    """Configuration for an AMD-style Compute Unit."""

    num_simd_units: int = 4
    wave_width: int = 64            # 64 (GCN), 32 (RDNA native)
    max_wavefronts: int = 40        # max resident wavefronts
    max_work_groups: int = 16
    scheduling_policy: SchedulingPolicy = SchedulingPolicy.LRR

    # Register files
    vgpr_per_simd: int = 256        # vector GPRs per SIMD unit
    sgpr_count: int = 104           # scalar GPRs (shared)

    # Memory
    lds_size: int = 65536           # 64 KB Local Data Share
    l1_vector_cache: int = 16384    # 16 KB
    l1_scalar_cache: int = 16384    # 16 KB
    l1_instruction_cache: int = 32768

    float_format: FloatFormat = FP32
    isa: InstructionSet = GenericISA()
    memory_latency_cycles: int = 200
```

## Compute Unit 3: MatrixMultiplyUnit (Google TPU MXU)

The TPU's MXU is fundamentally different — there are no threads, no warps,
no schedulers. Instead, it has:

1. **Systolic arrays** — the main compute engine (from Layer 8)
2. **Vector unit** — for element-wise operations (activation functions, etc.)
3. **Scalar unit** — for loop control and addressing
4. **Accumulators** — for storing partial matrix results

### Architecture

```
MatrixMultiplyUnit (TPU v2-style)
┌─────────────────────────────────────────────────────────────────┐
│                                                                 │
│  Control Sequencer                                              │
│  ┌──────────────────────────────────────────────────────┐       │
│  │ Tile schedule: load A[0:128], matmul, load A[128:256]│       │
│  └──────────────────────────────────────────────────────┘       │
│                                                                 │
│  ┌─────────────────────────────────────────────┐                │
│  │ Systolic Array (128×128)                     │                │
│  │                                              │                │
│  │   Weights pre-loaded into PEs               │                │
│  │   Activations stream in from left           │                │
│  │   Partial sums flow down to accumulators    │                │
│  │                                              │                │
│  └──────────────────────────────────────────────┘                │
│                    │                                             │
│                    ▼                                             │
│  ┌─────────────────────────────────────────────┐                │
│  │ Accumulators (128 × FP32)                    │                │
│  │ Store accumulated results from systolic array│                │
│  └──────────────────┬──────────────────────────┘                │
│                     │                                            │
│                     ▼                                            │
│  ┌─────────────────────────────────────────────┐                │
│  │ Vector Unit (128-wide)                       │                │
│  │ ReLU, sigmoid, add bias, normalize, etc.    │                │
│  └─────────────────────────────────────────────┘                │
│                                                                 │
│  ┌───────────────┐                                              │
│  │ Scalar Unit   │  ← loop control, tiling addresses            │
│  └───────────────┘                                              │
│                                                                 │
│  ┌──────────────────────────────┐                               │
│  │ HBM Interface                │  ← high-bandwidth memory      │
│  │ (900 GB/s on TPU v4)        │                                │
│  └──────────────────────────────┘                               │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### MXU Configuration

```python
@dataclass
class MXUConfig:
    """Configuration for a TPU-style Matrix Multiply Unit."""

    # Systolic array
    array_rows: int = 128
    array_cols: int = 128
    systolic_format: FloatFormat = BF16   # TPUs use BF16 for inputs
    accumulator_format: FloatFormat = FP32  # FP32 for accumulation

    # Vector unit
    vector_width: int = 128               # element-wise operations
    vector_format: FloatFormat = FP32

    # Memory
    accumulator_count: int = 128          # number of accumulator registers
    weight_buffer_size: int = 4194304     # 4 MB weight staging buffer
    activation_buffer_size: int = 2097152 # 2 MB activation buffer
```

### MXU Execution Model

The MXU doesn't "schedule warps." Instead, it processes **tiles** of a larger
matrix operation. The control sequencer manages the tiling:

```
Large matmul: C[1024×1024] = A[1024×1024] × B[1024×1024]

The MXU can only do 128×128 at a time, so:

for i in range(0, 1024, 128):        # 8 row tiles
    for j in range(0, 1024, 128):    # 8 column tiles
        acc = 0
        for k in range(0, 1024, 128):  # 8 reduction tiles
            load A[i:i+128, k:k+128] into activation buffer
            load B[k:k+128, j:j+128] into weight buffer → systolic array
            acc += systolic_matmul(A_tile, B_tile)
        C[i:i+128, j:j+128] = apply_vector_ops(acc)  # activation, etc.
```

This tiling loop is the "scheduling" — it's determined at compile time by
XLA, not at runtime by hardware.

## Compute Unit 4: XeCore (Intel)

Intel's Xe Core is a hybrid: it has SIMD execution units like AMD, but
with hardware threads like NVIDIA, wrapped in a unique organizational
structure.

### Configuration

```python
@dataclass
class XeCoreConfig:
    """Configuration for an Intel Xe Core."""

    num_eus: int = 16                 # Execution Units per Xe Core
    threads_per_eu: int = 7           # hardware threads per EU
    simd_width: int = 8               # SIMD8 operations
    grf_per_eu: int = 128             # General Register File entries

    # Shared resources
    slm_size: int = 65536             # 64 KB Shared Local Memory
    l1_cache_size: int = 196608       # 192 KB L1 cache
    instruction_cache_size: int = 65536

    scheduling_policy: SchedulingPolicy = SchedulingPolicy.ROUND_ROBIN
    float_format: FloatFormat = FP32
    isa: InstructionSet = GenericISA()
    memory_latency_cycles: int = 200
```

## Compute Unit 5: NeuralEngineCore (Apple ANE)

Apple's Neural Engine Core is the simplest compute unit — it's essentially
a MAC array with DMA and activation hardware, all compiler-scheduled.

### Configuration

```python
@dataclass
class ANECoreConfig:
    """Configuration for an Apple Neural Engine Core."""

    num_macs: int = 16                # MAC units per core
    mac_format: FloatFormat = FP16    # FP16 for inference
    accumulator_format: FloatFormat = FP32

    # On-chip memory
    sram_size: int = 4194304          # 4 MB on-chip SRAM
    activation_buffer: int = 131072   # 128 KB
    weight_buffer: int = 524288       # 512 KB
    output_buffer: int = 131072       # 128 KB

    # DMA for moving data to/from main memory
    dma_bandwidth: int = 10           # elements per cycle
```

## Shared Memory — A Deep Dive

Shared memory (NVIDIA) / LDS (AMD) / SLM (Intel) is one of the most important
concepts in GPU programming. It's a small, fast, programmer-managed scratchpad
that's visible to all threads in a thread block.

### Why shared memory matters

```
Global memory (VRAM):  ~400 cycles latency, ~1 TB/s bandwidth
Shared memory:         ~1-4 cycles latency, ~10 TB/s bandwidth
Registers:             0 cycles latency (immediate)
```

That's a 100× latency difference. If your kernel loads the same data multiple
times, you can load it once into shared memory and reuse it from there.

### Bank conflicts

Shared memory is divided into **banks** (typically 32). Each bank can serve
one request per cycle. If two threads access the same bank (but different
addresses), they serialize — this is a **bank conflict**.

```
32 banks, each 4 bytes wide:

Address 0x00 → Bank 0    Address 0x04 → Bank 1    ...    Address 0x7C → Bank 31
Address 0x80 → Bank 0    Address 0x84 → Bank 1    ...    Address 0xFC → Bank 31

Thread 0 reads bank 0  ──→ OK (parallel)
Thread 1 reads bank 1  ──→ OK (parallel)
Thread 2 reads bank 0  ──→ BANK CONFLICT with thread 0 (serializes!)
```

Our shared memory implementation tracks bank conflicts and reports them
in traces for educational purposes.

```python
@dataclass
class SharedMemory:
    """Shared memory with bank conflict detection.

    This is the programmer-visible scratchpad memory shared by all
    threads in a thread block. It's fast but small, and bank conflicts
    can reduce its effective bandwidth.
    """

    size: int                          # total bytes
    num_banks: int = 32                # bank count
    bank_width: int = 4                # bytes per bank
    _data: bytearray = field(init=False)
    _access_log: list[tuple[int, int]] = field(default_factory=list, init=False)

    def read(self, address: int, thread_id: int) -> float: ...
    def write(self, address: int, value: float, thread_id: int) -> None: ...
    def check_bank_conflicts(self, addresses: list[int]) -> list[list[int]]: ...
```

## Clock Integration

All compute units are clock-driven:

```python
clock = Clock(frequency_hz=1_500_000_000)  # 1.5 GHz (typical GPU clock)

sm = StreamingMultiprocessor(SMConfig(), clock)
clock.register_listener(lambda edge: sm.step(edge) if edge.is_rising else None)

# Dispatch a thread block
sm.dispatch(WorkItem(
    work_id=0,
    program=[limm(0, 2.0), limm(1, 3.0), fmul(2, 0, 1), halt()],
    thread_count=128,
))

# Run until complete
traces = sm.run()
print(f"Completed in {len(traces)} cycles, occupancy: {sm.occupancy:.1%}")
```

## Dependencies

- **parallel-execution-engine**: `WarpEngine`, `WavefrontEngine`, `SystolicArray`,
  `MACArrayEngine`, `SubsliceEngine`, `EngineTrace`, `ExecutionModel`
- **gpu-core**: `GPUCore`, `Instruction`, `InstructionSet`, `GenericISA`, all
  opcode helpers, `FPRegisterFile`, `LocalMemory`
- **fp-arithmetic**: `FloatBits`, `FloatFormat`, `FP32`/`FP16`/`BF16`, all FP ops
- **clock**: `Clock`, `ClockEdge`

## Package name

`compute-unit` across all languages (Ruby: `compute_unit`).

## Implementation order

1. **Protocols + enums** — ComputeUnit protocol, Architecture, WorkItem,
   WarpState, SchedulingPolicy, SharedMemory, ComputeUnitTrace
2. **StreamingMultiprocessor** — most complex, most educational
3. **AMDComputeUnit** — shows contrast with NVIDIA
4. **MatrixMultiplyUnit** — completely different (no warps, tiling-based)
5. **XeCore** — Intel's hybrid approach
6. **NeuralEngineCore** — simplest (compiler-scheduled)

## Test strategy

1. **Scheduling tests**: Dispatch work, verify scheduler picks correct warps/wavefronts
2. **Occupancy tests**: Vary register/shared-mem usage, verify occupancy computation
3. **Latency hiding tests**: Simulate memory stalls, verify scheduler switches warps
4. **Shared memory tests**: Read/write, bank conflict detection
5. **Thread block decomposition**: Verify blocks are split into correct number of warps
6. **Cross-architecture tests**: Same matmul on SM, CU, MXU — same results, different characteristics
7. **Resource exhaustion tests**: Dispatch too much work, verify proper error handling

Coverage target: 90%+ for protocols and scheduling logic, 85%+ for each compute unit.

## Implementation languages

Python, Ruby, Go, TypeScript, Rust — all five languages in the repo.
