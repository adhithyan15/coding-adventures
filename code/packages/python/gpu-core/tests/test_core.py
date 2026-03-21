"""Tests for the GPUCore — the main processing element simulator."""

from __future__ import annotations

import pytest
from fp_arithmetic import FP16, FP32

from gpu_core.core import GPUCore
from gpu_core.generic_isa import GenericISA
from gpu_core.opcodes import fadd, fmul, halt, limm, nop
from gpu_core.protocols import ProcessingElement


class TestConstruction:
    """Test core creation and configuration."""

    def test_default_construction(self) -> None:
        """Default core has GenericISA, FP32, 32 regs, 4KB memory."""
        core = GPUCore()
        assert core.isa.name == "Generic"
        assert core.fmt == FP32
        assert core.registers.num_registers == 32
        assert core.memory.size == 4096
        assert core.pc == 0
        assert not core.halted

    def test_custom_isa(self) -> None:
        """Can provide a custom ISA."""
        isa = GenericISA()
        core = GPUCore(isa=isa)
        assert core.isa is isa

    def test_custom_registers(self) -> None:
        """Can configure register count for different vendors."""
        core = GPUCore(num_registers=255)  # NVIDIA
        assert core.registers.num_registers == 255

    def test_custom_format(self) -> None:
        """Can configure floating-point format."""
        core = GPUCore(fmt=FP16)
        assert core.fmt == FP16

    def test_custom_memory(self) -> None:
        """Can configure memory size."""
        core = GPUCore(memory_size=1024)
        assert core.memory.size == 1024

    def test_implements_processing_element(self) -> None:
        """GPUCore satisfies the ProcessingElement protocol."""
        core = GPUCore()
        assert isinstance(core, ProcessingElement)

    def test_repr(self) -> None:
        """Repr shows ISA, register count, format, and status."""
        core = GPUCore()
        r = repr(core)
        assert "Generic" in r
        assert "running" in r


class TestLoadProgram:
    """Test program loading."""

    def test_load_program(self) -> None:
        """Loading a program resets PC and cycle count."""
        core = GPUCore()
        core.load_program([limm(0, 1.0), halt()])
        assert core.pc == 0
        assert core.cycle == 0
        assert not core.halted

    def test_load_replaces_program(self) -> None:
        """Loading a new program replaces the old one."""
        core = GPUCore()
        core.load_program([limm(0, 1.0), halt()])
        core.run()
        assert core.halted
        core.load_program([limm(0, 2.0), halt()])
        assert not core.halted
        assert core.pc == 0


class TestStep:
    """Test single-step execution."""

    def test_step_limm(self) -> None:
        """Step through a LIMM instruction."""
        core = GPUCore()
        core.load_program([limm(0, 42.0), halt()])
        trace = core.step()
        assert trace.pc == 0
        assert trace.cycle == 1
        assert core.registers.read_float(0) == 42.0
        assert core.pc == 1

    def test_step_fadd(self) -> None:
        """Step through an FADD instruction."""
        core = GPUCore()
        core.load_program([limm(0, 1.0), limm(1, 2.0), fadd(2, 0, 1), halt()])
        core.step()  # limm R0, 1.0
        core.step()  # limm R1, 2.0
        trace = core.step()  # fadd R2, R0, R1
        assert core.registers.read_float(2) == 3.0
        assert "3.0" in trace.description

    def test_step_halt(self) -> None:
        """Stepping into HALT sets halted flag."""
        core = GPUCore()
        core.load_program([halt()])
        trace = core.step()
        assert trace.halted is True
        assert core.halted is True

    def test_step_when_halted_raises(self) -> None:
        """Stepping a halted core raises RuntimeError."""
        core = GPUCore()
        core.load_program([halt()])
        core.step()
        with pytest.raises(RuntimeError, match="halted"):
            core.step()

    def test_step_out_of_bounds_raises(self) -> None:
        """Stepping past program end raises RuntimeError."""
        core = GPUCore()
        core.load_program([nop()])
        core.step()  # PC now 1, program has only 1 instruction
        with pytest.raises(RuntimeError, match="PC=1 out of program range"):
            core.step()

    def test_step_increments_cycle(self) -> None:
        """Each step increments the cycle counter."""
        core = GPUCore()
        core.load_program([nop(), nop(), halt()])
        core.step()
        assert core.cycle == 1
        core.step()
        assert core.cycle == 2


class TestRun:
    """Test full program execution."""

    def test_simple_program(self) -> None:
        """Run a simple 3-instruction program."""
        core = GPUCore()
        core.load_program([limm(0, 3.0), limm(1, 4.0), fmul(2, 0, 1), halt()])
        traces = core.run()
        assert len(traces) == 4
        assert core.registers.read_float(2) == 12.0
        assert core.halted

    def test_max_steps_limit(self) -> None:
        """Infinite loop hits max_steps limit."""
        from gpu_core.opcodes import jmp

        core = GPUCore()
        core.load_program([jmp(0)])  # infinite loop
        with pytest.raises(RuntimeError, match="Execution limit"):
            core.run(max_steps=100)

    def test_empty_program_raises(self) -> None:
        """Running an empty program raises immediately."""
        core = GPUCore()
        core.load_program([])
        with pytest.raises(RuntimeError, match="out of program range"):
            core.run()


class TestReset:
    """Test core reset functionality."""

    def test_reset_clears_registers(self) -> None:
        """Reset clears all register values."""
        core = GPUCore()
        core.load_program([limm(0, 42.0), halt()])
        core.run()
        core.reset()
        assert core.registers.read_float(0) == 0.0

    def test_reset_clears_pc(self) -> None:
        """Reset sets PC back to 0."""
        core = GPUCore()
        core.load_program([nop(), halt()])
        core.run()
        assert core.pc != 0 or core.halted
        core.reset()
        assert core.pc == 0

    def test_reset_clears_halted(self) -> None:
        """Reset clears the halted flag."""
        core = GPUCore()
        core.load_program([halt()])
        core.run()
        assert core.halted
        core.reset()
        assert not core.halted

    def test_reset_preserves_program(self) -> None:
        """Reset doesn't clear the loaded program — can run again."""
        core = GPUCore()
        core.load_program([limm(0, 99.0), halt()])
        core.run()
        core.reset()
        core.run()
        assert core.registers.read_float(0) == 99.0

    def test_reset_clears_memory(self) -> None:
        """Reset clears memory."""
        core = GPUCore()
        core.memory.store_python_float(0, 42.0)
        core.reset()
        assert core.memory.load_float_as_python(0) == 0.0

    def test_reset_clears_cycle(self) -> None:
        """Reset sets cycle counter back to 0."""
        core = GPUCore()
        core.load_program([nop(), halt()])
        core.run()
        assert core.cycle > 0
        core.reset()
        assert core.cycle == 0


class TestTraces:
    """Test execution trace output."""

    def test_trace_fields(self) -> None:
        """Trace has all expected fields."""
        core = GPUCore()
        core.load_program([limm(0, 1.0), halt()])
        trace = core.step()
        assert trace.cycle == 1
        assert trace.pc == 0
        assert trace.next_pc == 1
        assert not trace.halted
        assert trace.description != ""

    def test_trace_format(self) -> None:
        """Trace.format() returns readable multi-line string."""
        core = GPUCore()
        core.load_program([limm(0, 1.0), halt()])
        trace = core.step()
        formatted = trace.format()
        assert "[Cycle 1]" in formatted
        assert "PC=0" in formatted

    def test_halt_trace(self) -> None:
        """Halt trace shows HALTED."""
        core = GPUCore()
        core.load_program([halt()])
        trace = core.step()
        assert "HALTED" in trace.format()

    def test_trace_registers_changed(self) -> None:
        """Trace records which registers changed."""
        core = GPUCore()
        core.load_program([limm(5, 3.14), halt()])
        trace = core.step()
        assert "R5" in trace.registers_changed
