"""Integration tests — multi-instruction GPU programs.

These tests verify that the GPU core correctly executes complete programs,
not just individual instructions. They serve as both tests and examples
of what GPU programs look like at the core level.
"""

from __future__ import annotations

import pytest
from fp_arithmetic import BF16, FP16

from gpu_core.core import GPUCore
from gpu_core.opcodes import (
    beq,
    blt,
    bne,
    fadd,
    ffma,
    fmul,
    fsub,
    halt,
    jmp,
    limm,
    load,
    mov,
    nop,
    store,
)


class TestSAXPY:
    """SAXPY: y = a * x + y — the "hello world" of GPU programming.

    In real GPU code, SAXPY runs across thousands of threads, each computing
    one element. Here we simulate what a single thread does: one FMA.
    """

    def test_saxpy_single_element(self) -> None:
        """y = 2.0 * 3.0 + 1.0 = 7.0."""
        core = GPUCore()
        core.load_program([
            limm(0, 2.0),      # R0 = a = 2.0
            limm(1, 3.0),      # R1 = x = 3.0
            limm(2, 1.0),      # R2 = y = 1.0
            ffma(3, 0, 1, 2),  # R3 = a * x + y = 7.0
            halt(),
        ])
        traces = core.run()
        assert core.registers.read_float(3) == 7.0
        assert len(traces) == 5

    def test_saxpy_zero_alpha(self) -> None:
        """y = 0.0 * x + y = y (alpha=0 means just copy y)."""
        core = GPUCore()
        core.load_program([
            limm(0, 0.0),      # a = 0
            limm(1, 99.0),     # x = 99 (doesn't matter)
            limm(2, 5.0),      # y = 5
            ffma(3, 0, 1, 2),  # R3 = 0*99 + 5 = 5
            halt(),
        ])
        core.run()
        assert core.registers.read_float(3) == 5.0


class TestDotProduct:
    """Dot product: sum of element-wise products.

    dot(A, B) = A[0]*B[0] + A[1]*B[1] + A[2]*B[2]

    This is the fundamental operation in neural networks — every neuron
    computes a dot product of its inputs and weights.
    """

    def test_dot_product_3d(self) -> None:
        """dot([1,2,3], [4,5,6]) = 4 + 10 + 18 = 32."""
        core = GPUCore()
        core.load_program([
            # Load vector A
            limm(0, 1.0),      # A[0]
            limm(1, 2.0),      # A[1]
            limm(2, 3.0),      # A[2]
            # Load vector B
            limm(3, 4.0),      # B[0]
            limm(4, 5.0),      # B[1]
            limm(5, 6.0),      # B[2]
            # Accumulate with FMA
            limm(6, 0.0),        # acc = 0
            ffma(6, 0, 3, 6),   # acc = 1*4 + 0 = 4
            ffma(6, 1, 4, 6),   # acc = 2*5 + 4 = 14
            ffma(6, 2, 5, 6),   # acc = 3*6 + 14 = 32
            halt(),
        ])
        core.run()
        assert core.registers.read_float(6) == 32.0


class TestLoop:
    """Test programs with loops (branches)."""

    def test_sum_1_to_4(self) -> None:
        """Sum of 1 + 2 + 3 + 4 = 10 using a loop.

        Program:
            R0 = sum = 0
            R1 = i = 1
            R2 = increment = 1
            R3 = limit = 5
        loop:
            sum += i           (PC=4)
            i += increment     (PC=5)
            if i < limit: goto loop  (PC=6, branch offset = -2 → PC=4)
            halt               (PC=7)
        """
        core = GPUCore()
        core.load_program([
            limm(0, 0.0),       # R0 = sum
            limm(1, 1.0),       # R1 = i
            limm(2, 1.0),       # R2 = 1 (increment)
            limm(3, 5.0),       # R3 = limit
            fadd(0, 0, 1),      # sum += i        (PC=4)
            fadd(1, 1, 2),      # i += 1          (PC=5)
            blt(1, 3, -2),      # if i < 5: back  (PC=6)
            halt(),             #                  (PC=7)
        ])
        core.run()
        assert core.registers.read_float(0) == 10.0

    def test_countdown(self) -> None:
        """Count down from 3 to 0.

        R0 = counter = 3
        R1 = decrement = 1
        R2 = zero = 0
        loop: counter -= decrement; if counter != zero: loop
        """
        core = GPUCore()
        core.load_program([
            limm(0, 3.0),       # counter
            limm(1, 1.0),       # decrement
            limm(2, 0.0),       # zero
            fsub(0, 0, 1),      # counter -= 1   (PC=3)
            bne(0, 2, -1),      # if counter != 0: back (PC=4)
            halt(),             #                (PC=5)
        ])
        core.run()
        assert core.registers.read_float(0) == 0.0


class TestMemoryPrograms:
    """Test programs that use load/store."""

    def test_store_and_load_array(self) -> None:
        """Store 3 values to memory, load them back, sum them.

        This simulates a GPU thread loading input data from memory,
        computing on it, and writing the result back.
        """
        core = GPUCore()
        # Pre-store some values in memory
        core.memory.store_python_float(0, 10.0)
        core.memory.store_python_float(4, 20.0)
        core.memory.store_python_float(8, 30.0)

        core.load_program([
            limm(10, 0.0),       # R10 = base address
            load(0, 10, 0.0),    # R0 = Mem[0] = 10.0
            load(1, 10, 4.0),    # R1 = Mem[4] = 20.0
            load(2, 10, 8.0),    # R2 = Mem[8] = 30.0
            fadd(3, 0, 1),       # R3 = 10 + 20 = 30
            fadd(3, 3, 2),       # R3 = 30 + 30 = 60
            store(10, 3, 12.0),  # Mem[12] = 60.0
            halt(),
        ])
        core.run()
        assert core.registers.read_float(3) == 60.0
        assert core.memory.load_float_as_python(12) == 60.0


class TestConditional:
    """Test conditional execution patterns."""

    def test_max_of_two(self) -> None:
        """Compute max(a, b) using a branch.

        if a < b:
            result = b
        else:
            result = a
        """
        core = GPUCore()
        core.load_program([
            limm(0, 3.0),       # R0 = a
            limm(1, 7.0),       # R1 = b
            blt(0, 1, 2),       # if a < b: skip to "result = b"  (PC=2)
            mov(2, 0),          # result = a                       (PC=3)
            jmp(5),             # skip "result = b"                (PC=4)
            mov(2, 1),          # result = b                       (PC=5)
            halt(),             #                                  (PC=6)
        ])
        core.run()
        assert core.registers.read_float(2) == 7.0

    def test_max_reversed(self) -> None:
        """max(7, 3) = 7 — takes the else branch."""
        core = GPUCore()
        core.load_program([
            limm(0, 7.0),
            limm(1, 3.0),
            blt(0, 1, 2),       # 7 < 3? No → fall through
            mov(2, 0),          # result = a = 7
            jmp(6),             # skip "result = b"
            mov(2, 1),          # skipped
            halt(),             # PC=6
        ])
        core.run()
        assert core.registers.read_float(2) == 7.0


class TestPrecisionModes:
    """Test with different floating-point formats."""

    def test_fp16_execution(self) -> None:
        """Run a program in FP16 mode."""
        core = GPUCore(fmt=FP16)
        core.load_program([
            limm(0, 1.0),
            limm(1, 2.0),
            fadd(2, 0, 1),
            halt(),
        ])
        core.run()
        assert core.registers.read_float(2) == 3.0

    def test_bf16_execution(self) -> None:
        """Run a program in BF16 mode."""
        core = GPUCore(fmt=BF16)
        core.load_program([
            limm(0, 4.0),
            limm(1, 5.0),
            fmul(2, 0, 1),
            halt(),
        ])
        core.run()
        assert core.registers.read_float(2) == 20.0


class TestEdgeCases:
    """Test edge cases and error conditions."""

    def test_nop_program(self) -> None:
        """A program of only NOPs and HALT."""
        core = GPUCore()
        core.load_program([nop(), nop(), nop(), halt()])
        traces = core.run()
        assert len(traces) == 4
        assert core.halted

    def test_self_modifying_register(self) -> None:
        """An instruction can read and write the same register."""
        core = GPUCore()
        core.load_program([
            limm(0, 5.0),
            fadd(0, 0, 0),  # R0 = R0 + R0 = 10.0
            halt(),
        ])
        core.run()
        assert core.registers.read_float(0) == 10.0

    def test_large_register_index(self) -> None:
        """Use high-numbered registers (NVIDIA-scale)."""
        core = GPUCore(num_registers=256)
        core.load_program([
            limm(200, 42.0),
            limm(255, 1.0),
            fadd(254, 200, 255),
            halt(),
        ])
        core.run()
        assert core.registers.read_float(254) == 43.0

    def test_beq_with_zero_offset(self) -> None:
        """BEQ with offset 0 creates an infinite loop (caught by max_steps)."""
        core = GPUCore()
        core.load_program([
            limm(0, 1.0),
            limm(1, 1.0),
            beq(0, 1, 0),  # infinite: jump to self
            halt(),
        ])
        with pytest.raises(RuntimeError, match="Execution limit"):
            core.run(max_steps=50)
