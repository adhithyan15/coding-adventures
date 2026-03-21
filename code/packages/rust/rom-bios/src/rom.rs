//! ROM (Read-Only Memory) implementation.
//!
//! ROM is a memory region where writes are silently ignored. Real computers
//! have a ROM chip containing firmware that the CPU executes on power-on.
//! The program counter starts at the ROM's base address (0xFFFF0000).
//!
//! Analogy: ROM is like a recipe card laminated in plastic -- you can read
//! it any number of times, but you cannot write on it.

/// Default base address: top of 32-bit address space minus 64 KB.
pub const DEFAULT_ROM_BASE: u32 = 0xFFFF0000;

/// Default ROM size: 64 KB (65536 bytes).
pub const DEFAULT_ROM_SIZE: usize = 65536;

/// Configuration for the ROM memory region.
#[derive(Debug, Clone)]
pub struct RomConfig {
    pub base_address: u32,
    pub size: usize,
}

impl Default for RomConfig {
    fn default() -> Self {
        Self {
            base_address: DEFAULT_ROM_BASE,
            size: DEFAULT_ROM_SIZE,
        }
    }
}

/// Read-only memory region. Writes are silently ignored.
///
/// # Example
/// ```
/// use rom_bios::rom::{Rom, RomConfig};
///
/// let rom = Rom::new(RomConfig::default(), &[0xAA, 0xBB, 0xCC, 0xDD]);
/// assert_eq!(rom.read(0xFFFF0000), 0xAA);
/// ```
pub struct Rom {
    config: RomConfig,
    data: Vec<u8>,
}

impl Rom {
    /// Create a ROM loaded with the given firmware bytes.
    ///
    /// If firmware is shorter than config.size, remaining bytes are zero-filled.
    /// Panics if firmware is larger than config.size.
    pub fn new(config: RomConfig, firmware: &[u8]) -> Self {
        assert!(firmware.len() <= config.size, "firmware larger than ROM size");
        let mut data = vec![0u8; config.size];
        data[..firmware.len()].copy_from_slice(firmware);
        Self { config, data }
    }

    /// Read a single byte from the given absolute address.
    /// Out-of-range addresses return 0.
    pub fn read(&self, address: u32) -> u8 {
        match self.address_to_offset(address) {
            Some(offset) => self.data[offset],
            None => 0,
        }
    }

    /// Read a 32-bit little-endian word at the given absolute address.
    pub fn read_word(&self, address: u32) -> u32 {
        match self.address_to_offset(address) {
            Some(offset) if offset + 3 < self.data.len() => {
                u32::from_le_bytes([
                    self.data[offset],
                    self.data[offset + 1],
                    self.data[offset + 2],
                    self.data[offset + 3],
                ])
            }
            _ => 0,
        }
    }

    /// Attempt to write a byte to ROM (silently ignored).
    pub fn write(&self, _address: u32, _value: u8) {
        // ROM is read-only. Silently ignored.
    }

    /// Total size of ROM in bytes.
    pub fn size(&self) -> usize {
        self.config.size
    }

    /// Base address of ROM.
    pub fn base_address(&self) -> u32 {
        self.config.base_address
    }

    /// True if address falls within the ROM region.
    pub fn contains(&self, address: u32) -> bool {
        self.address_to_offset(address).is_some()
    }

    fn address_to_offset(&self, address: u32) -> Option<usize> {
        if address < self.config.base_address {
            return None;
        }
        let offset = (address - self.config.base_address) as usize;
        if offset >= self.config.size {
            return None;
        }
        Some(offset)
    }
}
