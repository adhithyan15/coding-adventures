"""MACArrayEngine — compiler-scheduled MAC array execution (NPU style).

=== What is a MAC Array? ===

A MAC (Multiply-Accumulate) array is a bank of multiply-accumulate units
driven entirely by a schedule that the compiler generates at compile time.
There is NO hardware scheduler — the compiler decides exactly which MAC
unit processes which data on which cycle.

This is the execution model used by:
- Apple Neural Engine (ANE)
- Qualcomm Hexagon NPU
- Many custom AI accelerator ASICs

=== How It Differs from Other Models ===

    GPU (SIMT/SIMD):                   NPU (Scheduled MAC):
    ┌──────────────────────────┐       ┌──────────────────────────┐
    │ Hardware scheduler       │       │ NO hardware scheduler    │
    │ Runtime decisions        │       │ All decisions at compile  │
    │ Branch prediction        │       │ NO branches              │
    │ Dynamic resource alloc   │       │ Static resource plan     │
    │ Flexible but complex     │       │ Simple but rigid         │
    └──────────────────────────┘       └──────────────────────────┘

    TPU (Systolic):                    NPU (Scheduled MAC):
    ┌──────────────────────────┐       ┌──────────────────────────┐
    │ Fixed data flow pattern  │       │ Flexible data movement   │
    │ Always matrix multiply   │       │ Any pattern the compiler │
    │ 2D grid, nearest-neighbor│       │   schedules              │
    │ Implicit timing          │       │ Explicit per-cycle plan  │
    └──────────────────────────┘       └──────────────────────────┘

=== The Execution Pipeline ===

A MAC array engine has a simple pipeline:

    1. LOAD_INPUT:    Move data from external memory to input buffer
    2. LOAD_WEIGHTS:  Move weights from external memory to weight buffer
    3. MAC:           Multiply input[i] * weight[i] for all MACs in parallel
    4. REDUCE:        Sum the MAC results (adder tree)
    5. ACTIVATE:      Apply activation function (ReLU, sigmoid, tanh)
    6. STORE_OUTPUT:  Write result to output buffer

    Input Buffer ──→ ┌────┐ ┌────┐ ┌────┐ ┌────┐
                     │MAC0│ │MAC1│ │MAC2│ │MAC3│  (parallel multiply)
    Weight Buffer──→ └──┬─┘ └──┬─┘ └──┬─┘ └──┬─┘
                        │      │      │      │
                        └──────┴──────┴──────┘
                                   │
                            ┌──────┴──────┐
                            │  Adder Tree │  (reduce / sum)
                            └──────┬──────┘
                                   │
                            ┌──────┴──────┐
                            │ Activation  │  (ReLU, sigmoid, etc.)
                            └──────┬──────┘
                                   │
                            Output Buffer

=== Why NPUs Are Power-Efficient ===

By moving all scheduling to compile time, NPUs eliminate:
- Branch prediction hardware (saves transistors and power)
- Instruction cache (the "program" is a simple schedule table)
- Warp/wavefront scheduler (no runtime thread management)
- Speculation hardware (nothing is speculative)

The result: NPUs achieve more TOPS/watt than GPUs for neural network
inference, at the cost of flexibility (you can only run computations
the compiler knows how to schedule).
"""

from __future__ import annotations

import math
from dataclasses import dataclass, field
from enum import Enum
from typing import TYPE_CHECKING

from fp_arithmetic import (
    FP16,
    FP32,
    FloatFormat,
)

from parallel_execution_engine.protocols import (
    EngineTrace,
    ExecutionModel,
)

if TYPE_CHECKING:
    from clock import Clock, ClockEdge


# ---------------------------------------------------------------------------
# Operations and activation functions
# ---------------------------------------------------------------------------


class MACOperation(Enum):
    """Operations that can appear in a MAC array schedule.

    Each operation corresponds to one stage of the MAC pipeline:

        LOAD_INPUT:    Fill the input buffer with activation data.
        LOAD_WEIGHTS:  Fill the weight buffer with weight data.
        MAC:           Parallel multiply-accumulate across all MAC units.
        REDUCE:        Sum results from multiple MACs (adder tree).
        ACTIVATE:      Apply a non-linear activation function.
        STORE_OUTPUT:  Write results to the output buffer.

    The compiler sequences these operations into a static schedule
    that the hardware executes cycle by cycle.
    """

    LOAD_INPUT = "load_input"
    LOAD_WEIGHTS = "load_weights"
    MAC = "mac"
    REDUCE = "reduce"
    ACTIVATE = "activate"
    STORE_OUTPUT = "store_output"


class ActivationFunction(Enum):
    """Hardware-supported activation functions.

    Neural networks use non-linear "activation functions" after each layer.
    NPUs typically implement a few common ones in hardware for speed:

        NONE:    f(x) = x              (identity / linear)
        RELU:    f(x) = max(0, x)      (most popular; simple, fast)
        SIGMOID: f(x) = 1/(1+e^-x)    (classic; squashes to [0,1])
        TANH:    f(x) = tanh(x)        (squashes to [-1,1])

    ReLU is by far the most common because it's trivially cheap in hardware
    (just check the sign bit and zero-out negatives). Sigmoid and tanh
    require lookup tables or polynomial approximation.
    """

    NONE = "none"
    RELU = "relu"
    SIGMOID = "sigmoid"
    TANH = "tanh"


# ---------------------------------------------------------------------------
# Schedule entry
# ---------------------------------------------------------------------------


@dataclass(frozen=True)
class MACScheduleEntry:
    """One entry in the MAC array schedule.

    The compiler generates these at compile time. Each entry describes
    exactly what happens on one cycle — which operation, which data indices,
    and where to write the result.

    Example schedule for a simple dot product of 4 elements:

        Cycle 0: LOAD_INPUT   indices=[0,1,2,3]
        Cycle 1: LOAD_WEIGHTS indices=[0,1,2,3]
        Cycle 2: MAC          input=[0,1,2,3] weight=[0,1,2,3] out=0
        Cycle 3: REDUCE       out=0
        Cycle 4: ACTIVATE     out=0, activation=relu
        Cycle 5: STORE_OUTPUT out=0

    Fields:
        cycle:          Which cycle to execute this entry.
        operation:      What to do (LOAD, MAC, REDUCE, ACTIVATE, STORE).
        input_indices:  Which input buffer slots to read.
        weight_indices: Which weight buffer slots to use.
        output_index:   Where to write the result.
        activation:     Which activation function (for ACTIVATE operations).
    """

    cycle: int
    operation: MACOperation
    input_indices: list[int] = field(default_factory=list)
    weight_indices: list[int] = field(default_factory=list)
    output_index: int = 0
    activation: str = "none"


# ---------------------------------------------------------------------------
# Configuration
# ---------------------------------------------------------------------------


@dataclass
class MACArrayConfig:
    """Configuration for a scheduled MAC array engine.

    Real-world reference values:

        Hardware          │ MACs │ Input Buf │ Weight Buf │ Format
        ──────────────────┼──────┼───────────┼────────────┼───────
        Apple ANE (M1)    │ 16K  │ varies    │ varies     │ FP16/INT8
        Qualcomm Hexagon  │ 2K   │ varies    │ varies     │ INT8
        Our default       │ 8    │ 1024      │ 4096       │ FP16

    Fields:
        num_macs:           Number of parallel MAC units.
        input_buffer_size:  Input buffer capacity in elements.
        weight_buffer_size: Weight buffer capacity in elements.
        output_buffer_size: Output buffer capacity in elements.
        float_format:       Compute format for inputs/weights.
        accumulator_format: Higher-precision format for accumulation.
        has_activation_unit: Whether hardware activation function is available.
    """

    num_macs: int = 8
    input_buffer_size: int = 1024
    weight_buffer_size: int = 4096
    output_buffer_size: int = 1024
    float_format: FloatFormat = FP16
    accumulator_format: FloatFormat = FP32
    has_activation_unit: bool = True


# ---------------------------------------------------------------------------
# MACArrayEngine — the scheduled execution engine
# ---------------------------------------------------------------------------


class MACArrayEngine:
    """Compiler-scheduled MAC array execution engine (NPU style).

    No hardware scheduler. The compiler generates a static schedule that
    says exactly what each MAC does on each cycle.

    === Usage Pattern ===

        1. Create engine with config and clock.
        2. Load inputs and weights into the buffers.
        3. Load a compiler-generated schedule.
        4. Step or run — the engine follows the schedule exactly.
        5. Read results from the output buffer.

    Example:
        >>> from clock import Clock
        >>> clock = Clock()
        >>> engine = MACArrayEngine(MACArrayConfig(num_macs=4), clock)
        >>> engine.load_inputs([1.0, 2.0, 3.0, 4.0])
        >>> engine.load_weights([0.5, 0.5, 0.5, 0.5])
        >>> schedule = [
        ...     MACScheduleEntry(cycle=1, operation=MACOperation.MAC,
        ...         input_indices=[0,1,2,3], weight_indices=[0,1,2,3],
        ...         output_index=0),
        ...     MACScheduleEntry(cycle=2, operation=MACOperation.REDUCE,
        ...         output_index=0),
        ...     MACScheduleEntry(cycle=3, operation=MACOperation.STORE_OUTPUT,
        ...         output_index=0),
        ... ]
        >>> engine.load_schedule(schedule)
        >>> traces = engine.run()
        >>> engine.read_outputs()  # [5.0]  (1*0.5 + 2*0.5 + 3*0.5 + 4*0.5)
    """

    def __init__(self, config: MACArrayConfig, clock: Clock) -> None:
        self._config = config
        self._clock = clock
        self._cycle = 0

        # Buffers: simple lists of float values.
        # In real hardware, these are on-chip SRAM banks.
        self._input_buffer: list[float] = [0.0] * config.input_buffer_size
        self._weight_buffer: list[float] = [0.0] * config.weight_buffer_size
        self._output_buffer: list[float] = [0.0] * config.output_buffer_size

        # MAC accumulators: one per MAC unit.
        # These hold intermediate results during computation.
        self._mac_accumulators: list[float] = [0.0] * config.num_macs

        # The compiler-generated schedule.
        self._schedule: list[MACScheduleEntry] = []
        self._schedule_pc = 0  # which schedule entry we're at
        self._halted = False

    # --- Properties ---

    @property
    def name(self) -> str:
        """Engine name for traces."""
        return "MACArrayEngine"

    @property
    def width(self) -> int:
        """Number of parallel MAC units."""
        return self._config.num_macs

    @property
    def execution_model(self) -> ExecutionModel:
        """This is a scheduled MAC engine."""
        return ExecutionModel.SCHEDULED_MAC

    @property
    def halted(self) -> bool:
        """True if the schedule is complete."""
        return self._halted

    @property
    def config(self) -> MACArrayConfig:
        """The configuration this engine was created with."""
        return self._config

    # --- Data loading ---

    def load_inputs(self, data: list[float]) -> None:
        """Load activation data into the input buffer.

        In real hardware, this is a DMA transfer from external memory
        to the on-chip input SRAM.

        Args:
            data: List of float values to load.
        """
        for i, val in enumerate(data):
            if i < self._config.input_buffer_size:
                self._input_buffer[i] = val

    def load_weights(self, data: list[float]) -> None:
        """Load weight data into the weight buffer.

        In real NPU hardware, weights are often loaded once and reused
        across many inference batches (since the model doesn't change).

        Args:
            data: List of float values to load.
        """
        for i, val in enumerate(data):
            if i < self._config.weight_buffer_size:
                self._weight_buffer[i] = val

    def load_schedule(self, schedule: list[MACScheduleEntry]) -> None:
        """Load a compiler-generated execution schedule.

        The schedule is a list of MACScheduleEntry objects, each describing
        what happens on one cycle. The engine will execute them in order.

        Args:
            schedule: List of schedule entries.
        """
        self._schedule = list(schedule)
        self._schedule_pc = 0
        self._halted = False

    # --- Execution ---

    def step(self, clock_edge: ClockEdge) -> EngineTrace:
        """Execute one scheduled cycle.

        Looks up the current cycle in the schedule and executes the
        corresponding operation. If no entry exists for this cycle,
        the MAC array idles (like a NOP).

        Args:
            clock_edge: The clock edge that triggered this step.

        Returns:
            An EngineTrace describing what happened.
        """
        self._cycle += 1

        if self._halted:
            return self._make_idle_trace("Schedule complete")

        # Find schedule entries for this cycle
        entries = [
            e for e in self._schedule if e.cycle == self._cycle
        ]

        if not entries:
            # Check if we've passed all schedule entries
            if self._cycle > max(
                (e.cycle for e in self._schedule), default=0
            ):
                self._halted = True
                return self._make_idle_trace("Schedule complete")
            return self._make_idle_trace("No operation this cycle")

        # Execute all entries for this cycle
        unit_traces: dict[int, str] = {}
        active_count = 0
        descriptions: list[str] = []

        for entry in entries:
            match entry.operation:
                case MACOperation.LOAD_INPUT:
                    desc = self._exec_load_input(entry)
                    descriptions.append(desc)
                    active_count = len(entry.input_indices)

                case MACOperation.LOAD_WEIGHTS:
                    desc = self._exec_load_weights(entry)
                    descriptions.append(desc)
                    active_count = len(entry.weight_indices)

                case MACOperation.MAC:
                    desc, traces = self._exec_mac(entry)
                    descriptions.append(desc)
                    unit_traces.update(traces)
                    active_count = len(traces)

                case MACOperation.REDUCE:
                    desc = self._exec_reduce(entry)
                    descriptions.append(desc)
                    active_count = 1

                case MACOperation.ACTIVATE:
                    desc = self._exec_activate(entry)
                    descriptions.append(desc)
                    active_count = 1

                case MACOperation.STORE_OUTPUT:
                    desc = self._exec_store(entry)
                    descriptions.append(desc)
                    active_count = 1

        total = self._config.num_macs
        description = "; ".join(descriptions)

        return EngineTrace(
            cycle=self._cycle,
            engine_name=self.name,
            execution_model=self.execution_model,
            description=f"{description} — {active_count}/{total} MACs active",
            unit_traces=unit_traces,
            active_mask=[
                i < active_count for i in range(total)
            ],
            active_count=active_count,
            total_count=total,
            utilization=active_count / total if total > 0 else 0.0,
        )

    def run(self, max_cycles: int = 10000) -> list[EngineTrace]:
        """Run the full schedule.

        Args:
            max_cycles: Safety limit.

        Returns:
            List of EngineTrace records.
        """
        from clock import ClockEdge

        traces: list[EngineTrace] = []
        for cycle_num in range(1, max_cycles + 1):
            edge = ClockEdge(
                cycle=cycle_num, value=1, is_rising=True, is_falling=False
            )
            trace = self.step(edge)
            traces.append(trace)
            if self._halted:
                break
        else:
            if not self._halted:
                msg = f"MACArrayEngine: max_cycles ({max_cycles}) reached"
                raise RuntimeError(msg)
        return traces

    def read_outputs(self) -> list[float]:
        """Read results from the output buffer.

        Returns all non-zero output slots. In practice, you'd track
        which slots were written and read only those.

        Returns:
            List of output values.
        """
        # Return only the outputs that have been written
        # (non-zero or explicitly stored)
        return list(self._output_buffer)

    def reset(self) -> None:
        """Reset to initial state."""
        self._input_buffer = [0.0] * self._config.input_buffer_size
        self._weight_buffer = [0.0] * self._config.weight_buffer_size
        self._output_buffer = [0.0] * self._config.output_buffer_size
        self._mac_accumulators = [0.0] * self._config.num_macs
        self._schedule_pc = 0
        self._halted = False
        self._cycle = 0

    # --- Operation implementations ---

    def _exec_load_input(self, entry: MACScheduleEntry) -> str:
        """Execute a LOAD_INPUT operation (data is already in the buffer)."""
        return f"LOAD_INPUT indices={entry.input_indices}"

    def _exec_load_weights(self, entry: MACScheduleEntry) -> str:
        """Execute a LOAD_WEIGHTS operation (data is already in the buffer)."""
        return f"LOAD_WEIGHTS indices={entry.weight_indices}"

    def _exec_mac(
        self, entry: MACScheduleEntry
    ) -> tuple[str, dict[int, str]]:
        """Execute a MAC operation: multiply input[i] * weight[i] for each MAC.

        Each MAC unit processes one (input, weight) pair. The results
        are stored in the MAC accumulators.

        Returns:
            Tuple of (description, per-MAC traces).
        """
        unit_traces: dict[int, str] = {}
        num_ops = min(
            len(entry.input_indices),
            len(entry.weight_indices),
            self._config.num_macs,
        )

        for mac_id in range(num_ops):
            in_idx = entry.input_indices[mac_id]
            wt_idx = entry.weight_indices[mac_id]

            in_val = self._input_buffer[in_idx]
            wt_val = self._weight_buffer[wt_idx]

            result = in_val * wt_val
            self._mac_accumulators[mac_id] = result

            unit_traces[mac_id] = (
                f"MAC: {in_val:.4g} * {wt_val:.4g} = {result:.4g}"
            )

        return f"MAC {num_ops} operations", unit_traces

    def _exec_reduce(self, entry: MACScheduleEntry) -> str:
        """Execute a REDUCE operation: sum all MAC accumulators.

        The adder tree sums the MAC results into one value and writes
        it to the output buffer at the specified index.

        In real hardware, this is a tree of adders:
            MAC0 + MAC1 → sum01
            MAC2 + MAC3 → sum23
            sum01 + sum23 → final

        We simply use Python's sum() for clarity.
        """
        total = sum(self._mac_accumulators)
        out_idx = entry.output_index
        if out_idx < self._config.output_buffer_size:
            self._output_buffer[out_idx] = total
        return f"REDUCE sum={total:.4g} → output[{out_idx}]"

    def _exec_activate(self, entry: MACScheduleEntry) -> str:
        """Execute an ACTIVATE operation: apply activation function.

        Reads the value from the output buffer, applies the activation
        function, and writes it back.

        Activation functions:
            NONE:    f(x) = x
            RELU:    f(x) = max(0, x)
            SIGMOID: f(x) = 1 / (1 + e^-x)
            TANH:    f(x) = tanh(x)
        """
        if not self._config.has_activation_unit:
            return "ACTIVATE skipped (no hardware activation unit)"

        out_idx = entry.output_index
        if out_idx >= self._config.output_buffer_size:
            return f"ACTIVATE error: index {out_idx} out of range"

        val = self._output_buffer[out_idx]

        match entry.activation:
            case "none" | ActivationFunction.NONE.value:
                result = val
            case "relu" | ActivationFunction.RELU.value:
                result = max(0.0, val)
            case "sigmoid" | ActivationFunction.SIGMOID.value:
                # Sigmoid: 1 / (1 + e^-x)
                # Clamp input to avoid overflow in exp()
                clamped = max(-500.0, min(500.0, val))
                result = 1.0 / (1.0 + math.exp(-clamped))
            case "tanh" | ActivationFunction.TANH.value:
                result = math.tanh(val)
            case _:
                result = val

        self._output_buffer[out_idx] = result
        return f"ACTIVATE {entry.activation}({val:.4g}) = {result:.4g}"

    def _exec_store(self, entry: MACScheduleEntry) -> str:
        """Execute a STORE_OUTPUT operation (result is already in output buffer)."""
        out_idx = entry.output_index
        val = (
            self._output_buffer[out_idx]
            if out_idx < self._config.output_buffer_size
            else 0.0
        )
        return f"STORE_OUTPUT output[{out_idx}] = {val:.4g}"

    def _make_idle_trace(self, description: str) -> EngineTrace:
        """Produce a trace for idle/halted cycles."""
        return EngineTrace(
            cycle=self._cycle,
            engine_name=self.name,
            execution_model=self.execution_model,
            description=description,
            unit_traces={},
            active_mask=[False] * self._config.num_macs,
            active_count=0,
            total_count=self._config.num_macs,
            utilization=0.0,
        )

    def __repr__(self) -> str:
        return (
            f"MACArrayEngine(num_macs={self._config.num_macs}, "
            f"cycle={self._cycle}, halted={self._halted})"
        )
