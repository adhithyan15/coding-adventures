//! I/O Block — bidirectional pad connecting FPGA internals to the outside world.
//!
//! # What is an I/O Block?
//!
//! I/O blocks sit at the perimeter of the FPGA and provide the interface
//! between the internal logic fabric and the external pins of the chip.
//!
//! Each I/O block can be configured in three modes:
//! - **Input**: External signal enters the FPGA (pad -> internal)
//! - **Output**: Internal signal exits the FPGA (internal -> pad)
//! - **Tristate**: Output is high-impedance (disconnected) when not enabled
//!
//! # I/O Block Architecture
//!
//! ```text
//! External Pin (pad)
//!      |
//!      v
//! +--------------------+
//! |    I/O Block        |
//! |                     |
//! |  +--------------+   |
//! |  | Input Reg    |   | -- (optional) register the input
//! |  +------+-------+   |
//! |         |            |
//! |  +------v-------+   |
//! |  | Tri-State     |   | -- output enable controls direction
//! |  | Buffer        |   |
//! |  +------+-------+   |
//! |         |            |
//! |  +------v-------+   |
//! |  | Output Reg   |   | -- (optional) register the output
//! |  +--------------+   |
//! |                     |
//! +--------------------+
//!      |
//!      v
//! To/From Internal Fabric
//! ```

use logic_gates::combinational::tri_state;

/// I/O block operating mode.
///
/// - `Input`: Pad drives internal signal (external -> fabric)
/// - `Output`: Fabric drives pad (fabric -> external)
/// - `Tristate`: Output is high-impedance (pad is disconnected)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IOMode {
    /// External signal enters the FPGA.
    Input,
    /// Internal signal exits the FPGA.
    Output,
    /// Output is high-impedance (disconnected).
    Tristate,
}

/// Bidirectional I/O pad for the FPGA perimeter.
///
/// Each I/O block connects one external pin to the internal fabric.
/// The mode determines the direction of data flow.
///
/// # Example — input pin
///
/// ```
/// use fpga::io_block::{IOBlock, IOMode};
/// let mut io = IOBlock::new("sensor_in".to_string(), IOMode::Input);
/// io.drive_pad(1);
/// assert_eq!(io.read_internal(), Some(1));
/// ```
///
/// # Example — output pin
///
/// ```
/// use fpga::io_block::{IOBlock, IOMode};
/// let mut io = IOBlock::new("led_0".to_string(), IOMode::Output);
/// io.drive_internal(1);
/// assert_eq!(io.read_pad(), Some(1));
/// ```
///
/// # Example — tri-state (disconnected)
///
/// ```
/// use fpga::io_block::{IOBlock, IOMode};
/// let mut io = IOBlock::new("bus_0".to_string(), IOMode::Tristate);
/// io.drive_internal(1);
/// assert_eq!(io.read_pad(), None); // high impedance
/// ```
#[derive(Debug, Clone)]
pub struct IOBlock {
    name: String,
    mode: IOMode,
    pad_value: u8,
    internal_value: u8,
}

impl IOBlock {
    /// Create a new I/O block.
    ///
    /// # Panics
    ///
    /// Panics if name is empty.
    pub fn new(name: String, mode: IOMode) -> Self {
        assert!(!name.is_empty(), "name must be a non-empty string");
        Self {
            name,
            mode,
            pad_value: 0,
            internal_value: 0,
        }
    }

    /// Change the I/O block's operating mode.
    pub fn configure(&mut self, mode: IOMode) {
        self.mode = mode;
    }

    /// Drive the external pad with a signal (used in INPUT mode).
    ///
    /// # Panics
    ///
    /// Panics if value is not 0 or 1.
    pub fn drive_pad(&mut self, value: u8) {
        assert!(value == 0 || value == 1, "value must be 0 or 1, got {value}");
        self.pad_value = value;
    }

    /// Drive the internal (fabric) side with a signal (used in OUTPUT mode).
    ///
    /// # Panics
    ///
    /// Panics if value is not 0 or 1.
    pub fn drive_internal(&mut self, value: u8) {
        assert!(value == 0 || value == 1, "value must be 0 or 1, got {value}");
        self.internal_value = value;
    }

    /// Read the signal visible to the internal fabric.
    ///
    /// In INPUT mode, returns the pad value (external -> fabric).
    /// In OUTPUT/TRISTATE mode, returns the internally driven value.
    pub fn read_internal(&self) -> Option<u8> {
        if self.mode == IOMode::Input {
            Some(self.pad_value)
        } else {
            Some(self.internal_value)
        }
    }

    /// Read the signal visible on the external pad.
    ///
    /// In INPUT mode, returns the pad value.
    /// In OUTPUT mode, returns the internally driven value.
    /// In TRISTATE mode, returns None (high impedance).
    pub fn read_pad(&self) -> Option<u8> {
        match self.mode {
            IOMode::Input => Some(self.pad_value),
            IOMode::Tristate => tri_state(self.internal_value, 0),
            IOMode::Output => tri_state(self.internal_value, 1),
        }
    }

    /// I/O block identifier.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Current operating mode.
    pub fn mode(&self) -> IOMode {
        self.mode
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_input_mode() {
        let mut io = IOBlock::new("in0".to_string(), IOMode::Input);
        io.drive_pad(1);
        assert_eq!(io.read_internal(), Some(1));
        assert_eq!(io.read_pad(), Some(1));
    }

    #[test]
    fn test_output_mode() {
        let mut io = IOBlock::new("out0".to_string(), IOMode::Output);
        io.drive_internal(1);
        assert_eq!(io.read_pad(), Some(1));
    }

    #[test]
    fn test_tristate_mode() {
        let mut io = IOBlock::new("bus0".to_string(), IOMode::Tristate);
        io.drive_internal(1);
        assert_eq!(io.read_pad(), None);
    }

    #[test]
    fn test_mode_change() {
        let mut io = IOBlock::new("pin0".to_string(), IOMode::Input);
        io.configure(IOMode::Output);
        assert_eq!(io.mode(), IOMode::Output);
    }
}
