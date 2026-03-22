"""Switch Matrix — programmable routing crossbar for the FPGA fabric.

=== What is a Switch Matrix? ===

The routing fabric is what makes an FPGA truly programmable. LUTs and
CLBs compute boolean functions, but the switch matrix determines how
those functions connect to each other.

A switch matrix sits at each intersection of the routing grid. It's a
crossbar that can connect any of its input wires to any of its output
wires, based on configuration bits stored in SRAM.

=== Grid Layout ===

In an FPGA, the routing fabric is organized as a grid:

    ┌─────┐     ┌─────┐     ┌─────┐
    │ CLB │──SW──│ CLB │──SW──│ CLB │
    └──┬──┘     └──┬──┘     └──┬──┘
       │SW          │SW          │SW
    ┌──┴──┐     ┌──┴──┐     ┌──┴──┐
    │ CLB │──SW──│ CLB │──SW──│ CLB │
    └─────┘     └─────┘     └─────┘

    SW = Switch Matrix

Each switch matrix connects wire segments from four directions (North,
South, East, West) plus the adjacent CLB's outputs.

=== Connection Model ===

We model the switch matrix as a set of named ports and a configurable
connection map. Each connection maps an input port to an output port.
When a signal arrives at an input port, the switch matrix routes it to
all connected output ports.

This is equivalent to the real hardware: SRAM bits control pass
transistors that connect wire segments through the crossbar.
"""

from __future__ import annotations


class SwitchMatrix:
    """Programmable routing crossbar.

    Connects named signal ports via configurable routes. Each route
    maps a source port to a destination port. Multiple routes can
    share the same source (fan-out) but each destination can only
    have one source (no bus contention).

    Parameters:
        ports: Set of port names (strings)

    Example:
        >>> sm = SwitchMatrix({"north", "south", "east", "west", "clb_out"})
        >>> sm.connect("clb_out", "east")
        >>> sm.connect("north", "south")
        >>> sm.route({"clb_out": 1, "north": 0})
        {'east': 1, 'south': 0}
    """

    def __init__(self, ports: set[str]) -> None:
        if not ports:
            msg = "ports must be non-empty"
            raise ValueError(msg)
        for p in ports:
            if not isinstance(p, str) or not p:
                msg = f"port names must be non-empty strings, got {p!r}"
                raise ValueError(msg)

        self._ports = frozenset(ports)
        # Maps destination → source
        self._connections: dict[str, str] = {}

    def connect(self, source: str, destination: str) -> None:
        """Create a route from source to destination.

        Parameters:
            source:      Name of the input port
            destination: Name of the output port

        Raises:
            ValueError: If ports are unknown or destination already connected.
        """
        if source not in self._ports:
            msg = f"unknown source port: {source!r}"
            raise ValueError(msg)
        if destination not in self._ports:
            msg = f"unknown destination port: {destination!r}"
            raise ValueError(msg)
        if source == destination:
            msg = f"cannot connect port {source!r} to itself"
            raise ValueError(msg)
        if destination in self._connections:
            msg = (
                f"destination {destination!r} already connected "
                f"to {self._connections[destination]!r}"
            )
            raise ValueError(msg)

        self._connections[destination] = source

    def disconnect(self, destination: str) -> None:
        """Remove the route to a destination port.

        Parameters:
            destination: The port to disconnect.

        Raises:
            ValueError: If port is unknown or not connected.
        """
        if destination not in self._ports:
            msg = f"unknown port: {destination!r}"
            raise ValueError(msg)
        if destination not in self._connections:
            msg = f"port {destination!r} is not connected"
            raise ValueError(msg)

        del self._connections[destination]

    def clear(self) -> None:
        """Remove all connections (reset the switch matrix)."""
        self._connections.clear()

    def route(self, inputs: dict[str, int]) -> dict[str, int]:
        """Propagate signals through the switch matrix.

        Parameters:
            inputs: Map of port name → signal value (0 or 1) for
                    ports that have external signals driving them.

        Returns:
            Map of destination port → routed signal value for all
            connected destinations whose source appears in inputs.
        """
        outputs: dict[str, int] = {}
        for dest, src in self._connections.items():
            if src in inputs:
                outputs[dest] = inputs[src]
        return outputs

    @property
    def ports(self) -> frozenset[str]:
        """Set of all port names."""
        return self._ports

    @property
    def connections(self) -> dict[str, str]:
        """Current connection map (destination → source). Returns a copy."""
        return dict(self._connections)

    @property
    def connection_count(self) -> int:
        """Number of active connections."""
        return len(self._connections)
