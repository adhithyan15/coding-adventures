"""ISR (Interrupt Service Routine) Registry.

Maps interrupt numbers to Python handler functions. This is the "software
side" of interrupt handling: the IDT maps interrupt numbers to memory
addresses (for hardware simulation), while the ISR Registry maps them to
actual Python functions (for emulation).

Why both? In a real CPU, the IDT entry's ISR address points to machine
code in memory. In our emulator, we need to map that same interrupt
number to a Python callable.
"""

from __future__ import annotations

from collections.abc import Callable
from typing import Any

from interrupt_handler.frame import InterruptFrame

# Type alias for ISR handler functions.
# The frame contains saved CPU state; kernel provides access to kernel facilities.
ISRHandler = Callable[[InterruptFrame, Any], None]


class ISRRegistry:
    """Maps interrupt numbers to Python handler functions.

    Usage:
        registry = ISRRegistry()
        registry.register(32, my_timer_handler)
        registry.dispatch(32, frame, kernel)
    """

    def __init__(self: ISRRegistry) -> None:
        """Create an empty ISR registry."""
        self._handlers: dict[int, ISRHandler] = {}

    def register(self: ISRRegistry, interrupt_number: int, handler: ISRHandler) -> None:
        """Install a handler for the given interrupt number.

        Overwrites any previously registered handler, matching real OS
        behavior (kernel replaces BIOS handlers during boot).
        """
        self._handlers[interrupt_number] = handler

    def dispatch(
        self: ISRRegistry,
        interrupt_number: int,
        frame: InterruptFrame,
        kernel: Any,  # noqa: ANN401
    ) -> None:
        """Call the registered handler for the given interrupt number.

        Raises:
            KeyError: If no handler is registered (double fault condition).
        """
        handler = self._handlers.get(interrupt_number)
        if handler is None:
            msg = f"No ISR handler registered for interrupt {interrupt_number}"
            raise KeyError(msg)
        handler(frame, kernel)

    def has_handler(self: ISRRegistry, interrupt_number: int) -> bool:
        """Return True if a handler is registered for this interrupt number."""
        return interrupt_number in self._handlers
