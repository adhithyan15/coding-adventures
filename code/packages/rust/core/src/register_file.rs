//! RegisterFile -- general-purpose register file for the Core.
//!
//! # Why a Custom Register File?
//!
//! The Core needs a register file that:
//!   - Uses i64 values (matching PipelineToken fields)
//!   - Supports configurable width (32 or 64 bit)
//!   - Optionally hardwires register 0 to zero (RISC-V convention)
//!   - Returns 0 for out-of-range access instead of panicking
//!
//! # Zero Register Convention
//!
//! In RISC-V and MIPS, register x0 (or $zero) is hardwired to the value 0.
//! Writes to it are silently discarded. This simplifies instruction encoding:
//!
//! ```text
//!   MOV Rd, Rs  = ADD Rd, Rs, x0   (add zero)
//!   NOP         = ADD x0, x0, x0   (write nothing to zero register)
//!   NEG Rd, Rs  = SUB Rd, x0, Rs   (subtract from zero)
//! ```
//!
//! ARM does NOT have a zero register (all 31 registers are general-purpose).
//! x86 does not have one either. The `zero_register` config controls this.

use std::fmt;

use crate::config::RegisterFileConfig;

/// The Core's register file -- fast, small storage that the pipeline reads
/// and writes every cycle.
pub struct RegisterFile {
    /// Register file configuration.
    config: RegisterFileConfig,

    /// Register values. values[0] is R0.
    values: Vec<i64>,

    /// Bit mask for the register width (e.g., 0xFFFFFFFF for 32-bit).
    mask: i64,
}

impl RegisterFile {
    /// Creates a new register file from the given configuration.
    ///
    /// All registers are initialized to 0. If config is None, the default
    /// configuration is used (16 registers, 32-bit, zero register enabled).
    pub fn new(config: Option<&RegisterFileConfig>) -> Self {
        let cfg = match config {
            Some(c) => c.clone(),
            None => RegisterFileConfig::default(),
        };

        // Compute the bit mask for the register width.
        // For 32-bit: mask = 0xFFFFFFFF
        // For 64-bit: mask = i64::MAX (all bits except sign bit)
        let mask = if cfg.width >= 64 {
            i64::MAX
        } else {
            (1i64 << cfg.width) - 1
        };

        RegisterFile {
            values: vec![0i64; cfg.count],
            config: cfg,
            mask,
        }
    }

    /// Returns the value of the register at the given index.
    ///
    /// If the zero register convention is enabled, reading register 0 always
    /// returns 0, regardless of what was written to it.
    ///
    /// Returns 0 if the index is out of range (defensive -- avoids panics in
    /// the pipeline, which processes untrusted instruction data).
    pub fn read(&self, index: i64) -> i64 {
        if index < 0 || index as usize >= self.config.count {
            return 0;
        }
        if self.config.zero_register && index == 0 {
            return 0;
        }
        self.values[index as usize]
    }

    /// Stores a value into the register at the given index.
    ///
    /// The value is masked to the register width (e.g., 32-bit mask for 32-bit
    /// registers). Writes to register 0 are silently ignored when the zero
    /// register convention is enabled.
    ///
    /// Writes to out-of-range indices are silently ignored (defensive).
    pub fn write(&mut self, index: i64, value: i64) {
        if index < 0 || index as usize >= self.config.count {
            return;
        }
        if self.config.zero_register && index == 0 {
            return; // writes to zero register are discarded
        }
        self.values[index as usize] = value & self.mask;
    }

    /// Returns a copy of all register values (for inspection and debugging).
    pub fn values(&self) -> Vec<i64> {
        self.values.clone()
    }

    /// Returns the number of registers.
    pub fn count(&self) -> usize {
        self.config.count
    }

    /// Returns the bit width of each register.
    pub fn width(&self) -> usize {
        self.config.width
    }

    /// Returns a clone of the register file configuration.
    pub fn config(&self) -> RegisterFileConfig {
        self.config.clone()
    }

    /// Sets all registers to zero.
    pub fn reset(&mut self) {
        for v in self.values.iter_mut() {
            *v = 0;
        }
    }
}

impl fmt::Display for RegisterFile {
    /// Returns a human-readable dump of all registers.
    ///
    /// Format: `RegisterFile(16x32): R1=42 R2=100 ...`
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "RegisterFile({}x{}):", self.config.count, self.config.width)?;
        for i in 0..self.config.count {
            if self.values[i] != 0 {
                write!(f, " R{}={}", i, self.values[i])?;
            }
        }
        Ok(())
    }
}

// =========================================================================
// Tests
// =========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_register_file() {
        let rf = RegisterFile::new(None);
        assert_eq!(rf.count(), 16);
        assert_eq!(rf.width(), 32);
        // All registers start at 0.
        for i in 0..16 {
            assert_eq!(rf.read(i), 0);
        }
    }

    #[test]
    fn test_zero_register_reads_zero() {
        let mut rf = RegisterFile::new(None);
        rf.write(0, 42);
        assert_eq!(rf.read(0), 0, "zero register should always read 0");
    }

    #[test]
    fn test_write_and_read() {
        let mut rf = RegisterFile::new(None);
        rf.write(1, 42);
        rf.write(2, 100);
        assert_eq!(rf.read(1), 42);
        assert_eq!(rf.read(2), 100);
    }

    #[test]
    fn test_out_of_range_read_returns_zero() {
        let rf = RegisterFile::new(None);
        assert_eq!(rf.read(-1), 0);
        assert_eq!(rf.read(999), 0);
    }

    #[test]
    fn test_out_of_range_write_ignored() {
        let mut rf = RegisterFile::new(None);
        rf.write(-1, 42); // should not panic
        rf.write(999, 42); // should not panic
    }

    #[test]
    fn test_32bit_mask() {
        let mut rf = RegisterFile::new(None);
        // Write a value larger than 32 bits -- should be masked.
        rf.write(1, 0x1_FFFF_FFFF);
        assert_eq!(rf.read(1), 0xFFFF_FFFF);
    }

    #[test]
    fn test_64bit_registers() {
        let cfg = RegisterFileConfig {
            count: 8,
            width: 64,
            zero_register: false,
        };
        let mut rf = RegisterFile::new(Some(&cfg));
        assert_eq!(rf.count(), 8);
        assert_eq!(rf.width(), 64);
        rf.write(0, 42);
        assert_eq!(rf.read(0), 42, "no zero register, so R0 is writable");
    }

    #[test]
    fn test_reset() {
        let mut rf = RegisterFile::new(None);
        rf.write(1, 42);
        rf.write(2, 100);
        rf.reset();
        assert_eq!(rf.read(1), 0);
        assert_eq!(rf.read(2), 0);
    }

    #[test]
    fn test_values_snapshot() {
        let mut rf = RegisterFile::new(None);
        rf.write(3, 99);
        let vals = rf.values();
        assert_eq!(vals[3], 99);
        assert_eq!(vals.len(), 16);
    }

    #[test]
    fn test_display() {
        let mut rf = RegisterFile::new(None);
        rf.write(1, 42);
        let s = format!("{}", rf);
        assert!(s.contains("R1=42"));
        assert!(s.contains("RegisterFile(16x32)"));
    }

    #[test]
    fn test_config_accessor() {
        let rf = RegisterFile::new(None);
        let cfg = rf.config();
        assert_eq!(cfg.count, 16);
        assert_eq!(cfg.width, 32);
        assert!(cfg.zero_register);
    }
}
