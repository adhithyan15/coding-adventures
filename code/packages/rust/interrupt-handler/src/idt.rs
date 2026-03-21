//! Interrupt Descriptor Table (IDT) — maps interrupt numbers to ISR addresses.
//!
//! The IDT is an array of 256 entries stored at address 0x00000000 in memory.
//! Each entry maps an interrupt number to the address of its handler (ISR).
//!
//! # Binary format (8 bytes per entry)
//!
//! ```text
//! Bytes 0-3: ISR address (little-endian uint32)
//! Byte 4:    Present (0x00 or 0x01)
//! Byte 5:    Privilege level (uint8)
//! Bytes 6-7: Reserved (0x00, 0x00)
//! ```

/// Each IDT entry occupies 8 bytes in memory.
pub const IDT_ENTRY_SIZE: usize = 8;

/// Total IDT size: 256 entries * 8 bytes = 2048 bytes.
pub const IDT_SIZE: usize = 256 * IDT_ENTRY_SIZE;

/// Default memory location of the IDT.
pub const IDT_BASE_ADDRESS: u32 = 0x00000000;

/// One row in the Interrupt Descriptor Table.
///
/// - `isr_address`: Where the CPU jumps when this interrupt fires.
/// - `present`: `true` if this entry is valid. `false` triggers double fault.
/// - `privilege_level`: 0 = kernel only.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IDTEntry {
    pub isr_address: u32,
    pub present: bool,
    pub privilege_level: u8,
}

impl Default for IDTEntry {
    fn default() -> Self {
        Self {
            isr_address: 0,
            present: false,
            privilege_level: 0,
        }
    }
}

/// 256-entry table mapping interrupt numbers to ISR addresses.
///
/// Why 256 entries? Matches x86 convention:
/// - 0-31: CPU exceptions
/// - 32-47: Hardware device interrupts
/// - 128: System call (ecall)
pub struct InterruptDescriptorTable {
    pub entries: [IDTEntry; 256],
}

impl InterruptDescriptorTable {
    /// Create a new IDT with all 256 entries marked as not present.
    pub fn new() -> Self {
        Self {
            entries: [IDTEntry::default(); 256],
        }
    }

    /// Install a handler at the given interrupt number (0-255).
    ///
    /// # Panics
    ///
    /// Panics if `number` is out of range.
    pub fn set_entry(&mut self, number: usize, entry: IDTEntry) {
        assert!(number < 256, "IDT entry number must be 0-255");
        self.entries[number] = entry;
    }

    /// Return the entry for the given interrupt number (0-255).
    ///
    /// # Panics
    ///
    /// Panics if `number` is out of range.
    pub fn get_entry(&self, number: usize) -> IDTEntry {
        assert!(number < 256, "IDT entry number must be 0-255");
        self.entries[number]
    }

    /// Serialize the IDT into a byte slice at the given base address.
    /// Uses little-endian format (RISC-V convention).
    pub fn write_to_memory(&self, memory: &mut [u8], base_address: usize) {
        for i in 0..256 {
            let offset = base_address + i * IDT_ENTRY_SIZE;
            let entry = &self.entries[i];

            // Bytes 0-3: ISR address (little-endian)
            let addr_bytes = entry.isr_address.to_le_bytes();
            memory[offset..offset + 4].copy_from_slice(&addr_bytes);

            // Byte 4: Present bit
            memory[offset + 4] = if entry.present { 0x01 } else { 0x00 };

            // Byte 5: Privilege level
            memory[offset + 5] = entry.privilege_level;

            // Bytes 6-7: Reserved
            memory[offset + 6] = 0x00;
            memory[offset + 7] = 0x00;
        }
    }

    /// Deserialize the IDT from a byte slice at the given base address.
    pub fn load_from_memory(&mut self, memory: &[u8], base_address: usize) {
        for i in 0..256 {
            let offset = base_address + i * IDT_ENTRY_SIZE;

            // Bytes 0-3: ISR address (little-endian)
            let isr_address = u32::from_le_bytes([
                memory[offset],
                memory[offset + 1],
                memory[offset + 2],
                memory[offset + 3],
            ]);

            // Byte 4: Present bit
            let present = memory[offset + 4] != 0x00;

            // Byte 5: Privilege level
            let privilege_level = memory[offset + 5];

            self.entries[i] = IDTEntry {
                isr_address,
                present,
                privilege_level,
            };
        }
    }
}

impl Default for InterruptDescriptorTable {
    fn default() -> Self {
        Self::new()
    }
}
