"""vcd-writer: streaming VCD waveform output (IEEE 1364 §18).

Decoupled from the hardware-vm package by design. Sources call
``writer.value_change(time, signal_id, value)``; this package handles
formatting and file output.
"""

from vcd_writer.writer import (
    Scope,
    VarDef,
    VcdWriter,
    attach_to_callback_emitter,
)

__version__ = "0.1.0"

__all__ = [
    "Scope",
    "VarDef",
    "VcdWriter",
    "__version__",
    "attach_to_callback_emitter",
]
