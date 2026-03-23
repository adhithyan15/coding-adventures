"""FPGA Fabric — the top-level FPGA model.

=== What is an FPGA? ===

An FPGA (Field-Programmable Gate Array) is a chip containing:
- A grid of CLBs (Configurable Logic Blocks) for computation
- A routing fabric (switch matrices) for interconnection
- I/O blocks at the perimeter for external connections
- Block RAM tiles for on-chip memory

The key property: **all of this is programmable**. By loading a
bitstream (configuration data), the same physical chip can become
any digital circuit — a CPU, a signal processor, a network switch,
or anything else that fits within its resources.

=== Our FPGA Model ===

We model a simplified but structurally accurate FPGA:

    ┌────────────────────────────────────────────────────┐
    │                    FPGA Fabric                      │
    │                                                     │
    │  [IO] [IO] [IO] [IO] [IO] [IO] [IO] [IO]          │
    │                                                     │
    │  [IO] [CLB]──[SW]──[CLB]──[SW]──[CLB] [IO]        │
    │         │            │            │                  │
    │        [SW]         [SW]         [SW]               │
    │         │            │            │                  │
    │  [IO] [CLB]──[SW]──[CLB]──[SW]──[CLB] [IO]        │
    │                                                     │
    │  [IO] [IO] [IO] [IO] [IO] [IO] [IO] [IO]          │
    │                                                     │
    │            [BRAM]        [BRAM]                     │
    └────────────────────────────────────────────────────┘

The FPGA class:
1. Creates CLBs, switch matrices, and I/O blocks from a bitstream
2. Configures each element according to the bitstream
3. Provides a simulate() method for stepping through clock cycles
"""

from __future__ import annotations

from dataclasses import dataclass, field

from fpga.bitstream import Bitstream
from fpga.clb import CLB, CLBOutput
from fpga.io_block import IOBlock, IOMode
from fpga.switch_matrix import SwitchMatrix


@dataclass
class SimResult:
    """Result of an FPGA simulation.

    Attributes:
        outputs: Per-cycle output values for each I/O pin.
                 Maps pin_name → list of values (one per cycle).
        cycles:  Number of cycles simulated.
    """

    outputs: dict[str, list[int | None]] = field(default_factory=dict)
    cycles: int = 0


class FPGA:
    """Top-level FPGA fabric model.

    Creates and configures CLBs, switch matrices, and I/O blocks
    from a Bitstream, then simulates the configured circuit.

    Parameters:
        bitstream: Configuration data for the fabric.

    Example — simple AND gate:
        >>> from fpga.bitstream import Bitstream
        >>> config = {
        ...     "clbs": {
        ...         "clb_0": {
        ...             "slice0": {
        ...                 "lut_a": [0,0,0,0,0,0,0,1,0,0,0,0,0,0,0,0],
        ...             }
        ...         }
        ...     },
        ...     "io": {
        ...         "in_a": {"mode": "input"},
        ...         "in_b": {"mode": "input"},
        ...         "out":  {"mode": "output"},
        ...     }
        ... }
        >>> bs = Bitstream.from_dict(config)
        >>> fpga = FPGA(bs)
    """

    def __init__(self, bitstream: Bitstream) -> None:
        self._bitstream = bitstream
        self._clbs: dict[str, CLB] = {}
        self._switches: dict[str, SwitchMatrix] = {}
        self._ios: dict[str, IOBlock] = {}

        self._configure(bitstream)

    def _configure(self, bs: Bitstream) -> None:
        """Apply bitstream configuration to create and program all elements."""
        # Create and configure CLBs
        for name, clb_cfg in bs.clbs.items():
            clb = CLB(lut_inputs=bs.lut_k)

            clb.slice0.configure(
                lut_a_table=clb_cfg.slice0.lut_a,
                lut_b_table=clb_cfg.slice0.lut_b,
                ff_a_enabled=clb_cfg.slice0.ff_a_enabled,
                ff_b_enabled=clb_cfg.slice0.ff_b_enabled,
                carry_enabled=clb_cfg.slice0.carry_enabled,
            )
            clb.slice1.configure(
                lut_a_table=clb_cfg.slice1.lut_a,
                lut_b_table=clb_cfg.slice1.lut_b,
                ff_a_enabled=clb_cfg.slice1.ff_a_enabled,
                ff_b_enabled=clb_cfg.slice1.ff_b_enabled,
                carry_enabled=clb_cfg.slice1.carry_enabled,
            )

            self._clbs[name] = clb

        # Create and configure switch matrices
        for sw_name, routes in bs.routing.items():
            # Collect all port names referenced in routes
            ports: set[str] = set()
            for route in routes:
                ports.add(route.source)
                ports.add(route.destination)

            if ports:
                sm = SwitchMatrix(ports)
                for route in routes:
                    sm.connect(route.source, route.destination)
                self._switches[sw_name] = sm

        # Create I/O blocks
        mode_map = {
            "input": IOMode.INPUT,
            "output": IOMode.OUTPUT,
            "tristate": IOMode.TRISTATE,
        }
        for pin_name, io_cfg in bs.io.items():
            mode = mode_map.get(io_cfg.mode, IOMode.INPUT)
            self._ios[pin_name] = IOBlock(pin_name, mode=mode)

    def evaluate_clb(
        self,
        clb_name: str,
        slice0_inputs_a: list[int],
        slice0_inputs_b: list[int],
        slice1_inputs_a: list[int],
        slice1_inputs_b: list[int],
        clock: int,
        carry_in: int = 0,
    ) -> CLBOutput:
        """Evaluate a specific CLB.

        Parameters:
            clb_name: Name of the CLB to evaluate.
            slice0_inputs_a/b: Inputs for slice 0's LUTs.
            slice1_inputs_a/b: Inputs for slice 1's LUTs.
            clock: Clock signal (0 or 1).
            carry_in: External carry input.

        Returns:
            CLBOutput from the evaluated CLB.

        Raises:
            KeyError: If clb_name not found.
        """
        if clb_name not in self._clbs:
            msg = f"CLB {clb_name!r} not found"
            raise KeyError(msg)

        return self._clbs[clb_name].evaluate(
            slice0_inputs_a,
            slice0_inputs_b,
            slice1_inputs_a,
            slice1_inputs_b,
            clock,
            carry_in,
        )

    def route(
        self, switch_name: str, signals: dict[str, int]
    ) -> dict[str, int]:
        """Route signals through a switch matrix.

        Parameters:
            switch_name: Name of the switch matrix.
            signals: Input signals (port_name → value).

        Returns:
            Routed output signals.

        Raises:
            KeyError: If switch_name not found.
        """
        if switch_name not in self._switches:
            msg = f"Switch matrix {switch_name!r} not found"
            raise KeyError(msg)

        return self._switches[switch_name].route(signals)

    def set_input(self, pin_name: str, value: int) -> None:
        """Drive an input pin.

        Parameters:
            pin_name: Name of the I/O pin.
            value: Signal value (0 or 1).

        Raises:
            KeyError: If pin_name not found.
        """
        if pin_name not in self._ios:
            msg = f"I/O pin {pin_name!r} not found"
            raise KeyError(msg)
        self._ios[pin_name].drive_pad(value)

    def read_output(self, pin_name: str) -> int | None:
        """Read an output pin.

        Parameters:
            pin_name: Name of the I/O pin.

        Returns:
            Signal value (0, 1, or None for tri-state).

        Raises:
            KeyError: If pin_name not found.
        """
        if pin_name not in self._ios:
            msg = f"I/O pin {pin_name!r} not found"
            raise KeyError(msg)
        return self._ios[pin_name].read_pad()

    def drive_output(self, pin_name: str, value: int) -> None:
        """Drive the internal side of an output pin (fabric → external).

        Parameters:
            pin_name: Name of the I/O pin.
            value: Signal value (0 or 1).

        Raises:
            KeyError: If pin_name not found.
        """
        if pin_name not in self._ios:
            msg = f"I/O pin {pin_name!r} not found"
            raise KeyError(msg)
        self._ios[pin_name].drive_internal(value)

    @property
    def clbs(self) -> dict[str, CLB]:
        """All CLBs in the fabric."""
        return dict(self._clbs)

    @property
    def switches(self) -> dict[str, SwitchMatrix]:
        """All switch matrices in the fabric."""
        return dict(self._switches)

    @property
    def ios(self) -> dict[str, IOBlock]:
        """All I/O blocks."""
        return dict(self._ios)

    @property
    def bitstream(self) -> Bitstream:
        """The loaded bitstream configuration."""
        return self._bitstream
