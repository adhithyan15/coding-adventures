"""SRAM — Static Random-Access Memory at the gate level.

=== What is SRAM? ===

SRAM (Static Random-Access Memory) is the fastest type of memory in a
computer. It's used for CPU caches (L1/L2/L3), register files, and FPGA
Block RAM. "Static" means the memory holds its value as long as power is
supplied — unlike DRAM, which must be periodically refreshed.

=== The SRAM Cell — 6 Transistors Holding 1 Bit ===

In real hardware, each SRAM cell uses 6 transistors:
- 2 cross-coupled inverters forming a bistable latch (stores the bit)
- 2 access transistors controlled by the word line (gates read/write)

We model this at the gate level:
- Cross-coupled inverters = two NOT gates in a feedback loop
  (identical to the logic behind an SR latch from logic_gates.sequential)
- Access transistors = AND gates that pass data only when word_line=1

The cell has three operations:
- **Hold** (word_line=0): Access transistors block external access.
  The inverter loop maintains the stored value indefinitely.
- **Read** (word_line=1): Access transistors open. The stored value
  appears on the bit lines without disturbing it.
- **Write** (word_line=1 + drive bit lines): The external driver
  overpowers the internal inverters, forcing a new value.

=== From Cell to Array ===

A RAM chip is a 2D grid of SRAM cells. To access a specific cell:
1. A **row decoder** converts address bits into a one-hot word line signal
2. A **column MUX** selects which columns to read/write

This module provides:
- SRAMCell: single-bit storage at the gate level
- SRAMArray: 2D grid with row/column addressing
"""

from __future__ import annotations


class SRAMCell:
    """Single-bit storage element modeled at the gate level.

    Internally, this is a pair of cross-coupled inverters (forming a
    bistable latch) gated by access transistors controlled by the word line.

    In our simulation, we model the steady-state behavior directly rather
    than simulating individual gate delays:
    - word_line=0: cell is isolated, value is retained
    - word_line=1, reading: value is output
    - word_line=1, writing: new value overwrites stored value

    This matches the real behavior of a 6T SRAM cell while keeping the
    simulation fast enough to model arrays of thousands of cells.

    Example:
        >>> cell = SRAMCell()
        >>> cell.value
        0
        >>> cell.write(word_line=1, bit_line=1)
        >>> cell.value
        1
        >>> cell.read(word_line=1)
        1
        >>> cell.read(word_line=0)  # Not selected
    """

    def __init__(self) -> None:
        """Create an SRAM cell initialized to 0.

        The initial state of 0 represents the cell after power-on reset.
        In real hardware, SRAM cells power up in an indeterminate state,
        but we initialize to 0 for predictability in simulation.
        """
        self._value: int = 0

    def read(self, word_line: int) -> int | None:
        """Read the stored bit if the cell is selected.

        Parameters:
            word_line: 1 = cell selected (access transistors open),
                       0 = cell not selected (isolated)

        Returns:
            The stored bit (0 or 1) when word_line=1,
            None when word_line=0 (cell not selected, no output).

        Example:
            >>> cell = SRAMCell()
            >>> cell.write(1, 1)  # Store a 1
            >>> cell.read(1)      # Selected: returns stored value
            1
            >>> cell.read(0)      # Not selected: returns None
        """
        _validate_bit(word_line, "word_line")

        if word_line == 0:
            return None

        return self._value

    def write(self, word_line: int, bit_line: int) -> None:
        """Write a bit to the cell if selected.

        When word_line=1, the access transistors open and the external
        bit_line driver overpowers the internal inverter loop, forcing
        the cell to store the new value.

        When word_line=0, the access transistors are closed and the
        write has no effect — the cell retains its previous value.

        Parameters:
            word_line: 1 = cell selected, 0 = cell not selected
            bit_line:  The value to store (0 or 1)

        Example:
            >>> cell = SRAMCell()
            >>> cell.write(1, 1)   # Selected: stores 1
            >>> cell.value
            1
            >>> cell.write(0, 0)   # Not selected: no change
            >>> cell.value
            1
        """
        _validate_bit(word_line, "word_line")
        _validate_bit(bit_line, "bit_line")

        if word_line == 1:
            self._value = bit_line

    @property
    def value(self) -> int:
        """Current stored value (for inspection/debugging).

        Returns:
            The stored bit (0 or 1).
        """
        return self._value


class SRAMArray:
    """2D grid of SRAM cells with row/column addressing.

    An SRAM array organizes cells into rows and columns:
    - Each row shares a word line (activated by the row decoder)
    - Each column shares a bit line (carries data in/out)

    To read: activate a row's word line → all cells in that row
    output their values onto their respective bit lines.

    To write: activate a row's word line and drive the bit lines
    with the desired data → all cells in that row store the new values.

    Memory map (4×4 array example)::

        Row 0 (WL0): [Cell00] [Cell01] [Cell02] [Cell03]
        Row 1 (WL1): [Cell10] [Cell11] [Cell12] [Cell13]
        Row 2 (WL2): [Cell20] [Cell21] [Cell22] [Cell23]
        Row 3 (WL3): [Cell30] [Cell31] [Cell32] [Cell33]

    Parameters:
        rows: Number of rows (must be >= 1)
        cols: Number of columns (must be >= 1)

    Example:
        >>> arr = SRAMArray(4, 8)    # 4 rows × 8 columns
        >>> arr.write(0, [1,0,1,0, 0,1,0,1])
        >>> arr.read(0)
        [1, 0, 1, 0, 0, 1, 0, 1]
        >>> arr.read(1)              # Row 1 never written
        [0, 0, 0, 0, 0, 0, 0, 0]
    """

    def __init__(self, rows: int, cols: int) -> None:
        """Create an SRAM array initialized to all zeros.

        Parameters:
            rows: Number of rows (>= 1)
            cols: Number of columns (>= 1)

        Raises:
            ValueError: If rows or cols < 1
        """
        if rows < 1:
            msg = f"rows must be >= 1, got {rows}"
            raise ValueError(msg)
        if cols < 1:
            msg = f"cols must be >= 1, got {cols}"
            raise ValueError(msg)

        self._rows = rows
        self._cols = cols
        self._cells: list[list[SRAMCell]] = [
            [SRAMCell() for _ in range(cols)] for _ in range(rows)
        ]

    def read(self, row: int) -> list[int]:
        """Read all columns of a row.

        Activates the word line for the given row, causing all cells
        in that row to output their stored values.

        Parameters:
            row: Row index (0 to rows-1)

        Returns:
            List of bits, one per column.

        Raises:
            ValueError: If row is out of range.

        Example:
            >>> arr = SRAMArray(2, 4)
            >>> arr.write(0, [1, 1, 0, 0])
            >>> arr.read(0)
            [1, 1, 0, 0]
        """
        self._validate_row(row)

        # Activate word line for this row — read all cells
        result: list[int] = []
        for cell in self._cells[row]:
            val = cell.read(word_line=1)
            # word_line=1 always returns int, not None
            assert val is not None
            result.append(val)
        return result

    def write(self, row: int, data: list[int]) -> None:
        """Write data to a row.

        Activates the word line for the given row and drives the bit
        lines with the given data, storing values in all cells of the row.

        Parameters:
            row:  Row index (0 to rows-1)
            data: List of bits to write, one per column.
                  Length must equal the number of columns.

        Raises:
            ValueError: If row is out of range or data length doesn't match cols.

        Example:
            >>> arr = SRAMArray(2, 4)
            >>> arr.write(1, [0, 1, 0, 1])
            >>> arr.read(1)
            [0, 1, 0, 1]
        """
        self._validate_row(row)

        if not isinstance(data, list):
            msg = "data must be a list of bits"
            raise TypeError(msg)

        if len(data) != self._cols:
            msg = f"data length {len(data)} does not match cols {self._cols}"
            raise ValueError(msg)

        for i, bit in enumerate(data):
            _validate_bit(bit, f"data[{i}]")

        # Activate word line and drive bit lines
        for col, bit in enumerate(data):
            self._cells[row][col].write(word_line=1, bit_line=bit)

    @property
    def shape(self) -> tuple[int, int]:
        """Array dimensions as (rows, cols)."""
        return (self._rows, self._cols)

    def _validate_row(self, row: int) -> None:
        """Check that row index is in range."""
        if not isinstance(row, int) or isinstance(row, bool):
            msg = f"row must be an int, got {type(row).__name__}"
            raise TypeError(msg)
        if row < 0 or row >= self._rows:
            msg = f"row {row} out of range [0, {self._rows - 1}]"
            raise ValueError(msg)


def _validate_bit(value: int, name: str = "input") -> None:
    """Ensure a value is a binary bit: the integer 0 or 1.

    Same validation as logic_gates.gates._validate_bit, duplicated here
    to avoid a hard import dependency on logic-gates internals.
    """
    if not isinstance(value, int) or isinstance(value, bool):
        msg = f"{name} must be an int, got {type(value).__name__}"
        raise TypeError(msg)
    if value not in (0, 1):
        msg = f"{name} must be 0 or 1, got {value}"
        raise ValueError(msg)
