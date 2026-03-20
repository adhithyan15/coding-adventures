"""Protocols — shared types for all compute unit simulators.

=== What is a Compute Unit? ===

A compute unit is the organizational structure that wraps execution engines
(Layer 8) with scheduling, shared memory, register files, and caches to form
a complete computational building block. Think of it as the "factory floor"
analogy from the spec:

    Workers         = execution engines (warps, wavefronts, systolic arrays)
    Floor manager   = warp/wavefront scheduler
    Shared toolbox  = shared memory / LDS (data accessible to all teams)
    Supply closet   = L1 cache (recent data kept nearby)
    Filing cabinets = register file (massive, partitioned among teams)
    Work orders     = thread blocks / work groups queued for execution

Every vendor has a different name for this level of the hierarchy:

    NVIDIA:   Streaming Multiprocessor (SM)
    AMD:      Compute Unit (CU) / Work Group Processor (WGP in RDNA)
    Intel:    Xe Core (or Subslice in older gen)
    Google:   Matrix Multiply Unit (MXU) + Vector/Scalar units
    Apple:    Neural Engine Core

Despite the naming differences, they all serve the same purpose: take
execution engines, add scheduling and shared resources, and present a
coherent compute unit to the device layer above.

=== Protocol-Based Design ===

Just like Layer 8 (parallel-execution-engine), we use Python protocols to
define a common interface that all compute units implement. This allows
higher layers to drive any compute unit uniformly, regardless of vendor.

A Protocol is Python's version of an "interface" or "trait" — any class
that has the right methods satisfies the protocol, no inheritance needed.
This is structural subtyping: if it looks like a compute unit and steps
like a compute unit, it IS a compute unit.
"""

from __future__ import annotations

import struct
from dataclasses import dataclass, field
from enum import Enum
from typing import TYPE_CHECKING, Protocol, runtime_checkable

if TYPE_CHECKING:
    from clock import ClockEdge
    from gpu_core import Instruction
    from parallel_execution_engine import EngineTrace


# ---------------------------------------------------------------------------
# Architecture — which vendor's compute unit this is
# ---------------------------------------------------------------------------


class Architecture(Enum):
    """Vendor architectures supported at the compute unit level.

    Each architecture represents a fundamentally different approach to
    organizing parallel computation. They are NOT interchangeable — each
    has unique scheduling strategies, memory hierarchies, and execution
    models.

    Comparison table:

        Architecture      | Scheduling    | Memory Model  | Execution
        ──────────────────┼───────────────┼───────────────┼──────────────
        NVIDIA SM         | Warp sched.   | Shared mem    | SIMT warps
        AMD CU            | Wave sched.   | LDS           | SIMD wavefronts
        Google MXU        | Compile-time  | Weight buffer | Systolic array
        Intel Xe Core     | Thread disp.  | SLM           | SIMD + threads
        Apple ANE Core    | Compiler      | SRAM + DMA    | Scheduled MAC
    """

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


# ---------------------------------------------------------------------------
# WarpState — possible states of a warp in the scheduler
# ---------------------------------------------------------------------------


class WarpState(Enum):
    """Possible states of a warp (or wavefront, or thread) in the scheduler.

    A warp moves through these states during its lifetime:

        READY ──→ RUNNING ──→ READY (if more instructions)
          │                      │
          │         ┌────────────┘
          │         │
          ├──→ STALLED_MEMORY ──→ READY (when data arrives)
          ├──→ STALLED_BARRIER ──→ READY (when all warps reach barrier)
          ├──→ STALLED_DEPENDENCY ──→ READY (when register available)
          └──→ COMPLETED

    The scheduler's job is to find a READY warp and issue it to an engine.
    When a warp stalls (e.g., on a memory access), the scheduler switches
    to another READY warp — this is how GPUs hide latency.
    """

    READY = "ready"
    """Warp has an instruction ready to issue. Can be scheduled."""

    RUNNING = "running"
    """Warp is currently executing on an engine this cycle."""

    STALLED_MEMORY = "stalled_memory"
    """Warp is waiting for a memory operation to complete.

    Memory accesses to global (off-chip) memory take ~200-400 cycles on a
    real GPU. During this time, the warp cannot execute and the scheduler
    must find another warp to keep the hardware busy.
    """

    STALLED_BARRIER = "stalled_barrier"
    """Warp is waiting at a __syncthreads() barrier.

    Thread block synchronization requires ALL warps in the block to reach
    the barrier before any can proceed. This is how threads cooperate
    through shared memory — write to shared mem, barrier, read from it.
    """

    STALLED_DEPENDENCY = "stalled_dependency"
    """Warp is waiting for a register dependency to resolve.

    This happens when an instruction needs the result of a previous
    instruction that hasn't completed yet (data hazard).
    """

    COMPLETED = "completed"
    """Warp has executed its HALT instruction. Done."""


# ---------------------------------------------------------------------------
# SchedulingPolicy — how the scheduler picks which warp to issue
# ---------------------------------------------------------------------------


class SchedulingPolicy(Enum):
    """How the warp scheduler picks which warp to issue next.

    Real GPUs use sophisticated scheduling policies that balance throughput,
    fairness, and latency hiding. Here are the most common ones:

        Policy       | Strategy              | Used by
        ─────────────┼───────────────────────┼──────────────
        ROUND_ROBIN  | Fair rotation         | Teaching, some AMD
        GREEDY       | Most-ready-first      | Throughput-focused
        OLDEST_FIRST | Longest-waiting-first | Fairness-focused
        GTO          | Same warp til stall   | NVIDIA (common)
        LRR          | Skip-stalled rotation | AMD (common)

    GTO (Greedy-Then-Oldest) is particularly interesting: it keeps issuing
    from the same warp until it stalls, then switches to the oldest ready
    warp. This reduces context-switch overhead because warps that don't
    stall get maximum throughput.
    """

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


# ---------------------------------------------------------------------------
# WorkItem — a unit of parallel work dispatched to a compute unit
# ---------------------------------------------------------------------------


@dataclass(frozen=True)
class WorkItem:
    """A unit of parallel work dispatched to a compute unit.

    In CUDA terms, this is a **thread block** (or cooperative thread array).
    In OpenCL terms, this is a **work group**.
    In TPU terms, this is a **tile** of a matrix operation.
    In NPU terms, this is an **inference tile**.

    The WorkItem is the bridge between the application (which says "compute
    this") and the hardware (which says "here are my execution engines").
    The compute unit takes a WorkItem and decomposes it into warps/wavefronts
    /tiles that can run on the engines.

    === Thread Block Decomposition (NVIDIA example) ===

    A WorkItem with thread_count=256 on an NVIDIA SM:

        WorkItem(thread_count=256)
        ├── Warp 0:  threads 0-31    (first 32 threads)
        ├── Warp 1:  threads 32-63
        ├── Warp 2:  threads 64-95
        ├── ...
        └── Warp 7:  threads 224-255 (last 32 threads)

    All 8 warps share the same shared memory and can synchronize with
    __syncthreads(). This is how threads cooperate on shared data.

    Fields:
        work_id:         Unique identifier for this work item.
        program:         Instruction list for instruction-stream architectures.
        thread_count:    Number of parallel threads/lanes in this block.
        per_thread_data: Per-thread initial register values.
                         per_thread_data[thread_id][register_index] = value
        input_data:      Activation matrix for dataflow architectures (TPU/NPU).
        weight_data:     Weight matrix for dataflow architectures.
        schedule:        MAC schedule for NPU-style architectures.
        shared_mem_bytes: Shared memory requested by this work item.
        registers_per_thread: Registers needed per thread (for occupancy calc).
    """

    work_id: int
    program: list[Instruction] | None = None
    thread_count: int = 32
    per_thread_data: dict[int, dict[int, float]] = field(default_factory=dict)

    # For dataflow architectures (TPU/NPU):
    input_data: list[list[float]] | None = None
    weight_data: list[list[float]] | None = None
    schedule: list | None = None

    # Resource requirements
    shared_mem_bytes: int = 0
    registers_per_thread: int = 32


# ---------------------------------------------------------------------------
# ComputeUnitTrace — record of one clock cycle across the compute unit
# ---------------------------------------------------------------------------


@dataclass(frozen=True)
class ComputeUnitTrace:
    """Record of one clock cycle across the entire compute unit.

    Captures scheduler decisions, engine activity, memory accesses, and
    resource utilization — everything needed to understand what the compute
    unit did in one cycle.

    === Why Trace Everything? ===

    Tracing is how you learn what GPUs actually do. Without traces, a GPU
    is a black box: data in, data out, who knows what happened inside.
    With traces, you can see:

    - Which warp the scheduler picked and why
    - How many warps are stalled on memory
    - What occupancy looks like cycle by cycle
    - Where bank conflicts happen in shared memory

    This is the same information that tools like NVIDIA Nsight Compute
    show for real GPUs. Our traces are simpler but serve the same
    educational purpose.

    Fields:
        cycle:               Clock cycle number.
        unit_name:           Which compute unit produced this trace.
        architecture:        Which vendor architecture.
        scheduler_action:    What the scheduler decided this cycle.
        active_warps:        How many warps/wavefronts are currently active.
        total_warps:         Maximum warps this unit can hold.
        engine_traces:       Per-engine traces (engine_id -> EngineTrace).
        shared_memory_used:  Bytes of shared memory in use.
        shared_memory_total: Total shared memory available.
        register_file_used:  Registers currently allocated.
        register_file_total: Total registers available.
        occupancy:           active_warps / max_warps (0.0 to 1.0).
        l1_hits:             L1 cache hits this cycle.
        l1_misses:           L1 cache misses this cycle.
    """

    cycle: int
    unit_name: str
    architecture: Architecture

    # Scheduler state
    scheduler_action: str
    active_warps: int
    total_warps: int

    # Per-engine traces (engine_id -> EngineTrace from Layer 8)
    engine_traces: dict[int, EngineTrace]

    # Resource utilization
    shared_memory_used: int
    shared_memory_total: int
    register_file_used: int
    register_file_total: int
    occupancy: float

    # Cache stats (if applicable)
    l1_hits: int = 0
    l1_misses: int = 0

    def format(self) -> str:
        """Pretty-print the trace for educational display.

        Returns a multi-line string showing scheduler action, occupancy,
        resource usage, and per-engine details.

        Example output:

            [Cycle 5] SM (nvidia_sm) — 75.0% occupancy (48/64 warps)
              Scheduler: issued warp 3 (GTO policy)
              Shared memory: 49152/98304 bytes (50.0%)
              Registers: 32768/65536 (50.0%)
              Engine 0: FMUL R2, R0, R1 — 32/32 threads active
              Engine 1: (idle)
        """
        occ_pct = f"{self.occupancy * 100:.1f}%"
        lines = [
            f"[Cycle {self.cycle}] {self.unit_name} "
            f"({self.architecture.value}) "
            f"— {occ_pct} occupancy "
            f"({self.active_warps}/{self.total_warps} warps)"
        ]
        lines.append(f"  Scheduler: {self.scheduler_action}")

        if self.shared_memory_total > 0:
            smem_pct = (
                self.shared_memory_used / self.shared_memory_total * 100
            )
            lines.append(
                f"  Shared memory: {self.shared_memory_used}"
                f"/{self.shared_memory_total} bytes ({smem_pct:.1f}%)"
            )

        if self.register_file_total > 0:
            reg_pct = (
                self.register_file_used / self.register_file_total * 100
            )
            lines.append(
                f"  Registers: {self.register_file_used}"
                f"/{self.register_file_total} ({reg_pct:.1f}%)"
            )

        for eid in sorted(self.engine_traces):
            lines.append(
                f"  Engine {eid}: {self.engine_traces[eid].description}"
            )

        return "\n".join(lines)


# ---------------------------------------------------------------------------
# SharedMemory — programmer-visible scratchpad with bank conflict detection
# ---------------------------------------------------------------------------


class SharedMemory:
    """Shared memory with bank conflict detection.

    === What is Shared Memory? ===

    Shared memory is a small, fast, programmer-managed scratchpad that's
    visible to all threads in a thread block. It's the GPU equivalent of
    a team whiteboard — everyone on the team can read and write to it.

    Performance comparison:

        Memory Level      | Latency    | Bandwidth
        ──────────────────┼────────────┼──────────────
        Registers         | 0 cycles   | unlimited
        Shared memory     | ~1-4 cycles| ~10 TB/s
        L1 cache          | ~30 cycles | ~2 TB/s
        Global (VRAM)     | ~400 cycles| ~1 TB/s

    That's a 100x latency difference between shared memory and global
    memory. Kernels that reuse data should load it into shared memory
    once and access it from there.

    === Bank Conflicts — The Hidden Performance Trap ===

    Shared memory is divided into **banks** (typically 32). Each bank can
    serve one request per cycle. If two threads access the same bank but
    at different addresses, they **serialize** — this is a bank conflict.

    Bank mapping (32 banks, 4 bytes per bank):

        Address 0x00 -> Bank 0    Address 0x04 -> Bank 1    ...
        Address 0x80 -> Bank 0    Address 0x84 -> Bank 1    ...

    The bank for an address is: (address // bank_width) % num_banks

    No conflict example (each thread hits a different bank):
        Thread 0 -> addr 0x00 (bank 0)   Thread 1 -> addr 0x04 (bank 1)
        Thread 2 -> addr 0x08 (bank 2)   Thread 3 -> addr 0x0C (bank 3)
        All 4 accesses happen in parallel. 1 cycle total.

    2-way conflict example:
        Thread 0 -> addr 0x00 (bank 0)   Thread 1 -> addr 0x80 (bank 0)  CONFLICT!
        Thread 2 -> addr 0x08 (bank 2)   Thread 3 -> addr 0x0C (bank 3)
        Threads 0 and 1 serialize. 2 cycles total.

    Fields:
        size:       Total bytes of shared memory.
        num_banks:  Number of memory banks (typically 32).
        bank_width: Bytes per bank (typically 4).
    """

    def __init__(
        self,
        size: int,
        num_banks: int = 32,
        bank_width: int = 4,
    ) -> None:
        self.size = size
        self.num_banks = num_banks
        self.bank_width = bank_width
        self._data = bytearray(size)
        self._total_accesses = 0
        self._total_conflicts = 0

    def read(self, address: int, thread_id: int) -> float:
        """Read a 4-byte float from shared memory.

        Args:
            address:   Byte address to read from (must be 4-byte aligned).
            thread_id: Which thread is reading (for conflict tracking).

        Returns:
            The float value at that address.

        Raises:
            IndexError: If address is out of range.
        """
        if address < 0 or address + 4 > self.size:
            msg = f"Shared memory address {address} out of range [0, {self.size})"
            raise IndexError(msg)
        self._total_accesses += 1
        raw = self._data[address : address + 4]
        return struct.unpack("<f", raw)[0]

    def write(
        self, address: int, value: float, thread_id: int
    ) -> None:
        """Write a 4-byte float to shared memory.

        Args:
            address:   Byte address to write to (must be 4-byte aligned).
            value:     The float value to write.
            thread_id: Which thread is writing (for conflict tracking).

        Raises:
            IndexError: If address is out of range.
        """
        if address < 0 or address + 4 > self.size:
            msg = f"Shared memory address {address} out of range [0, {self.size})"
            raise IndexError(msg)
        self._total_accesses += 1
        raw = struct.pack("<f", value)
        self._data[address : address + 4] = raw

    def check_bank_conflicts(
        self, addresses: list[int]
    ) -> list[list[int]]:
        """Detect bank conflicts for a set of simultaneous accesses.

        Given a list of addresses (one per thread), determine which
        accesses conflict (hit the same bank). Returns a list of conflict
        groups — each group is a list of thread indices that conflict.

        === How Bank Conflict Detection Works ===

        1. Compute the bank for each address:
           bank = (address // bank_width) % num_banks

        2. Group threads by bank.

        3. Any bank accessed by more than one thread is a conflict.
           The threads in that bank must serialize — taking N cycles
           for N conflicting accesses instead of 1 cycle.

        Args:
            addresses: List of byte addresses, one per thread.

        Returns:
            List of conflict groups. Each group is a list of thread
            indices that conflict with each other. Groups of size 1
            (no conflict) are NOT included — only actual conflicts.

        Example:
            >>> smem = SharedMemory(size=1024)
            >>> # Threads 0 and 2 both hit bank 0 (addresses 0 and 128)
            >>> smem.check_bank_conflicts([0, 4, 128, 12])
            [[0, 2]]  # threads 0 and 2 conflict on bank 0
        """
        # Map bank -> list of thread indices
        bank_to_threads: dict[int, list[int]] = {}
        for thread_idx, addr in enumerate(addresses):
            bank = (addr // self.bank_width) % self.num_banks
            if bank not in bank_to_threads:
                bank_to_threads[bank] = []
            bank_to_threads[bank].append(thread_idx)

        # Find conflicts (banks with more than one thread)
        conflicts: list[list[int]] = []
        for threads in bank_to_threads.values():
            if len(threads) > 1:
                conflicts.append(threads)
                self._total_conflicts += len(threads) - 1

        return conflicts

    def reset(self) -> None:
        """Clear all data and reset statistics."""
        self._data = bytearray(self.size)
        self._total_accesses = 0
        self._total_conflicts = 0

    @property
    def total_accesses(self) -> int:
        """Total number of read/write accesses."""
        return self._total_accesses

    @property
    def total_conflicts(self) -> int:
        """Total bank conflicts detected."""
        return self._total_conflicts


# ---------------------------------------------------------------------------
# ComputeUnit Protocol — the unified interface
# ---------------------------------------------------------------------------


@runtime_checkable
class ComputeUnit(Protocol):
    """Any compute unit: SM, CU, MXU, Xe Core, ANE Core.

    A compute unit manages multiple execution engines, schedules work
    across them, and provides shared resources. It's the integration
    point between raw parallel execution and the device layer above.

    === Why a Protocol? ===

    Despite radical differences between NVIDIA SMs, AMD CUs, and Google
    MXUs, they all share this common interface:

    1. dispatch(work) — accept work
    2. step(clock_edge) — advance one cycle
    3. run(max_cycles) — run until done
    4. idle — is all work complete?
    5. reset() — clear all state

    This lets the device layer above treat all compute units uniformly,
    the same way a factory manager can manage different production lines
    without knowing the details of each machine.
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
        """Accept a work item (thread block, work group, tile).

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
