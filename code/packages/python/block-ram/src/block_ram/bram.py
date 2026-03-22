"""Configurable Block RAM — FPGA-style memory with reconfigurable aspect ratio.

=== What is Block RAM? ===

In an FPGA, Block RAM (BRAM) tiles are dedicated memory blocks separate
from the configurable logic. Each tile has a fixed total storage (typically
18 Kbit or 36 Kbit) but can be configured with different width/depth ratios:

    18 Kbit BRAM configurations:
    ┌───────────────┬───────┬───────┬────────────┐
    │ Configuration │ Depth │ Width │ Total bits │
    ├───────────────┼───────┼───────┼────────────┤
    │ 16K × 1       │ 16384 │     1 │      16384 │
    │  8K × 2       │  8192 │     2 │      16384 │
    │  4K × 4       │  4096 │     4 │      16384 │
    │  2K × 8       │  2048 │     8 │      16384 │
    │  1K × 16      │  1024 │    16 │      16384 │
    │ 512 × 32      │   512 │    32 │      16384 │
    └───────────────┴───────┴───────┴────────────┘

The total storage is fixed; you trade depth for width by changing how the
address decoder and column MUX are configured. The underlying SRAM cells
don't change — only the access pattern changes.

This module wraps DualPortRAM with reconfiguration support.
"""

from __future__ import annotations

from block_ram.ram import DualPortRAM, _validate_bit


class ConfigurableBRAM:
    """Block RAM with configurable aspect ratio.

    Total storage is fixed at initialization. Width and depth can be
    reconfigured as long as width × depth <= total_bits.

    Supports dual-port access when dual_port=True (default).

    Parameters:
        total_bits: Total storage in bits (default: 18432 = 18 Kbit)
        width:      Initial bits per word (default: 8)
        dual_port:  Enable dual-port access (default: True)

    Example:
        >>> bram = ConfigurableBRAM(total_bits=1024, width=8)
        >>> bram.depth  # 1024 / 8 = 128
        128
        >>> bram.reconfigure(width=16)
        >>> bram.depth  # 1024 / 16 = 64
        64
    """

    def __init__(
        self,
        total_bits: int = 18432,
        width: int = 8,
        dual_port: bool = True,
    ) -> None:
        if total_bits < 1:
            msg = f"total_bits must be >= 1, got {total_bits}"
            raise ValueError(msg)
        if width < 1:
            msg = f"width must be >= 1, got {width}"
            raise ValueError(msg)
        if total_bits % width != 0:
            msg = f"width {width} does not evenly divide total_bits {total_bits}"
            raise ValueError(msg)

        self._total_bits = total_bits
        self._width = width
        self._dual_port = dual_port
        self._depth = total_bits // width
        self._ram = DualPortRAM(self._depth, self._width)
        self._prev_clock = 0
        self._last_read_a: list[int] = [0] * width
        self._last_read_b: list[int] = [0] * width

    def reconfigure(self, width: int) -> None:
        """Change the aspect ratio. Clears all stored data.

        Parameters:
            width: New bits per word. Must evenly divide total_bits.

        Raises:
            ValueError: If width doesn't divide total_bits or is < 1.
        """
        if width < 1:
            msg = f"width must be >= 1, got {width}"
            raise ValueError(msg)
        if self._total_bits % width != 0:
            msg = f"width {width} does not evenly divide total_bits {self._total_bits}"
            raise ValueError(msg)

        self._width = width
        self._depth = self._total_bits // width
        self._ram = DualPortRAM(self._depth, self._width)
        self._prev_clock = 0
        self._last_read_a = [0] * width
        self._last_read_b = [0] * width

    def tick_a(
        self,
        clock: int,
        address: int,
        data_in: list[int],
        write_enable: int,
    ) -> list[int]:
        """Port A operation.

        Parameters:
            clock:        Clock signal (0 or 1)
            address:      Word address (0 to depth-1)
            data_in:      Write data (list of width bits)
            write_enable: 0 = read, 1 = write

        Returns:
            data_out: list of width bits.
        """
        _validate_bit(clock, "clock")

        # Use the dual-port RAM with port B idle (read address 0)
        zeros = [0] * self._width
        out_a, _ = self._ram.tick(
            clock,
            address_a=address,
            data_in_a=data_in,
            write_enable_a=write_enable,
            address_b=0,
            data_in_b=zeros,
            write_enable_b=0,
        )
        return out_a

    def tick_b(
        self,
        clock: int,
        address: int,
        data_in: list[int],
        write_enable: int,
    ) -> list[int]:
        """Port B operation.

        Parameters:
            clock:        Clock signal (0 or 1)
            address:      Word address (0 to depth-1)
            data_in:      Write data (list of width bits)
            write_enable: 0 = read, 1 = write

        Returns:
            data_out: list of width bits.
        """
        _validate_bit(clock, "clock")

        # Use the dual-port RAM with port A idle
        zeros = [0] * self._width
        _, out_b = self._ram.tick(
            clock,
            address_a=0,
            data_in_a=zeros,
            write_enable_a=0,
            address_b=address,
            data_in_b=data_in,
            write_enable_b=write_enable,
        )
        return out_b

    @property
    def depth(self) -> int:
        """Number of addressable words at current configuration."""
        return self._depth

    @property
    def width(self) -> int:
        """Bits per word at current configuration."""
        return self._width

    @property
    def total_bits(self) -> int:
        """Total storage capacity in bits (fixed)."""
        return self._total_bits
