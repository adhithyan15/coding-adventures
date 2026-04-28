"""Wrapper-specific tests for ``BrainfuckVM``.

These tests cover the wrapper's contract — bounds checks, fuel cap,
``jit=True`` deferral — that the execution-parity tests in
:mod:`test_execute` do not exercise.
"""

from __future__ import annotations

import pytest
from interpreter_ir import IIRModule

from brainfuck_iir_compiler import BrainfuckError, BrainfuckVM

# ---------------------------------------------------------------------------
# Construction guards
# ---------------------------------------------------------------------------


def test_jit_true_raises_not_implemented_until_bf05() -> None:
    with pytest.raises(NotImplementedError) as excinfo:
        BrainfuckVM(jit=True)
    assert "BF05" in str(excinfo.value)


def test_invalid_tape_size_rejected() -> None:
    with pytest.raises(ValueError):
        BrainfuckVM(tape_size=0)
    with pytest.raises(ValueError):
        BrainfuckVM(tape_size=-1)


def test_invalid_max_steps_rejected() -> None:
    with pytest.raises(ValueError):
        BrainfuckVM(max_steps=0)
    with pytest.raises(ValueError):
        BrainfuckVM(max_steps=-5)


# ---------------------------------------------------------------------------
# compile() vs run()
# ---------------------------------------------------------------------------


def test_compile_returns_iir_module_without_executing() -> None:
    vm = BrainfuckVM()
    module = vm.compile("++.")
    assert isinstance(module, IIRModule)
    # Compile alone should not produce output — metrics is still None.
    assert vm.metrics is None


def test_execute_module_runs_a_pre_compiled_module() -> None:
    vm = BrainfuckVM()
    module = vm.compile("+.")
    out = vm.execute_module(module)
    assert out == b"\x01"


# ---------------------------------------------------------------------------
# Tape bounds
# ---------------------------------------------------------------------------


def test_pointer_walking_off_right_edge_raises() -> None:
    vm = BrainfuckVM(tape_size=4)
    # Move the pointer right 5 times, then store — the store at position
    # 4 (out of [0, 4)) must raise.
    with pytest.raises(BrainfuckError) as excinfo:
        vm.run(">>>>+")
    assert "out of bounds" in str(excinfo.value)


def test_pointer_walking_off_left_edge_raises_on_store() -> None:
    vm = BrainfuckVM(tape_size=8)
    # Move left from cell 0; the read returns 0 (lazy infinite tape) but
    # the subsequent store to addr=-1 must raise.
    with pytest.raises(BrainfuckError):
        vm.run("<+")


def test_out_of_bounds_read_returns_zero() -> None:
    # `>>>>` (move four cells right with default tape size) and then
    # output: the cell at position 4 was never written, so reads return 0.
    vm = BrainfuckVM()
    assert vm.run(">>>>.") == b"\x00"


# ---------------------------------------------------------------------------
# Fuel cap
# ---------------------------------------------------------------------------


def test_max_steps_terminates_runaway_loop() -> None:
    # `+[]` is the canonical infinite loop (cell stays nonzero forever).
    # max_steps caps the number of label crossings.
    vm = BrainfuckVM(max_steps=50)
    with pytest.raises(BrainfuckError) as excinfo:
        vm.run("+[]")
    assert "max_steps" in str(excinfo.value)


def test_finite_program_runs_to_completion_under_fuel_cap() -> None:
    # A bounded loop should finish well under any reasonable cap.
    vm = BrainfuckVM(max_steps=10_000)
    assert vm.run("+++++[-].") == b"\x00"


# ---------------------------------------------------------------------------
# Metrics + underlying VM access
# ---------------------------------------------------------------------------


def test_metrics_populated_after_run() -> None:
    vm = BrainfuckVM()
    vm.run("+.")
    metrics = vm.metrics
    assert metrics is not None
    assert metrics.total_instructions_executed > 0
    # No JIT yet, so total_jit_hits must be zero.
    assert metrics.total_jit_hits == 0


def test_underlying_vm_exposed_after_run() -> None:
    vm = BrainfuckVM()
    vm.run("+")
    assert vm.vm is not None
    # A simple sanity check — the VM must have written cell 0.
    assert vm.vm.memory.get(0) == 1


def test_tape_size_property_reflects_constructor() -> None:
    vm = BrainfuckVM(tape_size=64)
    assert vm.tape_size == 64
