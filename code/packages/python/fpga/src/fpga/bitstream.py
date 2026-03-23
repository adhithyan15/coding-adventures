"""Bitstream — FPGA configuration data.

=== What is a Bitstream? ===

In a real FPGA, a bitstream is a binary blob that programs every
configurable element: LUT truth tables, flip-flop enables, carry chain
enables, routing switch states, I/O pad modes, and Block RAM contents.

The bitstream is loaded at power-up (or during runtime for partial
reconfiguration) and writes to the SRAM cells that control the fabric.

=== Our JSON Configuration ===

Instead of a binary format, we use JSON for readability and education.
The JSON configuration specifies:

1. **CLBs**: Which LUTs get which truth tables, FF enables, carry enables
2. **Routing**: Which switch matrix ports are connected
3. **I/O**: Pin names, modes, and mappings
4. **BRAM**: Width configuration and initial contents

Example JSON::

    {
        "clbs": {
            "clb_0_0": {
                "slice0": {
                    "lut_a": [0,0,0,0,0,0,0,1,0,0,0,0,0,0,0,0],
                    "lut_b": [0,1,1,0,1,0,0,1,1,0,0,1,0,1,1,0],
                    "ff_a": true,
                    "ff_b": false,
                    "carry": false
                },
                "slice1": { ... }
            }
        },
        "routing": {
            "sw_0_0": [
                {"src": "clb_out_a", "dst": "east"}
            ]
        },
        "io": {
            "pin_A0": {"mode": "input"},
            "pin_B0": {"mode": "output"}
        }
    }
"""

from __future__ import annotations

import json
from dataclasses import dataclass, field
from pathlib import Path


@dataclass
class SliceConfig:
    """Configuration for one slice.

    Attributes:
        lut_a:         Truth table for LUT A (2^k entries)
        lut_b:         Truth table for LUT B (2^k entries)
        ff_a_enabled:  Route LUT A through flip-flop
        ff_b_enabled:  Route LUT B through flip-flop
        carry_enabled: Enable carry chain
    """

    lut_a: list[int]
    lut_b: list[int]
    ff_a_enabled: bool = False
    ff_b_enabled: bool = False
    carry_enabled: bool = False


@dataclass
class CLBConfig:
    """Configuration for one CLB (2 slices)."""

    slice0: SliceConfig
    slice1: SliceConfig


@dataclass
class RouteConfig:
    """A single routing connection."""

    source: str
    destination: str


@dataclass
class IOConfig:
    """Configuration for one I/O block."""

    mode: str  # "input", "output", or "tristate"


@dataclass
class Bitstream:
    """FPGA configuration data — the 'program' for the fabric.

    Attributes:
        clbs:    CLB configurations keyed by name (e.g., "clb_0_0")
        routing: Switch matrix connections keyed by matrix name
        io:      I/O block configurations keyed by pin name
        lut_k:   Number of LUT inputs (default 4)
    """

    clbs: dict[str, CLBConfig] = field(default_factory=dict)
    routing: dict[str, list[RouteConfig]] = field(default_factory=dict)
    io: dict[str, IOConfig] = field(default_factory=dict)
    lut_k: int = 4

    @classmethod
    def from_json(cls, path: str | Path) -> Bitstream:
        """Load a bitstream from a JSON file.

        Parameters:
            path: Path to the JSON configuration file.

        Returns:
            A Bitstream instance.
        """
        with Path(path).open() as f:
            data = json.load(f)

        return cls.from_dict(data)

    @classmethod
    def from_dict(cls, data: dict) -> Bitstream:  # type: ignore[type-arg]
        """Create a Bitstream from a dictionary.

        Parameters:
            data: Configuration dictionary (same structure as JSON).

        Returns:
            A Bitstream instance.
        """
        lut_k = data.get("lut_k", 4)

        # Parse CLBs
        clbs: dict[str, CLBConfig] = {}
        for name, clb_data in data.get("clbs", {}).items():
            s0 = clb_data.get("slice0", {})
            s1 = clb_data.get("slice1", {})
            clbs[name] = CLBConfig(
                slice0=SliceConfig(
                    lut_a=s0.get("lut_a", [0] * (1 << lut_k)),
                    lut_b=s0.get("lut_b", [0] * (1 << lut_k)),
                    ff_a_enabled=s0.get("ff_a", False),
                    ff_b_enabled=s0.get("ff_b", False),
                    carry_enabled=s0.get("carry", False),
                ),
                slice1=SliceConfig(
                    lut_a=s1.get("lut_a", [0] * (1 << lut_k)),
                    lut_b=s1.get("lut_b", [0] * (1 << lut_k)),
                    ff_a_enabled=s1.get("ff_a", False),
                    ff_b_enabled=s1.get("ff_b", False),
                    carry_enabled=s1.get("carry", False),
                ),
            )

        # Parse routing
        routing: dict[str, list[RouteConfig]] = {}
        for sw_name, routes in data.get("routing", {}).items():
            routing[sw_name] = [
                RouteConfig(source=r["src"], destination=r["dst"])
                for r in routes
            ]

        # Parse I/O
        io: dict[str, IOConfig] = {}
        for pin_name, io_data in data.get("io", {}).items():
            io[pin_name] = IOConfig(mode=io_data.get("mode", "input"))

        return cls(clbs=clbs, routing=routing, io=io, lut_k=lut_k)
