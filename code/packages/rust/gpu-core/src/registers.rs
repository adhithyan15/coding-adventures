//! FPRegisterFile -- floating-point register storage for GPU cores.
//!
//! # What is a Register File?
//!
//! A register file is the fastest storage in a processor -- faster than cache,
//! faster than RAM. It's where the processor keeps the values it's currently
//! working with. Think of it like the handful of numbers you can keep in your
//! head while doing mental math.
//!
//! ```text
//! Register file (in your head):
//!     "first number"  = 3.14
//!     "second number" = 2.71
//!     "result"        = ???
//!
//! Register file (in a GPU core):
//!     R0  = 3.14  (FloatBits: sign=0, exp=[...], mantissa=[...])
//!     R1  = 2.71  (FloatBits: sign=0, exp=[...], mantissa=[...])
//!     R2  = 0.00  (will hold the result)
//! ```
//!
//! # GPU vs CPU Register Files
//!
//! CPU registers hold integers (32 or 64 bits of binary). GPU registers hold
//! floating-point numbers (IEEE 754 FloatBits). This reflects their different
//! purposes:
//!
//! ```text
//! CPU: general-purpose computation (loops, pointers, addresses -> integers)
//! GPU: parallel numeric computation (vertices, pixels, gradients -> floats)
//! ```
//!
//! # Why Configurable?
//!
//! Different GPU vendors use different register counts:
//!
//! ```text
//! NVIDIA CUDA Core:    up to 255 registers per thread
//! AMD Stream Processor: 256 VGPRs (Vector General Purpose Registers)
//! Intel Vector Engine:  128 GRF entries (General Register File)
//! ARM Mali:            64 registers per thread
//! ```
//!
//! By making the register count a constructor parameter, the same GPUCore
//! struct can simulate any vendor's register architecture.
//!
//! # Register File Diagram
//!
//! ```text
//! +---------------------------------------------+
//! |           FP Register File                   |
//! |         (32 registers x FP32)                |
//! +---------------------------------------------+
//! |  R0:  [0][01111111][00000000000...0]  = +1.0 |
//! |  R1:  [0][10000000][00000000000...0]  = +2.0 |
//! |  R2:  [0][00000000][00000000000...0]  = +0.0 |
//! |  ...                                         |
//! |  R31: [0][00000000][00000000000...0]  = +0.0 |
//! +---------------------------------------------+
//! ```

use std::collections::HashMap;
use std::fmt;

use fp_arithmetic::{FloatBits, FloatFormat, FP32, float_to_bits, bits_to_float};

/// A configurable floating-point register file.
///
/// Stores [`FloatBits`] values (from the `fp-arithmetic` crate) in a fixed
/// number of registers. Provides both raw [`FloatBits`] and convenience `f64`
/// interfaces for reading and writing.
///
/// # Arguments
///
/// - `num_registers`: How many registers (1-256).
/// - `fmt`: The floating-point format (FP32, FP16, BF16).
///
/// # Example
///
/// ```
/// use fp_arithmetic::FP32;
/// use gpu_core::registers::FPRegisterFile;
///
/// let mut regs = FPRegisterFile::new(32, FP32);
/// regs.write_float(0, 3.14);
/// assert!((regs.read_float(0) - 3.14).abs() < 0.01);
/// ```
pub struct FPRegisterFile {
    /// The number of registers in this file.
    pub num_registers: usize,
    /// The floating-point format used by all registers.
    pub fmt: FloatFormat,
    /// The register storage. Each entry is a FloatBits value.
    values: Vec<FloatBits>,
}

impl FPRegisterFile {
    /// Create a new register file with all registers initialized to +0.0.
    ///
    /// # Panics
    ///
    /// Panics if `num_registers` is 0 or greater than 256.
    pub fn new(num_registers: usize, fmt: FloatFormat) -> Self {
        assert!(
            num_registers >= 1 && num_registers <= 256,
            "num_registers must be 1-256, got {}",
            num_registers
        );
        let zero = float_to_bits(0.0, fmt);
        let values = vec![zero; num_registers];
        Self {
            num_registers,
            fmt,
            values,
        }
    }

    /// Validate a register index, returning an error string if out of bounds.
    fn check_index(&self, index: usize) -> Result<(), String> {
        if index >= self.num_registers {
            Err(format!(
                "Register index {} out of range [0, {}]",
                index,
                self.num_registers - 1
            ))
        } else {
            Ok(())
        }
    }

    /// Read a register as a [`FloatBits`] value.
    ///
    /// # Errors
    ///
    /// Returns an error if `index` is out of range.
    pub fn read(&self, index: usize) -> Result<FloatBits, String> {
        self.check_index(index)?;
        Ok(self.values[index].clone())
    }

    /// Write a [`FloatBits`] value to a register.
    ///
    /// # Errors
    ///
    /// Returns an error if `index` is out of range.
    pub fn write(&mut self, index: usize, value: FloatBits) -> Result<(), String> {
        self.check_index(index)?;
        self.values[index] = value;
        Ok(())
    }

    /// Convenience: read a register as an `f64`.
    ///
    /// This decodes the FloatBits back to a float, which is useful for
    /// inspection and testing but loses the bit-level detail.
    pub fn read_float(&self, index: usize) -> f64 {
        bits_to_float(&self.values[index])
    }

    /// Convenience: write an `f64` to a register.
    ///
    /// This encodes the float as FloatBits in the register file's format,
    /// then stores it. Useful for setting up test inputs.
    pub fn write_float(&mut self, index: usize, value: f64) {
        self.values[index] = float_to_bits(value, self.fmt);
    }

    /// Return all non-zero register values as a map of "R{n}" to f64.
    ///
    /// Useful for debugging and test assertions. Only includes non-zero
    /// registers to reduce noise.
    pub fn dump(&self) -> HashMap<String, f64> {
        let mut result = HashMap::new();
        for i in 0..self.num_registers {
            let val = bits_to_float(&self.values[i]);
            if val != 0.0 {
                result.insert(format!("R{}", i), val);
            }
        }
        result
    }

    /// Return ALL register values as a map of "R{n}" to f64.
    ///
    /// Unlike [`dump()`](Self::dump), this includes zero-valued registers.
    pub fn dump_all(&self) -> HashMap<String, f64> {
        let mut result = HashMap::new();
        for i in 0..self.num_registers {
            result.insert(format!("R{}", i), bits_to_float(&self.values[i]));
        }
        result
    }
}

impl fmt::Debug for FPRegisterFile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let non_zero = self.dump();
        if non_zero.is_empty() {
            write!(f, "FPRegisterFile({} regs, all zero)", self.num_registers)
        } else {
            let entries: Vec<String> = non_zero
                .iter()
                .map(|(k, v)| format!("{}={}", k, v))
                .collect();
            write!(f, "FPRegisterFile({})", entries.join(", "))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_register_file() {
        let regs = FPRegisterFile::new(32, FP32);
        assert_eq!(regs.num_registers, 32);
        for i in 0..32 {
            assert_eq!(regs.read_float(i), 0.0);
        }
    }

    #[test]
    #[should_panic(expected = "num_registers must be 1-256")]
    fn test_zero_registers_panics() {
        FPRegisterFile::new(0, FP32);
    }

    #[test]
    #[should_panic(expected = "num_registers must be 1-256")]
    fn test_too_many_registers_panics() {
        FPRegisterFile::new(257, FP32);
    }

    #[test]
    fn test_write_and_read_float() {
        let mut regs = FPRegisterFile::new(8, FP32);
        regs.write_float(0, 3.14);
        regs.write_float(1, -2.5);
        // FP32 has limited precision, so we check approximate equality
        assert!((regs.read_float(0) - 3.14).abs() < 0.001);
        assert!((regs.read_float(1) - (-2.5)).abs() < 0.001);
    }

    #[test]
    fn test_write_and_read_bits() {
        let mut regs = FPRegisterFile::new(8, FP32);
        let bits = float_to_bits(42.0, FP32);
        regs.write(3, bits).unwrap();
        let read_back = regs.read(3).unwrap();
        assert_eq!(bits_to_float(&read_back), 42.0);
    }

    #[test]
    fn test_out_of_bounds_read() {
        let regs = FPRegisterFile::new(4, FP32);
        assert!(regs.read(4).is_err());
        assert!(regs.read(100).is_err());
    }

    #[test]
    fn test_out_of_bounds_write() {
        let mut regs = FPRegisterFile::new(4, FP32);
        let bits = float_to_bits(1.0, FP32);
        assert!(regs.write(4, bits).is_err());
    }

    #[test]
    fn test_dump_non_zero() {
        let mut regs = FPRegisterFile::new(8, FP32);
        regs.write_float(2, 5.0);
        regs.write_float(5, -1.0);
        let d = regs.dump();
        assert_eq!(d.len(), 2);
        assert_eq!(*d.get("R2").unwrap(), 5.0);
        assert_eq!(*d.get("R5").unwrap(), -1.0);
    }

    #[test]
    fn test_dump_all() {
        let regs = FPRegisterFile::new(4, FP32);
        let d = regs.dump_all();
        assert_eq!(d.len(), 4);
        for i in 0..4 {
            assert_eq!(*d.get(&format!("R{}", i)).unwrap(), 0.0);
        }
    }

    #[test]
    fn test_debug_format() {
        let regs = FPRegisterFile::new(4, FP32);
        let debug = format!("{:?}", regs);
        assert!(debug.contains("all zero"));

        let mut regs2 = FPRegisterFile::new(4, FP32);
        regs2.write_float(0, 1.0);
        let debug2 = format!("{:?}", regs2);
        assert!(debug2.contains("R0=1"));
    }
}
