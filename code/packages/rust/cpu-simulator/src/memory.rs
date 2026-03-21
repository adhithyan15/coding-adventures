//! # Memory -- the CPU's large, slow storage.
//!
//! ## What is memory?
//!
//! Memory (RAM -- Random Access Memory) is a large array of bytes that the CPU
//! can read from and write to. Unlike registers (which are tiny and fast),
//! memory can hold megabytes or gigabytes of data, but accessing it takes
//! many clock cycles.
//!
//! Every byte in memory has an "address" -- a number that identifies its
//! location, like a house number on a street. To read a byte, you tell the
//! memory controller "give me the byte at address 42." To write, you say
//! "put the value 7 at address 42."
//!
//! ## Memory in our simulator
//!
//! We simulate memory as a `Vec<u8>` -- a growable array of unsigned bytes.
//! Each element is one byte (0-255). Multi-byte values (like 32-bit integers)
//! are stored in consecutive bytes.
//!
//! ## Byte ordering (Endianness)
//!
//! When storing a multi-byte value (like the 32-bit integer `0x12345678`),
//! there are two ways to lay out the bytes:
//!
//! ```text
//!   Big-endian:    [0x12] [0x34] [0x56] [0x78]   (most significant byte first)
//!   Little-endian: [0x78] [0x56] [0x34] [0x12]   (least significant byte first)
//! ```
//!
//! RISC-V and x86 use little-endian. ARM supports both. Our simulator
//! defaults to little-endian because that's what RISC-V uses.
//!
//! Think of it like writing the number 1234:
//!
//! - **Big-endian** is like English: you write the thousands digit first (1, 2, 3, 4)
//! - **Little-endian** is the opposite: ones digit first (4, 3, 2, 1)

/// Byte-addressable memory.
///
/// Memory is a flat array of bytes. Each byte is addressed by an integer
/// starting from 0.
///
/// ```text
///     Address:  0     1     2     3     4     5    ...
///     Value:   [00]  [00]  [00]  [00]  [00]  [00]  ...
/// ```
///
/// # Example
///
/// ```
/// use cpu_simulator::Memory;
///
/// let mut mem = Memory::new(1024); // 1 KB of memory
/// mem.write_byte(0, 42);
/// assert_eq!(mem.read_byte(0), 42);
/// mem.write_word(4, 0x12345678); // Write a 32-bit value
/// assert_eq!(mem.read_word(4), 0x12345678);
/// ```
#[derive(Debug, Clone)]
pub struct Memory {
    /// Internal byte storage.
    data: Vec<u8>,

    /// Total number of bytes in this memory.
    pub size: usize,
}

impl Memory {
    /// Create a memory of `size` bytes, all initialized to 0.
    ///
    /// # Panics
    ///
    /// Panics if `size` is 0. You need at least 1 byte.
    pub fn new(size: usize) -> Self {
        assert!(size >= 1, "Memory size must be at least 1 byte");
        Memory {
            data: vec![0u8; size],
            size,
        }
    }

    /// Verify an address range is within bounds.
    ///
    /// This is the single guard that every read/write method calls.
    /// If the access would go past the end of memory, we panic with
    /// a clear error message showing the address, access size, and
    /// total memory size.
    fn check_address(&self, address: usize, num_bytes: usize) {
        assert!(
            address + num_bytes <= self.size,
            "Memory access out of bounds: address {}, size {}, memory size {}",
            address,
            num_bytes,
            self.size
        );
    }

    /// Read a single byte (8 bits, value 0-255) from memory.
    ///
    /// # Panics
    ///
    /// Panics if `address` is out of bounds.
    ///
    /// # Example
    ///
    /// ```
    /// use cpu_simulator::Memory;
    ///
    /// let mut mem = Memory::new(16);
    /// mem.write_byte(3, 0xFF);
    /// assert_eq!(mem.read_byte(3), 255);
    /// ```
    pub fn read_byte(&self, address: usize) -> u8 {
        self.check_address(address, 1);
        self.data[address]
    }

    /// Write a single byte to memory.
    ///
    /// # Panics
    ///
    /// Panics if `address` is out of bounds.
    ///
    /// # Example
    ///
    /// ```
    /// use cpu_simulator::Memory;
    ///
    /// let mut mem = Memory::new(16);
    /// mem.write_byte(0, 42);
    /// assert_eq!(mem.read_byte(0), 42);
    /// ```
    pub fn write_byte(&mut self, address: usize, value: u8) {
        self.check_address(address, 1);
        self.data[address] = value;
    }

    /// Read a 32-bit word (4 bytes) from memory, little-endian.
    ///
    /// Little-endian means the least significant byte is at the lowest
    /// address. For example, the value `0x12345678` is stored as:
    ///
    /// ```text
    ///     Address:   [addr]  [addr+1]  [addr+2]  [addr+3]
    ///     Value:      0x78    0x56      0x34      0x12
    ///                 ^^^^                        ^^^^
    ///                 LSB (least significant)     MSB (most significant)
    /// ```
    ///
    /// # Panics
    ///
    /// Panics if `address + 4` exceeds memory size.
    ///
    /// # Example
    ///
    /// ```
    /// use cpu_simulator::Memory;
    ///
    /// let mut mem = Memory::new(16);
    /// mem.write_word(0, 0x12345678);
    /// assert_eq!(mem.read_word(0), 0x12345678);
    /// ```
    pub fn read_word(&self, address: usize) -> u32 {
        self.check_address(address, 4);
        // Assemble 4 bytes into a u32, little-endian:
        //   byte[0] is bits  0..7  (least significant)
        //   byte[1] is bits  8..15
        //   byte[2] is bits 16..23
        //   byte[3] is bits 24..31 (most significant)
        u32::from(self.data[address])
            | (u32::from(self.data[address + 1]) << 8)
            | (u32::from(self.data[address + 2]) << 16)
            | (u32::from(self.data[address + 3]) << 24)
    }

    /// Write a 32-bit word to memory, little-endian.
    ///
    /// The 32-bit value is split into 4 bytes and stored with the
    /// least significant byte at the lowest address.
    ///
    /// # Panics
    ///
    /// Panics if `address + 4` exceeds memory size.
    ///
    /// # Example
    ///
    /// ```
    /// use cpu_simulator::Memory;
    ///
    /// let mut mem = Memory::new(16);
    /// mem.write_word(0, 3);        // 3 = 0x00000003
    /// assert_eq!(mem.read_byte(0), 3);   // LSB
    /// assert_eq!(mem.read_byte(1), 0);   // next byte
    /// ```
    pub fn write_word(&mut self, address: usize, value: u32) {
        self.check_address(address, 4);
        // Split the 32-bit value into 4 bytes, little-endian.
        // Rust's u32 is unsigned, so right-shift fills with zeros -- no
        // sign-extension surprises (unlike JavaScript's >> operator).
        self.data[address] = (value & 0xFF) as u8;
        self.data[address + 1] = ((value >> 8) & 0xFF) as u8;
        self.data[address + 2] = ((value >> 16) & 0xFF) as u8;
        self.data[address + 3] = ((value >> 24) & 0xFF) as u8;
    }

    /// Load a sequence of bytes into memory starting at `address`.
    ///
    /// This is how programs are loaded: the machine code bytes are copied
    /// into memory starting at address 0 (or wherever the program begins).
    ///
    /// # Panics
    ///
    /// Panics if the data would extend past the end of memory.
    ///
    /// # Example
    ///
    /// ```
    /// use cpu_simulator::Memory;
    ///
    /// let mut mem = Memory::new(16);
    /// mem.load_bytes(0, &[0x01, 0x02, 0x03]);
    /// assert_eq!(mem.read_byte(0), 1);
    /// assert_eq!(mem.read_byte(1), 2);
    /// assert_eq!(mem.read_byte(2), 3);
    /// ```
    pub fn load_bytes(&mut self, address: usize, data: &[u8]) {
        self.check_address(address, data.len());
        self.data[address..address + data.len()].copy_from_slice(data);
    }

    /// Return a slice of memory as a vector of byte values (copied).
    ///
    /// Useful for debugging -- see what's stored in a range of addresses.
    ///
    /// # Panics
    ///
    /// Panics if the range extends past the end of memory.
    ///
    /// # Example
    ///
    /// ```
    /// use cpu_simulator::Memory;
    ///
    /// let mut mem = Memory::new(16);
    /// mem.write_byte(0, 0xAB);
    /// let dump = mem.dump(0, 4);
    /// assert_eq!(dump, vec![0xAB, 0, 0, 0]);
    /// ```
    pub fn dump(&self, start: usize, length: usize) -> Vec<u8> {
        self.check_address(start, length);
        self.data[start..start + length].to_vec()
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[should_panic(expected = "Memory size must be at least 1 byte")]
    fn new_memory_zero_size_panics() {
        Memory::new(0);
    }

    #[test]
    #[should_panic(expected = "Memory access out of bounds")]
    fn read_byte_out_of_bounds_panics() {
        let mem = Memory::new(10);
        mem.read_byte(10);
    }

    #[test]
    fn read_write_byte() {
        let mut mem = Memory::new(16);
        mem.write_byte(5, 255);
        assert_eq!(mem.read_byte(5), 255);
    }

    #[test]
    fn load_bytes_and_read_back() {
        let mut mem = Memory::new(16);
        mem.load_bytes(0, &[1, 2, 3]);
        assert_eq!(mem.read_byte(0), 1);
        assert_eq!(mem.read_byte(1), 2);
        assert_eq!(mem.read_byte(2), 3);
    }

    #[test]
    fn dump_returns_copy_of_range() {
        let mut mem = Memory::new(16);
        mem.load_bytes(0, &[1, 2, 3]);
        let dump = mem.dump(0, 3);
        assert_eq!(dump, vec![1, 2, 3]);
    }

    #[test]
    fn little_endian_word_storage() {
        // Writing 0x12345678 in little-endian should store:
        //   address+0: 0x78 (LSB)
        //   address+1: 0x56
        //   address+2: 0x34
        //   address+3: 0x12 (MSB)
        let mut mem = Memory::new(16);
        mem.write_word(0, 0x12345678);

        assert_eq!(mem.read_byte(0), 0x78, "LSB at address 0");
        assert_eq!(mem.read_byte(1), 0x56, "byte 1");
        assert_eq!(mem.read_byte(2), 0x34, "byte 2");
        assert_eq!(mem.read_byte(3), 0x12, "MSB at address 3");

        // Round-trip: reading the word back should give the original value.
        assert_eq!(mem.read_word(0), 0x12345678);
    }

    #[test]
    fn word_read_write_round_trip() {
        let mut mem = Memory::new(16);
        mem.write_word(4, 0xDEADBEEF);
        assert_eq!(mem.read_word(4), 0xDEADBEEF);
    }

    #[test]
    fn memory_initialized_to_zero() {
        let mem = Memory::new(8);
        for i in 0..8 {
            assert_eq!(mem.read_byte(i), 0, "byte {} should start at 0", i);
        }
    }

    #[test]
    #[should_panic(expected = "Memory access out of bounds")]
    fn write_word_out_of_bounds_panics() {
        let mut mem = Memory::new(6);
        // Address 4 needs 4 bytes (4..8), but memory only has 6 bytes.
        mem.write_word(4, 0);
    }

    #[test]
    #[should_panic(expected = "Memory access out of bounds")]
    fn load_bytes_out_of_bounds_panics() {
        let mut mem = Memory::new(4);
        mem.load_bytes(2, &[1, 2, 3]); // needs 3 bytes at offset 2 => 5 total
    }
}
