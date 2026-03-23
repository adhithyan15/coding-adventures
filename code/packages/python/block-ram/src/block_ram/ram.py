"""RAM Modules — synchronous memory with read/write ports.

=== From Array to Module ===

An SRAM array (sram.py) provides raw row-level read/write. A RAM module
adds the interface that digital circuits actually use:

1. **Address decoding** — binary address bits select a row
2. **Synchronous operation** — reads and writes happen on clock edges
3. **Read modes** — what the output shows during a write operation
4. **Dual-port access** — two independent ports for simultaneous operations

=== Read Modes ===

During a write operation, what should the data output show? There are
three valid answers, and different designs need different behaviors:

1. **Read-first**: Output shows the OLD value at the address being written.
   The read happens before the write within the same cycle. Useful when
   you need to know what was there before overwriting it.

2. **Write-first** (read-after-write): Output shows the NEW value being
   written. The write happens first, then the read sees the new value.
   Useful for pipeline forwarding.

3. **No-change**: Output retains its previous value during writes. This
   saves power in FPGA Block RAMs because the read circuitry doesn't
   activate during writes.

=== Dual-Port RAM ===

Two completely independent ports (A and B), each with its own address,
data, and write enable. Both can operate simultaneously:
- Read A + Read B at different addresses → both get their data
- Write A + Read B at different addresses → both succeed
- Write A + Write B at the SAME address → **collision** (undefined in
  hardware, we raise an error)
"""

from __future__ import annotations

from enum import Enum

from block_ram.sram import SRAMArray, _validate_bit


class ReadMode(Enum):
    """Controls what data_out shows during a write operation.

    READ_FIRST:  data_out = old value (read before write)
    WRITE_FIRST: data_out = new value (write before read)
    NO_CHANGE:   data_out = previous read value (output unchanged)
    """

    READ_FIRST = "read_first"
    WRITE_FIRST = "write_first"
    NO_CHANGE = "no_change"


class WriteCollisionError(Exception):
    """Raised when both ports of a dual-port RAM write to the same address.

    In real hardware, simultaneous writes to the same address produce
    undefined results (the cell may store either value, or a corrupted
    value). We detect this and raise an error to prevent silent bugs.

    Attributes:
        address: The conflicting address (as integer).
    """

    def __init__(self, address: int) -> None:
        self.address = address
        super().__init__(
            f"Write collision: both ports writing to address {address}"
        )


class SinglePortRAM:
    """Single-port synchronous RAM.

    One address port, one data bus. Each clock cycle you can do ONE
    operation: read OR write (controlled by write_enable).

    Interface::

                    ┌──────────────────────────┐
      address ──────┤                          │
                    │     Single-Port RAM      │
      data_in ──────┤                          ├──── data_out
                    │     (depth × width)      │
      write_en ─────┤                          │
                    │                          │
      clock ────────┤                          │
                    └──────────────────────────┘

    Operations happen on the rising edge of the clock (transition 0→1).

    Parameters:
        depth:     Number of addressable words (>= 1)
        width:     Bits per word (>= 1)
        read_mode: What data_out shows during writes (default: READ_FIRST)

    Example:
        >>> ram = SinglePortRAM(depth=256, width=8)
        >>> # Write 0xFF to address 0
        >>> ram.tick(0, address=0, data_in=[1]*8, write_enable=1)
        >>> out = ram.tick(1, address=0, data_in=[1]*8, write_enable=1)
        >>> # Read from address 0
        >>> ram.tick(0, address=0, data_in=[0]*8, write_enable=0)
        >>> out = ram.tick(1, address=0, data_in=[0]*8, write_enable=0)
        >>> out
        [1, 1, 1, 1, 1, 1, 1, 1]
    """

    def __init__(
        self,
        depth: int,
        width: int,
        read_mode: ReadMode = ReadMode.READ_FIRST,
    ) -> None:
        if depth < 1:
            msg = f"depth must be >= 1, got {depth}"
            raise ValueError(msg)
        if width < 1:
            msg = f"width must be >= 1, got {width}"
            raise ValueError(msg)

        self._depth = depth
        self._width = width
        self._read_mode = read_mode
        self._array = SRAMArray(depth, width)
        self._prev_clock = 0
        self._last_read: list[int] = [0] * width

    def tick(
        self,
        clock: int,
        address: int,
        data_in: list[int],
        write_enable: int,
    ) -> list[int]:
        """Execute one half-cycle. Operations happen on rising edge (0→1).

        Parameters:
            clock:        Clock signal (0 or 1)
            address:      Word address (integer, 0 to depth-1)
            data_in:      Data to write (list of width bits, LSB first)
            write_enable: 0 = read, 1 = write

        Returns:
            data_out: list of width bits read from the address.
            During writes, behavior depends on read_mode.

        Raises:
            ValueError: If address out of range or data_in wrong length.
        """
        _validate_bit(clock, "clock")
        _validate_bit(write_enable, "write_enable")
        self._validate_address(address)
        self._validate_data(data_in)

        # Detect rising edge: previous clock was 0, now it's 1
        rising_edge = self._prev_clock == 0 and clock == 1
        self._prev_clock = clock

        if not rising_edge:
            return list(self._last_read)

        # Rising edge: perform the operation
        if write_enable == 0:
            # Read operation
            self._last_read = self._array.read(address)
            return list(self._last_read)

        # Write operation — behavior depends on read mode
        if self._read_mode == ReadMode.READ_FIRST:
            # Read the old value first, then write
            self._last_read = self._array.read(address)
            self._array.write(address, data_in)
            return list(self._last_read)

        if self._read_mode == ReadMode.WRITE_FIRST:
            # Write first, then read back the new value
            self._array.write(address, data_in)
            self._last_read = list(data_in)
            return list(self._last_read)

        # NO_CHANGE: write but don't update data_out
        self._array.write(address, data_in)
        return list(self._last_read)

    @property
    def depth(self) -> int:
        """Number of addressable words."""
        return self._depth

    @property
    def width(self) -> int:
        """Bits per word."""
        return self._width

    def dump(self) -> list[list[int]]:
        """Return all contents for inspection.

        Returns:
            List of rows, each row is a list of bits.
        """
        return [self._array.read(row) for row in range(self._depth)]

    def _validate_address(self, address: int) -> None:
        """Check address is in range."""
        if not isinstance(address, int) or isinstance(address, bool):
            msg = f"address must be an int, got {type(address).__name__}"
            raise TypeError(msg)
        if address < 0 or address >= self._depth:
            msg = f"address {address} out of range [0, {self._depth - 1}]"
            raise ValueError(msg)

    def _validate_data(self, data_in: list[int]) -> None:
        """Check data_in is correct length and all bits."""
        if not isinstance(data_in, list):
            msg = "data_in must be a list of bits"
            raise TypeError(msg)
        if len(data_in) != self._width:
            msg = f"data_in length {len(data_in)} does not match width {self._width}"
            raise ValueError(msg)
        for i, bit in enumerate(data_in):
            _validate_bit(bit, f"data_in[{i}]")


class DualPortRAM:
    """True dual-port synchronous RAM.

    Two independent ports (A and B), each with its own address, data,
    and write enable. Both ports can operate simultaneously on different
    addresses.

    Interface::

      ┌────────────────────────────────────────────┐
      │               Dual-Port RAM                │
      │  Port A                      Port B        │
      │  addr_a, din_a, we_a        addr_b, din_b  │
      │  dout_a                      we_b, dout_b  │
      └────────────────────────────────────────────┘

    Write collision: if both ports write to the same address in the
    same cycle, a WriteCollisionError is raised.

    Parameters:
        depth:       Number of addressable words (>= 1)
        width:       Bits per word (>= 1)
        read_mode_a: Read mode for port A (default: READ_FIRST)
        read_mode_b: Read mode for port B (default: READ_FIRST)
    """

    def __init__(
        self,
        depth: int,
        width: int,
        read_mode_a: ReadMode = ReadMode.READ_FIRST,
        read_mode_b: ReadMode = ReadMode.READ_FIRST,
    ) -> None:
        if depth < 1:
            msg = f"depth must be >= 1, got {depth}"
            raise ValueError(msg)
        if width < 1:
            msg = f"width must be >= 1, got {width}"
            raise ValueError(msg)

        self._depth = depth
        self._width = width
        self._read_mode_a = read_mode_a
        self._read_mode_b = read_mode_b
        self._array = SRAMArray(depth, width)
        self._prev_clock = 0
        self._last_read_a: list[int] = [0] * width
        self._last_read_b: list[int] = [0] * width

    def tick(
        self,
        clock: int,
        # Port A
        address_a: int,
        data_in_a: list[int],
        write_enable_a: int,
        # Port B
        address_b: int,
        data_in_b: list[int],
        write_enable_b: int,
    ) -> tuple[list[int], list[int]]:
        """Execute one half-cycle on both ports.

        Parameters:
            clock:          Clock signal (0 or 1)
            address_a:      Port A word address
            data_in_a:      Port A write data
            write_enable_a: Port A write enable (0=read, 1=write)
            address_b:      Port B word address
            data_in_b:      Port B write data
            write_enable_b: Port B write enable (0=read, 1=write)

        Returns:
            (data_out_a, data_out_b): Read data from each port.

        Raises:
            WriteCollisionError: Both ports write to the same address.
        """
        _validate_bit(clock, "clock")
        _validate_bit(write_enable_a, "write_enable_a")
        _validate_bit(write_enable_b, "write_enable_b")
        self._validate_address(address_a, "address_a")
        self._validate_address(address_b, "address_b")
        self._validate_data(data_in_a, "data_in_a")
        self._validate_data(data_in_b, "data_in_b")

        rising_edge = self._prev_clock == 0 and clock == 1
        self._prev_clock = clock

        if not rising_edge:
            return (list(self._last_read_a), list(self._last_read_b))

        # Check for write collision
        if (
            write_enable_a == 1
            and write_enable_b == 1
            and address_a == address_b
        ):
            raise WriteCollisionError(address_a)

        # Process port A
        out_a = self._process_port(
            address_a,
            data_in_a,
            write_enable_a,
            self._read_mode_a,
            self._last_read_a,
        )
        self._last_read_a = out_a

        # Process port B
        out_b = self._process_port(
            address_b,
            data_in_b,
            write_enable_b,
            self._read_mode_b,
            self._last_read_b,
        )
        self._last_read_b = out_b

        return (list(out_a), list(out_b))

    def _process_port(
        self,
        address: int,
        data_in: list[int],
        write_enable: int,
        read_mode: ReadMode,
        last_read: list[int],
    ) -> list[int]:
        """Process a single port operation."""
        if write_enable == 0:
            return self._array.read(address)

        if read_mode == ReadMode.READ_FIRST:
            result = self._array.read(address)
            self._array.write(address, data_in)
            return result

        if read_mode == ReadMode.WRITE_FIRST:
            self._array.write(address, data_in)
            return list(data_in)

        # NO_CHANGE
        self._array.write(address, data_in)
        return list(last_read)

    @property
    def depth(self) -> int:
        """Number of addressable words."""
        return self._depth

    @property
    def width(self) -> int:
        """Bits per word."""
        return self._width

    def _validate_address(self, address: int, name: str = "address") -> None:
        """Check address is in range."""
        if not isinstance(address, int) or isinstance(address, bool):
            msg = f"{name} must be an int, got {type(address).__name__}"
            raise TypeError(msg)
        if address < 0 or address >= self._depth:
            msg = f"{name} {address} out of range [0, {self._depth - 1}]"
            raise ValueError(msg)

    def _validate_data(
        self, data_in: list[int], name: str = "data_in"
    ) -> None:
        """Check data_in is correct length and all bits."""
        if not isinstance(data_in, list):
            msg = f"{name} must be a list of bits"
            raise TypeError(msg)
        if len(data_in) != self._width:
            msg = f"{name} length {len(data_in)} does not match width {self._width}"
            raise ValueError(msg)
        for i, bit in enumerate(data_in):
            _validate_bit(bit, f"{name}[{i}]")
