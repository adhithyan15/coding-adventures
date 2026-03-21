//! # CPU Simulator -- minimal types for ISA simulators
//!
//! This crate provides the foundational types that ISA-specific simulators
//! (like RISC-V) build upon:
//!
//! - `RegisterFile`: A fixed-size array of 32-bit registers
//! - `Memory`: Byte-addressable memory with word read/write support
//! - `SparseMemory`: Address-range-mapped memory for full 32-bit address spaces
//!
//! These are deliberately simple -- just data containers with no instruction
//! fetching or pipeline logic. The ISA simulator handles decode and execute.

pub mod sparse_memory;

/// A fixed-size array of 32-bit registers.
///
/// Register 0 can optionally be hardwired to zero (RISC-V convention).
/// Reads from x0 always return 0; writes to x0 are silently discarded.
pub struct RegisterFile {
    regs: Vec<u32>,
    zero_register: bool,
}

impl RegisterFile {
    /// Create a new register file with `count` registers.
    /// If `zero_register` is true, register 0 is hardwired to zero.
    pub fn new(count: usize, zero_register: bool) -> Self {
        Self {
            regs: vec![0u32; count],
            zero_register,
        }
    }

    /// Read a register value. Returns 0 for out-of-range or if index is 0
    /// and zero_register is enabled.
    pub fn read(&self, index: usize) -> u32 {
        if self.zero_register && index == 0 {
            return 0;
        }
        self.regs.get(index).copied().unwrap_or(0)
    }

    /// Write a value to a register. Writes to x0 are silently ignored
    /// when zero_register is enabled.
    pub fn write(&mut self, index: usize, value: u32) {
        if self.zero_register && index == 0 {
            return;
        }
        if index < self.regs.len() {
            self.regs[index] = value;
        }
    }
}

/// Byte-addressable memory with little-endian word operations.
///
/// Memory is a flat array of bytes, like real RAM. Multi-byte values
/// are stored in little-endian order (least significant byte first),
/// matching RISC-V and x86 conventions.
pub struct Memory {
    data: Vec<u8>,
}

impl Memory {
    /// Create memory of the specified size (in bytes), initialized to zero.
    pub fn new(size: usize) -> Self {
        Self {
            data: vec![0u8; size],
        }
    }

    /// Read a single byte from the given address.
    pub fn read_byte(&self, addr: usize) -> u8 {
        self.data.get(addr).copied().unwrap_or(0)
    }

    /// Write a single byte to the given address.
    pub fn write_byte(&mut self, addr: usize, value: u8) {
        if addr < self.data.len() {
            self.data[addr] = value;
        }
    }

    /// Read a 32-bit word (little-endian) from the given address.
    pub fn read_word(&self, addr: usize) -> u32 {
        let b0 = self.read_byte(addr) as u32;
        let b1 = self.read_byte(addr + 1) as u32;
        let b2 = self.read_byte(addr + 2) as u32;
        let b3 = self.read_byte(addr + 3) as u32;
        b0 | (b1 << 8) | (b2 << 16) | (b3 << 24)
    }

    /// Write a 32-bit word (little-endian) to the given address.
    pub fn write_word(&mut self, addr: usize, value: u32) {
        self.write_byte(addr, (value & 0xFF) as u8);
        self.write_byte(addr + 1, ((value >> 8) & 0xFF) as u8);
        self.write_byte(addr + 2, ((value >> 16) & 0xFF) as u8);
        self.write_byte(addr + 3, ((value >> 24) & 0xFF) as u8);
    }

    /// Load a byte slice into memory starting at the given address.
    pub fn load_bytes(&mut self, addr: usize, data: &[u8]) {
        for (i, &byte) in data.iter().enumerate() {
            self.write_byte(addr + i, byte);
        }
    }

    /// Return the size of memory in bytes.
    pub fn size(&self) -> usize {
        self.data.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_register_file_basic() {
        let mut regs = RegisterFile::new(32, false);
        regs.write(5, 42);
        assert_eq!(regs.read(5), 42);
    }

    #[test]
    fn test_register_file_zero_register() {
        let mut regs = RegisterFile::new(32, true);
        regs.write(0, 42);
        assert_eq!(regs.read(0), 0);
        regs.write(1, 100);
        assert_eq!(regs.read(1), 100);
    }

    #[test]
    fn test_memory_byte() {
        let mut mem = Memory::new(256);
        mem.write_byte(10, 0xAB);
        assert_eq!(mem.read_byte(10), 0xAB);
    }

    #[test]
    fn test_memory_word() {
        let mut mem = Memory::new(256);
        mem.write_word(0, 0x12345678);
        assert_eq!(mem.read_word(0), 0x12345678);
        // Little-endian check
        assert_eq!(mem.read_byte(0), 0x78);
        assert_eq!(mem.read_byte(1), 0x56);
        assert_eq!(mem.read_byte(2), 0x34);
        assert_eq!(mem.read_byte(3), 0x12);
    }

    #[test]
    fn test_memory_load_bytes() {
        let mut mem = Memory::new(256);
        mem.load_bytes(0, &[0xDE, 0xAD, 0xBE, 0xEF]);
        assert_eq!(mem.read_byte(0), 0xDE);
        assert_eq!(mem.read_byte(3), 0xEF);
    }
}
