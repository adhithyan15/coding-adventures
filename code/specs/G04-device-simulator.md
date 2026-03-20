# G04 — Device Simulator

## Overview

This package implements **Layer 6 of the accelerator computing stack** — the
full device simulator that assembles multiple compute units (Layer 7) into a
complete accelerator. This is the jump from "one SM" to "a whole GPU."

Layer 7 gave us individual compute units — an SM that can schedule warps, an
MXU that can tile and multiply matrices, an ANE core that can run MAC schedules.
Layer 6 takes **many** of those compute units, gives them a shared global memory
system, and adds a **work distributor** that assigns work items to compute units.

This is where the full device-level picture comes together:

- **NVIDIA GPU**: 132 SMs + 80GB HBM3 + L2 cache + GigaThread Engine
- **AMD GPU**: 120 CUs in Shader Engines + Infinity Cache + GDDR6
- **Google TPU**: MXU + Vector + Scalar units + HBM + ICI mesh
- **Intel GPU**: Xe-Cores in Xe-Slices + L2 + HBM/GDDR
- **Apple ANE**: 16 Neural Engine cores + shared SRAM + DMA + unified memory

## Layer position

```
Layer 11: Logic Gates (AND, OR, XOR, NAND)
    │
Layer 10: FP Arithmetic (IEEE 754 add/mul/fma)
    │
Layer 9:  Accelerator Core (gpu-core) — one core, one instruction at a time
    │
Layer 8:  Parallel Execution Engine — warps, wavefronts, systolic arrays
    │
Layer 7:  Compute Unit — SM, CU, MXU, XeCore, ANECore
    │
Layer 6:  Device Simulator ← YOU ARE HERE
    │
    ├──→ GPU (NVIDIA): NvidiaGPU  — many SMs + HBM + L2 + GigaThread
    ├──→ GPU (AMD):    AmdGPU     — CUs in Shader Engines + Infinity Cache
    ├──→ TPU (Google): GoogleTPU  — MXU + Vector + Scalar + HBM + ICI
    ├──→ GPU (Intel):  IntelGPU   — Xe-Cores in Xe-Slices + L2
    └──→ NPU (Apple):  AppleANE  — NE cores + SRAM + DMA + unified memory
    │
Layer 5:  ISA Simulator — PTX / HLO / ANE Instructions (future)
```

**Depends on:**
- `compute-unit` (Layer 7) — SM, CU, MXU, XeCore, ANECore
- `cache` (existing) — L2 cache simulation
- `clock` — cycle-driven simulation
- `fp-arithmetic` — shared FP operations

**Used by:** ISA Simulator (Layer 5, future), Runtime Simulator (Layer 4, future)

## The Big Picture: What Makes a "Device"?

A compute unit by itself is useful but limited. It has no way to get data from
the outside world. It has a small amount of shared memory (48-228 KB) but real
workloads need gigabytes. And it doesn't know about the other compute units
sitting right next to it on the same chip.

A **device** wraps compute units with everything they need:

```
┌─────────────────────────────────────────────────────────────┐
│                    Accelerator Device                        │
│                                                              │
│  ┌──────────────────────────────────────────────────────┐   │
│  │              Work Distributor / Command Processor      │   │
│  │  "Here's a kernel with 10,000 threads. Go."          │   │
│  │  Splits into thread blocks → assigns to compute units │   │
│  └──────────┬──────────┬──────────┬──────────┬──────────┘   │
│             │          │          │          │               │
│  ┌──────────┴┐  ┌──────┴─────┐  ┌┴─────────┐  ┌──────────┐ │
│  │  CU #0    │  │  CU #1     │  │  CU #2   │  │  CU #N   │ │
│  │ (SM/CU/   │  │ (SM/CU/    │  │          │  │          │ │
│  │  XeCore)  │  │  XeCore)   │  │  . . .   │  │          │ │
│  │           │  │            │  │          │  │          │ │
│  │ SharedMem │  │ SharedMem  │  │ SharedMem│  │ SharedMem│ │
│  └─────┬─────┘  └─────┬──────┘  └────┬─────┘  └────┬─────┘ │
│        │              │              │              │        │
│  ┌─────┴──────────────┴──────────────┴──────────────┴─────┐ │
│  │                    L2 Cache (shared)                     │ │
│  │            4-64 MB, ~200 cycle latency                  │ │
│  └────────────────────────┬────────────────────────────────┘ │
│                           │                                  │
│  ┌────────────────────────┴────────────────────────────────┐ │
│  │              Memory Controllers (4-12 channels)          │ │
│  │         Bandwidth: 1-3 TB/s, Latency: ~400 cycles       │ │
│  └────────────────────────┬────────────────────────────────┘ │
│                           │                                  │
│  ┌────────────────────────┴────────────────────────────────┐ │
│  │              Global Memory (VRAM / HBM)                  │ │
│  │                   24 - 80 GB                             │ │
│  └─────────────────────────────────────────────────────────┘ │
│                                                              │
│  ┌──────────────────────────────────────────────────────────┐│
│  │            Host Interface (PCIe / NVLink / Unified)      ││
│  │           CPU ←→ Device data transfer                    ││
│  └──────────────────────────────────────────────────────────┘│
└─────────────────────────────────────────────────────────────┘
```

There are **five new subsystems** at this layer:

1. **Global Memory (VRAM)** — the large device-wide memory
2. **L2 Cache** — shared cache between all compute units
3. **Memory Controllers** — bandwidth and latency modeling
4. **Work Distributor** — assigns work items to compute units
5. **Host Interface** — transfers data between CPU and device

## New Package: `global-memory`

We need a new package to model device-wide memory. The existing `cache` package
handles L1/L2/L3 caching. But we need a **VRAM / HBM** model that represents
the large, high-bandwidth device memory.

Why a separate package? Because global memory has unique properties:

- **Bandwidth-limited**: GPU memory bandwidth is 1-3 TB/s but shared across all
  compute units. Memory controllers must arbitrate access.
- **High latency**: ~400-800 cycles to access global memory (vs. ~1 cycle for
  registers, ~30 cycles for shared memory).
- **Coalescing**: Adjacent threads accessing adjacent addresses can be merged
  into a single wide transaction (128B or 256B). Non-coalesced accesses waste
  bandwidth.
- **Partitioned**: Memory is split across channels (e.g., 8 HBM2 stacks on H100),
  and access patterns that hit one partition create hotspots.

### GlobalMemory interface

```python
class GlobalMemory(Protocol):
    """Device-wide memory (VRAM, HBM, unified memory)."""

    def read(self, address: int, size: int) -> bytes:
        """Read `size` bytes from global memory.
        Returns the data after simulating latency."""

    def write(self, address: int, data: bytes) -> None:
        """Write data to global memory."""

    def allocate(self, size: int, alignment: int = 256) -> int:
        """Allocate `size` bytes, return start address.
        Like cudaMalloc — returns a device pointer."""

    def free(self, address: int) -> None:
        """Free a previous allocation. Like cudaFree."""

    def copy_from_host(self, dst_addr: int, data: bytes) -> int:
        """Copy from host (CPU) to device memory.
        Returns number of cycles consumed (transfer latency).
        Like cudaMemcpy(dst, src, size, cudaMemcpyHostToDevice)."""

    def copy_to_host(self, src_addr: int, size: int) -> tuple[bytes, int]:
        """Copy from device memory to host (CPU).
        Returns (data, cycles_consumed).
        Like cudaMemcpy(dst, src, size, cudaMemcpyDeviceToHost)."""

    def coalesce(self, addresses: list[int], size: int) -> list[MemoryTransaction]:
        """Given per-thread addresses, merge into coalesced transactions.
        Returns the minimum set of wide memory transactions needed."""

    @property
    def capacity(self) -> int:
        """Total memory in bytes."""

    @property
    def bandwidth(self) -> float:
        """Peak bandwidth in bytes/cycle."""

    @property
    def stats(self) -> GlobalMemoryStats:
        """Access statistics."""

    def reset(self) -> None:
        """Clear all data and statistics."""
```

### Memory coalescing — why it matters

When 32 threads in a warp each request 4 bytes, that's 128 bytes total. If
those addresses are contiguous (thread 0 reads address 0, thread 1 reads
address 4, ..., thread 31 reads address 124), the hardware merges them into
**one 128-byte transaction**. This is called **coalesced access**.

```
COALESCED — best case (1 transaction, full bandwidth):
Thread:     0    1    2    3    4   ...  31
Address:  [0]  [4]  [8]  [12] [16] ... [124]
           └────────────────────────────────┘
                  One 128B transaction

STRIDED — worst case (32 transactions, 1/32 bandwidth):
Thread:     0       1       2       3      ...  31
Address:  [0]   [512]  [1024]  [1536]     ... [15872]
           │       │       │       │             │
           ▼       ▼       ▼       ▼             ▼
        Trans 1  Trans 2  Trans 3  Trans 4 ... Trans 32

SCATTERED — random (varies):
Thread:     0      1     2      3      ...
Address:  [100]  [4]  [9000]  [52]   ...
           │      │      │      │
         Some may share a cache line → partially coalesced
```

Coalescing is modeled in the GlobalMemory.coalesce() method. The device
simulator calls this when a warp/wavefront issues a memory instruction.

### Memory partitioning

Modern GPUs split VRAM across multiple memory channels/stacks:

```
H100: 8 HBM3 stacks, each 10GB, total 80GB
      Each stack provides ~400 GB/s
      Total: ~3.35 TB/s peak bandwidth

Address mapping (simplified):
  Address bits: [tag | stack_id (3 bits) | offset]

  Stack 0: addresses 0, 8, 16, 24, ...
  Stack 1: addresses 1, 9, 17, 25, ...
  ...
  Stack 7: addresses 7, 15, 23, 31, ...

  (Interleaved at cache line granularity so sequential access
   spreads across all stacks evenly)
```

If all threads access addresses that map to the same stack, only 1/8 of the
bandwidth is available. This is a **partition conflict** (similar to shared
memory bank conflicts, but at the device level).

### MemoryTransaction and GlobalMemoryStats

```python
@dataclass
class MemoryTransaction:
    """A single wide memory transaction after coalescing."""
    address: int         # Aligned start address
    size: int            # Transaction size (32B, 64B, or 128B)
    thread_mask: int     # Which threads are served by this transaction

@dataclass
class GlobalMemoryStats:
    """Tracks memory access patterns and efficiency."""
    total_reads: int
    total_writes: int
    total_transactions: int    # After coalescing
    total_requests: int        # Before coalescing (per-thread)
    bytes_transferred: int
    coalescing_efficiency: float  # requests/transactions (ideal=1.0)
    partition_conflicts: int
    host_to_device_bytes: int
    device_to_host_bytes: int
    host_transfer_cycles: int
```

## Five Device Architectures

### 1. NVIDIA GPU (`NvidiaGPU`)

The quintessential GPU architecture. Many SMs connected by an on-chip
network to shared L2 cache and HBM.

```
┌────────────────────────────────────────────────────────┐
│                    NVIDIA GPU                           │
│                                                         │
│  ┌──────────────────────────────────────────────────┐  │
│  │          GigaThread Engine (Work Distributor)      │  │
│  │                                                    │  │
│  │  Receives kernel launches from host.               │  │
│  │  Creates thread blocks from grid dimensions.       │  │
│  │  Assigns thread blocks to SMs with available       │  │
│  │  resources (registers, shared memory, warp slots). │  │
│  └─────────────────────┬────────────────────────────┘  │
│                        │                                │
│  ┌────────┐ ┌────────┐ ┌────────┐ ... ┌────────┐      │
│  │  SM 0  │ │  SM 1  │ │  SM 2  │     │ SM 131 │      │
│  │        │ │        │ │        │     │        │      │
│  │ 4 warp │ │ 4 warp │ │ 4 warp │     │ 4 warp │      │
│  │ scheds │ │ scheds │ │ scheds │     │ scheds │      │
│  │        │ │        │ │        │     │        │      │
│  │ 128 FP │ │ 128 FP │ │ 128 FP │     │ 128 FP │      │
│  │ cores  │ │ cores  │ │ cores  │     │ cores  │      │
│  │        │ │        │ │        │     │        │      │
│  │ 96KB   │ │ 96KB   │ │ 96KB   │     │ 96KB   │      │
│  │ shmem  │ │ shmem  │ │ shmem  │     │ shmem  │      │
│  └───┬────┘ └───┬────┘ └───┬────┘     └───┬────┘      │
│      │          │          │              │            │
│  ┌───┴──────────┴──────────┴──────────────┴──────────┐ │
│  │              L2 Cache (50 MB, shared)               │ │
│  └────────────────────────┬────────────────────────────┘ │
│                           │                              │
│  ┌────────────────────────┴────────────────────────────┐ │
│  │       8 × HBM3 stacks = 80 GB, 3.35 TB/s           │ │
│  └─────────────────────────────────────────────────────┘ │
│                                                          │
│  ┌──────────────────────────────────────────────────────┐│
│  │     PCIe Gen5 x16 (64 GB/s) or NVLink (900 GB/s)    ││
│  └──────────────────────────────────────────────────────┘│
└──────────────────────────────────────────────────────────┘
```

**Configuration (H100-like defaults):**

| Parameter | Value |
|-----------|-------|
| SM count | 132 |
| Cores per SM | 128 FP32 |
| Warp schedulers per SM | 4 |
| Max warps per SM | 64 (2048 threads) |
| Shared memory per SM | 228 KB (configurable with L1) |
| Register file per SM | 256 KB |
| L2 cache | 50 MB |
| Global memory | 80 GB HBM3 |
| Memory bandwidth | 3.35 TB/s |
| Host interface | PCIe Gen5 x16 (64 GB/s) |

**Work distribution — GigaThread Engine:**

When a kernel is launched with a grid of thread blocks, the GigaThread Engine:

1. Maintains a queue of pending thread blocks
2. For each SM, checks if it has enough free resources:
   - Enough free warp slots for the block's warps
   - Enough free registers for the block's register usage
   - Enough free shared memory for the block's shared memory request
3. If resources available, dispatches the thread block to that SM
4. Continues until all thread blocks are dispatched
5. As SMs complete blocks, freed resources allow new blocks to be assigned

```
Kernel: matmul<<<grid(256,256), block(16,16)>>>

Grid = 256×256 = 65,536 thread blocks
Each block = 16×16 = 256 threads = 8 warps

GigaThread distributes across 132 SMs:
  SM 0:  blocks [0,0] [0,1] [0,2] ...  (up to resource limit)
  SM 1:  blocks [0,8] [0,9] [0,10] ...
  ...
  SM 131: blocks [255,248] [255,249] ...

Each SM can hold ~8 blocks (64 warps / 8 warps per block).
132 SMs × 8 blocks = ~1,056 blocks resident at once.
65,536 total blocks → waves of ~1,056 blocks until done.
```

### 2. AMD GPU (`AmdGPU`)

AMD organizes CUs into **Shader Engines** — a mid-level grouping that shares
a geometry processor and rasterizer. Each Shader Engine contains multiple
CUs and has its own L1 cache.

```
┌──────────────────────────────────────────────────────────┐
│                       AMD GPU (RDNA 3)                    │
│                                                           │
│  ┌────────────────────────────────────────────────────┐  │
│  │            Command Processor (Work Distributor)      │  │
│  │                                                      │  │
│  │  Dispatches work-groups to CUs based on resource     │  │
│  │  availability. Manages multiple hardware queues      │  │
│  │  (ACEs) for concurrent kernel execution.             │  │
│  └──────────────────────┬───────────────────────────────┘  │
│                         │                                  │
│  ┌──────────────────────┴───────────────────────────────┐  │
│  │              Shader Engine 0                           │  │
│  │  ┌──────┐ ┌──────┐ ┌──────┐ ... ┌──────┐            │  │
│  │  │ CU 0 │ │ CU 1 │ │ CU 2 │     │ CU 14│            │  │
│  │  │ 4×   │ │      │ │      │     │      │            │  │
│  │  │ SIMD │ │      │ │      │     │      │            │  │
│  │  │ 32   │ │      │ │      │     │      │            │  │
│  │  │ 64KB │ │      │ │      │     │      │            │  │
│  │  │ LDS  │ │      │ │      │     │      │            │  │
│  │  └──────┘ └──────┘ └──────┘     └──────┘            │  │
│  └──────────────────────────────────────────────────────┘  │
│                                                            │
│  ┌──────────────────────────────────────────────────────┐  │
│  │              Shader Engine 1                           │  │
│  │  ┌──────┐ ┌──────┐ ... ┌──────┐                      │  │
│  │  │CU 15 │ │CU 16 │     │CU 29 │                      │  │
│  │  └──────┘ └──────┘     └──────┘                      │  │
│  └──────────────────────────────────────────────────────┘  │
│  ... (up to 8 Shader Engines)                              │
│                                                            │
│  ┌──────────────────────────────────────────────────────┐  │
│  │       Infinity Cache (96 MB, shared L3-like)          │  │
│  └────────────────────────┬─────────────────────────────┘  │
│                           │                                │
│  ┌────────────────────────┴─────────────────────────────┐  │
│  │        GDDR6 384-bit, 24 GB, 960 GB/s                 │  │
│  └──────────────────────────────────────────────────────┘  │
│                                                            │
│  ┌──────────────────────────────────────────────────────┐  │
│  │              PCIe Gen4 x16 (32 GB/s)                  │  │
│  └──────────────────────────────────────────────────────┘  │
└──────────────────────────────────────────────────────────┘
```

**Configuration (RX 7900 XTX-like defaults):**

| Parameter | Value |
|-----------|-------|
| CU count | 96 (in 6 Shader Engines) |
| CUs per Shader Engine | 16 |
| SIMD units per CU | 4 (SIMD32) |
| Wavefront width | 32 (RDNA) or 64 (legacy GCN mode) |
| Max wavefronts per CU | 32 |
| LDS per CU | 64 KB |
| Infinity Cache | 96 MB |
| Global memory | 24 GB GDDR6 |
| Memory bandwidth | 960 GB/s |
| Memory bus | 384-bit |

**Key AMD difference — Shader Engines:**

AMD has an extra level of hierarchy between CU and device. Shader Engines
group CUs and share some fixed-function hardware (rasterizer, geometry
engine). For compute workloads (our focus), the main impact is that the
Command Processor assigns **entire work-groups to a single Shader Engine**
before distributing to CUs within it.

**Key AMD difference — Asynchronous Compute Engines (ACEs):**

AMD GPUs have multiple **hardware queues** (ACEs) that can dispatch work
to CUs simultaneously. This allows overlapping compute and copy operations,
or running multiple kernels concurrently on different CUs. NVIDIA has
similar functionality (streams) but AMD was earlier to hardware-level
multi-queue.

### 3. Google TPU (`GoogleTPU`)

The TPU is structured very differently from GPUs. It's designed around
a single large matrix multiply unit rather than many small cores.

```
┌──────────────────────────────────────────────────────────┐
│                     Google TPU v4                          │
│                                                           │
│  ┌────────────────────────────────────────────────────┐  │
│  │            Sequencer (Control Unit)                  │  │
│  │                                                      │  │
│  │  Fetches HLO instructions from instruction memory.   │  │
│  │  No thread blocks — operates on tiles/tensors.       │  │
│  │  Orchestrates data flow through the three units.     │  │
│  └────┬────────────────┬────────────────┬─────────────┘  │
│       │                │                │                 │
│  ┌────┴─────┐   ┌──────┴──────┐   ┌────┴──────┐         │
│  │  Scalar  │   │   Vector    │   │    MXU     │         │
│  │  Unit    │   │   Unit      │   │  (128×128) │         │
│  │          │   │             │   │            │         │
│  │ Control  │   │ Element-    │   │ Systolic   │         │
│  │ flow,    │   │ wise ops:   │   │ matrix     │         │
│  │ address  │   │ add, mul,   │   │ multiply:  │         │
│  │ calc,    │   │ activation, │   │ C += A×B   │         │
│  │ loop     │   │ normalize,  │   │            │         │
│  │ counters │   │ softmax     │   │ Feeds into │         │
│  │          │   │             │   │ accumulat- │         │
│  │          │   │ Width: 128  │   │ ors (FP32) │         │
│  └──────────┘   └─────────────┘   └────────────┘         │
│                                                           │
│  ┌────────────────────────────────────────────────────┐  │
│  │         Transpose / Permute Unit                     │  │
│  │    Rearranges data between MXU passes (free!)        │  │
│  └────────────────────────────────────────────────────┘  │
│                                                           │
│  ┌────────────────────────────────────────────────────┐  │
│  │              HBM2e (32 GB per chip, 1.2 TB/s)       │  │
│  └────────────────────────┬───────────────────────────┘  │
│                           │                               │
│  ┌────────────────────────┴───────────────────────────┐  │
│  │      ICI (Inter-Chip Interconnect) — 4D torus       │  │
│  │      Connects to other TPU chips in a pod           │  │
│  │      Bandwidth: ~500 GB/s per link                  │  │
│  └────────────────────────────────────────────────────┘  │
└──────────────────────────────────────────────────────────┘
```

**Configuration (TPU v4-like defaults):**

| Parameter | Value |
|-----------|-------|
| MXU size | 128×128 (16,384 MACs) |
| MXU input precision | BF16 |
| Accumulator precision | FP32 |
| Vector unit width | 128 lanes |
| Scalar registers | 32 |
| HBM per chip | 32 GB |
| HBM bandwidth | 1.2 TB/s |
| Peak TFLOPS (BF16) | 275 |
| ICI links | 6 (4D torus topology) |

**Key TPU difference — no thread blocks:**

TPUs don't have threads, warps, or thread blocks. The sequencer fetches
**HLO operations** (high-level operations like matrix multiply, convolution,
reduce) and orchestrates data flow through the three functional units.

The "work distribution" is fundamentally different:

```
GPU work distribution:
  Kernel(grid=65536 blocks, block=256 threads)
  → GigaThread assigns blocks to SMs
  → Each SM decomposes blocks into warps
  → Warp scheduler picks ready warps each cycle

TPU work distribution:
  HLO program: [matmul(A, B), add(C, bias), relu(), ...]
  → Sequencer tiles large matrices into MXU-sized chunks
  → MXU processes one 128×128 tile at a time
  → Vector unit handles element-wise post-processing
  → Scalar unit handles control flow and addressing
```

**Key TPU difference — three cooperating units:**

The Scalar, Vector, and MXU units operate in a **pipeline**. The Scalar unit
prepares addresses and loop counters. The MXU performs the heavy matrix
multiply. The Vector unit applies activation functions, normalization, etc.
These three overlap — while the MXU is crunching tile N, the Vector unit is
processing the output of tile N-1, and the Scalar unit is setting up tile N+1.

### 4. Intel GPU (`IntelGPU`)

Intel organizes Xe-Cores into **Xe-Slices**, with each slice sharing a
large L1 cache. Slices are grouped into **Render Slices** at the top.

```
┌──────────────────────────────────────────────────────────┐
│                    Intel GPU (Xe-HPG / Arc)                │
│                                                           │
│  ┌────────────────────────────────────────────────────┐  │
│  │           Command Streamer (Work Distributor)        │  │
│  │                                                      │  │
│  │  Distributes thread groups to Xe-Slices based on     │  │
│  │  resource availability. Supports multi-context       │  │
│  │  (simultaneous compute + graphics).                  │  │
│  └──────────────────────┬───────────────────────────────┘  │
│                         │                                  │
│  ┌──────────────────────┴───────────────────────────────┐  │
│  │              Xe-Slice 0                                │  │
│  │  ┌─────────┐ ┌─────────┐ ┌─────────┐ ┌─────────┐    │  │
│  │  │XeCore 0 │ │XeCore 1 │ │XeCore 2 │ │XeCore 3 │    │  │
│  │  │ 8 EUs   │ │ 8 EUs   │ │ 8 EUs   │ │ 8 EUs   │    │  │
│  │  │ SLM     │ │ SLM     │ │ SLM     │ │ SLM     │    │  │
│  │  └─────────┘ └─────────┘ └─────────┘ └─────────┘    │  │
│  │                                                       │  │
│  │  L1 Cache (192 KB, shared across Xe-Cores in slice)   │  │
│  └──────────────────────────────────────────────────────┘  │
│                                                            │
│  ┌──────────────────────────────────────────────────────┐  │
│  │              Xe-Slice 1 (same structure)               │  │
│  └──────────────────────────────────────────────────────┘  │
│  ... (4-8 Xe-Slices)                                       │
│                                                            │
│  ┌──────────────────────────────────────────────────────┐  │
│  │              L2 Cache (16 MB, shared across slices)    │  │
│  └────────────────────────┬─────────────────────────────┘  │
│                           │                                │
│  ┌────────────────────────┴─────────────────────────────┐  │
│  │        GDDR6 256-bit, 16 GB, 512 GB/s                 │  │
│  └──────────────────────────────────────────────────────┘  │
│                                                            │
│  ┌──────────────────────────────────────────────────────┐  │
│  │              PCIe Gen4 x16 (32 GB/s)                  │  │
│  └──────────────────────────────────────────────────────┘  │
└──────────────────────────────────────────────────────────┘
```

**Configuration (Arc A770-like defaults):**

| Parameter | Value |
|-----------|-------|
| Xe-Slices | 8 |
| Xe-Cores per slice | 4 |
| Total Xe-Cores | 32 |
| EUs per Xe-Core | 8 |
| Threads per EU | 8 |
| SIMD width | 8 (SIMD8) |
| SLM per Xe-Core | 64 KB |
| L1 per Xe-Slice | 192 KB |
| L2 cache | 16 MB |
| Global memory | 16 GB GDDR6 |
| Memory bandwidth | 512 GB/s |

**Key Intel difference — Xe-Slice hierarchy:**

Intel has an extra grouping level: Xe-Cores are grouped into Xe-Slices,
which share an L1 cache. This is similar to AMD's Shader Engines but at
a different granularity. The Command Streamer first assigns work to
Xe-Slices, then the slice distributes to individual Xe-Cores.

### 5. Apple ANE (`AppleANE`)

The Apple Neural Engine is radically different — it's not a GPU at all.
It's a fixed-function accelerator for neural network inference, optimized
for power efficiency over flexibility.

```
┌──────────────────────────────────────────────────────────┐
│                Apple Neural Engine (M-series)              │
│                                                           │
│  ┌────────────────────────────────────────────────────┐  │
│  │        DMA Controller (Work Distributor)             │  │
│  │                                                      │  │
│  │  Loads model weights and activations into SRAM.      │  │
│  │  Schedules layer-by-layer execution across cores.    │  │
│  │  No kernel launch — compiler generates a full        │  │
│  │  execution plan at compile time.                     │  │
│  └──────────────────────┬───────────────────────────────┘  │
│                         │                                  │
│  ┌──────┐ ┌──────┐ ┌──────┐ ... ┌──────┐                 │
│  │Core 0│ │Core 1│ │Core 2│     │Core15│                 │
│  │      │ │      │ │      │     │      │                 │
│  │ MAC  │ │ MAC  │ │ MAC  │     │ MAC  │                 │
│  │ Array│ │ Array│ │ Array│     │ Array│                 │
│  │      │ │      │ │      │     │      │                 │
│  │ Act. │ │ Act. │ │ Act. │     │ Act. │                 │
│  │Pipe  │ │Pipe  │ │Pipe  │     │Pipe  │                 │
│  └──┬───┘ └──┬───┘ └──┬───┘     └──┬───┘                 │
│     │        │        │            │                      │
│  ┌──┴────────┴────────┴────────────┴────────────────┐    │
│  │           Shared SRAM (32 MB)                      │    │
│  │    On-chip, low-latency, compiler-managed          │    │
│  └────────────────────────┬───────────────────────────┘    │
│                           │                                │
│  ┌────────────────────────┴───────────────────────────┐   │
│  │    Unified Memory (shared with CPU and GPU)         │   │
│  │    No copy needed — just change page table mapping  │   │
│  │    Bandwidth: ~200 GB/s (M3 Max)                    │   │
│  └────────────────────────────────────────────────────┘   │
└──────────────────────────────────────────────────────────┘
```

**Configuration (M3 Max ANE-like defaults):**

| Parameter | Value |
|-----------|-------|
| Neural Engine cores | 16 |
| MAC units per core | 256 |
| Precision | INT8, FP16, BF16 |
| Shared SRAM | 32 MB |
| Peak TOPS (INT8) | 35 |
| Unified memory | Shared with CPU/GPU |
| Memory bandwidth | ~200 GB/s (shared) |

**Key Apple difference — unified memory:**

Apple's ANE doesn't have its own VRAM. It shares unified memory with the
CPU and GPU. The `copy_from_host` / `copy_to_host` operations are
effectively **free** — they just update page table mappings rather than
physically moving data. This eliminates the PCIe bottleneck entirely.

Our GlobalMemory implementation for Apple will model this: zero-cost
host transfers but shared bandwidth with other components.

**Key Apple difference — compiler-driven scheduling:**

There's no hardware work distributor like NVIDIA's GigaThread Engine.
The CoreML compiler generates a **complete execution plan** at compile
time — which cores process which layers, when DMA transfers happen,
how tiles flow through the MAC arrays. The "work distributor" in our
model simply replays this pre-computed schedule.

## AcceleratorDevice Protocol

The unified interface that all five device types implement:

```python
class AcceleratorDevice(Protocol):
    """A complete accelerator device."""

    # === Identity ===

    @property
    def name(self) -> str:
        """Device name (e.g., 'NVIDIA H100', 'AMD RX 7900 XTX')."""

    @property
    def arch(self) -> Architecture:
        """The underlying compute unit architecture."""

    @property
    def config(self) -> DeviceConfig:
        """Full device configuration."""

    # === Memory management ===

    @property
    def global_memory(self) -> GlobalMemory:
        """Access to device-wide memory (VRAM, HBM, unified)."""

    def malloc(self, size: int) -> int:
        """Allocate device memory. Returns device pointer."""

    def free(self, address: int) -> None:
        """Free device memory allocation."""

    def memcpy_host_to_device(self, dst: int, data: bytes) -> int:
        """Copy from host to device. Returns cycles consumed."""

    def memcpy_device_to_host(self, src: int, size: int) -> tuple[bytes, int]:
        """Copy from device to host. Returns (data, cycles)."""

    # === Kernel launch (GPU-style) ===

    def launch_kernel(self, kernel: KernelDescriptor) -> None:
        """Submit a kernel for execution.

        For GPU-style devices, this creates thread blocks from the
        grid/block dimensions and queues them for distribution.

        For TPU/NPU-style devices, this submits an operation
        (matmul, conv, etc.) with its input/weight descriptors.
        """

    # === Simulation ===

    def step(self, clock_edge: ClockEdge) -> DeviceTrace:
        """Advance the entire device by one clock cycle.

        The work distributor assigns pending work to idle CUs.
        Each CU steps. L2 cache and memory handle requests.
        Returns a trace covering all activity this cycle.
        """

    def run(self, max_cycles: int) -> list[DeviceTrace]:
        """Run until all kernels complete or max_cycles reached."""

    @property
    def idle(self) -> bool:
        """True when all CUs are idle and no pending work remains."""

    def reset(self) -> None:
        """Reset all state — CUs, memory, caches, work queues."""

    # === Observability ===

    @property
    def stats(self) -> DeviceStats:
        """Aggregate statistics across all compute units and memory."""

    @property
    def compute_units(self) -> list[ComputeUnit]:
        """Direct access to individual compute units (for inspection)."""
```

## KernelDescriptor — What Gets Launched

A kernel descriptor packages everything needed to launch work on the device:

```python
@dataclass
class KernelDescriptor:
    """Describes a kernel launch (GPU) or operation (TPU/NPU)."""

    # Common fields
    name: str                     # Kernel name
    kernel_id: int                # Unique launch ID

    # === GPU-style fields ===
    program: list[Instruction]    # The instruction stream (PTX-like)
    grid_dim: tuple[int,int,int]  # Grid dimensions (blocks)
    block_dim: tuple[int,int,int] # Block dimensions (threads per block)
    shared_mem_bytes: int         # Dynamic shared memory per block
    registers_per_thread: int     # Register usage per thread

    # === Dataflow-style fields (TPU/NPU) ===
    operation: str                # "matmul", "conv2d", "elementwise_add", etc.
    input_data: list[list[float]] # Input tensor (device addresses in real impl)
    weight_data: list[list[float]]# Weight tensor
    output_address: int           # Where to write results in global memory

    # === Computed ===
    @property
    def total_threads(self) -> int:
        x, y, z = self.grid_dim
        bx, by, bz = self.block_dim
        return x * y * z * bx * by * bz

    @property
    def total_blocks(self) -> int:
        x, y, z = self.grid_dim
        return x * y * z

    @property
    def threads_per_block(self) -> int:
        bx, by, bz = self.block_dim
        return bx * by * bz
```

## Work Distribution Strategies

The work distributor is the brain of the device. It decides which compute
unit gets which work item and when.

### GPU-style: Block-to-CU Assignment

```python
class GPUWorkDistributor:
    """Distributes thread blocks to compute units.

    Used by NVIDIA, AMD, and Intel devices.
    """

    def __init__(self, compute_units: list[ComputeUnit], policy: str = "round_robin"):
        self.compute_units = compute_units
        self.pending_blocks: deque[WorkItem] = deque()
        self.policy = policy  # "round_robin", "least_loaded", "fill_first"

    def submit_kernel(self, kernel: KernelDescriptor) -> None:
        """Decompose kernel into thread blocks and queue them."""
        for block_id in range(kernel.total_blocks):
            # Convert grid coordinates to block index
            bx = block_id % kernel.grid_dim[0]
            by = (block_id // kernel.grid_dim[0]) % kernel.grid_dim[1]
            bz = block_id // (kernel.grid_dim[0] * kernel.grid_dim[1])

            work = WorkItem(
                work_id=block_id,
                program=kernel.program,
                thread_count=kernel.threads_per_block,
                registers_per_thread=kernel.registers_per_thread,
                shared_mem_bytes=kernel.shared_mem_bytes,
                block_idx=(bx, by, bz),
            )
            self.pending_blocks.append(work)

    def step(self) -> list[str]:
        """Try to assign pending blocks to available CUs.
        Returns list of assignment descriptions."""
        assignments = []
        for cu in self._select_order():
            while self.pending_blocks and cu.can_accept(self.pending_blocks[0]):
                block = self.pending_blocks.popleft()
                cu.dispatch(block)
                assignments.append(f"Block {block.work_id} → {cu.name}")
        return assignments
```

**Assignment policies:**

| Policy | Description | Best for |
|--------|-------------|----------|
| `round_robin` | Cycle through CUs evenly | Uniform workloads |
| `fill_first` | Fill one CU before moving to next | Maximize occupancy per CU |
| `least_loaded` | Assign to CU with fewest active warps | Load balancing |

### TPU-style: Operation Sequencing

```python
class TPUSequencer:
    """Orchestrates operations through Scalar + Vector + MXU units.

    No thread blocks — operations flow through a pipeline.
    """

    def __init__(self, scalar_unit, vector_unit, mxu: ComputeUnit):
        self.scalar = scalar_unit
        self.vector = vector_unit
        self.mxu = mxu
        self.operation_queue: deque[KernelDescriptor] = deque()

    def submit_operation(self, kernel: KernelDescriptor) -> None:
        """Queue an HLO operation for execution."""
        self.operation_queue.append(kernel)

    def step(self) -> list[str]:
        """Pipeline: MXU works on tile N, Vector on tile N-1, Scalar on N+1."""
        actions = []
        # Scalar: prepare next tile addresses
        # MXU: process current tile
        # Vector: post-process previous tile output
        return actions
```

### ANE-style: Schedule Replay

```python
class ANEScheduleReplayer:
    """Replays a compiler-generated execution schedule.

    No dynamic scheduling — the compiler decided everything.
    """

    def __init__(self, cores: list[ComputeUnit], dma):
        self.cores = cores
        self.dma = dma
        self.schedule: list[ScheduleEntry] = []
        self.current_step: int = 0

    def load_schedule(self, kernel: KernelDescriptor) -> None:
        """Convert operation into a pre-computed schedule.
        In real hardware, the CoreML compiler generates this."""
        # Tile the operation
        # Assign tiles to cores
        # Schedule DMA transfers
        # Build timeline

    def step(self) -> list[str]:
        """Execute the next step in the pre-computed schedule."""
        if self.current_step >= len(self.schedule):
            return []
        entry = self.schedule[self.current_step]
        # Execute whatever the schedule says for this cycle
        self.current_step += 1
        return [entry.description]
```

## DeviceTrace — Cycle-by-Cycle Visibility

```python
@dataclass
class DeviceTrace:
    """One cycle of device-wide activity."""
    cycle: int
    device_name: str

    # Work distribution
    distributor_actions: list[str]     # "Block 42 → SM 7"
    pending_blocks: int                # Blocks still waiting
    active_blocks: int                 # Blocks currently running

    # Per-CU traces
    cu_traces: list[ComputeUnitTrace]  # One per compute unit

    # Memory system
    l2_hits: int
    l2_misses: int
    memory_transactions: int           # Global memory accesses this cycle
    memory_bandwidth_used: float       # Fraction of peak bandwidth

    # Aggregate metrics
    total_active_warps: int            # Across all CUs
    device_occupancy: float            # active / max across all CUs
    flops_this_cycle: int              # Total FP operations executed

    def format(self) -> str:
        """Human-readable summary of this cycle."""
```

## DeviceConfig — Full Device Specification

```python
@dataclass
class DeviceConfig:
    """Complete device specification."""

    # Identity
    name: str
    arch: Architecture

    # Compute
    num_compute_units: int
    cu_config: Any                     # SMConfig, AMDCUConfig, etc.

    # Memory hierarchy
    l2_cache_size: int                 # Bytes
    l2_cache_latency: int              # Cycles
    l2_cache_associativity: int

    global_memory_size: int            # Bytes
    global_memory_bandwidth: float     # Bytes per cycle
    global_memory_latency: int         # Cycles
    memory_channels: int               # Number of memory partitions

    # Host interface
    host_bandwidth: float              # Bytes per cycle (PCIe, NVLink, etc.)
    host_latency: int                  # Cycles for host transfer initiation
    unified_memory: bool               # True for Apple (zero-copy)

    # Scheduling
    max_concurrent_kernels: int        # Hardware queue depth
    work_distribution_policy: str
```

### Default Configurations

```python
def default_nvidia_config() -> DeviceConfig:
    """H100-like configuration."""
    return DeviceConfig(
        name="NVIDIA H100",
        arch=Architecture.NVIDIA_SM,
        num_compute_units=132,
        cu_config=DefaultSMConfig(),
        l2_cache_size=50 * 1024 * 1024,      # 50 MB
        l2_cache_latency=200,
        l2_cache_associativity=32,
        global_memory_size=80 * 1024**3,      # 80 GB
        global_memory_bandwidth=3350e9 / 1e9, # 3.35 TB/s
        global_memory_latency=400,
        memory_channels=8,                     # 8 HBM3 stacks
        host_bandwidth=64e9 / 1e9,            # PCIe Gen5 x16
        host_latency=1000,
        unified_memory=False,
        max_concurrent_kernels=128,
        work_distribution_policy="round_robin",
    )

def default_amd_config() -> DeviceConfig:
    """RX 7900 XTX-like configuration."""
    return DeviceConfig(
        name="AMD RX 7900 XTX",
        arch=Architecture.AMD_CU,
        num_compute_units=96,
        cu_config=DefaultAMDCUConfig(),
        l2_cache_size=6 * 1024 * 1024,        # 6 MB L2
        l2_cache_latency=150,
        l2_cache_associativity=16,
        global_memory_size=24 * 1024**3,       # 24 GB
        global_memory_bandwidth=960e9 / 1e9,
        global_memory_latency=350,
        memory_channels=6,
        host_bandwidth=32e9 / 1e9,             # PCIe Gen4 x16
        host_latency=1000,
        unified_memory=False,
        max_concurrent_kernels=8,
        work_distribution_policy="round_robin",
    )

def default_tpu_config() -> DeviceConfig:
    """TPU v4-like configuration."""
    return DeviceConfig(
        name="Google TPU v4",
        arch=Architecture.GOOGLE_MXU,
        num_compute_units=1,                    # One MXU
        cu_config=DefaultMXUConfig(),
        l2_cache_size=0,                        # No L2 — MXU has accumulators
        l2_cache_latency=0,
        l2_cache_associativity=0,
        global_memory_size=32 * 1024**3,        # 32 GB HBM
        global_memory_bandwidth=1200e9 / 1e9,
        global_memory_latency=300,
        memory_channels=4,
        host_bandwidth=500e9 / 1e9,             # ICI link
        host_latency=500,
        unified_memory=False,
        max_concurrent_kernels=1,               # Sequential operations
        work_distribution_policy="sequential",
    )

def default_intel_config() -> DeviceConfig:
    """Arc A770-like configuration."""
    return DeviceConfig(
        name="Intel Arc A770",
        arch=Architecture.INTEL_XE_CORE,
        num_compute_units=32,
        cu_config=DefaultXeCoreConfig(),
        l2_cache_size=16 * 1024 * 1024,        # 16 MB
        l2_cache_latency=180,
        l2_cache_associativity=16,
        global_memory_size=16 * 1024**3,        # 16 GB
        global_memory_bandwidth=512e9 / 1e9,
        global_memory_latency=350,
        memory_channels=4,
        host_bandwidth=32e9 / 1e9,
        host_latency=1000,
        unified_memory=False,
        max_concurrent_kernels=16,
        work_distribution_policy="round_robin",
    )

def default_apple_config() -> DeviceConfig:
    """M3 Max ANE-like configuration."""
    return DeviceConfig(
        name="Apple M3 Max ANE",
        arch=Architecture.APPLE_ANE_CORE,
        num_compute_units=16,
        cu_config=DefaultANECoreConfig(),
        l2_cache_size=0,                        # SRAM instead of L2
        l2_cache_latency=0,
        l2_cache_associativity=0,
        global_memory_size=128 * 1024**3,       # Unified memory (shared)
        global_memory_bandwidth=200e9 / 1e9,    # Shared bandwidth
        global_memory_latency=100,              # Lower — on-chip fabric
        memory_channels=8,
        host_bandwidth=200e9 / 1e9,             # Same as global — unified!
        host_latency=0,                         # Zero-copy
        unified_memory=True,
        max_concurrent_kernels=1,
        work_distribution_policy="scheduled",
    )
```

## AMD-Specific: Shader Engine Grouping

AMD's extra hierarchy level deserves explicit modeling. CUs are grouped
into Shader Engines, and the Command Processor assigns work at the
Shader Engine level first.

```python
@dataclass
class ShaderEngineConfig:
    """Configuration for one AMD Shader Engine."""
    cus_per_engine: int = 16
    shared_l1_size: int = 32 * 1024   # 32 KB shared across CUs in engine

@dataclass
class AmdGPUConfig(DeviceConfig):
    """AMD-specific device config with Shader Engine info."""
    num_shader_engines: int = 6
    se_config: ShaderEngineConfig = field(default_factory=ShaderEngineConfig)
    infinity_cache_size: int = 96 * 1024 * 1024  # 96 MB
    infinity_cache_latency: int = 50              # Cycles
    num_aces: int = 4                             # Asynchronous Compute Engines
```

## Intel-Specific: Xe-Slice Grouping

Similarly, Intel has Xe-Slices between Xe-Core and device:

```python
@dataclass
class XeSliceConfig:
    """Configuration for one Intel Xe-Slice."""
    xe_cores_per_slice: int = 4
    l1_cache_per_slice: int = 192 * 1024  # 192 KB

@dataclass
class IntelGPUConfig(DeviceConfig):
    """Intel-specific config with Xe-Slice hierarchy."""
    num_xe_slices: int = 8
    slice_config: XeSliceConfig = field(default_factory=XeSliceConfig)
```

## TPU-Specific: Multi-Chip ICI Topology

TPUs are designed for multi-chip pods connected by ICI:

```python
@dataclass
class ICILink:
    """One ICI link to another TPU chip."""
    target_chip_id: int
    bandwidth: float     # Bytes per cycle
    latency: int         # Cycles

@dataclass
class TPUConfig(DeviceConfig):
    """TPU-specific config with ICI topology and Vector/Scalar units."""
    vector_unit_width: int = 128
    scalar_registers: int = 32
    transpose_unit: bool = True
    ici_links: list[ICILink] = field(default_factory=list)  # Empty = standalone
```

## Apple-Specific: DMA and SRAM

```python
@dataclass
class ANEConfig(DeviceConfig):
    """Apple ANE-specific config."""
    shared_sram_size: int = 32 * 1024 * 1024  # 32 MB
    sram_bandwidth: float = 1000e9 / 1e9      # Very fast on-chip
    sram_latency: int = 5                      # Cycles
    dma_channels: int = 4
    dma_bandwidth: float = 100e9 / 1e9
```

## End-to-End Example: SAXPY Kernel on Each Device

`Y = alpha * X + Y` with 1,000,000 elements.

### NVIDIA GPU

```
1. Host: cudaMalloc X, Y on device (2 × 4MB = 8MB)
2. Host: cudaMemcpy X, Y from host to device
   → 8 MB / 64 GB/s = 125 μs over PCIe
3. Host: launch saxpy<<<3907, 256>>>(alpha, X, Y, N)
   → 3,907 blocks × 256 threads = 1,000,192 threads
   → 3,907 blocks × 8 warps = 31,256 warps
4. GigaThread: assigns blocks to 132 SMs
   → Each SM gets ~30 blocks (240 warps, but max 64 resident)
   → Wave 1: 132 × 8 = 1,056 blocks
   → Wave 2: 1,056 more blocks
   → ~4 waves total
5. Each SM: warps execute LOAD X[i], LOAD Y[i], FMUL, FADD, STORE
   → Memory-bound: 12 bytes per element, 1M elements = 12 MB
   → 12 MB / 3.35 TB/s = 3.6 μs compute time
6. Host: cudaMemcpy Y from device to host
   → 4 MB / 64 GB/s = 62.5 μs
7. Total: ~190 μs (dominated by PCIe transfers!)
```

### Apple ANE

```
1. CoreML compiler generates schedule at compile time
2. "Host transfer": just remap page tables (0 μs!)
3. DMA loads tiles of X, Y into SRAM
4. 16 cores process tiles in parallel
   → 12 MB / 200 GB/s = 60 μs compute time
5. "Host transfer back": remap pages (0 μs!)
6. Total: ~60 μs (no PCIe overhead!)
   → Slower compute but faster overall for small problems
```

### Google TPU

```
1. Load X, Y to HBM (host transfer or ICI from another chip)
2. Sequencer: this isn't really a matmul, it's element-wise
   → Vector unit handles this, not MXU
   → MXU sits idle for SAXPY (wrong workload for TPU!)
3. Vector unit: 128 lanes × FP32 multiply-add
   → 1M / 128 = 7,813 vector operations
   → Memory-bound: 12 MB / 1.2 TB/s = 10 μs
4. Total: ~10 μs compute + host transfer time
```

## DeviceStats — Aggregate Metrics

```python
@dataclass
class DeviceStats:
    """Device-wide aggregate statistics."""

    # Time
    total_cycles: int
    active_cycles: int                  # At least one CU busy
    idle_cycles: int                    # All CUs idle

    # Compute
    total_flops: int                    # FP operations completed
    achieved_tflops: float              # TFLOPS achieved
    peak_tflops: float                  # Theoretical max TFLOPS
    compute_utilization: float          # achieved / peak

    # Memory
    global_memory_stats: GlobalMemoryStats
    l2_hit_rate: float
    memory_bandwidth_utilization: float # achieved / peak

    # Work distribution
    total_kernels_launched: int
    total_blocks_dispatched: int
    avg_blocks_per_cu: float
    load_imbalance: float               # std_dev / mean of blocks per CU

    # Per-CU breakdown
    per_cu_active_cycles: list[int]
    per_cu_occupancy: list[float]
```

## Package Structure

```
device-simulator/
├── src/
│   ├── protocols.py          # AcceleratorDevice, GlobalMemory, KernelDescriptor
│   ├── global_memory.py      # GlobalMemory implementation, coalescing, partitioning
│   ├── work_distributor.py   # GPUWorkDistributor, TPUSequencer, ANEScheduleReplayer
│   ├── nvidia_gpu.py         # NvidiaGPU device
│   ├── amd_gpu.py            # AmdGPU device with Shader Engines
│   ├── google_tpu.py         # GoogleTPU device with Scalar/Vector/MXU pipeline
│   ├── intel_gpu.py          # IntelGPU device with Xe-Slices
│   ├── apple_ane.py          # AppleANE device with DMA and unified memory
│   └── trace.py              # DeviceTrace, DeviceStats
└── tests/
    ├── test_global_memory.py     # Coalescing, partitioning, allocation
    ├── test_work_distributor.py  # Block assignment, load balancing
    ├── test_nvidia_gpu.py        # SAXPY kernel, multi-wave dispatch
    ├── test_amd_gpu.py           # Shader Engine distribution
    ├── test_google_tpu.py        # Matmul tiling, Vector/MXU pipeline
    ├── test_intel_gpu.py         # Xe-Slice hierarchy
    ├── test_apple_ane.py         # Zero-copy, schedule replay
    └── test_cross_device.py      # Same kernel on all devices, compare traces
```

## Reusing the Existing Cache Package

The existing `cache` package (all 5 languages) already supports:
- Configurable size, associativity, line size
- LRU replacement
- Write-back and write-through policies
- Hit/miss statistics

We'll use it directly for the **L2 cache** in GPU devices. The device
simulator creates a Cache instance from the cache package, configured for
L2 parameters (50MB, 32-way, etc.), and routes memory accesses through it
before they reach global memory.

For AMD's **Infinity Cache**, we'll create a second Cache instance with
the appropriate configuration (96 MB, acting as L3-like).

## Implementation Order

1. Write spec (this document) — commit
2. **Python** implementation:
   a. `protocols.py` — all interfaces and data classes
   b. `global_memory.py` — VRAM with coalescing and partitioning
   c. `work_distributor.py` — GPU, TPU, and ANE distributors
   d. `nvidia_gpu.py` — first full device (most familiar architecture)
   e. `amd_gpu.py` — adds Shader Engine hierarchy
   f. `google_tpu.py` — adds Scalar/Vector/MXU pipeline
   g. `intel_gpu.py` — adds Xe-Slice hierarchy
   h. `apple_ane.py` — adds DMA and unified memory
   i. Tests for each
3. **TypeScript** — port
4. **Rust** — port
5. **Go** — port
6. **Ruby** — port
7. READMEs, CHANGELOGs, BUILD files (for ALL languages!)
8. PR

## Verification

Per language:
- All tests pass
- Coverage 90%+
- Linters pass
- BUILD file exists
- Can run SAXPY on all 5 device types and get traces
- Can run matmul on GPU and TPU and get traces
- Global memory coalescing produces correct transaction counts
- Work distributor evenly distributes blocks
- L2 cache hit rates are reasonable for sequential access patterns

## Dependencies

- **Consumes:** `compute-unit` (Layer 7), `cache`, `clock`, `fp-arithmetic`
- **Consumed by (future):** ISA Simulator (Layer 5), Runtime Simulator (Layer 4)
