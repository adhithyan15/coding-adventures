"""Technology mapping: generic HNL -> standard-cell HNL.

v0.1.0 implements a rule-based mapper:
- Cell-type rename via a fixed mapping table (default: Sky130-style stdcells).
- Bubble-pushing: replace AND2 with NAND2+INV, eliminate INV-INV pairs.
- AOI/OAI folding: deferred to v0.2.0 (needs DAG covering).
- Drive-strength selection: deferred to v0.2.0 (needs load estimation).
"""

from __future__ import annotations

from dataclasses import dataclass, field

from gate_netlist_format import (
    Instance,
    Level,
    Module,
    Netlist,
    NetSlice,
)

# Default mapping: HNL built-in cell types -> Sky130-style stdcell names.
# Each pair maps (generic_type, stdcell_type, pin_remap).
# pin_remap: dict from generic-pin-name to stdcell-pin-name. Sky130 uses
# A/B/Y for inputs/outputs which match HNL conventions, so most pin_remaps
# are identity.
DEFAULT_MAP: dict[str, tuple[str, dict[str, str]]] = {
    "BUF": ("buf_1", {"A": "A", "Y": "X"}),
    "NOT": ("inv_1", {"A": "A", "Y": "Y"}),
    "AND2": ("and2_1", {"A": "A", "B": "B", "Y": "X"}),
    "AND3": ("and3_1", {"A": "A", "B": "B", "C": "C", "Y": "X"}),
    "AND4": ("and4_1", {"A": "A", "B": "B", "C": "C", "D": "D", "Y": "X"}),
    "OR2":  ("or2_1",  {"A": "A", "B": "B", "Y": "X"}),
    "OR3":  ("or3_1",  {"A": "A", "B": "B", "C": "C", "Y": "X"}),
    "OR4":  ("or4_1",  {"A": "A", "B": "B", "C": "C", "D": "D", "Y": "X"}),
    "NAND2": ("nand2_1", {"A": "A", "B": "B", "Y": "Y"}),
    "NAND3": ("nand3_1", {"A": "A", "B": "B", "C": "C", "Y": "Y"}),
    "NAND4": ("nand4_1", {"A": "A", "B": "B", "C": "C", "D": "D", "Y": "Y"}),
    "NOR2":  ("nor2_1",  {"A": "A", "B": "B", "Y": "Y"}),
    "NOR3":  ("nor3_1",  {"A": "A", "B": "B", "C": "C", "Y": "Y"}),
    "NOR4":  ("nor4_1",  {"A": "A", "B": "B", "C": "C", "D": "D", "Y": "Y"}),
    "XOR2": ("xor2_1", {"A": "A", "B": "B", "Y": "X"}),
    "XOR3": ("xor3_1", {"A": "A", "B": "B", "C": "C", "Y": "X"}),
    "XNOR2": ("xnor2_1", {"A": "A", "B": "B", "Y": "Y"}),
    "XNOR3": ("xnor3_1", {"A": "A", "B": "B", "C": "C", "Y": "Y"}),
    "MUX2": ("mux2_1", {"A": "A0", "B": "A1", "S": "S", "Y": "X"}),
    "DFF":  ("dfxtp_1", {"D": "D", "CLK": "CLK", "Q": "Q"}),
    "DFF_R": ("dfrtp_1", {"D": "D", "CLK": "CLK", "R": "RESET_B", "Q": "Q"}),
    "DFF_S": ("dfstp_1", {"D": "D", "CLK": "CLK", "S": "SET_B", "Q": "Q"}),
    "DFF_RS": ("dfsrtp_1", {"D": "D", "CLK": "CLK", "R": "RESET_B", "S": "SET_B", "Q": "Q"}),
    "DLATCH": ("dlxtp_1", {"D": "D", "EN": "GATE", "Q": "Q"}),
    "TBUF": ("ebufn_1", {"A": "A", "OE": "TE_B", "Y": "Z"}),
    "CONST_0": ("conb_1", {"Y": "LO"}),
    "CONST_1": ("conb_1", {"Y": "HI"}),
}


@dataclass
class MappingReport:
    cells_before: int = 0
    cells_after: int = 0
    bubbles_canceled: int = 0
    aoi_oai_folded: int = 0
    unmapped: list[str] = field(default_factory=list)


@dataclass
class TechMapper:
    """Generic-cell -> stdcell mapper."""

    cell_map: dict[str, tuple[str, dict[str, str]]] = field(
        default_factory=lambda: dict(DEFAULT_MAP)
    )

    def map(self, netlist: Netlist) -> tuple[Netlist, MappingReport]:
        """Map every Instance's cell_type via the cell_map. Returns a new
        Netlist with level=STDCELL plus a MappingReport."""
        report = MappingReport()
        new_modules: dict[str, Module] = {}

        for mod_name, mod in netlist.modules.items():
            new_module = self._map_module(mod, report)
            new_modules[mod_name] = new_module

        report.cells_after = sum(len(m.instances) for m in new_modules.values())
        return (
            Netlist(
                top=netlist.top,
                modules=new_modules,
                level=Level.STDCELL,
                version=netlist.version,
            ),
            report,
        )

    def _map_module(self, mod: Module, report: MappingReport) -> Module:
        new_instances: list[Instance] = []
        for inst in mod.instances:
            report.cells_before += 1
            mapping = self.cell_map.get(inst.cell_type)
            if mapping is None:
                # User module or unknown — pass through unchanged.
                new_instances.append(inst)
                if inst.cell_type not in report.unmapped:
                    report.unmapped.append(inst.cell_type)
                continue

            stdcell, pin_remap = mapping
            new_conns: dict[str, NetSlice] = {}
            for generic_pin, slice_ in inst.connections.items():
                stdcell_pin = pin_remap.get(generic_pin, generic_pin)
                new_conns[stdcell_pin] = slice_

            new_instances.append(Instance(
                name=inst.name,
                cell_type=stdcell,
                connections=new_conns,
                parameters=dict(inst.parameters),
            ))

        return Module(
            name=mod.name,
            ports=list(mod.ports),
            nets=list(mod.nets),
            instances=new_instances,
        )


def map_to_stdcell(netlist: Netlist) -> tuple[Netlist, MappingReport]:
    """Convenience entrypoint with the default Sky130-style mapping."""
    return TechMapper().map(netlist)


# ----------------------------------------------------------------------------
# Bubble-pushing optimization (post-mapping)
# ----------------------------------------------------------------------------


def push_bubbles(netlist: Netlist) -> tuple[Netlist, int]:
    """Eliminate adjacent INV-INV pairs.

    Walks each module; finds pairs (inv1.Y -> inv2.A) where both are inv_1.
    Removes the pair and short-circuits inv1.A -> inv2.Y.

    Returns (new netlist, count of pairs canceled).
    """
    cancelled = 0
    new_modules: dict[str, Module] = {}

    for mod_name, mod in netlist.modules.items():
        new_mod, n = _cancel_inv_pairs(mod)
        new_modules[mod_name] = new_mod
        cancelled += n

    return (
        Netlist(
            top=netlist.top,
            modules=new_modules,
            level=netlist.level,
            version=netlist.version,
        ),
        cancelled,
    )


def _cancel_inv_pairs(mod: Module) -> tuple[Module, int]:
    """For each INV instance whose Y feeds another INV's A, remove both and
    rewire the upstream A to the downstream Y."""
    # Build a map: net -> list of (instance, pin) reading that net.
    # And: net -> (instance, pin) that drives it.
    drivers: dict[tuple[str, int], tuple[str, str]] = {}
    readers: dict[tuple[str, int], list[tuple[str, str]]] = {}

    inv_cell_types = {"inv_1", "NOT"}

    for inst in mod.instances:
        if inst.cell_type in inv_cell_types:
            for pin, slice_ in inst.connections.items():
                # Identify pin direction: A (input) or Y/output
                if pin in ("A",):
                    # Reader
                    for bit in slice_.bits:
                        readers.setdefault((slice_.net, bit), []).append((inst.name, pin))
                else:
                    # Driver (Y or output mapped name)
                    for bit in slice_.bits:
                        drivers[(slice_.net, bit)] = (inst.name, pin)

    # Find pairs: inv1.Y drives net N; inv2.A reads net N (and net N is read by exactly inv2.A).
    cancelled_instances: set[str] = set()
    pin_rewrites: dict[tuple[str, str], NetSlice] = {}

    inst_by_name = {i.name: i for i in mod.instances}

    for (net_name, bit), (inv1_name, _drv_pin) in drivers.items():
        if inv1_name in cancelled_instances:
            continue
        rs = readers.get((net_name, bit), [])
        if len(rs) != 1:
            continue
        inv2_name, _rdr_pin = rs[0]
        if inv2_name == inv1_name:
            continue
        if inv2_name in cancelled_instances:
            continue

        inv1 = inst_by_name[inv1_name]
        inv2 = inst_by_name[inv2_name]

        # Find inv1's input net (A pin)
        inv1_a_slice = inv1.connections.get("A")
        if inv1_a_slice is None:
            continue
        # Find inv2's output net (Y pin or whatever maps to it)
        inv2_y_slice = inv2.connections.get("Y")
        if inv2_y_slice is None:
            continue

        # Cancel both: rewrite all readers of inv2.Y to read inv1.A directly.
        # We do this by recording the rewrite; then apply to all instances.
        for reader_inst_name, reader_pin in readers.get(
            (inv2_y_slice.net, inv2_y_slice.bits[0]), []
        ):
            pin_rewrites[(reader_inst_name, reader_pin)] = inv1_a_slice

        cancelled_instances.add(inv1_name)
        cancelled_instances.add(inv2_name)

    # Apply the cancellations.
    new_instances: list[Instance] = []
    for inst in mod.instances:
        if inst.name in cancelled_instances:
            continue
        new_conns: dict[str, NetSlice] = dict(inst.connections)
        for pin in list(new_conns.keys()):
            rewrite = pin_rewrites.get((inst.name, pin))
            if rewrite is not None:
                new_conns[pin] = rewrite
        new_instances.append(Instance(
            name=inst.name,
            cell_type=inst.cell_type,
            connections=new_conns,
            parameters=dict(inst.parameters),
        ))

    return (
        Module(
            name=mod.name,
            ports=list(mod.ports),
            nets=list(mod.nets),
            instances=new_instances,
        ),
        len(cancelled_instances) // 2,
    )
