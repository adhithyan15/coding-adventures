"""Tests for the WASM stack-based virtual machine simulator."""

import pytest

from wasm_simulator.simulator import (
    WasmDecoder,
    WasmExecutor,
    WasmSimulator,
    assemble_wasm,
    encode_end,
    encode_i32_add,
    encode_i32_const,
    encode_i32_sub,
    encode_local_get,
    encode_local_set,
)


class TestEncoding:
    """Verify instruction encoding produces correct byte sequences."""

    def test_encode_i32_const_1(self) -> None:
        """i32.const 1 should be [0x41, 0x01, 0x00, 0x00, 0x00]."""
        assert encode_i32_const(1) == b"\x41\x01\x00\x00\x00"

    def test_encode_i32_const_negative(self) -> None:
        """i32.const -1 should encode as signed little-endian (0xFFFFFFFF)."""
        assert encode_i32_const(-1) == b"\x41\xff\xff\xff\xff"

    def test_encode_i32_const_large(self) -> None:
        """i32.const 256 should encode correctly in little-endian."""
        assert encode_i32_const(256) == b"\x41\x00\x01\x00\x00"

    def test_encode_i32_add(self) -> None:
        """i32.add is a single byte: 0x6A."""
        assert encode_i32_add() == b"\x6a"

    def test_encode_i32_sub(self) -> None:
        """i32.sub is a single byte: 0x6B."""
        assert encode_i32_sub() == b"\x6b"

    def test_encode_local_get(self) -> None:
        """local.get 0 should be [0x20, 0x00]."""
        assert encode_local_get(0) == b"\x20\x00"

    def test_encode_local_set(self) -> None:
        """local.set 2 should be [0x21, 0x02]."""
        assert encode_local_set(2) == b"\x21\x02"

    def test_encode_end(self) -> None:
        """end should be a single byte: 0x0B."""
        assert encode_end() == b"\x0b"


class TestDecoder:
    """Verify the decoder correctly reads variable-width instructions."""

    def test_decode_i32_const(self) -> None:
        decoder = WasmDecoder()
        bytecode = encode_i32_const(42)
        result = decoder.decode(bytecode, pc=0)
        assert result.mnemonic == "i32.const"
        assert result.operand == 42
        assert result.size == 5

    def test_decode_i32_const_negative(self) -> None:
        decoder = WasmDecoder()
        bytecode = encode_i32_const(-5)
        result = decoder.decode(bytecode, pc=0)
        assert result.mnemonic == "i32.const"
        assert result.operand == -5
        assert result.size == 5

    def test_decode_i32_add(self) -> None:
        decoder = WasmDecoder()
        bytecode = encode_i32_add()
        result = decoder.decode(bytecode, pc=0)
        assert result.mnemonic == "i32.add"
        assert result.operand is None
        assert result.size == 1

    def test_decode_i32_sub(self) -> None:
        decoder = WasmDecoder()
        bytecode = encode_i32_sub()
        result = decoder.decode(bytecode, pc=0)
        assert result.mnemonic == "i32.sub"
        assert result.operand is None
        assert result.size == 1

    def test_decode_local_get(self) -> None:
        decoder = WasmDecoder()
        bytecode = encode_local_get(3)
        result = decoder.decode(bytecode, pc=0)
        assert result.mnemonic == "local.get"
        assert result.operand == 3
        assert result.size == 2

    def test_decode_local_set(self) -> None:
        decoder = WasmDecoder()
        bytecode = encode_local_set(1)
        result = decoder.decode(bytecode, pc=0)
        assert result.mnemonic == "local.set"
        assert result.operand == 1
        assert result.size == 2

    def test_decode_end(self) -> None:
        decoder = WasmDecoder()
        bytecode = encode_end()
        result = decoder.decode(bytecode, pc=0)
        assert result.mnemonic == "end"
        assert result.operand is None
        assert result.size == 1

    def test_decode_at_offset(self) -> None:
        """Decoder should handle non-zero PC offsets correctly."""
        decoder = WasmDecoder()
        # Put i32.add (0x6A) at offset 5, preceded by 5 bytes of i32.const
        bytecode = encode_i32_const(99) + encode_i32_add()
        result = decoder.decode(bytecode, pc=5)
        assert result.mnemonic == "i32.add"

    def test_decode_unknown_opcode(self) -> None:
        """Unknown opcodes should raise ValueError."""
        decoder = WasmDecoder()
        with pytest.raises(ValueError, match="Unknown WASM opcode"):
            decoder.decode(b"\xFF", pc=0)


class TestExecutor:
    """Verify executor operations on the stack and locals."""

    def test_i32_const_pushes(self) -> None:
        """i32.const should push its operand onto the stack."""
        sim = WasmSimulator(num_locals=4)
        program = assemble_wasm([encode_i32_const(7), encode_end()])
        sim.load(program)
        trace = sim.step()
        assert trace.stack_before == []
        assert trace.stack_after == [7]
        assert sim.stack == [7]

    def test_i32_add_pops_two_pushes_sum(self) -> None:
        """i32.add should pop two values and push their sum."""
        sim = WasmSimulator(num_locals=4)
        program = assemble_wasm([
            encode_i32_const(10),
            encode_i32_const(20),
            encode_i32_add(),
            encode_end(),
        ])
        sim.load(program)
        sim.step()  # push 10
        sim.step()  # push 20
        trace = sim.step()  # add
        assert trace.stack_before == [10, 20]
        assert trace.stack_after == [30]
        assert sim.stack == [30]

    def test_i32_sub_pops_two_pushes_difference(self) -> None:
        """i32.sub should compute second-to-top minus top."""
        sim = WasmSimulator(num_locals=4)
        program = assemble_wasm([
            encode_i32_const(10),
            encode_i32_const(3),
            encode_i32_sub(),
            encode_end(),
        ])
        sim.load(program)
        sim.step()  # push 10
        sim.step()  # push 3
        trace = sim.step()  # sub: 10 - 3 = 7
        assert trace.stack_before == [10, 3]
        assert trace.stack_after == [7]

    def test_local_set_get_roundtrip(self) -> None:
        """local.set followed by local.get should round-trip a value."""
        sim = WasmSimulator(num_locals=4)
        program = assemble_wasm([
            encode_i32_const(42),   # push 42
            encode_local_set(1),    # pop 42, store in locals[1]
            encode_local_get(1),    # push locals[1] = 42
            encode_end(),
        ])
        sim.load(program)
        sim.step()  # push 42
        sim.step()  # local.set 1
        assert sim.locals[1] == 42
        assert sim.stack == []
        trace = sim.step()  # local.get 1
        assert trace.stack_after == [42]
        assert sim.stack == [42]


class TestWasmSimulator:
    """End-to-end tests running actual WASM programs."""

    def test_x_equals_1_plus_2(self) -> None:
        """The target program: x = 1 + 2 → locals[0] should be 3.

        Program:
            i32.const 1    push 1
            i32.const 2    push 2
            i32.add        pop 2 and 1, push 3
            local.set 0    pop 3, store in local 0
            end            halt
        """
        sim = WasmSimulator(num_locals=4)
        program = assemble_wasm([
            encode_i32_const(1),
            encode_i32_const(2),
            encode_i32_add(),
            encode_local_set(0),
            encode_end(),
        ])
        traces = sim.run(program)

        assert len(traces) == 5
        assert sim.locals[0] == 3
        assert sim.halted is True
        assert sim.stack == []  # Stack should be empty after local.set

    def test_stack_state_at_each_step(self) -> None:
        """Verify the stack evolves correctly through x = 1 + 2.

        Step 0: i32.const 1   →  [] → [1]
        Step 1: i32.const 2   →  [1] → [1, 2]
        Step 2: i32.add       →  [1, 2] → [3]
        Step 3: local.set 0   →  [3] → []
        Step 4: end            →  [] → []  (halt)
        """
        sim = WasmSimulator(num_locals=4)
        program = assemble_wasm([
            encode_i32_const(1),
            encode_i32_const(2),
            encode_i32_add(),
            encode_local_set(0),
            encode_end(),
        ])
        traces = sim.run(program)

        # Step 0: push 1
        assert traces[0].stack_before == []
        assert traces[0].stack_after == [1]
        assert traces[0].instruction.mnemonic == "i32.const"

        # Step 1: push 2
        assert traces[1].stack_before == [1]
        assert traces[1].stack_after == [1, 2]
        assert traces[1].instruction.mnemonic == "i32.const"

        # Step 2: add → pop 2 and 1, push 3
        assert traces[2].stack_before == [1, 2]
        assert traces[2].stack_after == [3]
        assert traces[2].instruction.mnemonic == "i32.add"

        # Step 3: local.set 0 → pop 3
        assert traces[3].stack_before == [3]
        assert traces[3].stack_after == []
        assert traces[3].instruction.mnemonic == "local.set"
        assert traces[3].locals_snapshot[0] == 3

        # Step 4: end → halt
        assert traces[4].instruction.mnemonic == "end"
        assert traces[4].halted is True

    def test_halt_raises_on_extra_step(self) -> None:
        """Stepping after halt should raise RuntimeError."""
        sim = WasmSimulator(num_locals=4)
        program = assemble_wasm([encode_end()])
        sim.run(program)
        with pytest.raises(RuntimeError, match="halted"):
            sim.step()

    def test_subtraction_program(self) -> None:
        """x = 10 - 3 → locals[0] should be 7."""
        sim = WasmSimulator(num_locals=4)
        program = assemble_wasm([
            encode_i32_const(10),
            encode_i32_const(3),
            encode_i32_sub(),
            encode_local_set(0),
            encode_end(),
        ])
        sim.run(program)
        assert sim.locals[0] == 7

    def test_multiple_locals(self) -> None:
        """Store different values in different locals, then retrieve them."""
        sim = WasmSimulator(num_locals=4)
        program = assemble_wasm([
            encode_i32_const(10),   # push 10
            encode_local_set(0),    # locals[0] = 10
            encode_i32_const(20),   # push 20
            encode_local_set(1),    # locals[1] = 20
            encode_local_get(0),    # push 10
            encode_local_get(1),    # push 20
            encode_i32_add(),       # push 30
            encode_local_set(2),    # locals[2] = 30
            encode_end(),
        ])
        sim.run(program)
        assert sim.locals[0] == 10
        assert sim.locals[1] == 20
        assert sim.locals[2] == 30

    def test_pc_advances_correctly(self) -> None:
        """PC should advance by each instruction's byte width.

        i32.const = 5 bytes, i32.add = 1 byte, local.set = 2 bytes, end = 1 byte
        PCs: 0, 5, 10, 11, 13
        """
        sim = WasmSimulator(num_locals=4)
        program = assemble_wasm([
            encode_i32_const(1),    # 5 bytes, PC 0→5
            encode_i32_const(2),    # 5 bytes, PC 5→10
            encode_i32_add(),       # 1 byte,  PC 10→11
            encode_local_set(0),    # 2 bytes, PC 11→13
            encode_end(),           # 1 byte,  PC 13→14
        ])
        traces = sim.run(program)

        assert traces[0].pc == 0
        assert traces[1].pc == 5
        assert traces[2].pc == 10
        assert traces[3].pc == 11
        assert traces[4].pc == 13

    def test_add_large_numbers(self) -> None:
        """100 + 200 = 300."""
        sim = WasmSimulator(num_locals=4)
        program = assemble_wasm([
            encode_i32_const(100),
            encode_i32_const(200),
            encode_i32_add(),
            encode_local_set(0),
            encode_end(),
        ])
        sim.run(program)
        assert sim.locals[0] == 300


# ===========================================================================
# simulator-protocol conformance tests
# ===========================================================================
# These tests verify that WasmSimulator satisfies the Simulator[WasmState]
# protocol: get_state(), execute(), and reset() behave correctly and
# the returned types match the protocol contract.


class TestSimulatorProtocolConformance:
    """Verify Simulator[WasmState] protocol conformance for WasmSimulator."""

    def test_get_state_returns_wasm_state(self) -> None:
        """get_state() returns a WasmState frozen dataclass with correct field types."""
        from wasm_simulator.state import WasmState

        sim = WasmSimulator(num_locals=4)
        state = sim.get_state()

        assert isinstance(state, WasmState)
        assert isinstance(state.stack, tuple)
        assert isinstance(state.locals, tuple)
        assert isinstance(state.pc, int)
        assert isinstance(state.halted, bool)
        assert isinstance(state.cycle, int)

    def test_get_state_is_immutable_snapshot(self) -> None:
        """get_state() snapshots are independent — mutating sim does not affect them."""
        sim = WasmSimulator(num_locals=4)
        program = assemble_wasm([encode_i32_const(1), encode_end()])
        sim.load(program)
        state_before = sim.get_state()
        sim.step()  # push 1 onto the stack
        state_after = sim.get_state()

        # Snapshot taken before step() must NOT reflect the new stack
        assert state_before.stack == ()
        assert state_after.stack == (1,)

    def test_execute_simple_program_ok(self) -> None:
        """execute() runs x = 1 + 2 and returns ok=True with correct final state."""
        from simulator_protocol import ExecutionResult

        sim = WasmSimulator(num_locals=4)
        program = assemble_wasm([
            encode_i32_const(1),
            encode_i32_const(2),
            encode_i32_add(),
            encode_local_set(0),
            encode_end(),
        ])
        result = sim.execute(program)

        assert isinstance(result, ExecutionResult)
        assert result.ok
        assert result.halted
        assert result.error is None
        assert result.final_state.locals[0] == 3
        assert result.steps == 5

    def test_execute_cycle_counter_in_final_state(self) -> None:
        """execute() captures the cycle counter in final_state.cycle."""
        sim = WasmSimulator(num_locals=4)
        program = assemble_wasm([
            encode_i32_const(7),
            encode_local_set(0),
            encode_end(),
        ])
        result = sim.execute(program)

        assert result.ok
        # 3 instructions executed: i32.const, local.set, end
        assert result.final_state.cycle == 3

    def test_execute_traces_contain_step_traces(self) -> None:
        """execute() populates result.traces with one StepTrace per instruction."""
        from simulator_protocol import StepTrace

        sim = WasmSimulator(num_locals=4)
        program = assemble_wasm([
            encode_i32_const(3),
            encode_local_set(0),
            encode_end(),
        ])
        result = sim.execute(program)

        assert len(result.traces) == 3
        for trace in result.traces:
            assert isinstance(trace, StepTrace)
            assert isinstance(trace.mnemonic, str)
            assert len(trace.mnemonic) > 0

    def test_reset_clears_state(self) -> None:
        """reset() restores the simulator to its initial power-on state."""
        sim = WasmSimulator(num_locals=4)
        program = assemble_wasm([
            encode_i32_const(5),
            encode_local_set(0),
            encode_end(),
        ])
        sim.execute(program)

        # After execution the simulator is halted with locals[0] = 5
        assert sim.halted
        assert sim.locals[0] == 5
        assert sim.cycle > 0

        sim.reset()

        assert not sim.halted
        assert sim.stack == []
        assert sim.locals[0] == 0
        assert sim.pc == 0
        assert sim.cycle == 0
