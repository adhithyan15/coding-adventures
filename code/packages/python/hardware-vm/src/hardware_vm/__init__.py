"""hardware-vm: event-driven simulator for HIR.

Drives an HIR design with input stimulus, runs continuous assignments, and
emits value-change events to subscribers (e.g., vcd-writer)."""

from hardware_vm.eval import (
    evaluate,
    referenced_signals,
)
from hardware_vm.vm import Event, HardwareVM, RunResult

__version__ = "0.1.0"

__all__ = [
    "Event",
    "HardwareVM",
    "RunResult",
    "__version__",
    "evaluate",
    "referenced_signals",
]
