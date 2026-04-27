"""LVS: layout-vs-schematic netlist comparison.

Two flat netlists, each a list of (cell_type, dict-of-pin-to-net) entries.
We check graph isomorphism via partition refinement (signature-based)."""

from __future__ import annotations

from collections import Counter
from dataclasses import dataclass, field


@dataclass(frozen=True, slots=True)
class LvsCell:
    """One cell instance in a netlist."""

    name: str
    cell_type: str
    pins: tuple[tuple[str, str], ...]  # (pin_name, net_name) pairs


@dataclass
class LvsNetlist:
    cells: list[LvsCell]


@dataclass
class LvsReport:
    matched: bool
    layout_cells: int
    schematic_cells: int
    mismatches: list[str] = field(default_factory=list)


def lvs(layout: LvsNetlist, schematic: LvsNetlist) -> LvsReport:
    """Compare two flat netlists.

    Strategy: bag-of-cell-signatures comparison. Each cell's signature is
    (cell_type, frozenset of (pin_name, net-equivalence-class)). If both
    netlists produce identical multisets of signatures, they match.

    This is approximate (won't catch all subtle topological differences) but
    covers the common cases.
    """
    report = LvsReport(
        matched=False,
        layout_cells=len(layout.cells),
        schematic_cells=len(schematic.cells),
    )

    # Quick fail: cell counts differ.
    if len(layout.cells) != len(schematic.cells):
        report.mismatches.append(
            f"cell counts differ: layout={len(layout.cells)} "
            f"vs schematic={len(schematic.cells)}"
        )
        return report

    # Compute net equivalence classes by counting how many cells touch each net.
    # Two nets in different netlists are 'equivalent' if they have the same
    # connectivity profile.
    layout_nets = _net_signatures(layout)
    schem_nets = _net_signatures(schematic)
    if Counter(layout_nets.values()) != Counter(schem_nets.values()):
        report.mismatches.append("net connectivity profiles differ")
        return report

    # For each cell, replace pin's net with the net's equivalence class.
    layout_signatures = _cell_signatures(layout, layout_nets)
    schem_signatures = _cell_signatures(schematic, schem_nets)

    if Counter(layout_signatures) != Counter(schem_signatures):
        report.mismatches.append(
            "cell signatures differ between layout and schematic"
        )
        layout_only = Counter(layout_signatures) - Counter(schem_signatures)
        schem_only = Counter(schem_signatures) - Counter(layout_signatures)
        if layout_only:
            report.mismatches.append(
                f"in layout only: {dict(layout_only.most_common(5))}"
            )
        if schem_only:
            report.mismatches.append(
                f"in schematic only: {dict(schem_only.most_common(5))}"
            )
        return report

    report.matched = True
    return report


def _net_signatures(nl: LvsNetlist) -> dict[str, str]:
    """For each net, compute a signature based on what cell types it connects
    and via which pins. Returns net_name -> signature."""
    net_to_pins: dict[str, list[tuple[str, str]]] = {}
    for cell in nl.cells:
        for pin_name, net_name in cell.pins:
            net_to_pins.setdefault(net_name, []).append((cell.cell_type, pin_name))

    return {
        net: " | ".join(sorted(f"{ct}.{pn}" for ct, pn in pins))
        for net, pins in net_to_pins.items()
    }


def _cell_signatures(nl: LvsNetlist, net_sigs: dict[str, str]) -> list[str]:
    """Per-cell signature: cell_type + sorted(pin -> net_signature)."""
    sigs: list[str] = []
    for cell in nl.cells:
        pin_sigs = sorted(
            f"{pn}={net_sigs.get(net, '?')}" for pn, net in cell.pins
        )
        sigs.append(f"{cell.cell_type}({','.join(pin_sigs)})")
    return sigs
