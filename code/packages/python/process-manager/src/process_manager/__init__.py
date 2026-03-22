"""Process Manager — Advanced process management for the coding-adventures OS.

This package implements the Unix process management model: fork, exec, wait,
signals, and priority scheduling. It extends the basic process model from the
OS kernel (S04) with dynamic process creation and inter-process communication.

Modules:
    pcb: Extended ProcessControlBlock with parent/child relationships and signals.
    signals: Signal enum and signal delivery/masking logic.
    process_manager: Core process lifecycle (fork, exec, wait, kill, exit).
    priority_scheduler: Priority-based scheduler with round-robin within levels.
"""

from process_manager.pcb import ProcessControlBlock, ProcessState
from process_manager.priority_scheduler import PriorityScheduler
from process_manager.process_manager import ProcessManager
from process_manager.signals import Signal, SignalManager

__all__ = [
    "ProcessControlBlock",
    "ProcessState",
    "Signal",
    "SignalManager",
    "ProcessManager",
    "PriorityScheduler",
]
