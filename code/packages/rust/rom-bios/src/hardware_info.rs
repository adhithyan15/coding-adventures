//! HardwareInfo -- the boot protocol structure written by BIOS.
//!
//! The BIOS writes this struct at address 0x00001000 after initialization.
//! The bootloader and kernel read it to discover hardware configuration.
//!
//! Memory layout (28 bytes, all little-endian u32):
//!
//! | Offset | Field             | Default     |
//! |--------|-------------------|-------------|
//! | 0x00   | memory_size       | (probed)    |
//! | 0x04   | display_columns   | 80          |
//! | 0x08   | display_rows      | 25          |
//! | 0x0C   | framebuffer_base  | 0xFFFB0000  |
//! | 0x10   | idt_base          | 0x00000000  |
//! | 0x14   | idt_entries       | 256         |
//! | 0x18   | bootloader_entry  | 0x00010000  |

/// Fixed address where BIOS writes HardwareInfo.
pub const HARDWARE_INFO_ADDRESS: u32 = 0x00001000;

/// Size of HardwareInfo: 7 fields * 4 bytes = 28 bytes.
pub const HARDWARE_INFO_SIZE: usize = 28;

/// Hardware configuration discovered and set by the BIOS.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HardwareInfo {
    pub memory_size: u32,
    pub display_columns: u32,
    pub display_rows: u32,
    pub framebuffer_base: u32,
    pub idt_base: u32,
    pub idt_entries: u32,
    pub bootloader_entry: u32,
}

impl Default for HardwareInfo {
    fn default() -> Self {
        Self {
            memory_size: 0,
            display_columns: 80,
            display_rows: 25,
            framebuffer_base: 0xFFFB0000,
            idt_base: 0x00000000,
            idt_entries: 256,
            bootloader_entry: 0x00010000,
        }
    }
}

impl HardwareInfo {
    /// Serialize to 28-byte little-endian buffer.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(HARDWARE_INFO_SIZE);
        for &val in &[
            self.memory_size, self.display_columns, self.display_rows,
            self.framebuffer_base, self.idt_base, self.idt_entries,
            self.bootloader_entry,
        ] {
            buf.extend_from_slice(&val.to_le_bytes());
        }
        buf
    }

    /// Deserialize from a byte slice (must be >= 28 bytes).
    pub fn from_bytes(data: &[u8]) -> Self {
        assert!(data.len() >= HARDWARE_INFO_SIZE, "data too short for HardwareInfo");
        let read_u32 = |offset: usize| -> u32 {
            u32::from_le_bytes([data[offset], data[offset+1], data[offset+2], data[offset+3]])
        };
        Self {
            memory_size: read_u32(0),
            display_columns: read_u32(4),
            display_rows: read_u32(8),
            framebuffer_base: read_u32(12),
            idt_base: read_u32(16),
            idt_entries: read_u32(20),
            bootloader_entry: read_u32(24),
        }
    }
}
