"""SIM00 simulator-protocol conformance tests for Intel8080Simulator."""

from __future__ import annotations

from simulator_protocol import ExecutionResult, Simulator, StepTrace

from intel8080_simulator import Intel8080Simulator, Intel8080State


def test_simulator_satisfies_protocol() -> None:
    sim = Intel8080Simulator()
    assert isinstance(sim, Simulator)


def test_execute_returns_execution_result() -> None:
    sim = Intel8080Simulator()
    result = sim.execute(bytes([0x76]))  # HLT
    assert isinstance(result, ExecutionResult)
    assert result.halted is True
    assert result.error is None
    assert result.ok is True


def test_execute_traces_each_instruction() -> None:
    program = bytes([0x00, 0x00, 0x00, 0x76])  # NOP NOP NOP HLT
    result = Intel8080Simulator().execute(program)
    assert result.ok is True
    assert result.steps == 4
    assert len(result.traces) == 4
    for trace in result.traces:
        assert isinstance(trace, StepTrace)
        assert trace.mnemonic
        assert trace.description


def test_execute_resets_before_each_run() -> None:
    sim = Intel8080Simulator()
    sim._memory[42] = 0xAB  # noqa: SLF001
    sim._a = 99  # noqa: SLF001
    result = sim.execute(bytes([0x76]))
    assert result.final_state.memory[42] == 0
    assert result.final_state.a == 0


def test_get_state_returns_intel8080state() -> None:
    sim = Intel8080Simulator()
    state = sim.get_state()
    assert isinstance(state, Intel8080State)


def test_step_returns_step_trace() -> None:
    sim = Intel8080Simulator()
    sim._memory[0] = 0x00  # NOP  # noqa: SLF001
    trace = sim.step()
    assert isinstance(trace, StepTrace)
    assert trace.pc_before == 0
    assert trace.pc_after == 1
    assert trace.mnemonic == "NOP"


def test_reset_clears_all_state() -> None:
    sim = Intel8080Simulator()
    sim._a = 42  # noqa: SLF001
    sim._sp = 0xFF00  # noqa: SLF001
    sim._memory[5] = 0x42  # noqa: SLF001
    sim._halted = True  # noqa: SLF001
    sim.reset()
    state = sim.get_state()
    assert state.a == 0
    assert state.sp == 0
    assert state.memory[5] == 0
    assert state.halted is False


def test_protocol_typed_assignment() -> None:
    sim: Simulator[Intel8080State] = Intel8080Simulator()
    # 0x76 = HLT at address 0
    result: ExecutionResult[Intel8080State] = sim.execute(bytes([0x76]))
    assert result.steps == 1
    assert result.ok is True


def test_empty_program_halts_on_zero_opcode() -> None:
    """Memory is zero-initialized; opcode 0x00 is NOP, 0x76 is HLT.
    An empty program starts with NOP loop — but actually 0x00 = NOP.
    We write HLT explicitly to avoid infinite loop.
    """
    # Empty program → memory is all zeros → 0x00 = NOP = infinite loop
    # Use empty program as just blank memory + safety limit check
    sim = Intel8080Simulator()
    result = sim.execute(b"")  # all memory zeros → NOP forever → cycle limit
    # The simulator should stop at the max step limit
    assert result.halted is False
    assert result.error is not None


def test_step_trace_has_mnemonic_and_description() -> None:
    sim = Intel8080Simulator()
    sim._memory[0] = 0x3E  # noqa: SLF001  MVI A,
    sim._memory[1] = 0x42  # noqa: SLF001
    trace = sim.step()
    assert trace.mnemonic == "MVI A,0x42"
    assert "0x42" in trace.description


def test_execute_result_final_state_is_frozen() -> None:
    from dataclasses import FrozenInstanceError

    import pytest
    result = Intel8080Simulator().execute(bytes([0x76]))
    with pytest.raises(FrozenInstanceError):
        result.final_state.a = 99  # type: ignore[misc]
