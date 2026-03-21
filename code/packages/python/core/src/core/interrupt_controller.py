"""InterruptController -- routes interrupts to cores.

# What are Interrupts?

An interrupt is a signal that temporarily diverts the CPU from its current
work to handle an urgent event. Examples:

  - Timer interrupt: "100ms have passed, let the OS scheduler run"
  - I/O interrupt: "keyboard key was pressed" or "network packet arrived"
  - Inter-processor interrupt (IPI): "Core 0 needs Core 1 to flush its TLB"
  - Software interrupt: "this program wants to make a system call"

# How the Controller Works

The interrupt controller is the traffic cop for interrupts:

 1. An external device (or another core) raises an interrupt.
 2. The controller queues it and decides which core should handle it.
 3. On the next cycle, the controller signals the target core.
 4. The core acknowledges the interrupt and begins handling it.

In real hardware, interrupt controllers are sophisticated:
  - ARM GIC (Generic Interrupt Controller): prioritized, masked, routable
  - x86 APIC (Advanced Programmable Interrupt Controller): similar

This implementation is a simplified shell -- it queues interrupts and
routes them, but does not model priorities or masking.
"""

from __future__ import annotations

from dataclasses import dataclass


@dataclass
class PendingInterrupt:
    """An interrupt waiting to be delivered.

    Attributes:
        interrupt_id: Identifies the interrupt source (e.g., timer=0, keyboard=1).
        target_core: Which core should handle it. -1 means "any available core".
    """

    interrupt_id: int
    target_core: int


@dataclass
class AcknowledgedInterrupt:
    """Records a core acknowledging an interrupt.

    Attributes:
        core_id: Which core acknowledged the interrupt.
        interrupt_id: Which interrupt was acknowledged.
    """

    core_id: int
    interrupt_id: int


class InterruptController:
    """Manages interrupt routing in a multi-core system.

    This is a simplified shell: it queues interrupts and routes them to
    specific cores, but does not model priorities or masking.
    """

    def __init__(self, num_cores: int) -> None:
        """Create an interrupt controller for the given number of cores.

        Args:
            num_cores: Total number of cores in the system.
        """
        self._num_cores = num_cores
        self._pending: list[PendingInterrupt] = []
        self._acknowledged: list[AcknowledgedInterrupt] = []

    def raise_interrupt(self, interrupt_id: int, target_core: int) -> None:
        """Queue an interrupt for delivery.

        If target_core is -1, the interrupt will be routed to core 0
        (simplest routing policy).

        Args:
            interrupt_id: The interrupt source identifier.
            target_core: Which core should handle it, or -1 for default routing.
        """
        if target_core == -1:
            target_core = 0
        if target_core >= self._num_cores:
            target_core = 0
        self._pending.append(
            PendingInterrupt(interrupt_id=interrupt_id, target_core=target_core)
        )

    def acknowledge(self, core_id: int, interrupt_id: int) -> None:
        """Record that a core has begun handling an interrupt.

        In real hardware, acknowledgment tells the interrupt controller that
        the core has received the signal and started executing the handler.

        Args:
            core_id: Which core is acknowledging.
            interrupt_id: Which interrupt is being acknowledged.
        """
        self._acknowledged.append(
            AcknowledgedInterrupt(core_id=core_id, interrupt_id=interrupt_id)
        )

        # Remove from pending (first matching occurrence only).
        remaining: list[PendingInterrupt] = []
        removed = False
        for p in self._pending:
            if (
                not removed
                and p.interrupt_id == interrupt_id
                and p.target_core == core_id
            ):
                removed = True
                continue
            remaining.append(p)
        self._pending = remaining

    def pending_for_core(self, core_id: int) -> list[PendingInterrupt]:
        """Return all pending interrupts targeted at a specific core.

        Args:
            core_id: The core to query.

        Returns:
            List of pending interrupts for this core.
        """
        return [p for p in self._pending if p.target_core == core_id]

    @property
    def pending_count(self) -> int:
        """Return the total number of pending (unacknowledged) interrupts."""
        return len(self._pending)

    @property
    def acknowledged_count(self) -> int:
        """Return the total number of acknowledged interrupts."""
        return len(self._acknowledged)

    def reset(self) -> None:
        """Clear all pending and acknowledged interrupts."""
        self._pending = []
        self._acknowledged = []
