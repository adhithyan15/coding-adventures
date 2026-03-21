//! LocalMemory -- byte-addressable scratchpad with floating-point load/store.
//!
//! # What is Local Memory?
//!
//! Every GPU thread has a small, private memory area called "local memory" or
//! "scratchpad." It's used for temporary storage that doesn't fit in registers:
//! spilled variables, array elements, intermediate results.
//!
//! ```text
//! +---------------------------------------------+
//! |              Local Memory (4 KB)             |
//! +---------------------------------------------+
//! |  0x000: [42] [00] [48] [42]  <- 3.14 as FP32|
//! |  0x004: [EC] [51] [2D] [40]  <- 2.71 as FP32|
//! |  0x008: [00] [00] [00] [00]  <- 0.0          |
//! |  ...                                         |
//! |  0xFFC: [00] [00] [00] [00]                  |
//! +---------------------------------------------+
//! ```
//!
//! # How Floats Live in Memory
//!
//! A FloatBits value (sign + exponent + mantissa) must be converted to raw bytes
//! before it can be stored in memory. This is the same process that happens in
//! real hardware when a GPU core executes a STORE instruction:
//!
//! 1. Take the FloatBits fields: sign=0, exponent=[01111111], mantissa=[10010...]
//! 2. Concatenate into a bit string: `0_01111111_10010001000011111101101`
//! 3. Group into bytes: `[3F] [C9] [0F] [DB]` (that's 3.14159 in FP32)
//! 4. Write bytes to memory in little-endian order: `[DB] [0F] [C9] [3F]`
//!
//! Loading reverses this: read bytes, reassemble bits, create FloatBits.
//!
//! # Memory Sizes Across Vendors
//!
//! ```text
//! NVIDIA: 512 KB local memory per thread (rarely used, slow)
//! AMD:    Scratch memory, up to 4 MB per wavefront
//! ARM:    Stack memory region per thread
//! TPU:    No per-PE memory (data flows through systolic array)
//! ```
//!
//! Our default of 4 KB is small but sufficient for educational programs.

use std::fmt;

use fp_arithmetic::{FloatBits, FloatFormat, FP32, float_to_bits, bits_to_float};

/// Byte-addressable local scratchpad memory with FP-aware load/store.
///
/// Provides both raw byte access and convenient floating-point operations
/// that handle the conversion between FloatBits and byte sequences.
///
/// # Example
///
/// ```
/// use gpu_core::memory::LocalMemory;
///
/// let mut mem = LocalMemory::new(4096);
/// mem.store_python_float(0, 3.14);
/// let val = mem.load_float_as_f64(0);
/// assert!((val - 3.14).abs() < 0.01);
/// ```
pub struct LocalMemory {
    /// Total memory size in bytes.
    pub size: usize,
    /// The raw byte storage.
    data: Vec<u8>,
    /// The floating-point format used for load/store operations.
    pub fmt: FloatFormat,
}

impl LocalMemory {
    /// Create a new local memory of the given size, initialized to all zeros.
    ///
    /// # Panics
    ///
    /// Panics if `size` is 0.
    pub fn new(size: usize) -> Self {
        Self::with_format(size, FP32)
    }

    /// Create a new local memory with a specific floating-point format.
    ///
    /// # Panics
    ///
    /// Panics if `size` is 0.
    pub fn with_format(size: usize, fmt: FloatFormat) -> Self {
        assert!(size >= 1, "Memory size must be positive, got {}", size);
        Self {
            size,
            data: vec![0u8; size],
            fmt,
        }
    }

    /// Validate that an access is within bounds.
    fn check_bounds(&self, address: usize, num_bytes: usize) -> Result<(), String> {
        if address + num_bytes > self.size {
            Err(format!(
                "Memory access at {}:{} out of bounds [0, {})",
                address,
                address + num_bytes,
                self.size
            ))
        } else {
            Ok(())
        }
    }

    // --- Raw byte access ---

    /// Read a single byte from memory.
    pub fn read_byte(&self, address: usize) -> Result<u8, String> {
        self.check_bounds(address, 1)?;
        Ok(self.data[address])
    }

    /// Write a single byte to memory.
    pub fn write_byte(&mut self, address: usize, value: u8) -> Result<(), String> {
        self.check_bounds(address, 1)?;
        self.data[address] = value;
        Ok(())
    }

    /// Read multiple bytes from memory.
    pub fn read_bytes(&self, address: usize, count: usize) -> Result<Vec<u8>, String> {
        self.check_bounds(address, count)?;
        Ok(self.data[address..address + count].to_vec())
    }

    /// Write multiple bytes to memory.
    pub fn write_bytes(&mut self, address: usize, data: &[u8]) -> Result<(), String> {
        self.check_bounds(address, data.len())?;
        self.data[address..address + data.len()].copy_from_slice(data);
        Ok(())
    }

    // --- Floating-point access ---

    /// How many bytes a float format uses: FP32=4, FP16/BF16=2.
    fn float_byte_width(fmt: FloatFormat) -> usize {
        (fmt.total_bits / 8) as usize
    }

    /// Convert a FloatBits to raw bytes (little-endian).
    ///
    /// The process:
    /// 1. Concatenate sign + exponent + mantissa into one integer
    /// 2. Pack that integer into bytes using little-endian byte order
    ///
    /// # Example for FP32 value 1.0
    ///
    /// ```text
    /// sign=0, exponent=[0,1,1,1,1,1,1,1], mantissa=[0]*23
    /// -> bit string: 0_01111111_00000000000000000000000
    /// -> integer: 0x3F800000
    /// -> bytes (little-endian): [00, 00, 80, 3F]
    /// ```
    fn floatbits_to_bytes(value: &FloatBits) -> Vec<u8> {
        // Reassemble the bit pattern from FloatBits fields
        let mut bits: u32 = value.sign as u32;
        for &b in &value.exponent {
            bits = (bits << 1) | (b as u32);
        }
        for &b in &value.mantissa {
            bits = (bits << 1) | (b as u32);
        }

        let byte_width = Self::float_byte_width(value.fmt);
        if byte_width == 4 {
            bits.to_le_bytes().to_vec()
        } else if byte_width == 2 {
            (bits as u16).to_le_bytes().to_vec()
        } else {
            panic!("Unsupported float width: {} bytes", byte_width);
        }
    }

    /// Convert raw bytes (little-endian) back to a FloatBits.
    ///
    /// Reverses `floatbits_to_bytes`: unpack integer, split into fields.
    fn bytes_to_floatbits(data: &[u8], fmt: FloatFormat) -> FloatBits {
        let byte_width = Self::float_byte_width(fmt);
        let bits: u32 = if byte_width == 4 {
            let arr: [u8; 4] = data[..4].try_into().expect("need 4 bytes for FP32");
            u32::from_le_bytes(arr)
        } else if byte_width == 2 {
            let arr: [u8; 2] = data[..2].try_into().expect("need 2 bytes for FP16/BF16");
            u16::from_le_bytes(arr) as u32
        } else {
            panic!("Unsupported float width: {} bytes", byte_width);
        };

        let total_bits = fmt.total_bits as usize;
        let mantissa_bits = fmt.mantissa_bits as usize;
        let exponent_bits = fmt.exponent_bits as usize;

        // Mantissa is the lowest mantissa_bits bits
        let mantissa_mask = (1u32 << mantissa_bits) - 1;
        let mantissa_int = bits & mantissa_mask;
        let mantissa: Vec<u8> = (0..mantissa_bits)
            .map(|i| ((mantissa_int >> (mantissa_bits - 1 - i)) & 1) as u8)
            .collect();

        // Exponent is the next exponent_bits bits
        let exponent_mask = (1u32 << exponent_bits) - 1;
        let exponent_int = (bits >> mantissa_bits) & exponent_mask;
        let exponent: Vec<u8> = (0..exponent_bits)
            .map(|i| ((exponent_int >> (exponent_bits - 1 - i)) & 1) as u8)
            .collect();

        // Sign is the highest bit
        let sign = ((bits >> (total_bits - 1)) & 1) as u8;

        FloatBits {
            sign,
            exponent,
            mantissa,
            fmt,
        }
    }

    /// Load a floating-point value from memory.
    ///
    /// Reads the appropriate number of bytes (4 for FP32, 2 for FP16/BF16)
    /// starting at the given address, and converts them to a FloatBits.
    pub fn load_float(&self, address: usize, fmt: FloatFormat) -> Result<FloatBits, String> {
        let byte_width = Self::float_byte_width(fmt);
        let data = self.read_bytes(address, byte_width)?;
        Ok(Self::bytes_to_floatbits(&data, fmt))
    }

    /// Store a floating-point value to memory.
    ///
    /// Converts the FloatBits to bytes and writes them starting at the
    /// given address.
    pub fn store_float(&mut self, address: usize, value: &FloatBits) -> Result<(), String> {
        let data = Self::floatbits_to_bytes(value);
        self.write_bytes(address, &data)
    }

    /// Convenience: load a float and convert to f64.
    pub fn load_float_as_f64(&self, address: usize) -> f64 {
        let bits = self.load_float(address, self.fmt).expect("load_float_as_f64 failed");
        bits_to_float(&bits)
    }

    /// Convenience: store an f64 to memory.
    pub fn store_python_float(&mut self, address: usize, value: f64) {
        let bits = float_to_bits(value, self.fmt);
        self.store_float(address, &bits).expect("store_python_float failed");
    }

    /// Return a slice of memory as a list of byte values.
    ///
    /// Useful for debugging. Default shows the first 64 bytes.
    pub fn dump(&self, start: usize, length: usize) -> Vec<u8> {
        let end = std::cmp::min(start + length, self.size);
        self.data[start..end].to_vec()
    }
}

impl fmt::Debug for LocalMemory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let used: usize = self.data.iter().filter(|&&b| b != 0).count();
        write!(f, "LocalMemory({} bytes, {} non-zero)", self.size, used)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_memory() {
        let mem = LocalMemory::new(4096);
        assert_eq!(mem.size, 4096);
        for i in 0..4096 {
            assert_eq!(mem.read_byte(i).unwrap(), 0);
        }
    }

    #[test]
    #[should_panic(expected = "Memory size must be positive")]
    fn test_zero_size_panics() {
        LocalMemory::new(0);
    }

    #[test]
    fn test_read_write_byte() {
        let mut mem = LocalMemory::new(256);
        mem.write_byte(10, 0xAB).unwrap();
        assert_eq!(mem.read_byte(10).unwrap(), 0xAB);
    }

    #[test]
    fn test_out_of_bounds() {
        let mem = LocalMemory::new(16);
        assert!(mem.read_byte(16).is_err());
        assert!(mem.read_bytes(14, 4).is_err());
    }

    #[test]
    fn test_read_write_bytes() {
        let mut mem = LocalMemory::new(256);
        let data = vec![1, 2, 3, 4];
        mem.write_bytes(0, &data).unwrap();
        assert_eq!(mem.read_bytes(0, 4).unwrap(), data);
    }

    #[test]
    fn test_store_and_load_float() {
        let mut mem = LocalMemory::new(256);
        mem.store_python_float(0, 3.14);
        let val = mem.load_float_as_f64(0);
        assert!((val - 3.14).abs() < 0.001);
    }

    #[test]
    fn test_store_and_load_multiple_floats() {
        let mut mem = LocalMemory::new(256);
        mem.store_python_float(0, 1.0);
        mem.store_python_float(4, 2.0);
        mem.store_python_float(8, -3.5);
        assert_eq!(mem.load_float_as_f64(0), 1.0);
        assert_eq!(mem.load_float_as_f64(4), 2.0);
        assert_eq!(mem.load_float_as_f64(8), -3.5);
    }

    #[test]
    fn test_dump() {
        let mut mem = LocalMemory::new(256);
        mem.write_byte(0, 0xFF).unwrap();
        mem.write_byte(1, 0xAA).unwrap();
        let d = mem.dump(0, 4);
        assert_eq!(d, vec![0xFF, 0xAA, 0x00, 0x00]);
    }

    #[test]
    fn test_debug_format() {
        let mem = LocalMemory::new(64);
        let debug = format!("{:?}", mem);
        assert!(debug.contains("64 bytes"));
        assert!(debug.contains("0 non-zero"));
    }

    #[test]
    fn test_floatbits_roundtrip() {
        // Test that storing a FloatBits and loading it back gives the same value.
        let mut mem = LocalMemory::new(256);
        let original = float_to_bits(42.5, FP32);
        mem.store_float(0, &original).unwrap();
        let loaded = mem.load_float(0, FP32).unwrap();
        assert_eq!(bits_to_float(&loaded), 42.5);
    }
}
