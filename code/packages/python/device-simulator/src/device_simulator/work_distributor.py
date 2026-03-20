"""Work Distributor — assigns work to compute units.

=== Three Distribution Strategies ===

Different accelerator architectures distribute work in fundamentally
different ways. This module implements all three:

1. **GPU Block Distributor** (NVIDIA, AMD, Intel)
   - Takes a kernel launch with grid/block dimensions
   - Decomposes into thread blocks
   - Assigns blocks to compute units that have free resources
   - Continues assigning as CUs complete blocks (multi-wave)

2. **TPU Sequencer** (Google TPU)
   - Takes HLO operations (matmul, add, relu, etc.)
   - Tiles large operations to fit the MXU
   - Pipelines through Scalar → MXU → Vector units
   - One operation at a time (no thread blocks)

3. **ANE Schedule Replayer** (Apple Neural Engine)
   - Compiler generates a complete execution schedule at compile time
   - The "distributor" simply replays the schedule
   - No dynamic scheduling decisions — everything is predetermined
   - DMA loads data to SRAM, cores process it, DMA stores results

=== The GPU Work Distribution Problem ===

A kernel launch like `matmul<<<grid(256,256), block(16,16)>>>` creates
65,536 thread blocks. An H100 has 132 SMs, each holding ~8 blocks
(limited by registers, shared memory, warp slots). That's ~1,056
blocks at once. The GigaThread Engine must:

1. Queue all 65,536 blocks
2. Assign 1,056 to SMs in wave 1
3. As SMs complete blocks, assign more from the queue
4. Repeat for ~62 waves until all blocks are done

The distribution policy matters — round-robin spreads blocks evenly,
while fill-first maximizes per-SM occupancy.
"""

from __future__ import annotations

from collections import deque
from dataclasses import dataclass, field
from typing import TYPE_CHECKING

from compute_unit import ResourceError, WorkItem

from device_simulator.protocols import KernelDescriptor

if TYPE_CHECKING:
    from compute_unit import ComputeUnit


# =========================================================================
# GPU Block Distributor
# =========================================================================


class GPUWorkDistributor:
    """Distributes thread blocks to compute units.

    Used by NVIDIA (GigaThread Engine), AMD (Command Processor),
    and Intel (Command Streamer). The same algorithm works for all
    three — they differ only in CU-level resource limits.

    === Distribution Policies ===

        round_robin:  Cycle through CUs evenly. Fair, simple.
        fill_first:   Fill one CU before moving to next. Max occupancy per CU.
        least_loaded: Assign to CU with fewest active warps. Best balance.

    Args:
        compute_units: The CUs to distribute work to.
        policy:        Distribution policy name.
    """

    def __init__(
        self,
        compute_units: list[ComputeUnit],
        policy: str = "round_robin",
    ) -> None:
        self._cus = compute_units
        self._policy = policy
        self._pending: deque[WorkItem] = deque()
        self._rr_index = 0  # For round-robin policy
        self._total_dispatched = 0

    @property
    def pending_count(self) -> int:
        """Number of blocks waiting to be assigned."""
        return len(self._pending)

    @property
    def total_dispatched(self) -> int:
        """Total blocks dispatched so far."""
        return self._total_dispatched

    def submit_kernel(self, kernel: KernelDescriptor) -> None:
        """Decompose a kernel into thread blocks and queue them.

        Each thread block becomes a WorkItem. The block's position in
        the grid is encoded in the work_id (we use a linear index).

        === Grid Linearization ===

        A 3D grid (gx, gy, gz) is linearized:
            block_id = bz * gx * gy + by * gx + bx

        This is the same order CUDA uses for blockIdx.
        """
        for block_id in range(kernel.total_blocks):
            work = WorkItem(
                work_id=block_id,
                program=kernel.program,
                thread_count=kernel.threads_per_block,
                registers_per_thread=kernel.registers_per_thread,
                shared_mem_bytes=kernel.shared_mem_bytes,
            )
            self._pending.append(work)

    def step(self) -> list[str]:
        """Try to assign pending blocks to available CUs.

        Returns a list of human-readable assignment descriptions.
        Each entry looks like: "Block 42 → SM 7"

        === Algorithm ===

        For each CU (in policy order):
            While there are pending blocks:
                Try to dispatch the next block to this CU
                If CU rejects it (ResourceError), move to next CU
                If CU accepts it, log the assignment
        """
        if not self._pending:
            return []

        assignments: list[str] = []
        order = self._cu_order()

        for cu in order:
            while self._pending:
                block = self._pending[0]
                try:
                    cu.dispatch(block)
                    self._pending.popleft()
                    self._total_dispatched += 1
                    assignments.append(
                        f"Block {block.work_id} → {cu.name}"
                    )
                except (ResourceError, Exception):
                    # CU can't accept this block (full) — try next CU
                    break

        return assignments

    def _cu_order(self) -> list[ComputeUnit]:
        """Return CUs in the order dictated by the policy.

        round_robin:  Start from rr_index, wrap around.
        fill_first:   Just return in order (fill CU 0 first, then CU 1, ...).
        least_loaded: Sort by idle status (idle CUs first).
        """
        n = len(self._cus)
        if n == 0:
            return []

        if self._policy == "fill_first":
            return list(self._cus)

        if self._policy == "least_loaded":
            # Idle CUs first, then busy ones
            return sorted(self._cus, key=lambda cu: 0 if cu.idle else 1)

        # Default: round_robin
        ordered = []
        for i in range(n):
            idx = (self._rr_index + i) % n
            ordered.append(self._cus[idx])
        self._rr_index = (self._rr_index + 1) % n
        return ordered

    def reset(self) -> None:
        """Clear all pending work and reset counters."""
        self._pending.clear()
        self._rr_index = 0
        self._total_dispatched = 0


# =========================================================================
# TPU Sequencer
# =========================================================================


@dataclass
class TileOperation:
    """A single tile operation in the TPU pipeline."""

    tile_id: int
    operation: str  # "matmul", "add", "relu", etc.
    input_data: list[list[float]] | None = None
    weight_data: list[list[float]] | None = None
    status: str = "pending"  # pending, scalar, mxu, vector, done
    cycles_remaining: int = 0


class TPUSequencer:
    """Orchestrates operations through Scalar + Vector + MXU units.

    === TPU Execution Pipeline ===

    The TPU processes operations through a three-stage pipeline:

        Scalar Unit → MXU → Vector Unit

    Stage 1 (Scalar): Prepare addresses, loop counters, control flow.
    Stage 2 (MXU):    The heavy lifting — matrix multiply on the systolic array.
    Stage 3 (Vector): Post-processing — activation functions, normalization.

    These three stages overlap: while the MXU crunches tile N, the Vector
    unit processes tile N-1, and the Scalar unit prepares tile N+1.

        Time →
        Scalar: [tile 0] [tile 1] [tile 2] [tile 3] ...
        MXU:           [tile 0] [tile 1] [tile 2] ...
        Vector:               [tile 0] [tile 1] ...

    Args:
        mxu:              The MXU compute unit.
        mxu_size:         The systolic array dimension (e.g., 128 for 128×128).
        vector_width:     Width of the vector unit.
        scalar_latency:   Cycles for scalar setup per tile.
        mxu_latency:      Cycles for MXU processing per tile.
        vector_latency:   Cycles for vector post-processing per tile.
    """

    def __init__(
        self,
        mxu: ComputeUnit,
        mxu_size: int = 128,
        vector_width: int = 128,
        scalar_latency: int = 5,
        mxu_latency: int = 20,
        vector_latency: int = 10,
    ) -> None:
        self._mxu = mxu
        self._mxu_size = mxu_size
        self._vector_width = vector_width
        self._scalar_latency = scalar_latency
        self._mxu_latency = mxu_latency
        self._vector_latency = vector_latency

        self._pending: deque[TileOperation] = deque()
        self._scalar_tile: TileOperation | None = None
        self._mxu_tile: TileOperation | None = None
        self._vector_tile: TileOperation | None = None
        self._completed: list[TileOperation] = []
        self._total_dispatched = 0

    @property
    def pending_count(self) -> int:
        """Number of tiles waiting to be processed."""
        return len(self._pending)

    @property
    def total_dispatched(self) -> int:
        """Total tiles dispatched so far."""
        return self._total_dispatched

    def submit_operation(self, kernel: KernelDescriptor) -> None:
        """Tile a large operation and queue the tiles.

        === Tiling ===

        If the input matrix is 256×256 but the MXU is 128×128, we need
        to split it into 4 tiles:

            Tile 0: rows 0-127,   cols 0-127
            Tile 1: rows 0-127,   cols 128-255
            Tile 2: rows 128-255, cols 0-127
            Tile 3: rows 128-255, cols 128-255
        """
        input_data = kernel.input_data or [[0.0]]
        weight_data = kernel.weight_data or [[0.0]]

        rows = len(input_data)
        cols = len(weight_data[0]) if weight_data else 1
        mxu = self._mxu_size

        num_row_tiles = max(1, (rows + mxu - 1) // mxu)
        num_col_tiles = max(1, (cols + mxu - 1) // mxu)

        tile_id = 0
        for _rt in range(num_row_tiles):
            for _ct in range(num_col_tiles):
                tile = TileOperation(
                    tile_id=tile_id,
                    operation=kernel.operation or "matmul",
                    input_data=input_data,
                    weight_data=weight_data,
                    cycles_remaining=self._scalar_latency,
                )
                self._pending.append(tile)
                tile_id += 1

    def step(self) -> list[str]:
        """Advance the pipeline by one cycle.

        Returns descriptions of what happened this cycle.
        """
        actions: list[str] = []

        # Vector stage: finish processing
        if self._vector_tile is not None:
            self._vector_tile.cycles_remaining -= 1
            if self._vector_tile.cycles_remaining <= 0:
                self._vector_tile.status = "done"
                self._completed.append(self._vector_tile)
                actions.append(
                    f"Vector: completed tile {self._vector_tile.tile_id}"
                )
                self._vector_tile = None

        # MXU stage: process matrix multiply
        if self._mxu_tile is not None:
            self._mxu_tile.cycles_remaining -= 1
            if self._mxu_tile.cycles_remaining <= 0:
                self._mxu_tile.status = "vector"
                self._mxu_tile.cycles_remaining = self._vector_latency
                # Move to vector stage (if free)
                if self._vector_tile is None:
                    self._vector_tile = self._mxu_tile
                    self._mxu_tile = None
                    actions.append(
                        f"MXU → Vector: tile {self._vector_tile.tile_id}"
                    )

        # Scalar stage: prepare next tile
        if self._scalar_tile is not None:
            self._scalar_tile.cycles_remaining -= 1
            if self._scalar_tile.cycles_remaining <= 0:
                self._scalar_tile.status = "mxu"
                self._scalar_tile.cycles_remaining = self._mxu_latency
                # Move to MXU stage (if free)
                if self._mxu_tile is None:
                    self._mxu_tile = self._scalar_tile
                    self._scalar_tile = None
                    self._total_dispatched += 1
                    actions.append(
                        f"Scalar → MXU: tile {self._mxu_tile.tile_id}"
                    )

        # Feed from pending queue to scalar stage
        if self._scalar_tile is None and self._pending:
            self._scalar_tile = self._pending.popleft()
            self._scalar_tile.status = "scalar"
            self._scalar_tile.cycles_remaining = self._scalar_latency
            actions.append(
                f"Scalar: started tile {self._scalar_tile.tile_id}"
            )

        return actions

    @property
    def idle(self) -> bool:
        """True when all tiles are processed."""
        return (
            not self._pending
            and self._scalar_tile is None
            and self._mxu_tile is None
            and self._vector_tile is None
        )

    def reset(self) -> None:
        """Clear all state."""
        self._pending.clear()
        self._scalar_tile = None
        self._mxu_tile = None
        self._vector_tile = None
        self._completed.clear()
        self._total_dispatched = 0


# =========================================================================
# ANE Schedule Replayer
# =========================================================================


@dataclass
class ScheduleEntry:
    """One step in a compiler-generated ANE schedule.

    The CoreML compiler pre-determines everything:
    - Which core processes which tile
    - When DMA loads happen
    - When DMA stores happen
    - The exact order of operations
    """

    cycle: int
    action: str  # "dma_load", "compute", "dma_store", "activate"
    core_id: int = -1
    description: str = ""
    data: list[list[float]] | None = None
    weights: list[list[float]] | None = None


class ANEScheduleReplayer:
    """Replays a compiler-generated execution schedule.

    === Why No Dynamic Scheduling? ===

    Unlike GPUs (which have hardware schedulers that decide at runtime
    which warp to execute), the Apple Neural Engine relies entirely on
    the compiler. The CoreML compiler analyzes the neural network graph,
    determines the optimal tiling strategy, generates DMA transfer
    schedules, and produces a fixed execution plan.

    This makes the hardware simpler (no complex scheduler) and more
    power-efficient (no scheduling overhead), but less flexible —
    the ANE can only run workloads the compiler knows how to schedule.

    === Schedule Structure ===

        Step 0: DMA load input tile 0 → Core 0 SRAM
        Step 1: DMA load weights → Core 0 SRAM
        Step 2: Core 0 compute (MAC array)
        Step 3: Core 0 activate (ReLU)
        Step 4: DMA store result → output buffer
        Step 5: DMA load input tile 1 → Core 1 SRAM (overlaps with step 2-4!)
        ...

    Args:
        compute_units: The ANE cores to schedule onto.
        dma_latency:   Cycles per DMA transfer.
        compute_latency: Cycles per MAC array computation.
        activate_latency: Cycles per activation function.
    """

    def __init__(
        self,
        compute_units: list[ComputeUnit],
        dma_latency: int = 10,
        compute_latency: int = 20,
        activate_latency: int = 5,
    ) -> None:
        self._cus = compute_units
        self._dma_latency = dma_latency
        self._compute_latency = compute_latency
        self._activate_latency = activate_latency

        self._schedule: list[ScheduleEntry] = []
        self._current_step: int = 0
        self._total_dispatched = 0

    @property
    def pending_count(self) -> int:
        """Number of schedule steps remaining."""
        return max(0, len(self._schedule) - self._current_step)

    @property
    def total_dispatched(self) -> int:
        """Total operations dispatched so far."""
        return self._total_dispatched

    def submit_operation(self, kernel: KernelDescriptor) -> None:
        """Generate a schedule from a kernel descriptor.

        The compiler (us, acting as the compiler) determines:
        1. How to tile the input across available cores
        2. When to load data via DMA
        3. When each core computes
        4. When to apply activation functions
        5. When to store results via DMA
        """
        input_data = kernel.input_data or [[0.0]]
        weight_data = kernel.weight_data or [[0.0]]

        num_cores = len(self._cus)
        rows = len(input_data)
        tiles_per_core = max(1, (rows + num_cores - 1) // num_cores)

        cycle = 0
        for core_id in range(min(num_cores, rows)):
            # DMA load input
            self._schedule.append(ScheduleEntry(
                cycle=cycle,
                action="dma_load",
                core_id=core_id,
                description=f"DMA load input tile → Core {core_id}",
                data=input_data,
            ))
            cycle += self._dma_latency

            # DMA load weights
            self._schedule.append(ScheduleEntry(
                cycle=cycle,
                action="dma_load",
                core_id=core_id,
                description=f"DMA load weights → Core {core_id}",
                weights=weight_data,
            ))
            cycle += self._dma_latency

            # Compute
            self._schedule.append(ScheduleEntry(
                cycle=cycle,
                action="compute",
                core_id=core_id,
                description=f"Core {core_id}: MAC array compute",
            ))
            cycle += self._compute_latency

            # Activate
            self._schedule.append(ScheduleEntry(
                cycle=cycle,
                action="activate",
                core_id=core_id,
                description=f"Core {core_id}: activation (ReLU)",
            ))
            cycle += self._activate_latency

            # DMA store
            self._schedule.append(ScheduleEntry(
                cycle=cycle,
                action="dma_store",
                core_id=core_id,
                description=f"DMA store result from Core {core_id}",
            ))
            cycle += self._dma_latency

    def step(self) -> list[str]:
        """Execute the next step in the pre-computed schedule.

        Returns descriptions of what happened this cycle.
        """
        if self._current_step >= len(self._schedule):
            return []

        entry = self._schedule[self._current_step]
        self._current_step += 1
        self._total_dispatched += 1

        return [entry.description]

    @property
    def idle(self) -> bool:
        """True when the entire schedule has been replayed."""
        return self._current_step >= len(self._schedule)

    def reset(self) -> None:
        """Clear the schedule and reset."""
        self._schedule.clear()
        self._current_step = 0
        self._total_dispatched = 0
