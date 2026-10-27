//! Checked functional model of the classic PIC16F84A mid-range CPU.
//!
//! The model owns complete CPU and image state, executes real 14-bit words,
//! and commits each instruction transactionally.
//!
//! ```
//! use pic16f84a_simulator::{encoding, Pic16f84aSimulator};
//!
//! let mut cpu = Pic16f84aSimulator::new();
//! let result = cpu.execute(
//!     &[encoding::movlw(0x2a), encoding::halt()],
//!     0,
//!     2,
//! )?;
//! assert!(result.halted);
//! assert_eq!(result.final_state.w, 0x2a);
//! # Ok::<(), pic16f84a_simulator::Pic16f84aError>(())
//! ```

mod implementation;

pub use implementation::*;

#[cfg(test)]
mod tests;
