"""MatrixMultiplyUnit — Google TPU MXU simulator.

=== What is an MXU? ===

The Matrix Multiply Unit is the heart of Google's TPU (Tensor Processing
Unit). It's fundamentally different from GPU compute units — there are NO
threads, NO warps, NO schedulers. Instead, it has:

1. **Systolic arrays** — the main compute engine (from Layer 8)
2. **Vector unit** — for element-wise operations (activation functions)
3. **Accumulators** — for storing partial matrix results
4. **Control sequencer** — manages the tiling schedule

=== Why No Threads? ===

Matrix multiplication is perfectly predictable. You know exactly which
values need to be multiplied together and in what order. There's no
branching, no data-dependent control flow, no need for a runtime scheduler.

This predictability lets the compiler (XLA) generate a complete execution
plan at compile time — which tiles to load, when to multiply, when to
drain results. The MXU hardware just follows this plan cycle by cycle.

    GPU:  Complex hardware scheduler decides at runtime
    TPU:  Simple hardware follows compile-time plan

=== Tiling: How Large Matmuls Fit Small Arrays ===

A TPU v2 has a 128x128 systolic array, but neural networks often need
matmuls like 1024x1024 or even 4096x4096. The solution is **tiling**:

    Large matmul: C[1024x1024] = A[1024x1024] x B[1024x1024]

    The MXU can only do 128x128 at a time, so:

    for i in range(0, 1024, 128):        # 8 row tiles
        for j in range(0, 1024, 128):    # 8 column tiles
            acc = 0
            for k in range(0, 1024, 128):  # 8 reduction tiles
                load A[i:i+128, k:k+128] into activation buffer
                load B[k:k+128, j:j+128] into weight buffer
                acc += systolic_matmul(A_tile, B_tile)
            C[i:i+128, j:j+128] = apply_vector_ops(acc)

This nested loop is the MXU's "schedule." In our simulator, the control
sequencer manages this tiling automatically.

=== Architecture Diagram ===

    MatrixMultiplyUnit (TPU v2-style)
    +---------------------------------------------------------------+
    |                                                               |
    |  Control Sequencer                                            |
    |  +----------------------------------------------------------+ |
    |  | Tile schedule: load A[0:128], matmul, load A[128:256]    | |
    |  +----------------------------------------------------------+ |
    |                                                               |
    |  +---------------------------------------------+              |
    |  | Systolic Array (128x128)                     |              |
    |  |   Weights pre-loaded into PEs                |              |
    |  |   Activations stream in from left            |              |
    |  |   Partial sums flow down to accumulators     |              |
    |  +---------------------------------------------+              |
    |                    |                                          |
    |                    v                                          |
    |  +---------------------------------------------+              |
    |  | Accumulators (128 x FP32)                    |              |
    |  +---------------------------------------------+              |
    |                    |                                          |
    |                    v                                          |
    |  +---------------------------------------------+              |
    |  | Vector Unit (128-wide)                       |              |
    |  | ReLU, sigmoid, add bias, normalize           |              |
    |  +---------------------------------------------+              |
    +---------------------------------------------------------------+
"""

from __future__ import annotations

import math
from dataclasses import dataclass
from typing import TYPE_CHECKING

from fp_arithmetic import BF16, FP32, FloatFormat
from parallel_execution_engine import (
    SystolicArray,
    SystolicConfig,
)

from compute_unit.protocols import (
    Architecture,
    ComputeUnitTrace,
    WorkItem,
)

if TYPE_CHECKING:
    from clock import Clock, ClockEdge


# ---------------------------------------------------------------------------
# MXUConfig — configuration for a TPU-style Matrix Multiply Unit
# ---------------------------------------------------------------------------


@dataclass
class MXUConfig:
    """Configuration for a TPU-style Matrix Multiply Unit.

    Real-world MXU configurations:

        Parameter           | TPU v1       | TPU v2/v3    | TPU v4
        ────────────────────┼──────────────┼──────────────┼──────────
        Array size          | 256x256      | 128x128      | 128x128
        Input format        | INT8         | BF16         | BF16
        Accumulator format  | INT32        | FP32         | FP32
        Vector width        | 256          | 128          | 128
        HBM bandwidth       | 30 GB/s      | 900 GB/s     | 1200 GB/s

    Our default models a simplified TPU v2-style MXU with a smaller
    array for faster simulation.

    Fields:
        array_rows:            Systolic array rows.
        array_cols:            Systolic array columns.
        systolic_format:       FP format for systolic array inputs.
        accumulator_format:    FP format for accumulation (higher precision).
        vector_width:          Width of the vector unit.
        vector_format:         FP format for vector operations.
        accumulator_count:     Number of accumulator registers.
        weight_buffer_size:    Weight staging buffer in bytes.
        activation_buffer_size: Activation buffer in bytes.
    """

    array_rows: int = 128
    array_cols: int = 128
    systolic_format: FloatFormat = BF16
    accumulator_format: FloatFormat = FP32

    vector_width: int = 128
    vector_format: FloatFormat = FP32

    accumulator_count: int = 128
    weight_buffer_size: int = 4194304
    activation_buffer_size: int = 2097152


# ---------------------------------------------------------------------------
# MatrixMultiplyUnit — the main MXU simulator
# ---------------------------------------------------------------------------


class MatrixMultiplyUnit:
    """Google TPU Matrix Multiply Unit simulator.

    Uses a systolic array from Layer 8 to perform matrix multiplication,
    with tiling logic for matrices larger than the array, and a vector
    unit for post-processing (activation functions, bias add).

    === Execution Model ===

    The MXU has no threads or schedulers. Instead, it processes **tiles**
    of a larger matrix operation. The control sequencer manages:

    1. Loading weight tiles into the systolic array
    2. Streaming activation tiles through the array
    3. Accumulating partial results
    4. Applying vector operations (activation functions)
    5. Storing output tiles

    === How dispatch() Works ===

    A WorkItem for the MXU must provide input_data and weight_data
    (not a program). The MXU decomposes the matmul into tiles and
    processes them sequentially.

    === How step() Works ===

    Each step advances the tiling schedule by one cycle. Depending on
    the current phase:
    - LOADING: Loading weights into the systolic array
    - COMPUTING: Streaming activations through the array
    - DRAINING: Reading results from accumulators
    - VECTOR: Applying activation functions

    Example:
        >>> from clock import Clock
        >>> clock = Clock()
        >>> mxu = MatrixMultiplyUnit(MXUConfig(array_rows=4, array_cols=4), clock)
        >>> mxu.dispatch(WorkItem(
        ...     work_id=0,
        ...     input_data=[[1.0, 2.0], [3.0, 4.0]],
        ...     weight_data=[[5.0, 6.0], [7.0, 8.0]],
        ... ))
        >>> traces = mxu.run()
    """

    def __init__(self, config: MXUConfig, clock: Clock) -> None:
        self._config = config
        self._clock = clock
        self._cycle = 0

        # Create the systolic array engine
        self._array = SystolicArray(
            SystolicConfig(
                rows=config.array_rows,
                cols=config.array_cols,
                float_format=FP32,  # use FP32 internally for simulation
                accumulator_format=FP32,
            ),
            clock,
        )

        # Accumulators for storing partial tile results
        self._accumulators: list[list[float]] = []

        # Result storage
        self._result: list[list[float]] = []

        # Tile schedule
        self._work_items: list[WorkItem] = []
        self._idle = True
        self._current_result: list[list[float]] = []

    # --- Properties ---

    @property
    def name(self) -> str:
        """Compute unit name."""
        return "MXU"

    @property
    def architecture(self) -> Architecture:
        """This is a Google MXU."""
        return Architecture.GOOGLE_MXU

    @property
    def idle(self) -> bool:
        """True if no work remains."""
        return self._idle

    @property
    def config(self) -> MXUConfig:
        """The MXU configuration."""
        return self._config

    @property
    def result(self) -> list[list[float]]:
        """The result matrix from the last matmul."""
        return self._current_result

    @property
    def systolic_array(self) -> SystolicArray:
        """Access to the underlying systolic array."""
        return self._array

    # --- Dispatch ---

    def dispatch(self, work: WorkItem) -> None:
        """Dispatch a matrix multiply operation.

        The WorkItem must provide input_data (activation matrix) and
        weight_data (weight matrix). The MXU will perform:

            result = input_data x weight_data

        using tiling if the matrices are larger than the systolic array.

        Args:
            work: WorkItem with input_data and weight_data set.
        """
        self._work_items.append(work)
        self._idle = False

    # --- Execution ---

    def step(self, clock_edge: ClockEdge) -> ComputeUnitTrace:
        """Advance one cycle of the MXU.

        If work is pending, performs the matmul using the systolic array.
        The systolic array handles the actual computation — we manage
        the tiling and result collection.

        Args:
            clock_edge: The clock edge that triggered this step.

        Returns:
            A ComputeUnitTrace for this cycle.
        """
        self._cycle += 1

        if self._idle or not self._work_items:
            return self._make_idle_trace()

        # Process the first pending work item
        work = self._work_items[0]

        if work.input_data is not None and work.weight_data is not None:
            # Perform the full matmul using the systolic array's run_matmul
            self._current_result = self._array.run_matmul(
                activations=work.input_data,
                weights=work.weight_data,
            )
        else:
            self._current_result = []

        # Mark work as done
        self._work_items.pop(0)
        if not self._work_items:
            self._idle = True

        # Build trace
        rows = len(self._current_result)
        cols = len(self._current_result[0]) if self._current_result else 0

        return ComputeUnitTrace(
            cycle=self._cycle,
            unit_name=self.name,
            architecture=self.architecture,
            scheduler_action=(
                f"matmul complete: {rows}x{cols} result"
            ),
            active_warps=0 if self._idle else 1,
            total_warps=1,
            engine_traces={},
            shared_memory_used=0,
            shared_memory_total=self._config.weight_buffer_size,
            register_file_used=self._config.accumulator_count,
            register_file_total=self._config.accumulator_count,
            occupancy=0.0 if self._idle else 1.0,
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

    def run_matmul(
        self,
        activations: list[list[float]],
        weights: list[list[float]],
        activation_fn: str = "none",
    ) -> list[list[float]]:
        """Convenience: run a complete matmul with optional activation.

        This is the high-level API for performing a matrix multiply. It:
        1. Tiles the input matrices to fit the systolic array
        2. Multiplies the tiles
        3. Applies the activation function to the result

        === Supported Activation Functions ===

            none:    f(x) = x              (identity)
            relu:    f(x) = max(0, x)      (most popular)
            sigmoid: f(x) = 1/(1+e^-x)    (squashes to [0,1])
            tanh:    f(x) = tanh(x)        (squashes to [-1,1])

        Args:
            activations: Input matrix A (M x K).
            weights:     Weight matrix W (K x N).
            activation_fn: Activation function name.

        Returns:
            Result matrix C = activation_fn(A x W) (M x N).
        """
        # Use the systolic array to do the actual matmul
        result = self._array.run_matmul(activations, weights)

        # Apply activation function via the vector unit
        if activation_fn != "none":
            result = self._apply_activation(result, activation_fn)

        self._current_result = result
        return result

    def reset(self) -> None:
        """Reset all state."""
        self._array.reset()
        self._accumulators.clear()
        self._current_result.clear()
        self._work_items.clear()
        self._idle = True
        self._cycle = 0

    # --- Private helpers ---

    def _apply_activation(
        self,
        matrix: list[list[float]],
        fn_name: str,
    ) -> list[list[float]]:
        """Apply an activation function element-wise to a matrix.

        This simulates the MXU's vector unit, which processes one row
        at a time, applying the activation function to each element.

        Args:
            matrix: The input matrix.
            fn_name: Activation function name ("relu", "sigmoid", "tanh").

        Returns:
            Matrix with activation applied element-wise.
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
        """Produce a trace for when the MXU is idle."""
        return ComputeUnitTrace(
            cycle=self._cycle,
            unit_name=self.name,
            architecture=self.architecture,
            scheduler_action="idle",
            active_warps=0,
            total_warps=1,
            engine_traces={},
            shared_memory_used=0,
            shared_memory_total=self._config.weight_buffer_size,
            register_file_used=0,
            register_file_total=self._config.accumulator_count,
            occupancy=0.0,
        )

    def __repr__(self) -> str:
        return (
            f"MatrixMultiplyUnit("
            f"{self._config.array_rows}x{self._config.array_cols}, "
            f"idle={self._idle})"
        )
