"""gate-netlist-format: HNL (Hardware NetList) data structures + JSON.

HNL is the canonical netlist format consumed downstream of synthesis. JSON-
serializable, hierarchy-preserving, cell-typed. EDIF and BLIF importer/
exporters are provided for tool interop.
"""

from gate_netlist_format.cells import BUILTIN_CELL_TYPES, CellTypeSig
from gate_netlist_format.netlist import (
    Direction,
    Instance,
    Level,
    Module,
    Net,
    Netlist,
    NetlistStats,
    NetSlice,
    Port,
    ValidationReport,
)

__version__ = "0.1.0"

__all__ = [
    "BUILTIN_CELL_TYPES",
    "CellTypeSig",
    "Direction",
    "Instance",
    "Level",
    "Module",
    "Net",
    "NetSlice",
    "Netlist",
    "NetlistStats",
    "Port",
    "ValidationReport",
    "__version__",
]
