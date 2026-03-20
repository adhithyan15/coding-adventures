"""LocalMemory — byte-addressable scratchpad with floating-point load/store.

=== What is Local Memory? ===

Every GPU thread has a small, private memory area called "local memory" or
"scratchpad." It's used for temporary storage that doesn't fit in registers:
spilled variables, array elements, intermediate results.

    ┌─────────────────────────────────────────────┐
    │              Local Memory (4 KB)             │
    ├─────────────────────────────────────────────┤
    │  0x000: [42] [00] [48] [42]  ← 3.14 as FP32 │
    │  0x004: [EC] [51] [2D] [40]  ← 2.71 as FP32 │
    │  0x008: [00] [00] [00] [00]  ← 0.0           │
    │  ...                                         │
    │  0xFFC: [00] [00] [00] [00]                   │
    └─────────────────────────────────────────────┘

=== How Floats Live in Memory ===

A FloatBits value (sign + exponent + mantissa) must be converted to raw bytes
before it can be stored in memory. This is the same process that happens in
real hardware when a GPU core executes a STORE instruction:

    1. Take the FloatBits fields: sign=0, exponent=[01111111], mantissa=[10010...]
    2. Concatenate into a bit string: 0_01111111_10010001000011111101101
    3. Group into bytes: [3F] [C9] [0F] [DB]  (that's 3.14159 in FP32)
    4. Write bytes to memory in little-endian order: [DB] [0F] [C9] [3F]

Loading reverses this: read bytes, reassemble bits, create FloatBits.

=== Memory Sizes Across Vendors ===

    NVIDIA: 512 KB local memory per thread (rarely used, slow)
    AMD:    Scratch memory, up to 4 MB per wavefront
    ARM:    Stack memory region per thread
    TPU:    No per-PE memory (data flows through systolic array)

Our default of 4 KB is small but sufficient for educational programs.
"""

from __future__ import annotations

import struct

from fp_arithmetic import FP32, FloatBits, FloatFormat, bits_to_float, float_to_bits


class LocalMemory:
    """Byte-addressable local scratchpad memory with FP-aware load/store.

    Provides both raw byte access and convenient floating-point operations
    that handle the conversion between FloatBits and byte sequences.

    Args:
        size: Memory size in bytes (default 4096 = 4 KB).
    """

    def __init__(self, size: int = 4096) -> None:
        if size < 1:
            msg = f"Memory size must be positive, got {size}"
            raise ValueError(msg)
        self.size = size
        self._data = bytearray(size)

    def _check_bounds(self, address: int, num_bytes: int) -> None:
        """Validate that an access is within bounds."""
        if address < 0 or address + num_bytes > self.size:
            msg = (
                f"Memory access at {address}:{address + num_bytes} "
                f"out of bounds [0, {self.size})"
            )
            raise IndexError(msg)

    # --- Raw byte access ---

    def read_byte(self, address: int) -> int:
        """Read a single byte from memory."""
        self._check_bounds(address, 1)
        return self._data[address]

    def write_byte(self, address: int, value: int) -> None:
        """Write a single byte to memory."""
        self._check_bounds(address, 1)
        self._data[address] = value & 0xFF

    def read_bytes(self, address: int, count: int) -> bytes:
        """Read multiple bytes from memory."""
        self._check_bounds(address, count)
        return bytes(self._data[address : address + count])

    def write_bytes(self, address: int, data: bytes) -> None:
        """Write multiple bytes to memory."""
        self._check_bounds(address, len(data))
        self._data[address : address + len(data)] = data

    # --- Floating-point access ---

    def _float_byte_width(self, fmt: FloatFormat) -> int:
        """How many bytes a float format uses: FP32=4, FP16/BF16=2."""
        return fmt.total_bits // 8

    def _floatbits_to_bytes(self, value: FloatBits) -> bytes:
        """Convert a FloatBits to raw bytes (little-endian).

        The process:
        1. Concatenate sign + exponent + mantissa into one integer
        2. Pack that integer into bytes using struct

        Example for FP32 value 1.0:
            sign=0, exponent=[0,1,1,1,1,1,1,1], mantissa=[0]*23
            → bit string: 0_01111111_00000000000000000000000
            → integer: 0x3F800000
            → bytes (little-endian): [00, 00, 80, 3F]
        """
        # Reassemble the bit pattern from FloatBits fields
        bits = value.sign
        for b in value.exponent:
            bits = (bits << 1) | b
        for b in value.mantissa:
            bits = (bits << 1) | b

        # Pack as bytes using the appropriate struct format
        byte_width = self._float_byte_width(value.fmt)
        if byte_width == 4:
            return struct.pack("<I", bits)  # unsigned 32-bit, little-endian
        if byte_width == 2:
            return struct.pack("<H", bits)  # unsigned 16-bit, little-endian
        msg = f"Unsupported float width: {byte_width} bytes"
        raise ValueError(msg)

    def _bytes_to_floatbits(self, data: bytes, fmt: FloatFormat) -> FloatBits:
        """Convert raw bytes (little-endian) back to a FloatBits.

        Reverses _floatbits_to_bytes: unpack integer, split into fields.
        """
        byte_width = self._float_byte_width(fmt)
        if byte_width == 4:
            (bits,) = struct.unpack("<I", data)
        elif byte_width == 2:
            (bits,) = struct.unpack("<H", data)
        else:
            msg = f"Unsupported float width: {byte_width} bytes"
            raise ValueError(msg)

        # Extract fields by shifting and masking
        total_bits = fmt.total_bits
        mantissa_bits = fmt.mantissa_bits
        exponent_bits = fmt.exponent_bits

        # Mantissa is the lowest mantissa_bits bits
        mantissa_mask = (1 << mantissa_bits) - 1
        mantissa_int = bits & mantissa_mask
        mantissa = [
            (mantissa_int >> (mantissa_bits - 1 - i)) & 1
            for i in range(mantissa_bits)
        ]

        # Exponent is the next exponent_bits bits
        exponent_mask = (1 << exponent_bits) - 1
        exponent_int = (bits >> mantissa_bits) & exponent_mask
        exponent = [
            (exponent_int >> (exponent_bits - 1 - i)) & 1
            for i in range(exponent_bits)
        ]

        # Sign is the highest bit
        sign = (bits >> (total_bits - 1)) & 1

        return FloatBits(sign=sign, exponent=exponent, mantissa=mantissa, fmt=fmt)

    def load_float(self, address: int, fmt: FloatFormat = FP32) -> FloatBits:
        """Load a floating-point value from memory.

        Reads the appropriate number of bytes (4 for FP32, 2 for FP16/BF16)
        starting at the given address, and converts them to a FloatBits.

        Args:
            address: Byte address to read from.
            fmt: The floating-point format to interpret the bytes as.

        Returns:
            A FloatBits value decoded from the bytes at that address.
        """
        byte_width = self._float_byte_width(fmt)
        data = self.read_bytes(address, byte_width)
        return self._bytes_to_floatbits(data, fmt)

    def store_float(self, address: int, value: FloatBits) -> None:
        """Store a floating-point value to memory.

        Converts the FloatBits to bytes and writes them starting at the
        given address.

        Args:
            address: Byte address to write to.
            value: The FloatBits value to store.
        """
        data = self._floatbits_to_bytes(value)
        self.write_bytes(address, data)

    def load_float_as_python(
        self, address: int, fmt: FloatFormat = FP32
    ) -> float:
        """Convenience: load a float and convert to Python float."""
        return bits_to_float(self.load_float(address, fmt))

    def store_python_float(
        self, address: int, value: float, fmt: FloatFormat = FP32
    ) -> None:
        """Convenience: store a Python float to memory."""
        self.store_float(address, float_to_bits(value, fmt))

    def dump(self, start: int = 0, length: int = 64) -> list[int]:
        """Return a slice of memory as a list of byte values.

        Useful for debugging. Default shows the first 64 bytes.
        """
        end = min(start + length, self.size)
        return list(self._data[start:end])

    def __repr__(self) -> str:
        # Count non-zero bytes to give a sense of usage
        used = sum(1 for b in self._data if b != 0)
        return f"LocalMemory({self.size} bytes, {used} non-zero)"
