"""SIM00 simulator-protocol conformance tests.

Verify that ``IBM704Simulator`` structurally satisfies
``Simulator[IBM704State]`` and that ``execute`` returns a properly populated
``ExecutionResult[IBM704State]``.
"""

from __future__ import annotations

from simulator_protocol import ExecutionResult, Simulator, StepTrace

from ibm704_simulator import (
    OP_HTR,
    OP_NOP,
    IBM704Simulator,
    IBM704State,
    encode_type_b,
    pack_program,
)


def test_simulator_satisfies_protocol_at_runtime() -> None:
    """``Simulator`` is runtime-checkable; ``IBM704Simulator`` should match."""
    sim = IBM704Simulator()
    assert isinstance(sim, Simulator)


def test_execute_returns_execution_result() -> None:
    sim = IBM704Simulator()
    result = sim.execute(pack_program([encode_type_b(OP_HTR, 0, 5)]))
    assert isinstance(result, ExecutionResult)
    assert result.halted is True
    assert result.error is None
    assert result.ok is True


def test_execute_traces_each_instruction() -> None:
    sim = IBM704Simulator()
    program = pack_program(
        [
            encode_type_b(OP_NOP),
            encode_type_b(OP_NOP),
            encode_type_b(OP_NOP),
            encode_type_b(OP_HTR, 0, 3),
        ]
    )
    result = sim.execute(program)
    assert result.ok is True
    assert result.steps == 4
    assert len(result.traces) == 4
    for trace in result.traces:
        assert isinstance(trace, StepTrace)
        assert trace.mnemonic
        assert trace.description


def test_execute_resets_state_on_each_call() -> None:
    sim = IBM704Simulator()
    sim._memory[42] = 0xDEADBEEF  # noqa: SLF001
    sim._ac_magnitude = 999  # noqa: SLF001
    result = sim.execute(pack_program([encode_type_b(OP_HTR, 0, 0)]))
    assert result.ok is True
    # State was reset before run.
    assert result.final_state.memory[42] == 0
    assert result.final_state.accumulator_magnitude == 0


def test_get_state_returns_ibm704state() -> None:
    sim = IBM704Simulator()
    state = sim.get_state()
    assert isinstance(state, IBM704State)


def test_step_returns_step_trace() -> None:
    sim = IBM704Simulator()
    sim._memory[0] = encode_type_b(OP_NOP)  # noqa: SLF001
    trace = sim.step()
    assert isinstance(trace, StepTrace)
    assert trace.pc_before == 0
    assert trace.pc_after == 1
    assert trace.mnemonic == "NOP"


def test_reset_clears_all_state() -> None:
    sim = IBM704Simulator()
    sim._ac_magnitude = 42  # noqa: SLF001
    sim._index_a = 10  # noqa: SLF001
    sim._memory[5] = 0x123  # noqa: SLF001
    sim._halted = True  # noqa: SLF001
    sim.reset()
    state = sim.get_state()
    assert state.accumulator_magnitude == 0
    assert state.index_a == 0
    assert state.memory[5] == 0
    assert state.halted is False


def test_protocol_typed_assignment_compiles() -> None:
    """Type-check helper — should be assignable to Simulator[IBM704State]."""
    sim: Simulator[IBM704State] = IBM704Simulator()
    # Empty program: no bytes loaded, but memory is zero-filled and
    # opcode 0x000 = HTR. So executing zero-initialized memory produces
    # exactly one HTR step and halts cleanly.
    result: ExecutionResult[IBM704State] = sim.execute(b"")
    assert result.steps == 1
    assert result.ok is True
