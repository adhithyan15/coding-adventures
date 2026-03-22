"""Look-Up Table (LUT) — the atom of programmable logic.

=== What is a LUT? ===

A Look-Up Table is the fundamental building block of every FPGA. The key
insight behind programmable logic is deceptively simple:

    **A truth table IS a program.**

Any boolean function of K inputs can be described by a truth table with
2^K entries. A K-input LUT stores that truth table in SRAM and uses a
MUX tree to select the correct output for any combination of inputs.

This means a single LUT can implement ANY boolean function of K variables:
AND, OR, XOR, majority vote, parity — anything. To "reprogram" the LUT,
you just load a different truth table into the SRAM.

=== How it works ===

A 4-input LUT (K=4) has:
- 16 SRAM cells (2^4 = 16 truth table entries)
- A 16-to-1 MUX tree (built from 2:1 MUXes)
- 4 input signals that act as MUX select lines

Example — configuring a LUT as a 2-input AND gate (using only I0, I1):

    Inputs → Truth Table Entry → Output
    I3 I2 I1 I0
     0  0  0  0  → SRAM[0]  = 0
     0  0  0  1  → SRAM[1]  = 0
     0  0  1  0  → SRAM[2]  = 0
     0  0  1  1  → SRAM[3]  = 1  ← only case where I0 AND I1 = 1
     0  1  0  0  → SRAM[4]  = 0
     ...           (all others = 0 since we only care about I0, I1)

The truth table index is computed as:
    index = I0 + 2*I1 + 4*I2 + 8*I3  (binary number with I0 as LSB)

Then the MUX tree selects SRAM[index] as the output.

=== MUX Tree Structure ===

For a 4-input LUT, the MUX tree has 4 levels:

    SRAM[0] ─┐
              ├─ MUX(sel=I0) ─┐
    SRAM[1] ─┘                │
                               ├─ MUX(sel=I1) ─┐
    SRAM[2] ─┐                │                │
              ├─ MUX(sel=I0) ─┘                │
    SRAM[3] ─┘                                  ├─ MUX(sel=I2) ─┐
                                                │                │
    SRAM[4] ─┐                                  │                │
              ├─ MUX(sel=I0) ─┐                │                │
    SRAM[5] ─┘                │                │                │
                               ├─ MUX(sel=I1) ─┘                │
    SRAM[6] ─┐                │                                  │
              ├─ MUX(sel=I0) ─┘                                  ├─ Out
    SRAM[7] ─┘                                                   │
                                                                  │
    SRAM[8..15] ──── (same structure) ──── MUX(sel=I2) ─────────┘
                                                  (sel=I3)

This is exactly what mux_n from logic-gates does: it recursively builds
a 2^K-to-1 MUX tree from 2:1 MUXes, using the select bits to choose
one of the 2^K inputs.
"""

from __future__ import annotations

from block_ram.sram import SRAMCell, _validate_bit
from logic_gates.combinational import mux_n


class LUT:
    """K-input Look-Up Table — the atom of programmable logic.

    A LUT stores a truth table in SRAM cells and uses a MUX tree to
    select the output based on input signals. It can implement ANY
    boolean function of K variables.

    Parameters:
        k:           Number of inputs (2 to 6, default 4)
        truth_table: Initial truth table (2^k entries, each 0 or 1).
                     If None, all entries default to 0.

    Example — 2-input AND gate in a 4-input LUT:
        >>> # Truth table: output 1 only when I0=1 AND I1=1 (index 3)
        >>> and_table = [0]*16
        >>> and_table[3] = 1  # I0=1, I1=1 → index = 1 + 2 = 3
        >>> lut = LUT(k=4, truth_table=and_table)
        >>> lut.evaluate([0, 0, 0, 0])
        0
        >>> lut.evaluate([1, 1, 0, 0])  # I0=1, I1=1
        1

    Example — 2-input XOR gate:
        >>> xor_table = [0]*16
        >>> xor_table[1] = 1   # I0=1, I1=0
        >>> xor_table[2] = 1   # I0=0, I1=1
        >>> lut = LUT(k=4, truth_table=xor_table)
        >>> lut.evaluate([1, 0, 0, 0])
        1
        >>> lut.evaluate([1, 1, 0, 0])
        0
    """

    def __init__(
        self,
        k: int = 4,
        truth_table: list[int] | None = None,
    ) -> None:
        if not isinstance(k, int) or isinstance(k, bool):
            msg = f"k must be an int, got {type(k).__name__}"
            raise TypeError(msg)
        if k < 2 or k > 6:
            msg = f"k must be between 2 and 6, got {k}"
            raise ValueError(msg)

        self._k = k
        self._size = 1 << k  # 2^k

        # SRAM cells storing the truth table
        self._sram: list[SRAMCell] = [SRAMCell() for _ in range(self._size)]

        if truth_table is not None:
            self.configure(truth_table)

    def configure(self, truth_table: list[int]) -> None:
        """Load a new truth table (reprogram the LUT).

        Parameters:
            truth_table: List of 2^k bits (each 0 or 1).

        Raises:
            ValueError: If length doesn't match 2^k or entries aren't 0/1.
            TypeError: If truth_table is not a list.
        """
        if not isinstance(truth_table, list):
            msg = "truth_table must be a list of bits"
            raise TypeError(msg)
        if len(truth_table) != self._size:
            msg = (
                f"truth_table length {len(truth_table)} "
                f"does not match 2^k = {self._size}"
            )
            raise ValueError(msg)

        for i, bit in enumerate(truth_table):
            _validate_bit(bit, f"truth_table[{i}]")

        # Program each SRAM cell
        for i, bit in enumerate(truth_table):
            self._sram[i].write(word_line=1, bit_line=bit)

    def evaluate(self, inputs: list[int]) -> int:
        """Compute the LUT output for the given inputs.

        Uses a MUX tree (via mux_n) to select the correct truth table
        entry based on the input signals.

        Parameters:
            inputs: List of k input bits (each 0 or 1).
                    inputs[0] = I0 (LSB of truth table index)
                    inputs[k-1] = I_{k-1} (MSB of truth table index)

        Returns:
            The truth table output (0 or 1).

        Raises:
            ValueError: If inputs length != k or entries aren't 0/1.
        """
        if not isinstance(inputs, list):
            msg = "inputs must be a list of bits"
            raise TypeError(msg)
        if len(inputs) != self._k:
            msg = (
                f"inputs length {len(inputs)} does not match k = {self._k}"
            )
            raise ValueError(msg)

        for i, bit in enumerate(inputs):
            _validate_bit(bit, f"inputs[{i}]")

        # Read all SRAM cells to form the MUX data inputs
        data = []
        for cell in self._sram:
            val = cell.read(word_line=1)
            assert val is not None
            data.append(val)

        # Use MUX tree to select the output
        # mux_n(data, select) where select bits map inputs to MUX levels
        return mux_n(data, inputs)

    @property
    def k(self) -> int:
        """Number of inputs."""
        return self._k

    @property
    def truth_table(self) -> list[int]:
        """Current truth table (copy).

        Returns:
            List of 2^k bits representing the programmed truth table.
        """
        result = []
        for cell in self._sram:
            val = cell.read(word_line=1)
            assert val is not None
            result.append(val)
        return result
