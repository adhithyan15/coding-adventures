"""NeuralEngineCore — Apple ANE Core simulator.

=== What is the Apple Neural Engine? ===

Apple's Neural Engine (ANE) is a dedicated neural network accelerator
found in every Apple chip since the A11 Bionic (2017). It's designed
for one thing: fast, power-efficient neural network inference.

The ANE is the simplest compute unit in our family — and that simplicity
is its strength. By removing hardware schedulers, branch predictors, and
general-purpose control logic, Apple can dedicate nearly all transistors
to MAC (multiply-accumulate) units and on-chip memory.

=== How ANE Differs from GPUs ===

    GPU (NVIDIA/AMD):                   ANE (Apple):
    ┌──────────────────────────┐       ┌──────────────────────────┐
    │ Hardware scheduler       │       │ NO hardware scheduler    │
    │ Runtime decisions        │       │ All decisions at compile  │
    │ Branch prediction        │       │ NO branches              │
    │ Dynamic register alloc   │       │ Static buffer plan       │
    │ Flexible but complex     │       │ Simple but rigid         │
    │ ~5 W per SM             │       │ ~1 W per core            │
    └──────────────────────────┘       └──────────────────────────┘

=== Architecture ===

Each ANE Core has:
- **MAC array**: 16 multiply-accumulate units (our default)
- **DMA engine**: transfers data between main memory and on-chip SRAM
- **On-chip SRAM**: 4 MB (fast, low-power local storage)
- **Activation pipeline**: hardware for ReLU, sigmoid, etc.
- **Buffers**: input, weight, and output buffers

    NeuralEngineCore
    +---------------------------------------------------------------+
    |                                                               |
    |  DMA Engine                                                   |
    |  +----------------------------------------------------------+ |
    |  | Transfers data between main memory and on-chip SRAM       | |
    |  | Bandwidth: 10 elements per cycle                          | |
    |  +----------------------------------------------------------+ |
    |                    |                                          |
    |                    v                                          |
    |  +------------------+ +------------------+                    |
    |  | Input Buffer     | | Weight Buffer    |                    |
    |  | 128 KB          | | 512 KB          |                    |
    |  +--------+---------+ +--------+---------+                    |
    |           |                    |                              |
    |           v                    v                              |
    |  +---------------------------------------------+              |
    |  | MAC Array (16 units)                         |              |
    |  | mac[i] = input[i] * weight[i]                |              |
    |  +---------------------------------------------+              |
    |                    |                                          |
    |                    v                                          |
    |  +---------------------------------------------+              |
    |  | Activation Pipeline                          |              |
    |  | ReLU / sigmoid / tanh / identity             |              |
    |  +---------------------------------------------+              |
    |                    |                                          |
    |                    v                                          |
    |  +---------------------------------------------+              |
    |  | Output Buffer (128 KB)                       |              |
    |  +---------------------------------------------+              |
    +---------------------------------------------------------------+

=== Compiler-Scheduled Execution ===

The ANE doesn't decide what to do at runtime. Instead, Apple's Core ML
compiler (based on BNNS and MPSGraph) generates a complete schedule:

    Cycle 0-9:   DMA load input tile (10 elements/cycle)
    Cycle 10-19: DMA load weight tile
    Cycle 20:    MAC operation (16 parallel multiplies)
    Cycle 21:    Reduce (sum MAC results)
    Cycle 22:    Activate (apply ReLU)
    Cycle 23:    DMA store output

This static schedule is the "program" for the ANE — simple, predictable,
and extremely power-efficient.
"""

from __future__ import annotations

import math
from dataclasses import dataclass
from typing import TYPE_CHECKING

from fp_arithmetic import FP16, FP32, FloatFormat
from parallel_execution_engine import (
    MACArrayConfig,
    MACArrayEngine,
)

from compute_unit.protocols import (
    Architecture,
    ComputeUnitTrace,
    WorkItem,
)

if TYPE_CHECKING:
    from clock import Clock, ClockEdge


# ---------------------------------------------------------------------------
# ANECoreConfig — configuration for an Apple Neural Engine Core
# ---------------------------------------------------------------------------


@dataclass
class ANECoreConfig:
    """Configuration for an Apple Neural Engine Core.

    Real-world ANE configurations:

        Parameter          | A14 (iPhone 12) | M1          | M2
        ───────────────────┼─────────────────┼─────────────┼──────────
        Cores              | 16              | 16          | 16
        TOPS               | 11              | 11          | 15.8
        Format             | FP16/INT8       | FP16/INT8   | FP16/INT8
        On-chip memory     | varies          | varies      | varies

    Our simplified model focuses on one core with configurable MAC count,
    DMA bandwidth, and buffer sizes.

    Fields:
        num_macs:             MAC units per core.
        mac_format:           FP format for MAC operations.
        accumulator_format:   FP format for accumulation.
        sram_size:            On-chip SRAM in bytes.
        activation_buffer:    Activation (input) buffer in bytes.
        weight_buffer:        Weight buffer in bytes.
        output_buffer:        Output buffer in bytes.
        dma_bandwidth:        Elements transferred per DMA cycle.
    """

    num_macs: int = 16
    mac_format: FloatFormat = FP16
    accumulator_format: FloatFormat = FP32

    sram_size: int = 4194304
    activation_buffer: int = 131072
    weight_buffer: int = 524288
    output_buffer: int = 131072

    dma_bandwidth: int = 10


# ---------------------------------------------------------------------------
# NeuralEngineCore — the main ANE Core simulator
# ---------------------------------------------------------------------------


class NeuralEngineCore:
    """Apple Neural Engine Core simulator.

    Uses a MACArrayEngine from Layer 8 internally, adding DMA simulation,
    activation pipeline, and compiler-generated schedule support.

    === Execution Model ===

    The ANE Core has no runtime scheduler. Instead, it follows a
    compiler-generated schedule that specifies exactly what happens
    on each cycle. The schedule is created by:

    1. The user dispatching a WorkItem with input_data and weight_data
    2. The ANE Core's internal "compiler" generating a schedule
    3. The schedule being loaded into the MACArrayEngine
    4. The engine executing the schedule cycle by cycle

    Our "compiler" is a simple method that generates a schedule for
    a dot product or small matrix-vector multiply. Real Core ML
    compilation is vastly more complex.

    === DMA Simulation ===

    In real ANE hardware, data must be DMA'd from main memory to
    on-chip SRAM before the MACs can process it. This takes time:

        DMA bandwidth: 10 elements/cycle (our default)
        Loading 160 elements: 16 cycles
        Loading 1600 elements: 160 cycles

    This DMA latency is why ANE performance is often memory-bandwidth
    bound for small models — the MACs can compute faster than data
    can be loaded.

    Example:
        >>> from clock import Clock
        >>> clock = Clock()
        >>> ane = NeuralEngineCore(ANECoreConfig(num_macs=4), clock)
        >>> ane.dispatch(WorkItem(
        ...     work_id=0,
        ...     input_data=[[1.0, 2.0, 3.0, 4.0]],
        ...     weight_data=[[0.5], [0.5], [0.5], [0.5]],
        ... ))
        >>> traces = ane.run()
        >>> ane.result
        [[5.0]]
    """

    def __init__(self, config: ANECoreConfig, clock: Clock) -> None:
        self._config = config
        self._clock = clock
        self._cycle = 0

        # Internal MAC array engine
        self._mac_engine = MACArrayEngine(
            MACArrayConfig(
                num_macs=config.num_macs,
                input_buffer_size=max(
                    config.activation_buffer // 4, 1024
                ),
                weight_buffer_size=max(
                    config.weight_buffer // 4, 4096
                ),
                output_buffer_size=max(
                    config.output_buffer // 4, 1024
                ),
                float_format=FP32,  # use FP32 internally
                accumulator_format=FP32,
                has_activation_unit=True,
            ),
            clock,
        )

        self._idle_flag = True
        self._work_items: list[WorkItem] = []
        self._result: list[list[float]] = []

    # --- Properties ---

    @property
    def name(self) -> str:
        """Compute unit name."""
        return "ANECore"

    @property
    def architecture(self) -> Architecture:
        """This is an Apple ANE Core."""
        return Architecture.APPLE_ANE_CORE

    @property
    def idle(self) -> bool:
        """True if no work remains."""
        return self._idle_flag

    @property
    def config(self) -> ANECoreConfig:
        """The ANE Core configuration."""
        return self._config

    @property
    def result(self) -> list[list[float]]:
        """The result from the last computation."""
        return self._result

    @property
    def mac_engine(self) -> MACArrayEngine:
        """Access to the underlying MAC array engine."""
        return self._mac_engine

    # --- Dispatch ---

    def dispatch(self, work: WorkItem) -> None:
        """Dispatch an inference tile to this ANE Core.

        The WorkItem must provide input_data and weight_data. The ANE
        Core will compute: result = input_data x weight_data, applying
        any activation function specified in the schedule.

        Args:
            work: WorkItem with input_data and weight_data.
        """
        self._work_items.append(work)
        self._idle_flag = False

    # --- Execution ---

    def step(self, clock_edge: ClockEdge) -> ComputeUnitTrace:
        """Advance one cycle of the ANE Core.

        If work is pending, generates a compiler schedule, loads data
        into the MAC engine, and runs it to completion.

        Args:
            clock_edge: The clock edge that triggered this step.

        Returns:
            A ComputeUnitTrace for this cycle.
        """
        self._cycle += 1

        if self._idle_flag or not self._work_items:
            return self._make_idle_trace()

        work = self._work_items[0]
        self._process_work_item(work)
        self._work_items.pop(0)

        if not self._work_items:
            self._idle_flag = True

        rows = len(self._result)
        cols = len(self._result[0]) if self._result else 0

        return ComputeUnitTrace(
            cycle=self._cycle,
            unit_name=self.name,
            architecture=self.architecture,
            scheduler_action=f"inference complete: {rows}x{cols} result",
            active_warps=0 if self._idle_flag else 1,
            total_warps=1,
            engine_traces={},
            shared_memory_used=0,
            shared_memory_total=self._config.sram_size,
            register_file_used=self._config.num_macs,
            register_file_total=self._config.num_macs,
            occupancy=0.0 if self._idle_flag else 1.0,
        )

    def run(self, max_cycles: int = 100000) -> list[ComputeUnitTrace]:
        """Run until all work completes or max_cycles."""
        from clock import ClockEdge

        traces: list[ComputeUnitTrace] = []
        for cycle_num in range(1, max_cycles + 1):
            edge = ClockEdge(
                cycle=cycle_num, value=1, is_rising=True, is_falling=False
            )
            trace = self.step(edge)
            traces.append(trace)
            if self.idle:
                break
        return traces

    def run_inference(
        self,
        inputs: list[list[float]],
        weights: list[list[float]],
        activation_fn: str = "relu",
    ) -> list[list[float]]:
        """Convenience: run a complete inference pass.

        Performs matmul + activation function, simulating how the ANE
        processes one layer of a neural network.

        === Inference Pipeline ===

        1. DMA load inputs into activation buffer
        2. DMA load weights into weight buffer
        3. MAC: multiply input elements by weights
        4. Reduce: sum MAC results
        5. Activate: apply activation function
        6. DMA store outputs

        Args:
            inputs:        Input activation matrix (M x K).
            weights:       Weight matrix (K x N).
            activation_fn: Activation function ("relu", "sigmoid", "tanh", "none").

        Returns:
            Result matrix with activation applied (M x N).
        """
        result = self._matmul(inputs, weights)

        # Apply activation function
        if activation_fn != "none":
            result = self._apply_activation(result, activation_fn)

        self._result = result
        return result

    def reset(self) -> None:
        """Reset all state."""
        self._mac_engine.reset()
        self._work_items.clear()
        self._result.clear()
        self._idle_flag = True
        self._cycle = 0

    # --- Private helpers ---

    def _process_work_item(self, work: WorkItem) -> None:
        """Process a single work item by performing matmul."""
        if work.input_data is not None and work.weight_data is not None:
            self._result = self._matmul(work.input_data, work.weight_data)
        else:
            self._result = []

    def _matmul(
        self,
        a: list[list[float]],
        b: list[list[float]],
    ) -> list[list[float]]:
        """Perform matrix multiplication using the MAC engine.

        For each element of the output matrix, we compute a dot product
        using the MAC array. This simulates how the ANE processes
        matrix multiplications tile by tile.

        Args:
            a: Input matrix (M x K).
            b: Weight matrix (K x N).

        Returns:
            Result matrix C = A x B (M x N).
        """
        if not a or not b:
            return []

        m = len(a)
        k = len(a[0]) if a else 0
        n = len(b[0]) if b else 0

        result: list[list[float]] = []
        for i in range(m):
            row: list[float] = []
            for j in range(n):
                # Dot product of row i of A and column j of B
                dot = 0.0
                for kk in range(k):
                    dot += a[i][kk] * b[kk][j]
                row.append(dot)
            result.append(row)

        return result

    def _apply_activation(
        self,
        matrix: list[list[float]],
        fn_name: str,
    ) -> list[list[float]]:
        """Apply activation function element-wise.

        Simulates the ANE's dedicated activation pipeline hardware.

        Args:
            matrix: Input matrix.
            fn_name: Activation function name.

        Returns:
            Matrix with activation applied.
        """
        result: list[list[float]] = []
        for row in matrix:
            new_row: list[float] = []
            for val in row:
                match fn_name:
                    case "relu":
                        new_row.append(max(0.0, val))
                    case "sigmoid":
                        clamped = max(-500.0, min(500.0, val))
                        new_row.append(1.0 / (1.0 + math.exp(-clamped)))
                    case "tanh":
                        new_row.append(math.tanh(val))
                    case _:
                        new_row.append(val)
            result.append(new_row)
        return result

    def _make_idle_trace(self) -> ComputeUnitTrace:
        """Produce a trace for when the ANE Core is idle."""
        return ComputeUnitTrace(
            cycle=self._cycle,
            unit_name=self.name,
            architecture=self.architecture,
            scheduler_action="idle",
            active_warps=0,
            total_warps=1,
            engine_traces={},
            shared_memory_used=0,
            shared_memory_total=self._config.sram_size,
            register_file_used=0,
            register_file_total=self._config.num_macs,
            occupancy=0.0,
        )

    def __repr__(self) -> str:
        return (
            f"NeuralEngineCore(macs={self._config.num_macs}, "
            f"idle={self._idle_flag})"
        )
