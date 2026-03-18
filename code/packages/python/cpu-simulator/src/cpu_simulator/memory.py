"""Memory — the CPU's large, slow storage.

=== What is memory? ===

Memory (RAM — Random Access Memory) is a large array of bytes that the CPU
can read from and write to. Unlike registers (which are tiny and fast),
memory can hold megabytes or gigabytes of data, but accessing it takes
many clock cycles.

Every byte in memory has an "address" — a number that identifies its
location, like a house number on a street. To read a byte, you tell the
memory controller "give me the byte at address 42." To write, you say
"put the value 7 at address 42."

=== Memory in our simulator ===

We simulate memory as a Python bytearray — a simple array of bytes.
Each element is one byte (0-255). Multi-byte values (like 32-bit integers)
are stored in consecutive bytes.

=== Byte ordering (Endianness) ===

When storing a multi-byte value (like the 32-bit integer 0x12345678),
there are two ways to lay out the bytes:

  Big-endian:    [0x12] [0x34] [0x56] [0x78]   (most significant byte first)
  Little-endian: [0x78] [0x56] [0x34] [0x12]   (least significant byte first)

RISC-V and x86 use little-endian. ARM supports both. Our simulator
defaults to little-endian because that's what RISC-V uses.

Think of it like writing the number 1234:
  - Big-endian is like English: you write the thousands digit first (1, 2, 3, 4)
  - Little-endian is the opposite: ones digit first (4, 3, 2, 1)
"""


class Memory:
    """Byte-addressable memory.

    Memory is a flat array of bytes. Each byte is addressed by an integer
    starting from 0.

        Address:  0     1     2     3     4     5    ...
        Value:   [00]  [00]  [00]  [00]  [00]  [00]  ...

    Example:
        >>> mem = Memory(size=1024)  # 1 KB of memory
        >>> mem.write_byte(0, 42)
        >>> mem.read_byte(0)
        42
        >>> mem.write_word(4, 0x12345678)  # Write a 32-bit value
        >>> mem.read_word(4)
        0x12345678
    """

    def __init__(self, size: int = 65536) -> None:
        """Create a memory of `size` bytes, all initialized to 0.

        Args:
            size: Number of bytes. Default is 64 KB (65536 bytes), which is
                  enough for our simple programs. Real computers have billions
                  of bytes (gigabytes).
        """
        if size < 1:
            msg = "Memory size must be at least 1 byte"
            raise ValueError(msg)
        self._data = bytearray(size)
        self.size = size

    def _check_address(self, address: int, num_bytes: int = 1) -> None:
        """Verify an address is within bounds."""
        if address < 0 or address + num_bytes > self.size:
            msg = (
                f"Memory access out of bounds: address {address}, "
                f"size {num_bytes}, memory size {self.size}"
            )
            raise IndexError(msg)

    def read_byte(self, address: int) -> int:
        """Read a single byte (8 bits, value 0-255) from memory.

        Example:
            >>> mem = Memory(size=16)
            >>> mem.write_byte(3, 0xFF)
            >>> mem.read_byte(3)
            255
        """
        self._check_address(address)
        return self._data[address]

    def write_byte(self, address: int, value: int) -> None:
        """Write a single byte to memory. Value is masked to 0-255.

        Example:
            >>> mem = Memory(size=16)
            >>> mem.write_byte(0, 42)
            >>> mem.read_byte(0)
            42
        """
        self._check_address(address)
        self._data[address] = value & 0xFF

    def read_word(self, address: int) -> int:
        """Read a 32-bit word (4 bytes) from memory, little-endian.

        Little-endian means the least significant byte is at the lowest
        address. For example, the value 0x12345678 is stored as:

            Address:   [addr]  [addr+1]  [addr+2]  [addr+3]
            Value:      0x78    0x56      0x34      0x12
                        ^^^^                        ^^^^
                        LSB (least significant)     MSB (most significant)

        Example:
            >>> mem = Memory(size=16)
            >>> mem.write_word(0, 0x12345678)
            >>> hex(mem.read_word(0))
            '0x12345678'
        """
        self._check_address(address, 4)
        return (
            self._data[address]
            | (self._data[address + 1] << 8)
            | (self._data[address + 2] << 16)
            | (self._data[address + 3] << 24)
        )

    def write_word(self, address: int, value: int) -> None:
        """Write a 32-bit word to memory, little-endian.

        Example:
            >>> mem = Memory(size=16)
            >>> mem.write_word(0, 3)       # 3 = 0x00000003
            >>> mem.read_byte(0)           # LSB
            3
            >>> mem.read_byte(1)           # next byte
            0
        """
        self._check_address(address, 4)
        value = value & 0xFFFFFFFF  # Mask to 32 bits
        self._data[address] = value & 0xFF
        self._data[address + 1] = (value >> 8) & 0xFF
        self._data[address + 2] = (value >> 16) & 0xFF
        self._data[address + 3] = (value >> 24) & 0xFF

    def load_bytes(self, address: int, data: bytes) -> None:
        """Load a sequence of bytes into memory starting at `address`.

        This is how programs are loaded: the machine code bytes are copied
        into memory starting at address 0 (or wherever the program begins).

        Example:
            >>> mem = Memory(size=16)
            >>> mem.load_bytes(0, b'\\x01\\x02\\x03')
            >>> mem.read_byte(0), mem.read_byte(1), mem.read_byte(2)
            (1, 2, 3)
        """
        self._check_address(address, len(data))
        for i, byte in enumerate(data):
            self._data[address + i] = byte

    def dump(self, start: int = 0, length: int = 16) -> list[int]:
        """Return a slice of memory as a list of byte values.

        Useful for debugging — see what's stored in a range of addresses.

        Example:
            >>> mem = Memory(size=16)
            >>> mem.write_byte(0, 0xAB)
            >>> mem.dump(0, 4)
            [171, 0, 0, 0]
        """
        self._check_address(start, length)
        return list(self._data[start : start + length])
