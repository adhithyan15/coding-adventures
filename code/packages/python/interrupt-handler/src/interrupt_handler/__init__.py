"""S03 Interrupt Handler — IDT, ISR registry, interrupt controller, context save/restore.

This package implements the full interrupt lifecycle for the coding-adventures
simulated computer. Without interrupts, a CPU can only execute instructions
sequentially. Interrupts transform a calculator into a computer by enabling
response to external events (keyboard, timer), multitasking, and system services.

Analogy: Interrupts are like a phone ringing while you are cooking. You pause
cooking (save context), answer the phone (handle the interrupt), and resume
cooking exactly where you left off (restore context).
"""

from interrupt_handler.controller import InterruptController
from interrupt_handler.frame import InterruptFrame, restore_context, save_context
from interrupt_handler.idt import (
    IDT_BASE_ADDRESS,
    IDT_ENTRY_SIZE,
    IDT_SIZE,
    IDTEntry,
    InterruptDescriptorTable,
)
from interrupt_handler.isr import ISRRegistry

# Well-known interrupt numbers
INT_DIVISION_BY_ZERO = 0
INT_DEBUG = 1
INT_NMI = 2
INT_BREAKPOINT = 3
INT_OVERFLOW = 4
INT_INVALID_OPCODE = 5
INT_TIMER = 32
INT_KEYBOARD = 33
INT_SYSCALL = 128

__all__ = [
    "IDTEntry",
    "InterruptDescriptorTable",
    "InterruptFrame",
    "ISRRegistry",
    "InterruptController",
    "save_context",
    "restore_context",
    "IDT_ENTRY_SIZE",
    "IDT_SIZE",
    "IDT_BASE_ADDRESS",
    "INT_DIVISION_BY_ZERO",
    "INT_DEBUG",
    "INT_NMI",
    "INT_BREAKPOINT",
    "INT_OVERFLOW",
    "INT_INVALID_OPCODE",
    "INT_TIMER",
    "INT_KEYBOARD",
    "INT_SYSCALL",
]
