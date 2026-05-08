"""spice-engine: SPICE-compatible analog simulator (MNA + DC + transient + AC + DC sweep)."""

from spice_engine.elements import (
    BJT,
    Capacitor,
    CurrentSource,
    Diode,
    Element,
    Inductor,
    Mosfet,
    Resistor,
    VoltageSource,
)
from spice_engine.engine import (
    AcPoint,
    AcResult,
    Circuit,
    DcResult,
    DcSweepPoint,
    DcSweepResult,
    TfResult,
    TransientPoint,
    TransientResult,
    ac_sweep,
    dc_op,
    dc_sweep,
    tf,
    transient,
)

__version__ = "0.6.0"

__all__ = [
    "AcPoint",
    "AcResult",
    "BJT",
    "Capacitor",
    "Circuit",
    "CurrentSource",
    "DcResult",
    "DcSweepPoint",
    "DcSweepResult",
    "Diode",
    "Element",
    "Inductor",
    "Mosfet",
    "Resistor",
    "TfResult",
    "TransientPoint",
    "TransientResult",
    "VoltageSource",
    "__version__",
    "ac_sweep",
    "dc_op",
    "dc_sweep",
    "tf",
    "transient",
]
