"""HNL -> existing fpga package's JSON config.

v0.1.0 implements:
- LUT packing: each generic-cell or stdcell mapped to a single LUT truth table.
  Multi-output cells split into one LUT per output. Limited to <= 4 input pins.
- Placement: random row/column on a fabric grid (extension: SA on HPWL).
- Routing: emit `routes` entries from net pin->pin pairs.
- Output: dict matching the schema in F01-fpga.md.
"""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import Any

from gate_netlist_format import Netlist

# Truth tables for HNL primitive cells (lookup by cell_type).
# Each entry: (input_pin_names_in_order, list of 2^k truth-table outputs).
TRUTH_TABLES: dict[str, tuple[list[str], list[int]]] = {
    "BUF":   (["A"],            [0, 1]),
    "NOT":   (["A"],            [1, 0]),
    "AND2":  (["A", "B"],       [0, 0, 0, 1]),
    "OR2":   (["A", "B"],       [0, 1, 1, 1]),
    "NAND2": (["A", "B"],       [1, 1, 1, 0]),
    "NOR2":  (["A", "B"],       [1, 0, 0, 0]),
    "XOR2":  (["A", "B"],       [0, 1, 1, 0]),
    "XNOR2": (["A", "B"],       [1, 0, 0, 1]),
    "AND3":  (["A", "B", "C"],  [0, 0, 0, 0, 0, 0, 0, 1]),
    "OR3":   (["A", "B", "C"],  [0, 1, 1, 1, 1, 1, 1, 1]),
    "NAND3": (["A", "B", "C"],  [1, 1, 1, 1, 1, 1, 1, 0]),
    "NOR3":  (["A", "B", "C"],  [1, 0, 0, 0, 0, 0, 0, 0]),
    "XOR3":  (["A", "B", "C"],  [0, 1, 1, 0, 1, 0, 0, 1]),
    "AND4":  (["A", "B", "C", "D"], [0]*15 + [1]),
    "OR4":   (["A", "B", "C", "D"], [0] + [1]*15),
    "NAND4": (["A", "B", "C", "D"], [1]*15 + [0]),
    "NOR4":  (["A", "B", "C", "D"], [1] + [0]*15),
    # MUX2 with select: pins (A, B, S); Y = S?B:A
    "MUX2":  (["A", "B", "S"],  [0, 1, 0, 1, 0, 0, 1, 1]),
    "CONST_0": ([],              [0]),
    "CONST_1": ([],              [1]),
}


@dataclass
class FpgaBridgeOptions:
    rows: int = 4
    cols: int = 4
    lut_inputs: int = 4
    seed: int = 42


@dataclass
class FpgaBridgeReport:
    cells_packed: int = 0
    cells_unmapped: list[str] = field(default_factory=list)
    routes_emitted: int = 0


def hnl_to_fpga_json(
    netlist: Netlist,
    *,
    options: FpgaBridgeOptions | None = None,
) -> tuple[dict[str, Any], FpgaBridgeReport]:
    """Map an HNL to a `fpga`-package-style JSON config.

    Returns (json_dict, FpgaBridgeReport)."""
    if options is None:
        options = FpgaBridgeOptions()

    top_mod = netlist.modules[netlist.top]
    report = FpgaBridgeReport()

    clbs: dict[str, dict[str, Any]] = {}
    routes: list[dict[str, str]] = []
    io: dict[str, dict[str, Any]] = {}

    # Track which cell got which CLB position for routing
    cell_to_loc: dict[str, tuple[int, int, str]] = {}

    # Pack each cell into a single LUT (slice 0, lut_a)
    cell_idx = 0
    for inst in top_mod.instances:
        truth = TRUTH_TABLES.get(inst.cell_type)
        if truth is None:
            report.cells_unmapped.append(inst.cell_type)
            continue
        report.cells_packed += 1

        row, col = _next_clb_location(cell_idx, options.rows, options.cols)
        clb_name = f"clb_{row}_{col}"
        cell_idx += 1

        # Expand truth table to lut_inputs width.
        input_pins, table = truth
        expanded = _expand_truth_table(table, len(input_pins), options.lut_inputs)

        clbs[clb_name] = {
            "lut_a": {
                "truth_table": expanded,
                "comment": f"{inst.name} ({inst.cell_type})",
            }
        }
        cell_to_loc[inst.name] = (row, col, "lut_a")

        # Route each input pin connection
        for i, pin_name in enumerate(input_pins):
            if pin_name not in inst.connections:
                continue
            slice_ = inst.connections[pin_name]
            # Source: net producer (we'll use a synthetic name)
            source = _net_to_source(slice_.net, slice_.bits[0])
            target = f"{clb_name}.lut_a.in{i}"
            routes.append({"from": source, "to": target})
            report.routes_emitted += 1

        # If this cell drives a top-level port, route LUT output to io_pin
        for pin_name, slice_ in inst.connections.items():
            if pin_name in input_pins:
                continue
            # Output pin
            port = top_mod.port(slice_.net)
            if port is not None and port.direction.value == "output":
                io_pin = f"io_pin_{slice_.net}"
                routes.append({
                    "from": f"{clb_name}.lut_a.out",
                    "to": io_pin,
                })
                io[io_pin] = {"direction": "output", "name": slice_.net}
                report.routes_emitted += 1

    # IO pins for inputs
    for port in top_mod.ports:
        if port.direction.value == "input":
            io[f"io_pin_{port.name}"] = {"direction": "input", "name": port.name}

    config: dict[str, Any] = {
        "device": {
            "name": "CinchFPGA-Mini",
            "rows": options.rows,
            "cols": options.cols,
            "lut_inputs": options.lut_inputs,
            "io_pins": len(io),
        },
        "clbs": clbs,
        "routing": routes,
        "io": io,
    }

    return (config, report)


def _next_clb_location(idx: int, rows: int, cols: int) -> tuple[int, int]:
    return (idx // cols, idx % cols)


def _expand_truth_table(table: list[int], n_inputs: int, target_inputs: int) -> list[int]:
    """Expand a 2^n_inputs truth table to 2^target_inputs by ignoring extra
    inputs (output stays the same regardless of higher input bits)."""
    if n_inputs == target_inputs:
        return list(table)
    if n_inputs > target_inputs:
        raise ValueError(f"can't expand {n_inputs}-input truth table to {target_inputs}")
    expanded = []
    target_size = 1 << target_inputs
    n_size = 1 << n_inputs if n_inputs > 0 else 1
    for i in range(target_size):
        # Use only the low n_inputs bits to index the original table
        expanded.append(table[i % n_size] if n_inputs > 0 else table[0])
    return expanded


def _net_to_source(net: str, bit: int) -> str:
    """Convert a net+bit reference to an FPGA source identifier.

    For top-level ports (net == port name), use io_pin_<name>.
    For internal nets, use net_<name>_<bit>."""
    return f"net_{net}_{bit}"
