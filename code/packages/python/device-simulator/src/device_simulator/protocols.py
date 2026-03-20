"""Protocols — shared types for all device simulators.

=== What is a Device Simulator? ===

A device simulator models a **complete accelerator** — not just one compute
unit, but the entire chip with all its compute units, global memory, caches,
and the work distributor that ties them together.

Think of it as the difference between simulating one factory floor (Layer 7)
versus simulating the entire factory complex:

    Layer 7 (Compute Unit):    One SM / CU / MXU — a single factory floor
    Layer 6 (Device):          The whole factory — all floors + warehouse +
                               shipping dock + floor manager's office

The device layer adds four new concepts:

1. **Global Memory (VRAM)** — the large device-wide memory (the warehouse).
   All compute units share it. High bandwidth but high latency (~400 cycles).

2. **L2 Cache** — sits between compute units and global memory. Reduces the
   average latency for frequently-accessed data.

3. **Work Distributor** — takes kernel launches (work orders) and assigns
   thread blocks to compute units that have available resources.

4. **Host Interface** — the connection to the CPU. Data must be copied from
   CPU memory to device memory before the GPU can use it (except on Apple's
   unified memory, where it's zero-copy).

=== Protocol-Based Design ===

Like Layers 7-9, we use Python Protocols. The same AcceleratorDevice interface
works for NVIDIA GPUs, AMD GPUs, Google TPUs, Intel GPUs, and Apple ANEs.
The device layer above (ISA Simulator, Layer 5) can drive any device uniformly.

=== Memory Hierarchy at the Device Level ===

                ┌──────────────┐
    CPU RAM ──► │ Host Interface│ ──► PCIe / NVLink / unified
                └──────┬───────┘
                       │
                ┌──────┴───────┐
                │ Global Memory │  24-80 GB, ~400 cycle latency
                │  (HBM/GDDR)  │  1-3 TB/s bandwidth
                └──────┬───────┘
                       │
                ┌──────┴───────┐
                │   L2 Cache   │  4-96 MB, ~200 cycle latency
                │  (shared)    │
                └──┬───┬───┬───┘
                   │   │   │
                 CU 0 CU 1 ... CU N   (each with local shared memory)
"""

from __future__ import annotations

import math
import struct
from collections import deque
from dataclasses import dataclass, field
from enum import Enum
from typing import TYPE_CHECKING, Any, Protocol, runtime_checkable

if TYPE_CHECKING:
    from clock import ClockEdge
    from compute_unit import ComputeUnit, ComputeUnitTrace
    from gpu_core import Instruction


# =========================================================================
# MemoryTransaction — a single wide memory access after coalescing
# =========================================================================


@dataclass(frozen=True)
class MemoryTransaction:
    """A single wide memory transaction after coalescing.

    When 32 threads in a warp each request 4 bytes, those 128 bytes of
    requests might coalesce into a single 128-byte transaction (best case)
    or 32 separate transactions (worst case — scattered access).

    === Coalescing Visual ===

    Best case (1 transaction):
        Thread  0  1  2  3  4  ...  31
        Addr   [0][4][8][12][16]...[124]
               └──────────────────────┘
                 One 128B transaction

    Worst case (32 transactions):
        Thread  0     1      2      3
        Addr   [0]  [512]  [1024]  [1536]  ...
                │      │      │      │
                ▼      ▼      ▼      ▼
             Trans 1 Trans 2 Trans 3 Trans 4

    Fields:
        address:     Aligned start address of the transaction.
        size:        Transaction size in bytes (32, 64, or 128).
        thread_mask: Bitmask of which threads are served by this transaction.
                     Bit i is set if thread i's request falls in this range.
    """

    address: int
    size: int
    thread_mask: int


# =========================================================================
# GlobalMemoryStats — tracks memory access patterns and efficiency
# =========================================================================


@dataclass
class GlobalMemoryStats:
    """Tracks memory access patterns and efficiency.

    === Why Track These? ===

    Memory access patterns are the #1 performance bottleneck on GPUs.
    A kernel that achieves perfect coalescing uses 32x less bandwidth than
    one with fully scattered access. These stats tell you whether your
    memory accesses are efficient.

    Key metric: **coalescing_efficiency**
        = total_requests / total_transactions
        Ideal = 1.0 (every request coalesces into existing transactions)
        Worst = 32.0 for 32-wide warps (nothing coalesces)

    Fields:
        total_reads:          Number of read operations.
        total_writes:         Number of write operations.
        total_transactions:   Memory transactions after coalescing.
        total_requests:       Memory requests before coalescing (per-thread).
        bytes_transferred:    Total bytes moved through the memory system.
        coalescing_efficiency: requests / transactions (higher = better).
        partition_conflicts:  Times multiple requests hit same memory channel.
        host_to_device_bytes: Bytes copied from CPU to device.
        device_to_host_bytes: Bytes copied from device to CPU.
        host_transfer_cycles: Total cycles spent on host transfers.
    """

    total_reads: int = 0
    total_writes: int = 0
    total_transactions: int = 0
    total_requests: int = 0
    bytes_transferred: int = 0
    coalescing_efficiency: float = 0.0
    partition_conflicts: int = 0
    host_to_device_bytes: int = 0
    device_to_host_bytes: int = 0
    host_transfer_cycles: int = 0

    def update_efficiency(self) -> None:
        """Recalculate coalescing efficiency from current counts."""
        if self.total_transactions > 0:
            self.coalescing_efficiency = (
                self.total_requests / self.total_transactions
            )


# =========================================================================
# KernelDescriptor — what gets launched on the device
# =========================================================================


@dataclass(frozen=True)
class KernelDescriptor:
    """Describes a kernel launch (GPU) or operation (TPU/NPU).

    === Two Worlds ===

    GPU-style devices (NVIDIA, AMD, Intel) receive a **program** with grid
    and block dimensions — "run this code on this many threads."

    Dataflow-style devices (TPU, NPU) receive an **operation** with input
    and weight data — "multiply these matrices" or "apply this activation."

    The same KernelDescriptor handles both by having fields for each style.
    GPU devices use the program/grid/block fields. Dataflow devices use the
    operation/input/weight fields.

    === GPU Example ===

        # SAXPY: Y = alpha * X + Y
        kernel = KernelDescriptor(
            name="saxpy",
            kernel_id=0,
            program=[limm(0, alpha), load(1, ...), fmul(2, 0, 1), ...],
            grid_dim=(256, 1, 1),     # 256 blocks
            block_dim=(256, 1, 1),    # 256 threads per block
            shared_mem_bytes=0,
            registers_per_thread=8,
        )
        # Total: 256 × 256 = 65,536 threads

    === Dataflow Example ===

        # Matrix multiply: C = A × B
        kernel = KernelDescriptor(
            name="matmul",
            kernel_id=0,
            operation="matmul",
            input_data=A,     # MxK matrix
            weight_data=B,    # KxN matrix
        )
    """

    # Common fields
    name: str = "unnamed"
    kernel_id: int = 0

    # GPU-style fields
    program: list[Instruction] | None = None
    grid_dim: tuple[int, int, int] = (1, 1, 1)
    block_dim: tuple[int, int, int] = (32, 1, 1)
    shared_mem_bytes: int = 0
    registers_per_thread: int = 32

    # Dataflow-style fields (TPU/NPU)
    operation: str = ""
    input_data: list[list[float]] | None = None
    weight_data: list[list[float]] | None = None
    output_address: int = 0

    @property
    def total_threads(self) -> int:
        """Total number of threads across all blocks."""
        gx, gy, gz = self.grid_dim
        bx, by, bz = self.block_dim
        return gx * gy * gz * bx * by * bz

    @property
    def total_blocks(self) -> int:
        """Total number of thread blocks in the grid."""
        gx, gy, gz = self.grid_dim
        return gx * gy * gz

    @property
    def threads_per_block(self) -> int:
        """Number of threads in each block."""
        bx, by, bz = self.block_dim
        return bx * by * bz


# =========================================================================
# DeviceConfig — full device specification
# =========================================================================


@dataclass
class DeviceConfig:
    """Complete device specification.

    === The Knobs That Define a Device ===

    Every accelerator is characterized by:
    - How many compute units it has
    - How much and how fast its memory is
    - How it connects to the CPU
    - How it distributes work

    By changing these parameters, the same device simulator code can model
    anything from a laptop GPU to a datacenter TPU.

    === Memory Hierarchy Parameters ===

    The memory hierarchy is the heart of GPU performance:

        Host RAM ──[host_bandwidth]──► Global Memory (VRAM)
                                            │
                                    [global_memory_bandwidth]
                                            │
                                       L2 Cache
                                            │
                                    Compute Units (shared memory)
                                            │
                                       Registers

    Each level is faster but smaller. The config specifies the size,
    bandwidth, and latency at each level.
    """

    # Identity
    name: str = "Generic Accelerator"
    architecture: str = "generic"

    # Compute
    num_compute_units: int = 4
    cu_config: Any = None

    # Memory hierarchy
    l2_cache_size: int = 4 * 1024 * 1024  # 4 MB
    l2_cache_latency: int = 200
    l2_cache_associativity: int = 16
    l2_cache_line_size: int = 128

    global_memory_size: int = 16 * 1024 * 1024 * 1024  # 16 GB
    global_memory_bandwidth: float = 1000.0  # bytes per cycle
    global_memory_latency: int = 400  # cycles
    memory_channels: int = 8

    # Host interface
    host_bandwidth: float = 64.0  # bytes per cycle
    host_latency: int = 1000  # cycles
    unified_memory: bool = False

    # Scheduling
    max_concurrent_kernels: int = 1
    work_distribution_policy: str = "round_robin"


# =========================================================================
# Vendor-specific configs
# =========================================================================


@dataclass
class ShaderEngineConfig:
    """AMD Shader Engine — mid-level grouping of CUs.

    AMD organizes CUs into Shader Engines, each sharing a geometry
    processor and rasterizer. For compute workloads, the main effect
    is that the Command Processor assigns work at the SE level first.
    """

    cus_per_engine: int = 16
    shared_l1_size: int = 32 * 1024  # 32 KB


@dataclass
class AmdGPUConfig(DeviceConfig):
    """AMD-specific config with Shader Engine hierarchy."""

    num_shader_engines: int = 6
    se_config: ShaderEngineConfig = field(default_factory=ShaderEngineConfig)
    infinity_cache_size: int = 96 * 1024 * 1024  # 96 MB
    infinity_cache_latency: int = 50
    num_aces: int = 4  # Asynchronous Compute Engines


@dataclass
class XeSliceConfig:
    """Intel Xe-Slice — mid-level grouping of Xe-Cores.

    Intel groups Xe-Cores into Xe-Slices that share an L1 cache.
    Similar to AMD's Shader Engines but at a different granularity.
    """

    xe_cores_per_slice: int = 4
    l1_cache_per_slice: int = 192 * 1024  # 192 KB


@dataclass
class IntelGPUConfig(DeviceConfig):
    """Intel-specific config with Xe-Slice hierarchy."""

    num_xe_slices: int = 8
    slice_config: XeSliceConfig = field(default_factory=XeSliceConfig)


@dataclass
class ICILink:
    """One ICI link to another TPU chip.

    TPU pods use Inter-Chip Interconnect (ICI) to connect multiple
    TPU chips in a 4D torus topology. Each link provides high-bandwidth,
    low-latency communication for collective operations (all-reduce, etc.)
    """

    target_chip_id: int = 0
    bandwidth: float = 500.0  # bytes per cycle
    latency: int = 500  # cycles


@dataclass
class TPUConfig(DeviceConfig):
    """TPU-specific config with Vector/Scalar units and ICI."""

    vector_unit_width: int = 128
    scalar_registers: int = 32
    transpose_unit: bool = True
    ici_links: list[ICILink] = field(default_factory=list)


@dataclass
class ANEConfig(DeviceConfig):
    """Apple ANE-specific config with DMA and SRAM.

    The ANE is unique: it shares unified memory with CPU and GPU,
    eliminating the PCIe transfer bottleneck entirely. The 'copy'
    operation just remaps page tables — zero cycles, zero bytes moved.
    """

    shared_sram_size: int = 32 * 1024 * 1024  # 32 MB
    sram_bandwidth: float = 1000.0  # bytes per cycle (very fast on-chip)
    sram_latency: int = 5  # cycles
    dma_channels: int = 4
    dma_bandwidth: float = 100.0  # bytes per cycle


# =========================================================================
# Default configs — model real hardware
# =========================================================================


def default_nvidia_config() -> DeviceConfig:
    """H100-like configuration (scaled down for simulation)."""
    return DeviceConfig(
        name="NVIDIA H100",
        architecture="nvidia_sm",
        num_compute_units=132,
        l2_cache_size=50 * 1024 * 1024,
        l2_cache_latency=200,
        l2_cache_associativity=32,
        l2_cache_line_size=128,
        global_memory_size=80 * 1024 * 1024 * 1024,
        global_memory_bandwidth=3350.0,
        global_memory_latency=400,
        memory_channels=8,
        host_bandwidth=64.0,
        host_latency=1000,
        unified_memory=False,
        max_concurrent_kernels=128,
        work_distribution_policy="round_robin",
    )


def default_amd_config() -> AmdGPUConfig:
    """RX 7900 XTX-like configuration."""
    return AmdGPUConfig(
        name="AMD RX 7900 XTX",
        architecture="amd_cu",
        num_compute_units=96,
        l2_cache_size=6 * 1024 * 1024,
        l2_cache_latency=150,
        l2_cache_associativity=16,
        l2_cache_line_size=128,
        global_memory_size=24 * 1024 * 1024 * 1024,
        global_memory_bandwidth=960.0,
        global_memory_latency=350,
        memory_channels=6,
        host_bandwidth=32.0,
        host_latency=1000,
        unified_memory=False,
        max_concurrent_kernels=8,
        work_distribution_policy="round_robin",
        num_shader_engines=6,
        se_config=ShaderEngineConfig(cus_per_engine=16, shared_l1_size=32 * 1024),
        infinity_cache_size=96 * 1024 * 1024,
        infinity_cache_latency=50,
        num_aces=4,
    )


def default_tpu_config() -> TPUConfig:
    """TPU v4-like configuration."""
    return TPUConfig(
        name="Google TPU v4",
        architecture="google_mxu",
        num_compute_units=1,
        l2_cache_size=0,
        l2_cache_latency=0,
        l2_cache_associativity=0,
        l2_cache_line_size=128,
        global_memory_size=32 * 1024 * 1024 * 1024,
        global_memory_bandwidth=1200.0,
        global_memory_latency=300,
        memory_channels=4,
        host_bandwidth=500.0,
        host_latency=500,
        unified_memory=False,
        max_concurrent_kernels=1,
        work_distribution_policy="sequential",
        vector_unit_width=128,
        scalar_registers=32,
        transpose_unit=True,
    )


def default_intel_config() -> IntelGPUConfig:
    """Arc A770-like configuration."""
    return IntelGPUConfig(
        name="Intel Arc A770",
        architecture="intel_xe_core",
        num_compute_units=32,
        l2_cache_size=16 * 1024 * 1024,
        l2_cache_latency=180,
        l2_cache_associativity=16,
        l2_cache_line_size=128,
        global_memory_size=16 * 1024 * 1024 * 1024,
        global_memory_bandwidth=512.0,
        global_memory_latency=350,
        memory_channels=4,
        host_bandwidth=32.0,
        host_latency=1000,
        unified_memory=False,
        max_concurrent_kernels=16,
        work_distribution_policy="round_robin",
        num_xe_slices=8,
        slice_config=XeSliceConfig(xe_cores_per_slice=4, l1_cache_per_slice=192 * 1024),
    )


def default_apple_config() -> ANEConfig:
    """M3 Max ANE-like configuration."""
    return ANEConfig(
        name="Apple M3 Max ANE",
        architecture="apple_ane_core",
        num_compute_units=16,
        l2_cache_size=0,
        l2_cache_latency=0,
        l2_cache_associativity=0,
        l2_cache_line_size=128,
        global_memory_size=128 * 1024 * 1024 * 1024,
        global_memory_bandwidth=200.0,
        global_memory_latency=100,
        memory_channels=8,
        host_bandwidth=200.0,
        host_latency=0,
        unified_memory=True,
        max_concurrent_kernels=1,
        work_distribution_policy="scheduled",
        shared_sram_size=32 * 1024 * 1024,
        sram_bandwidth=1000.0,
        sram_latency=5,
        dma_channels=4,
        dma_bandwidth=100.0,
    )


# =========================================================================
# DeviceTrace — cycle-by-cycle visibility into the whole device
# =========================================================================


@dataclass(frozen=True)
class DeviceTrace:
    """One cycle of device-wide activity.

    === Why Trace the Whole Device? ===

    At the compute unit level (Layer 7), traces show what one SM/CU is doing.
    At the device level, we need to see all compute units simultaneously, plus
    the memory system and work distributor. This is the information that tools
    like NVIDIA Nsight Systems show — the big picture of device utilization.

    Key questions a DeviceTrace answers:
    - How many compute units are busy vs idle?
    - Is the memory system a bottleneck (high bandwidth utilization)?
    - Is the work distributor keeping up (many pending blocks)?
    - What's the overall device occupancy?
    """

    cycle: int
    device_name: str

    # Work distribution
    distributor_actions: tuple[str, ...] = ()
    pending_blocks: int = 0
    active_blocks: int = 0

    # Per-CU traces (can be empty for idle CUs)
    cu_traces: tuple[ComputeUnitTrace, ...] = ()

    # Memory system
    l2_hits: int = 0
    l2_misses: int = 0
    memory_transactions: int = 0
    memory_bandwidth_used: float = 0.0

    # Aggregate metrics
    total_active_warps: int = 0
    device_occupancy: float = 0.0
    flops_this_cycle: int = 0

    def format(self) -> str:
        """Human-readable summary of this cycle.

        Example output:

            [Cycle 10] NVIDIA H100 — 45.2% occupancy
              Distributor: Block 42 → SM 7, Block 43 → SM 12
              Pending: 890 blocks, Active: 1056 blocks
              L2: 342 hits, 12 misses (96.6% hit rate)
              Memory: 8 transactions, 45.2% bandwidth
              Active warps: 4234 / 8448 across 132 SMs
        """
        lines = [
            f"[Cycle {self.cycle}] {self.device_name} "
            f"— {self.device_occupancy * 100:.1f}% occupancy"
        ]

        if self.distributor_actions:
            actions_str = ", ".join(self.distributor_actions)
            lines.append(f"  Distributor: {actions_str}")

        lines.append(
            f"  Pending: {self.pending_blocks} blocks, "
            f"Active: {self.active_blocks} blocks"
        )

        total_l2 = self.l2_hits + self.l2_misses
        if total_l2 > 0:
            hit_rate = self.l2_hits / total_l2 * 100
            lines.append(
                f"  L2: {self.l2_hits} hits, {self.l2_misses} misses "
                f"({hit_rate:.1f}% hit rate)"
            )

        lines.append(
            f"  Memory: {self.memory_transactions} transactions, "
            f"{self.memory_bandwidth_used * 100:.1f}% bandwidth"
        )

        lines.append(f"  Active warps: {self.total_active_warps}")

        return "\n".join(lines)


# =========================================================================
# DeviceStats — aggregate metrics across the entire simulation
# =========================================================================


@dataclass
class DeviceStats:
    """Device-wide aggregate statistics.

    === Performance Analysis ===

    These stats answer the key performance questions:

    1. **Compute utilization**: Are the compute units busy or sitting idle?
       compute_utilization = achieved_flops / peak_flops

    2. **Memory bandwidth utilization**: Is the memory system saturated?
       memory_bandwidth_utilization = achieved_bw / peak_bw

    3. **Load imbalance**: Are some CUs doing more work than others?
       load_imbalance = std_dev(blocks_per_cu) / mean(blocks_per_cu)

    4. **L2 effectiveness**: Is the cache helping?
       l2_hit_rate = l2_hits / (l2_hits + l2_misses)

    A well-optimized kernel has high compute utilization, high memory
    bandwidth utilization (for memory-bound kernels), low load imbalance,
    and a reasonable L2 hit rate.
    """

    # Time
    total_cycles: int = 0
    active_cycles: int = 0
    idle_cycles: int = 0

    # Compute
    total_flops: int = 0
    achieved_tflops: float = 0.0
    peak_tflops: float = 0.0
    compute_utilization: float = 0.0

    # Memory
    global_memory_stats: GlobalMemoryStats = field(
        default_factory=GlobalMemoryStats
    )
    l2_hit_rate: float = 0.0
    memory_bandwidth_utilization: float = 0.0

    # Work distribution
    total_kernels_launched: int = 0
    total_blocks_dispatched: int = 0
    avg_blocks_per_cu: float = 0.0
    load_imbalance: float = 0.0

    # Per-CU breakdown
    per_cu_active_cycles: list[int] = field(default_factory=list)
    per_cu_occupancy: list[float] = field(default_factory=list)


# =========================================================================
# AcceleratorDevice Protocol — the unified device interface
# =========================================================================


@runtime_checkable
class AcceleratorDevice(Protocol):
    """Any accelerator device: GPU, TPU, NPU.

    This is the top-level interface for Layer 6. The ISA Simulator (Layer 5)
    and Runtime Simulator (Layer 4) will interact with devices through
    this protocol.

    === Why One Interface for All Devices? ===

    Despite radical differences between a GPU (thread-parallel, thousands of
    cores) and a TPU (dataflow, one large matrix unit), they share a common
    lifecycle:

        1. Allocate device memory
        2. Copy data from host to device
        3. Launch computation
        4. Wait for completion
        5. Copy results back to host

    This protocol captures that common lifecycle while leaving the
    implementation details to each device type.
    """

    @property
    def name(self) -> str:
        """Device name ('NVIDIA H100', 'Apple M3 Max ANE', etc.)."""
        ...

    @property
    def config(self) -> DeviceConfig:
        """Full device configuration."""
        ...

    # --- Memory management ---

    def malloc(self, size: int) -> int:
        """Allocate device memory. Returns device pointer (address)."""
        ...

    def free(self, address: int) -> None:
        """Free device memory allocation."""
        ...

    def memcpy_host_to_device(self, dst: int, data: bytes) -> int:
        """Copy from host to device. Returns cycles consumed."""
        ...

    def memcpy_device_to_host(
        self, src: int, size: int
    ) -> tuple[bytes, int]:
        """Copy from device to host. Returns (data, cycles)."""
        ...

    # --- Kernel launch ---

    def launch_kernel(self, kernel: KernelDescriptor) -> None:
        """Submit a kernel for execution."""
        ...

    # --- Simulation ---

    def step(self, clock_edge: ClockEdge) -> DeviceTrace:
        """Advance the entire device by one clock cycle."""
        ...

    def run(self, max_cycles: int) -> list[DeviceTrace]:
        """Run until all kernels complete or max_cycles reached."""
        ...

    @property
    def idle(self) -> bool:
        """True when all CUs are idle and no pending work remains."""
        ...

    def reset(self) -> None:
        """Reset all state — CUs, memory, caches, work queues."""
        ...

    # --- Observability ---

    @property
    def stats(self) -> DeviceStats:
        """Aggregate statistics across all compute units and memory."""
        ...

    @property
    def compute_units(self) -> list[ComputeUnit]:
        """Direct access to individual compute units."""
        ...
