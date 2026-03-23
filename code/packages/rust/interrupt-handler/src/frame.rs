//! Interrupt Frame — saved CPU context for interrupt entry/exit.
//!
//! When an interrupt fires, the CPU saves everything needed to resume the
//! interrupted code later. This is the interrupt frame (or trap frame).
//!
//! # Layout (136 bytes)
//!
//! ```text
//! PC (return address)         4 bytes
//! MStatus register            4 bytes
//! MCause register             4 bytes
//! x1-x31 (31 registers)      124 bytes
//! Total: 34 words = 136 bytes
//! ```
//!
//! Why save ALL 32 registers? The ISR is arbitrary code -- it might use any
//! register. Saving everything is safe and simple.

/// Holds all CPU state needed to resume after an interrupt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InterruptFrame {
    /// Saved program counter (where to resume).
    pub pc: u32,
    /// All 32 RISC-V general-purpose registers (x0-x31).
    pub registers: [u32; 32],
    /// Machine status register.
    pub mstatus: u32,
    /// What caused the interrupt (interrupt number).
    pub mcause: u32,
}

impl Default for InterruptFrame {
    fn default() -> Self {
        Self {
            pc: 0,
            registers: [0; 32],
            mstatus: 0,
            mcause: 0,
        }
    }
}

/// Create an InterruptFrame from the current CPU state.
///
/// Called at the beginning of interrupt handling, before the ISR runs.
/// The registers are copied (not referenced) for safety.
pub fn save_context(registers: [u32; 32], pc: u32, mstatus: u32, mcause: u32) -> InterruptFrame {
    InterruptFrame {
        pc,
        registers,
        mstatus,
        mcause,
    }
}

/// Extract CPU state from an InterruptFrame.
///
/// Called after the ISR completes, to resume the interrupted code.
/// Returns `(registers, pc, mstatus)`.
pub fn restore_context(frame: &InterruptFrame) -> ([u32; 32], u32, u32) {
    (frame.registers, frame.pc, frame.mstatus)
}
