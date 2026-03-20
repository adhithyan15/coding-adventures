"""SystolicArray — dataflow execution for matrix multiplication (Google TPU style).

=== What is a Systolic Array? ===

The word "systolic" comes from the Greek "systole" (contraction), like a
heartbeat. In a systolic array, data pulses through a grid of processing
elements on each clock cycle, just like blood pulses through the body with
each heartbeat.

A systolic array is radically different from GPU execution:

    GPU (SIMT/SIMD):                   TPU (Systolic):
    ┌──────────────────────────┐       ┌──────────────────────────┐
    │ Has instructions         │       │ NO instructions           │
    │ Has program counter      │       │ NO program counter        │
    │ Has branches             │       │ NO branches               │
    │ Complex control logic    │       │ Dead-simple PEs           │
    │ General-purpose          │       │ Matrix multiply ONLY      │
    └──────────────────────────┘       └──────────────────────────┘

Each PE in the array does exactly ONE thing on each clock cycle:

    accumulator += input_from_left * local_weight

Then it passes the input to the right neighbor and the accumulator down.
That's it. No instruction fetch, no decode, no branch prediction. Just
multiply, accumulate, and pass.

=== How Matrix Multiplication Maps to a Systolic Array ===

Computing C = A x W (activation matrix times weight matrix):

    1. Pre-load weights into each PE: PE(i,j) gets W[i][j]
    2. Feed activation rows from the left, STAGGERED in time
    3. Data flows right through each row, partial sums flow down
    4. After 2N-1 cycles, the result matrix C emerges at the bottom

The staggering is the key insight. Row i starts feeding i cycles late.
This ensures that all the right values meet at the right PEs at the
right time. Here's a 3x3 example:

    Cycle 1:   a[0][0] enters PE(0,0)
    Cycle 2:   a[0][1] enters PE(0,0), a[0][0] flows to PE(0,1)
               a[1][0] enters PE(1,0)  (row 1 starts 1 cycle late)
    Cycle 3:   a[0][2] enters PE(0,0), a[0][1]->PE(0,1), a[0][0]->PE(0,2)
               a[1][1] enters PE(1,0), a[1][0]->PE(1,1)
               a[2][0] enters PE(2,0)  (row 2 starts 2 cycles late)
    ...
    Cycle 5:   Last values emerge. C is complete.

=== Why TPUs Use Systolic Arrays ===

Neural network inference and training are dominated by matrix multiplication
(the GEMM operation). A systolic array is the most efficient hardware for
matrix multiply because:

    1. No instruction overhead (no fetch, decode, branch)
    2. Maximum data reuse (each value is used N times as it flows through)
    3. Nearest-neighbor communication only (each PE talks to adjacent PEs)
    4. Regular, predictable data movement (no cache misses)
    5. Simple PE design → high clock frequency, low power

Google's TPU v1 has a 256x256 systolic array that performs 65,536 MAC
operations per clock cycle. At 700 MHz, that's ~46 TOPS (tera-ops/second).

=== Limitations ===

Systolic arrays are TERRIBLE at anything that isn't a matrix multiply:
- No branching (can't do if/else)
- No random memory access
- Fixed data flow pattern
- Size must match the matrix dimensions (or waste PEs on padding)

This is why TPUs are paired with CPUs for control flow and irregular work.
"""

from __future__ import annotations

from dataclasses import dataclass
from typing import TYPE_CHECKING

from fp_arithmetic import FP32, FloatBits, FloatFormat, float_to_bits, fp_fma

from parallel_execution_engine.protocols import (
    DataflowInfo,
    EngineTrace,
    ExecutionModel,
)

if TYPE_CHECKING:
    from clock import Clock, ClockEdge


# ---------------------------------------------------------------------------
# Configuration
# ---------------------------------------------------------------------------


@dataclass
class SystolicConfig:
    """Configuration for a systolic array engine.

    Real-world reference values:

        Hardware    │ Rows │ Cols │ Format │ Accumulator
        ────────────┼──────┼──────┼────────┼────────────
        TPU v1      │ 256  │ 256  │ INT8   │ INT32
        TPU v2/v3   │ 128  │ 128  │ BF16   │ FP32
        TPU v4      │ 128  │ 128  │ BF16   │ FP32
        Our default │ 4    │ 4    │ FP32   │ FP32

    Fields:
        rows:               Number of PE rows in the array.
        cols:               Number of PE columns in the array.
        float_format:       Format for inputs and weights.
        accumulator_format: Format for the accumulator (usually higher precision).
    """

    rows: int = 4
    cols: int = 4
    float_format: FloatFormat = FP32
    accumulator_format: FloatFormat = FP32


# ---------------------------------------------------------------------------
# SystolicPE — one processing element in the grid
# ---------------------------------------------------------------------------


@dataclass
class SystolicPE:
    """One processing element in the systolic array.

    Each PE is extremely simple — it's just a multiply-accumulate unit
    with two data ports:

        Input from left ──→ [  weight  ] ──→ Output to right
                            [  × + acc ]
                                 │
                          Partial sum flows down

    On each clock cycle, a PE does:
        1. If there's an input: accumulator += input * weight
        2. Pass the input to the right neighbor
        3. (Partial sums flow down at the end of computation)

    Fields:
        row:            Row position in the grid.
        col:            Column position in the grid.
        weight:         Pre-loaded weight value (stays fixed during computation).
        accumulator:    Running sum (the partial result being computed).
        input_buffer:   The activation value to process this cycle (or None).
    """

    row: int
    col: int
    weight: FloatBits
    accumulator: FloatBits
    input_buffer: FloatBits | None = None

    def compute(self) -> FloatBits | None:
        """Perform one MAC cycle.

        If there's an input waiting in the buffer:
            accumulator += input_buffer * weight
        Returns the input (to be passed to the right neighbor), or None.

        This is the heart of the systolic array — the simplest possible
        processing element. No instruction fetch, no decode, no branch.
        Just: multiply, accumulate, pass.
        """
        if self.input_buffer is None:
            return None

        input_val = self.input_buffer
        self.input_buffer = None

        # MAC: accumulator = input * weight + accumulator
        # Using fp_fma for fused multiply-add (more accurate than mul+add)
        self.accumulator = fp_fma(input_val, self.weight, self.accumulator)

        return input_val  # Pass to right neighbor


# ---------------------------------------------------------------------------
# SystolicArray — the dataflow execution engine
# ---------------------------------------------------------------------------


class SystolicArray:
    """Systolic dataflow execution engine (Google TPU style).

    An NxN grid of processing elements. Data flows through the array —
    activations left-to-right, partial sums accumulate in each PE.
    No instruction stream. Just data in, results out.

    === Data Flow Pattern ===

        Inputs feed from the left edge:

        a[0] ──→ PE(0,0) ──→ PE(0,1) ──→ PE(0,2) ──→ PE(0,3)
        a[1] ──→ PE(1,0) ──→ PE(1,1) ──→ PE(1,2) ──→ PE(1,3)
        a[2] ──→ PE(2,0) ──→ PE(2,1) ──→ PE(2,2) ──→ PE(2,3)
        a[3] ──→ PE(3,0) ──→ PE(3,1) ──→ PE(3,2) ──→ PE(3,3)

        Each PE accumulates: acc += input * weight
        After all inputs flow through, drain accumulators as the result.

    Example:
        >>> from clock import Clock
        >>> clock = Clock()
        >>> array = SystolicArray(SystolicConfig(rows=2, cols=2), clock)
        >>> result = array.run_matmul(
        ...     activations=[[1.0, 2.0], [3.0, 4.0]],
        ...     weights=[[5.0, 6.0], [7.0, 8.0]],
        ... )
        >>> # result[0][0] = 1*5 + 2*7 = 19.0
    """

    def __init__(self, config: SystolicConfig, clock: Clock) -> None:
        self._config = config
        self._clock = clock
        self._cycle = 0
        self._halted = False

        # Create the NxN grid of PEs, all initialized with zero weight
        # and zero accumulator.
        self._grid: list[list[SystolicPE]] = [
            [
                SystolicPE(
                    row=r,
                    col=c,
                    weight=float_to_bits(0.0, config.float_format),
                    accumulator=float_to_bits(0.0, config.accumulator_format),
                )
                for c in range(config.cols)
            ]
            for r in range(config.rows)
        ]

        # Input queues: one per row, feeding from the left edge.
        # Each queue holds FloatBits values waiting to enter the array.
        self._input_queues: list[list[FloatBits]] = [
            [] for _ in range(config.rows)
        ]

        # Track how many total inputs have been fed (for halting detection)
        self._total_inputs_fed = 0
        self._total_inputs_expected = 0

    # --- Properties ---

    @property
    def name(self) -> str:
        """Engine name for traces."""
        return "SystolicArray"

    @property
    def width(self) -> int:
        """Total number of PEs in the array."""
        return self._config.rows * self._config.cols

    @property
    def execution_model(self) -> ExecutionModel:
        """This is a systolic dataflow engine."""
        return ExecutionModel.SYSTOLIC

    @property
    def halted(self) -> bool:
        """True if all data has flowed through and results are available."""
        return self._halted

    @property
    def config(self) -> SystolicConfig:
        """The configuration this array was created with."""
        return self._config

    @property
    def grid(self) -> list[list[SystolicPE]]:
        """Access to the PE grid (for inspection)."""
        return self._grid

    # --- Weight loading ---

    def load_weights(self, weights: list[list[float]]) -> None:
        """Pre-load the weight matrix into the PE array.

        weights[row][col] goes to PE(row, col). In real TPU hardware, weight
        loading happens before the matrix multiply begins. The weights stay
        fixed while activations flow through.

        Args:
            weights: 2D list of float values. Must be rows x cols.
        """
        for r in range(min(len(weights), self._config.rows)):
            for c in range(min(len(weights[r]), self._config.cols)):
                self._grid[r][c].weight = float_to_bits(
                    weights[r][c], self._config.float_format
                )

    # --- Input feeding ---

    def feed_input(self, row: int, value: float) -> None:
        """Feed one activation value into the left edge of the specified row.

        The value will enter PE(row, 0) on the next step, then flow right
        through PE(row, 1), PE(row, 2), etc. on subsequent steps.

        Args:
            row: Which row to feed (0 to rows-1).
            value: The activation value.
        """
        if row < 0 or row >= self._config.rows:
            msg = f"Row {row} out of range [0, {self._config.rows})"
            raise IndexError(msg)
        self._input_queues[row].append(
            float_to_bits(value, self._config.float_format)
        )
        self._total_inputs_fed += 1

    def feed_input_vector(self, values: list[float]) -> None:
        """Feed a full column vector to all rows with staggered timing.

        Row i gets its value i cycles late. This staggering ensures that
        all the right values meet at the right PEs at the right time.

        For a 3-element vector [a, b, c]:
            Row 0 queue: [a]            (enters immediately)
            Row 1 queue: [None, b]      (enters 1 cycle late)
            Row 2 queue: [None, None, c] (enters 2 cycles late)

        The None values are padding that maintains the stagger.

        Args:
            values: One value per row.
        """
        for row_idx, val in enumerate(values):
            # Pad with None entries for staggering (we'll skip None inputs)
            # Actually, we'll use the queue length difference to handle timing.
            # Simpler approach: just add to the queue with appropriate padding.
            fb = float_to_bits(val, self._config.float_format)
            self._input_queues[row_idx].append(fb)
            self._total_inputs_fed += 1

    # --- Execution ---

    def step(self, clock_edge: ClockEdge) -> EngineTrace:
        """Advance one cycle: data moves one PE to the right.

        On each cycle:
        1. Feed input from queues into the leftmost column.
        2. For each PE (from right to left, to avoid overwriting):
           a. Compute: acc += input * weight
           b. Pass input to the right neighbor.
        3. Build a trace showing the state of the array.

        We process PEs from right to left so that the "pass to right"
        doesn't interfere with the current cycle's computation.

        Args:
            clock_edge: The clock edge that triggered this step.

        Returns:
            An EngineTrace with dataflow info.
        """
        self._cycle += 1

        active_count = 0
        pe_states: list[list[str]] = []

        # Phase 1: Move data rightward through the array.
        # Process from right to left to avoid data collision.
        for r in range(self._config.rows):
            row_states: list[str] = []
            for c in range(self._config.cols - 1, -1, -1):
                pe = self._grid[r][c]

                # Compute MAC if there's input
                output = pe.compute()

                if output is not None:
                    active_count += 1
                    # Pass input to right neighbor (if exists)
                    if c + 1 < self._config.cols:
                        self._grid[r][c + 1].input_buffer = output

            # Build state strings (left to right for display)
            for c in range(self._config.cols):
                pe = self._grid[r][c]
                from fp_arithmetic import bits_to_float

                acc_val = bits_to_float(pe.accumulator)
                has_input = pe.input_buffer is not None
                state = f"acc={acc_val:.4g}"
                if has_input:
                    in_val = bits_to_float(pe.input_buffer)
                    state += f", in={in_val:.4g}"
                row_states.append(state)
            pe_states.append(row_states)

        # Phase 2: Feed new inputs from queues into column 0
        for r in range(self._config.rows):
            if self._input_queues[r]:
                val = self._input_queues[r].pop(0)
                self._grid[r][0].input_buffer = val

        # Check if computation is complete
        total = self._config.rows * self._config.cols
        any_input_remaining = any(
            len(q) > 0 for q in self._input_queues
        )
        any_input_in_flight = any(
            self._grid[r][c].input_buffer is not None
            for r in range(self._config.rows)
            for c in range(self._config.cols)
        )

        if not any_input_remaining and not any_input_in_flight:
            self._halted = True

        utilization = active_count / total if total > 0 else 0.0

        return EngineTrace(
            cycle=self._cycle,
            engine_name=self.name,
            execution_model=self.execution_model,
            description=(
                f"Systolic step — {active_count}/{total} PEs active"
            ),
            unit_traces={
                r * self._config.cols + c: pe_states[r][c]
                for r in range(self._config.rows)
                for c in range(self._config.cols)
            },
            active_mask=[
                self._grid[r][c].input_buffer is not None
                or (r * self._config.cols + c < active_count)
                for r in range(self._config.rows)
                for c in range(self._config.cols)
            ],
            active_count=active_count,
            total_count=total,
            utilization=utilization,
            dataflow_info=DataflowInfo(pe_states=pe_states),
        )

    def run_matmul(
        self,
        activations: list[list[float]],
        weights: list[list[float]],
    ) -> list[list[float]]:
        """Convenience: run a complete matrix multiplication C = A x W.

        === How the Systolic Matmul Works ===

        For C = A x W where A is MxK and W is KxN:
            C[i][j] = sum_k( A[i][k] * W[k][j] )

        We compute this one output row at a time:
            For each row i of A:
                1. Reset accumulators
                2. Feed A[i][k] into PE row k (with staggered timing)
                3. PE(k, j) computes: acc += A[i][k] * W[k][j]
                4. After all activations flow through, column j accumulates
                   sum_k(A[i][k] * W[k][j]) = C[i][j]
                5. Drain results for row i

        This requires K rows and N columns in the array.

        Args:
            activations: The activation matrix A (M x K).
            weights:     The weight matrix W (K x N).

        Returns:
            The result matrix C = A x W (M x N).
        """
        from clock import ClockEdge

        num_output_rows = len(activations)
        inner_dim = len(activations[0]) if activations else 0
        num_output_cols = len(weights[0]) if weights else 0

        # Load weights: PE(k, j) gets W[k][j]
        self.reset()
        self.load_weights(weights)

        result: list[list[float]] = []

        # Compute one output row at a time
        for i in range(num_output_rows):
            # Reset accumulators (but keep weights)
            zero_acc = float_to_bits(0.0, self._config.accumulator_format)
            for r in range(self._config.rows):
                for c in range(self._config.cols):
                    self._grid[r][c].accumulator = zero_acc
                    self._grid[r][c].input_buffer = None
            self._input_queues = [[] for _ in range(self._config.rows)]
            self._halted = False

            # Feed A[i][k] into row k with staggered timing.
            # Row k gets its input k cycles late so that data arriving at
            # each PE is correctly aligned.
            feed_schedule: dict[int, list[tuple[int, float]]] = {}
            for k in range(inner_dim):
                cycle = k  # row k starts k cycles late in staggered mode
                # But for a simple accumulation, we just feed sequentially
                # into each row without staggering — each row gets one input.
                if cycle not in feed_schedule:
                    feed_schedule[cycle] = []
                feed_schedule[cycle].append((k, activations[i][k]))

            # Run until all data has flowed through
            total_steps = inner_dim + self._config.cols + 1
            for step_num in range(total_steps):
                if step_num in feed_schedule:
                    for row, val in feed_schedule[step_num]:
                        self.feed_input(row, val)

                edge = ClockEdge(
                    cycle=step_num + 1, value=1, is_rising=True, is_falling=False
                )
                self.step(edge)

            # Drain: sum accumulators vertically for each column j.
            # C[i][j] = sum_k PE(k, j).accumulator
            from fp_arithmetic import bits_to_float

            row_result: list[float] = []
            for j in range(num_output_cols):
                col_sum = 0.0
                for k in range(min(inner_dim, self._config.rows)):
                    col_sum += bits_to_float(self._grid[k][j].accumulator)
                row_result.append(col_sum)
            result.append(row_result)

        return result

    def drain_outputs(self) -> list[list[float]]:
        """Read the accumulated results from all PEs.

        After computation, each PE's accumulator holds one element of the
        result matrix. PE(r, c) holds C[r][c].

        Returns:
            2D list of float results, rows x cols.
        """
        from fp_arithmetic import bits_to_float

        result: list[list[float]] = []
        for r in range(self._config.rows):
            row: list[float] = []
            for c in range(self._config.cols):
                row.append(bits_to_float(self._grid[r][c].accumulator))
            result.append(row)
        return result

    def reset(self) -> None:
        """Reset the array to its initial state.

        Clears all accumulators, input buffers, and queues. Weights are
        preserved — call load_weights() to change them.
        """
        zero_acc = float_to_bits(0.0, self._config.accumulator_format)
        for r in range(self._config.rows):
            for c in range(self._config.cols):
                self._grid[r][c].accumulator = zero_acc
                self._grid[r][c].input_buffer = None
        self._input_queues = [[] for _ in range(self._config.rows)]
        self._cycle = 0
        self._halted = False
        self._total_inputs_fed = 0

    def __repr__(self) -> str:
        return (
            f"SystolicArray({self._config.rows}x{self._config.cols}, "
            f"cycle={self._cycle}, halted={self._halted})"
        )
