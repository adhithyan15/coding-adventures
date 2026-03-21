//! S03 Interrupt Handler — IDT, ISR registry, interrupt controller, context save/restore.
//!
//! This crate implements the full interrupt lifecycle for the coding-adventures
//! simulated computer. Without interrupts, a CPU can only execute instructions
//! sequentially. Interrupts transform a calculator into a computer by enabling
//! response to external events (keyboard, timer), multitasking, and system services.
//!
//! # Analogy
//!
//! Interrupts are like a phone ringing while you are cooking. You pause cooking
//! (save context — remember what step you were on), answer the phone (handle the
//! interrupt), and resume cooking exactly where you left off (restore context).
//!
//! # Components
//!
//! - [`InterruptDescriptorTable`]: 256 entries mapping interrupt numbers to ISR addresses
//! - [`ISRRegistry`]: Maps interrupt numbers to Rust handler closures
//! - [`InterruptController`]: Pending queue, mask register, global enable/disable
//! - [`InterruptFrame`]: Save/restore 32 registers + PC + MStatus + MCause

mod idt;
mod frame;
mod isr;
mod controller;

pub use idt::{IDTEntry, InterruptDescriptorTable, IDT_ENTRY_SIZE, IDT_SIZE, IDT_BASE_ADDRESS};
pub use frame::{InterruptFrame, save_context, restore_context};
pub use isr::ISRRegistry;
pub use controller::InterruptController;

// =========================================================================
// Well-known interrupt numbers
// =========================================================================

/// Division by zero — CPU exception (interrupt 0).
pub const INT_DIVISION_BY_ZERO: usize = 0;
/// Debug exception — CPU (interrupt 1).
pub const INT_DEBUG: usize = 1;
/// Non-maskable interrupt — Hardware (interrupt 2).
pub const INT_NMI: usize = 2;
/// Breakpoint — CPU ebreak (interrupt 3).
pub const INT_BREAKPOINT: usize = 3;
/// Overflow — CPU arithmetic (interrupt 4).
pub const INT_OVERFLOW: usize = 4;
/// Invalid opcode — CPU (interrupt 5).
pub const INT_INVALID_OPCODE: usize = 5;
/// Timer — clock tick from timer chip (interrupt 32).
pub const INT_TIMER: usize = 32;
/// Keyboard — external keystroke (interrupt 33).
pub const INT_KEYBOARD: usize = 33;
/// System call — ecall instruction (interrupt 128).
pub const INT_SYSCALL: usize = 128;

#[cfg(test)]
mod tests;
