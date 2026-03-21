"""Interrupt Controller — manages pending queue, masking, and dispatch.

The interrupt controller is the central hub connecting hardware signals
to software handlers. It manages:

1. Pending queue: sorted list of raised interrupt numbers
2. Mask register: 32-bit value blocking specific interrupts (0-31)
3. Global enable flag: master switch for all interrupts
4. Priority dispatch: lower number = higher priority

Mask Register Layout:
    Bit 0 = interrupt 0 (division by zero)
    Bit 1 = interrupt 1 (debug)
    ...
    Bit 31 = interrupt 31
    1 = masked (blocked), 0 = unmasked (allowed)
    Interrupts 32+ are always unmasked (unless globally disabled).
"""

from __future__ import annotations

import bisect

from interrupt_handler.idt import InterruptDescriptorTable
from interrupt_handler.isr import ISRRegistry


class InterruptController:
    """Manages the full interrupt lifecycle.

    The lifecycle:
        1. Device calls raise_interrupt(number)
        2. Pipeline checks has_pending() between instructions
        3. If pending: next_pending() returns highest-priority interrupt
        4. CPU saves context, looks up IDT, dispatches ISR
        5. After ISR: acknowledge() removes from pending
        6. CPU restores context and resumes
    """

    def __init__(self: InterruptController) -> None:
        """Create a controller with empty IDT, registry, and no pending."""
        self.idt = InterruptDescriptorTable()
        self.registry = ISRRegistry()
        self._pending: list[int] = []
        self.mask_register: int = 0  # 32-bit mask, default all unmasked
        self.enabled: bool = True  # global enable flag

    def raise_interrupt(self: InterruptController, number: int) -> None:
        """Add an interrupt to the pending queue.

        If already pending, it is not added again (no duplicates).
        The queue stays sorted ascending (lower = higher priority).
        """
        if number not in self._pending:
            bisect.insort(self._pending, number)

    def has_pending(self: InterruptController) -> bool:
        """Return True if any unmasked pending interrupts exist and enabled."""
        if not self.enabled:
            return False
        return any(not self.is_masked(n) for n in self._pending)

    def next_pending(self: InterruptController) -> int:
        """Return highest-priority (lowest-numbered) unmasked pending interrupt.

        Returns -1 if none available or globally disabled.
        """
        if not self.enabled:
            return -1
        for n in self._pending:
            if not self.is_masked(n):
                return n
        return -1

    def acknowledge(self: InterruptController, number: int) -> None:
        """Remove the given interrupt from the pending queue.

        Called after the ISR completes (End of Interrupt / EOI).
        """
        try:
            self._pending.remove(number)
        except ValueError:
            pass  # not pending, nothing to do

    def set_mask(self: InterruptController, number: int, masked: bool) -> None:
        """Set or clear the mask for interrupt number (0-31 only).

        masked=True blocks the interrupt; masked=False allows it.
        Interrupts 32+ are not controlled by the mask register.
        """
        if number < 0 or number > 31:
            return  # only 0-31 are maskable
        if masked:
            self.mask_register |= 1 << number
        else:
            self.mask_register &= ~(1 << number)

    def is_masked(self: InterruptController, number: int) -> bool:
        """Return True if the interrupt is currently masked (blocked).

        Interrupts 32+ are never masked by the mask register.
        """
        if number < 0 or number > 31:
            return False
        return (self.mask_register & (1 << number)) != 0

    def enable(self: InterruptController) -> None:
        """Set the global interrupt enable flag."""
        self.enabled = True

    def disable(self: InterruptController) -> None:
        """Clear the global interrupt enable flag."""
        self.enabled = False

    def pending_count(self: InterruptController) -> int:
        """Return the number of pending interrupts (masked and unmasked)."""
        return len(self._pending)

    def clear_all(self: InterruptController) -> None:
        """Remove all pending interrupts."""
        self._pending.clear()
