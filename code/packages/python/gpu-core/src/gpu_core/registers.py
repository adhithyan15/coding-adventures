"""FPRegisterFile — floating-point register storage for GPU cores.

=== What is a Register File? ===

A register file is the fastest storage in a processor — faster than cache,
faster than RAM. It's where the processor keeps the values it's currently
working with. Think of it like the handful of numbers you can keep in your
head while doing mental math.

    Register file (in your head):
        "first number"  = 3.14
        "second number" = 2.71
        "result"        = ???

    Register file (in a GPU core):
        R0  = 3.14  (FloatBits: sign=0, exp=[...], mantissa=[...])
        R1  = 2.71  (FloatBits: sign=0, exp=[...], mantissa=[...])
        R2  = 0.00  (will hold the result)

=== GPU vs CPU Register Files ===

CPU registers hold integers (32 or 64 bits of binary). GPU registers hold
floating-point numbers (IEEE 754 FloatBits). This reflects their different
purposes:

    CPU: general-purpose computation (loops, pointers, addresses → integers)
    GPU: parallel numeric computation (vertices, pixels, gradients → floats)

=== Why Configurable? ===

Different GPU vendors use different register counts:

    NVIDIA CUDA Core:    up to 255 registers per thread
    AMD Stream Processor: 256 VGPRs (Vector General Purpose Registers)
    Intel Vector Engine:  128 GRF entries (General Register File)
    ARM Mali:            64 registers per thread

By making the register count a constructor parameter, the same GPUCore
class can simulate any vendor's register architecture.

=== Register File Diagram ===

    ┌─────────────────────────────────────────┐
    │           FP Register File              │
    │         (32 registers × FP32)           │
    ├─────────────────────────────────────────┤
    │  R0:  [0][01111111][00000000000...0]    │  = +1.0
    │  R1:  [0][10000000][00000000000...0]    │  = +2.0
    │  R2:  [0][00000000][00000000000...0]    │  = +0.0
    │  ...                                    │
    │  R31: [0][00000000][00000000000...0]    │  = +0.0
    └─────────────────────────────────────────┘

    Each register stores a FloatBits value:
        sign (1 bit) + exponent (8 bits for FP32) + mantissa (23 bits for FP32)
"""

from __future__ import annotations

from fp_arithmetic import FP32, FloatBits, FloatFormat, bits_to_float, float_to_bits


class FPRegisterFile:
    """A configurable floating-point register file.

    Stores FloatBits values (from the fp-arithmetic package) in a fixed
    number of registers. Provides both raw FloatBits and convenience float
    interfaces for reading and writing.

    Args:
        num_registers: How many registers (default 32, max 256).
        fmt: The floating-point format (FP32, FP16, BF16).
    """

    def __init__(
        self,
        num_registers: int = 32,
        fmt: FloatFormat = FP32,
    ) -> None:
        if num_registers < 1 or num_registers > 256:
            msg = f"num_registers must be 1-256, got {num_registers}"
            raise ValueError(msg)
        self.num_registers = num_registers
        self.fmt = fmt
        # Initialize all registers to +0.0 in the specified format.
        self._zero = float_to_bits(0.0, fmt)
        self._values: list[FloatBits] = [self._zero] * num_registers

    def _check_index(self, index: int) -> None:
        """Validate a register index, raising IndexError if out of bounds."""
        if index < 0 or index >= self.num_registers:
            msg = (
                f"Register index {index} out of range "
                f"[0, {self.num_registers - 1}]"
            )
            raise IndexError(msg)

    def read(self, index: int) -> FloatBits:
        """Read a register as a FloatBits value.

        Args:
            index: Register number (0 to num_registers-1).

        Returns:
            The FloatBits value stored in that register.

        Raises:
            IndexError: If index is out of range.
        """
        self._check_index(index)
        return self._values[index]

    def write(self, index: int, value: FloatBits) -> None:
        """Write a FloatBits value to a register.

        Args:
            index: Register number (0 to num_registers-1).
            value: The FloatBits value to store.

        Raises:
            IndexError: If index is out of range.
        """
        self._check_index(index)
        self._values[index] = value

    def read_float(self, index: int) -> float:
        """Convenience: read a register as a Python float.

        This decodes the FloatBits back to a float, which is useful for
        inspection and testing but loses the bit-level detail.
        """
        return bits_to_float(self.read(index))

    def write_float(self, index: int, value: float) -> None:
        """Convenience: write a Python float to a register.

        This encodes the float as FloatBits in the register file's format,
        then stores it. Useful for setting up test inputs.
        """
        self.write(index, float_to_bits(value, self.fmt))

    def dump(self) -> dict[str, float]:
        """Return all register values as a dict of "R{n}" → float.

        Useful for debugging and test assertions. Only includes non-zero
        registers to reduce noise.

        Returns:
            Dict mapping register names to their float values.
        """
        result: dict[str, float] = {}
        for i in range(self.num_registers):
            val = bits_to_float(self._values[i])
            if val != 0.0:
                result[f"R{i}"] = val
        return result

    def dump_all(self) -> dict[str, float]:
        """Return ALL register values as a dict of "R{n}" → float.

        Unlike dump(), this includes zero-valued registers.
        """
        return {
            f"R{i}": bits_to_float(self._values[i])
            for i in range(self.num_registers)
        }

    def __repr__(self) -> str:
        non_zero = self.dump()
        if not non_zero:
            return f"FPRegisterFile({self.num_registers} regs, all zero)"
        entries = ", ".join(f"{k}={v}" for k, v in non_zero.items())
        return f"FPRegisterFile({entries})"
