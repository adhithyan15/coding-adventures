"""Tests for mac_array_engine.py — scheduled MAC array execution (NPU style)."""

from __future__ import annotations

import math

import pytest
from clock import Clock, ClockEdge

from parallel_execution_engine import (
    ActivationFunction,
    ExecutionModel,
    MACArrayConfig,
    MACArrayEngine,
    MACOperation,
    MACScheduleEntry,
)

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def make_edge(cycle: int = 1) -> ClockEdge:
    return ClockEdge(cycle=cycle, value=1, is_rising=True, is_falling=False)


# ---------------------------------------------------------------------------
# MACOperation and ActivationFunction enums
# ---------------------------------------------------------------------------


class TestEnums:
    def test_mac_operations(self) -> None:
        assert MACOperation.LOAD_INPUT.value == "load_input"
        assert MACOperation.LOAD_WEIGHTS.value == "load_weights"
        assert MACOperation.MAC.value == "mac"
        assert MACOperation.REDUCE.value == "reduce"
        assert MACOperation.ACTIVATE.value == "activate"
        assert MACOperation.STORE_OUTPUT.value == "store_output"

    def test_activation_functions(self) -> None:
        assert ActivationFunction.NONE.value == "none"
        assert ActivationFunction.RELU.value == "relu"
        assert ActivationFunction.SIGMOID.value == "sigmoid"
        assert ActivationFunction.TANH.value == "tanh"


# ---------------------------------------------------------------------------
# MACScheduleEntry
# ---------------------------------------------------------------------------


class TestMACScheduleEntry:
    def test_creation(self) -> None:
        entry = MACScheduleEntry(
            cycle=1,
            operation=MACOperation.MAC,
            input_indices=[0, 1],
            weight_indices=[0, 1],
            output_index=0,
        )
        assert entry.cycle == 1
        assert entry.operation == MACOperation.MAC
        assert entry.input_indices == [0, 1]
        assert entry.output_index == 0

    def test_defaults(self) -> None:
        entry = MACScheduleEntry(cycle=0, operation=MACOperation.MAC)
        assert entry.input_indices == []
        assert entry.weight_indices == []
        assert entry.output_index == 0
        assert entry.activation == "none"

    def test_frozen(self) -> None:
        entry = MACScheduleEntry(cycle=0, operation=MACOperation.MAC)
        with pytest.raises(AttributeError):
            entry.cycle = 5  # type: ignore[misc]


# ---------------------------------------------------------------------------
# MACArrayConfig
# ---------------------------------------------------------------------------


class TestMACArrayConfig:
    def test_defaults(self) -> None:
        config = MACArrayConfig()
        assert config.num_macs == 8
        assert config.input_buffer_size == 1024
        assert config.weight_buffer_size == 4096
        assert config.output_buffer_size == 1024
        assert config.has_activation_unit is True


# ---------------------------------------------------------------------------
# MACArrayEngine — basic properties
# ---------------------------------------------------------------------------


class TestMACArrayEngineProperties:
    def test_name(self) -> None:
        engine = MACArrayEngine(MACArrayConfig(num_macs=4), Clock())
        assert engine.name == "MACArrayEngine"

    def test_width(self) -> None:
        engine = MACArrayEngine(MACArrayConfig(num_macs=8), Clock())
        assert engine.width == 8

    def test_execution_model(self) -> None:
        engine = MACArrayEngine(MACArrayConfig(num_macs=4), Clock())
        assert engine.execution_model == ExecutionModel.SCHEDULED_MAC

    def test_initial_halted(self) -> None:
        engine = MACArrayEngine(MACArrayConfig(num_macs=4), Clock())
        assert engine.halted is False

    def test_config_access(self) -> None:
        config = MACArrayConfig(num_macs=4)
        engine = MACArrayEngine(config, Clock())
        assert engine.config is config

    def test_repr(self) -> None:
        engine = MACArrayEngine(MACArrayConfig(num_macs=4), Clock())
        r = repr(engine)
        assert "MACArrayEngine" in r
        assert "num_macs=4" in r


# ---------------------------------------------------------------------------
# MACArrayEngine — data loading
# ---------------------------------------------------------------------------


class TestMACArrayEngineLoading:
    def test_load_inputs(self) -> None:
        engine = MACArrayEngine(MACArrayConfig(num_macs=4), Clock())
        engine.load_inputs([1.0, 2.0, 3.0, 4.0])
        outputs = engine.read_outputs()
        # Outputs should still be all zeros (no computation yet)
        assert outputs[0] == 0.0

    def test_load_weights(self) -> None:
        engine = MACArrayEngine(MACArrayConfig(num_macs=4), Clock())
        engine.load_weights([0.5, 0.5, 0.5, 0.5])
        # No error = success


# ---------------------------------------------------------------------------
# MACArrayEngine — execution
# ---------------------------------------------------------------------------


class TestMACArrayEngineExecution:
    def test_dot_product(self) -> None:
        """Compute a dot product: sum(input[i] * weight[i])."""
        engine = MACArrayEngine(MACArrayConfig(num_macs=4), Clock())
        engine.load_inputs([1.0, 2.0, 3.0, 4.0])
        engine.load_weights([1.0, 1.0, 1.0, 1.0])

        schedule = [
            MACScheduleEntry(
                cycle=1,
                operation=MACOperation.MAC,
                input_indices=[0, 1, 2, 3],
                weight_indices=[0, 1, 2, 3],
                output_index=0,
            ),
            MACScheduleEntry(
                cycle=2,
                operation=MACOperation.REDUCE,
                output_index=0,
            ),
            MACScheduleEntry(
                cycle=3,
                operation=MACOperation.STORE_OUTPUT,
                output_index=0,
            ),
        ]
        engine.load_schedule(schedule)
        engine.run()

        outputs = engine.read_outputs()
        # 1*1 + 2*1 + 3*1 + 4*1 = 10.0
        assert abs(outputs[0] - 10.0) < 0.01

    def test_weighted_sum(self) -> None:
        """Compute a weighted sum with different weights."""
        engine = MACArrayEngine(MACArrayConfig(num_macs=4), Clock())
        engine.load_inputs([2.0, 3.0, 4.0, 5.0])
        engine.load_weights([0.5, 0.25, 0.125, 0.0625])

        schedule = [
            MACScheduleEntry(
                cycle=1,
                operation=MACOperation.MAC,
                input_indices=[0, 1, 2, 3],
                weight_indices=[0, 1, 2, 3],
                output_index=0,
            ),
            MACScheduleEntry(
                cycle=2,
                operation=MACOperation.REDUCE,
                output_index=0,
            ),
            MACScheduleEntry(
                cycle=3,
                operation=MACOperation.STORE_OUTPUT,
                output_index=0,
            ),
        ]
        engine.load_schedule(schedule)
        engine.run()

        outputs = engine.read_outputs()
        expected = 2.0 * 0.5 + 3.0 * 0.25 + 4.0 * 0.125 + 5.0 * 0.0625
        assert abs(outputs[0] - expected) < 0.01

    def test_relu_activation(self) -> None:
        """Test ReLU activation: max(0, x)."""
        engine = MACArrayEngine(MACArrayConfig(num_macs=2), Clock())
        engine.load_inputs([3.0, -5.0])
        engine.load_weights([1.0, 1.0])

        schedule = [
            MACScheduleEntry(
                cycle=1,
                operation=MACOperation.MAC,
                input_indices=[0, 1],
                weight_indices=[0, 1],
                output_index=0,
            ),
            MACScheduleEntry(
                cycle=2,
                operation=MACOperation.REDUCE,
                output_index=0,
            ),
            MACScheduleEntry(
                cycle=3,
                operation=MACOperation.ACTIVATE,
                output_index=0,
                activation="relu",
            ),
            MACScheduleEntry(
                cycle=4,
                operation=MACOperation.STORE_OUTPUT,
                output_index=0,
            ),
        ]
        engine.load_schedule(schedule)
        engine.run()

        # 3*1 + (-5)*1 = -2 → ReLU(-2) = 0
        assert engine.read_outputs()[0] == 0.0

    def test_sigmoid_activation(self) -> None:
        """Test sigmoid activation: 1/(1+e^-x)."""
        engine = MACArrayEngine(MACArrayConfig(num_macs=1), Clock())
        engine.load_inputs([0.0])
        engine.load_weights([1.0])

        schedule = [
            MACScheduleEntry(
                cycle=1, operation=MACOperation.MAC,
                input_indices=[0], weight_indices=[0], output_index=0,
            ),
            MACScheduleEntry(
                cycle=2, operation=MACOperation.REDUCE, output_index=0,
            ),
            MACScheduleEntry(
                cycle=3, operation=MACOperation.ACTIVATE,
                output_index=0, activation="sigmoid",
            ),
        ]
        engine.load_schedule(schedule)
        engine.run()

        # sigmoid(0) = 0.5
        assert abs(engine.read_outputs()[0] - 0.5) < 0.01

    def test_tanh_activation(self) -> None:
        """Test tanh activation."""
        engine = MACArrayEngine(MACArrayConfig(num_macs=1), Clock())
        engine.load_inputs([1.0])
        engine.load_weights([1.0])

        schedule = [
            MACScheduleEntry(
                cycle=1, operation=MACOperation.MAC,
                input_indices=[0], weight_indices=[0], output_index=0,
            ),
            MACScheduleEntry(
                cycle=2, operation=MACOperation.REDUCE, output_index=0,
            ),
            MACScheduleEntry(
                cycle=3, operation=MACOperation.ACTIVATE,
                output_index=0, activation="tanh",
            ),
        ]
        engine.load_schedule(schedule)
        engine.run()

        assert abs(engine.read_outputs()[0] - math.tanh(1.0)) < 0.01

    def test_no_activation_unit(self) -> None:
        """When has_activation_unit=False, ACTIVATE is skipped."""
        engine = MACArrayEngine(
            MACArrayConfig(num_macs=1, has_activation_unit=False), Clock()
        )
        engine.load_inputs([5.0])
        engine.load_weights([1.0])

        schedule = [
            MACScheduleEntry(
                cycle=1, operation=MACOperation.MAC,
                input_indices=[0], weight_indices=[0], output_index=0,
            ),
            MACScheduleEntry(
                cycle=2, operation=MACOperation.REDUCE, output_index=0,
            ),
            MACScheduleEntry(
                cycle=3, operation=MACOperation.ACTIVATE,
                output_index=0, activation="relu",
            ),
        ]
        engine.load_schedule(schedule)
        engine.run()

        # Activation skipped, value should remain 5.0
        assert abs(engine.read_outputs()[0] - 5.0) < 0.01

    def test_load_input_operation(self) -> None:
        """LOAD_INPUT operation in schedule."""
        engine = MACArrayEngine(MACArrayConfig(num_macs=4), Clock())
        engine.load_inputs([1.0, 2.0])

        schedule = [
            MACScheduleEntry(
                cycle=1, operation=MACOperation.LOAD_INPUT,
                input_indices=[0, 1],
            ),
        ]
        engine.load_schedule(schedule)
        traces = engine.run()
        assert len(traces) >= 1
        assert "LOAD_INPUT" in traces[0].description

    def test_load_weights_operation(self) -> None:
        """LOAD_WEIGHTS operation in schedule."""
        engine = MACArrayEngine(MACArrayConfig(num_macs=4), Clock())
        engine.load_weights([1.0, 2.0])

        schedule = [
            MACScheduleEntry(
                cycle=1, operation=MACOperation.LOAD_WEIGHTS,
                weight_indices=[0, 1],
            ),
        ]
        engine.load_schedule(schedule)
        traces = engine.run()
        assert "LOAD_WEIGHTS" in traces[0].description


# ---------------------------------------------------------------------------
# MACArrayEngine — halting and idle cycles
# ---------------------------------------------------------------------------


class TestMACArrayEngineHalting:
    def test_halts_after_schedule(self) -> None:
        engine = MACArrayEngine(MACArrayConfig(num_macs=4), Clock())
        schedule = [
            MACScheduleEntry(cycle=1, operation=MACOperation.MAC,
                             input_indices=[0], weight_indices=[0]),
        ]
        engine.load_schedule(schedule)
        engine.run()
        assert engine.halted is True

    def test_idle_cycle(self) -> None:
        """Cycles with no schedule entry produce idle traces."""
        engine = MACArrayEngine(MACArrayConfig(num_macs=4), Clock())
        schedule = [
            MACScheduleEntry(cycle=3, operation=MACOperation.MAC,
                             input_indices=[0], weight_indices=[0]),
        ]
        engine.load_schedule(schedule)

        # Cycles 1 and 2 should be idle
        trace1 = engine.step(make_edge(1))
        assert "No operation" in trace1.description
        assert trace1.active_count == 0

    def test_halted_step(self) -> None:
        """Stepping after completion returns halted trace."""
        engine = MACArrayEngine(MACArrayConfig(num_macs=4), Clock())
        schedule = [
            MACScheduleEntry(cycle=1, operation=MACOperation.MAC,
                             input_indices=[0], weight_indices=[0]),
        ]
        engine.load_schedule(schedule)
        engine.run()

        trace = engine.step(make_edge(99))
        assert "complete" in trace.description.lower()


# ---------------------------------------------------------------------------
# MACArrayEngine — reset
# ---------------------------------------------------------------------------


class TestMACArrayEngineReset:
    def test_reset(self) -> None:
        engine = MACArrayEngine(MACArrayConfig(num_macs=4), Clock())
        engine.load_inputs([1.0, 2.0])
        engine.load_weights([0.5, 0.5])
        engine.load_schedule([
            MACScheduleEntry(cycle=1, operation=MACOperation.MAC,
                             input_indices=[0, 1], weight_indices=[0, 1]),
            MACScheduleEntry(cycle=2, operation=MACOperation.REDUCE, output_index=0),
        ])
        engine.run()

        engine.reset()
        assert engine.halted is False
        assert engine.read_outputs()[0] == 0.0
