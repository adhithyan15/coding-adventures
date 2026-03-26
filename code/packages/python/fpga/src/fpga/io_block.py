"""I/O Block — bidirectional pad connecting FPGA internals to the outside world.

=== What is an I/O Block? ===

I/O blocks sit at the perimeter of the FPGA and provide the interface
between the internal logic fabric and the external pins of the chip.

Each I/O block can be configured in three modes:
- **Input**: External signal enters the FPGA (pad → internal)
- **Output**: Internal signal exits the FPGA (internal → pad)
- **Tri-state**: Output is high-impedance (disconnected) when not enabled

=== I/O Block Architecture ===

    External Pin (pad)
         │
         ▼
    ┌──────────────────┐
    │    I/O Block      │
    │                   │
    │  ┌─────────────┐  │
    │  │ Input Reg   │  │ ── (optional) register the input
    │  └──────┬──────┘  │
    │         │         │
    │  ┌──────▼──────┐  │
    │  │ Tri-State   │  │ ── output enable controls direction
    │  │ Buffer      │  │
    │  └──────┬──────┘  │
    │         │         │
    │  ┌──────▼──────┐  │
    │  │ Output Reg  │  │ ── (optional) register the output
    │  └─────────────┘  │
    │                   │
    └──────────────────┘
         │
         ▼
    To/From Internal Fabric

The tri-state buffer is the key component: it uses the tri_state function
from logic-gates to produce 0, 1, or None (high-impedance).
"""

from __future__ import annotations

from enum import Enum

from logic_gates.combinational import tri_state


class IOMode(Enum):
    """I/O block operating mode.

    INPUT:     Pad drives internal signal (external → fabric)
    OUTPUT:    Fabric drives pad (fabric → external)
    TRISTATE:  Output is high-impedance (pad is disconnected)
    """

    INPUT = "input"
    OUTPUT = "output"
    TRISTATE = "tristate"


class IOBlock:
    """Bidirectional I/O pad for the FPGA perimeter.

    Each I/O block connects one external pin to the internal fabric.
    The mode determines the direction of data flow.

    Parameters:
        name: Identifier for this I/O block (e.g., "pin_A0", "led_0")
        mode: Initial operating mode (default: INPUT)

    Example — input pin:
        >>> io = IOBlock("sensor_in", mode=IOMode.INPUT)
        >>> io.drive_pad(1)       # External signal arrives
        >>> io.read_internal()     # Fabric sees the signal
        1

    Example — output pin:
        >>> io = IOBlock("led_0", mode=IOMode.OUTPUT)
        >>> io.drive_internal(1)  # Fabric sends signal
        >>> io.read_pad()         # External pin shows the signal
        1

    Example — tri-state (disconnected):
        >>> io = IOBlock("bus_0", mode=IOMode.TRISTATE)
        >>> io.drive_internal(1)
        >>> io.read_pad()         # High impedance
    """

    def __init__(
        self,
        name: str,
        mode: IOMode = IOMode.INPUT,
    ) -> None:
        if not isinstance(name, str) or not name:
            msg = "name must be a non-empty string"
            raise ValueError(msg)

        self._name = name
        self._mode = mode
        self._pad_value: int = 0       # Signal on the external pad
        self._internal_value: int = 0  # Signal on the fabric side

    def configure(self, mode: IOMode) -> None:
        """Change the I/O block's operating mode.

        Parameters:
            mode: New operating mode.
        """
        if not isinstance(mode, IOMode):
            msg = f"mode must be an IOMode, got {type(mode).__name__}"
            raise TypeError(msg)
        self._mode = mode

    def drive_pad(self, value: int) -> None:
        """Drive the external pad with a signal (used in INPUT mode).

        Parameters:
            value: Signal value (0 or 1)
        """
        if value not in (0, 1):
            msg = f"value must be 0 or 1, got {value}"
            raise ValueError(msg)
        self._pad_value = value

    def drive_internal(self, value: int) -> None:
        """Drive the internal (fabric) side with a signal (used in OUTPUT mode).

        Parameters:
            value: Signal value (0 or 1)
        """
        if value not in (0, 1):
            msg = f"value must be 0 or 1, got {value}"
            raise ValueError(msg)
        self._internal_value = value

    def read_internal(self) -> int | None:
        """Read the signal visible to the internal fabric.

        In INPUT mode, returns the pad value (external → fabric).
        In OUTPUT/TRISTATE mode, returns the internally driven value.

        Returns:
            Signal value (0 or 1).
        """
        if self._mode == IOMode.INPUT:
            return self._pad_value
        return self._internal_value

    def read_pad(self) -> int | None:
        """Read the signal visible on the external pad.

        In INPUT mode, returns the pad value.
        In OUTPUT mode, returns the internally driven value.
        In TRISTATE mode, returns None (high impedance).

        Returns:
            0 or 1 in INPUT/OUTPUT mode, None in TRISTATE mode.
        """
        if self._mode == IOMode.INPUT:
            return self._pad_value
        if self._mode == IOMode.TRISTATE:
            return tri_state(self._internal_value, enable=0)
        # OUTPUT mode: tri-state with enable=1
        return tri_state(self._internal_value, enable=1)

    @property
    def name(self) -> str:
        """I/O block identifier."""
        return self._name

    @property
    def mode(self) -> IOMode:
        """Current operating mode."""
        return self._mode
