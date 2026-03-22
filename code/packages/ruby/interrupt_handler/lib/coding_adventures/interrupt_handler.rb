# frozen_string_literal: true

# S03 Interrupt Handler — IDT, ISR registry, interrupt controller, context save/restore.
#
# This package implements the full interrupt lifecycle for the coding-adventures
# simulated computer. Without interrupts, a CPU can only execute instructions
# sequentially. Interrupts transform a calculator into a computer by enabling
# response to external events (keyboard, timer), multitasking, and system services.
#
# Analogy: Interrupts are like a phone ringing while you are cooking. You pause
# cooking (save context), answer the phone (handle the interrupt), and resume
# cooking exactly where you left off (restore context).

require_relative "interrupt_handler/version"
require_relative "interrupt_handler/idt_entry"
require_relative "interrupt_handler/idt"
require_relative "interrupt_handler/interrupt_frame"
require_relative "interrupt_handler/isr_registry"
require_relative "interrupt_handler/interrupt_controller"

module CodingAdventures
  module InterruptHandler
    # Well-known interrupt numbers. These follow x86/RISC-V conventions:
    #
    #   Number  Name                Source
    #   ------  ----                ------
    #   0       Division by Zero    CPU
    #   1       Debug Exception     CPU
    #   2       NMI                 Hardware (non-maskable)
    #   3       Breakpoint          CPU (ebreak)
    #   4       Overflow            CPU
    #   5       Invalid Opcode      CPU
    #   32      Timer               Timer chip
    #   33      Keyboard            Keyboard controller
    #   128     System Call          Software (ecall)
    INT_DIVISION_BY_ZERO = 0
    INT_DEBUG = 1
    INT_NMI = 2
    INT_BREAKPOINT = 3
    INT_OVERFLOW = 4
    INT_INVALID_OPCODE = 5
    INT_TIMER = 32
    INT_KEYBOARD = 33
    INT_SYSCALL = 128

    # Each IDT entry occupies 8 bytes in memory.
    IDT_ENTRY_SIZE = 8

    # Total IDT size: 256 entries * 8 bytes = 2048 bytes.
    IDT_SIZE = 256 * IDT_ENTRY_SIZE

    # Default memory location of the IDT.
    IDT_BASE_ADDRESS = 0x00000000
  end
end
