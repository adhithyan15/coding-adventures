//! Sparse Memory -- simulating a 32-bit address space without 4 GB.
//!
//! A real 32-bit CPU can address 4 GB of memory (2^32 bytes). But most of
//! that address space is empty. SparseMemory solves this by mapping only
//! the regions that actually exist. Each region is a named Vec<u8> at a
//! specific base address. Reads and writes are dispatched to the correct
//! region by checking address ranges.
//!
//! # Read-only regions
//!
//! Some regions should never be written to (ROM, read-only status registers).
//! When a region is marked `read_only`, writes are silently ignored, matching
//! real hardware where writing to ROM has no effect.

/// A contiguous block of addressable memory.
///
/// Each region has a base address, a size, and a backing byte vector.
/// The region occupies addresses `[base, base + size)`. Any access within
/// this range is translated to an offset into the data vector.
pub struct MemoryRegion {
    /// Starting address in the 32-bit address space.
    pub base: u32,
    /// Number of bytes in this region.
    pub size: u32,
    /// Backing storage.
    pub data: Vec<u8>,
    /// Human-readable label for debugging.
    pub name: String,
    /// When true, writes are silently discarded.
    pub read_only: bool,
}

impl MemoryRegion {
    /// Create a new region with zeroed data.
    pub fn new(base: u32, size: u32, name: &str, read_only: bool) -> Self {
        Self {
            base,
            size,
            data: vec![0u8; size as usize],
            name: name.to_string(),
            read_only,
        }
    }

    /// Create a new region with pre-populated data.
    pub fn with_data(base: u32, data: Vec<u8>, name: &str, read_only: bool) -> Self {
        let size = data.len() as u32;
        Self {
            base,
            size,
            data,
            name: name.to_string(),
            read_only,
        }
    }
}

/// Maps address ranges to backing byte vectors, enabling a full 32-bit
/// address space without allocating 4 GB.
///
/// On every access, SparseMemory searches through its regions to find one
/// that contains the target address. If no region contains the address,
/// the access panics (modeling a bus fault).
pub struct SparseMemory {
    regions: Vec<MemoryRegion>,
}

impl SparseMemory {
    /// Create a SparseMemory from a list of region definitions.
    pub fn new(regions: Vec<MemoryRegion>) -> Self {
        Self { regions }
    }

    /// Locate the region containing `[address, address + num_bytes)`.
    /// Returns (region_index, offset). Panics if unmapped.
    fn find_region(&self, address: u32, num_bytes: usize) -> (usize, usize) {
        let end = address as u64 + num_bytes as u64;
        for (i, r) in self.regions.iter().enumerate() {
            let region_end = r.base as u64 + r.size as u64;
            if address as u64 >= r.base as u64 && end <= region_end {
                let offset = (address - r.base) as usize;
                return (i, offset);
            }
        }
        panic!(
            "SparseMemory: unmapped address 0x{:08X} (accessing {} bytes)",
            address, num_bytes
        );
    }

    /// Read a single byte from the sparse address space.
    pub fn read_byte(&self, address: u32) -> u8 {
        let (idx, offset) = self.find_region(address, 1);
        self.regions[idx].data[offset]
    }

    /// Write a single byte. If the target region is read-only, the write
    /// is silently ignored.
    pub fn write_byte(&mut self, address: u32, value: u8) {
        let (idx, offset) = self.find_region(address, 1);
        if self.regions[idx].read_only {
            return;
        }
        self.regions[idx].data[offset] = value;
    }

    /// Read a 32-bit word (little-endian) from the sparse address space.
    pub fn read_word(&self, address: u32) -> u32 {
        let (idx, offset) = self.find_region(address, 4);
        let d = &self.regions[idx].data;
        d[offset] as u32
            | ((d[offset + 1] as u32) << 8)
            | ((d[offset + 2] as u32) << 16)
            | ((d[offset + 3] as u32) << 24)
    }

    /// Write a 32-bit word (little-endian). Read-only regions silently ignore.
    pub fn write_word(&mut self, address: u32, value: u32) {
        let (idx, offset) = self.find_region(address, 4);
        if self.regions[idx].read_only {
            return;
        }
        self.regions[idx].data[offset] = (value & 0xFF) as u8;
        self.regions[idx].data[offset + 1] = ((value >> 8) & 0xFF) as u8;
        self.regions[idx].data[offset + 2] = ((value >> 16) & 0xFF) as u8;
        self.regions[idx].data[offset + 3] = ((value >> 24) & 0xFF) as u8;
    }

    /// Load bytes into memory, bypassing the read_only check.
    /// Used for initialization (loading ROM contents before CPU starts).
    pub fn load_bytes(&mut self, address: u32, data: &[u8]) {
        let (idx, offset) = self.find_region(address, data.len());
        self.regions[idx].data[offset..offset + data.len()].copy_from_slice(data);
    }

    /// Return a copy of bytes from the sparse address space.
    pub fn dump(&self, start: u32, length: usize) -> Vec<u8> {
        let (idx, offset) = self.find_region(start, length);
        self.regions[idx].data[offset..offset + length].to_vec()
    }

    /// Return the number of mapped regions.
    pub fn region_count(&self) -> usize {
        self.regions.len()
    }

    /// Direct access to regions for testing/inspection.
    pub fn regions(&self) -> &[MemoryRegion] {
        &self.regions
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_sparse_memory() -> SparseMemory {
        SparseMemory::new(vec![
            MemoryRegion::new(0x00000000, 4096, "RAM", false),
            MemoryRegion::new(0xFFFF0000, 256, "ROM", true),
        ])
    }

    #[test]
    fn test_allocates_regions() {
        let mem = make_test_sparse_memory();
        assert_eq!(mem.region_count(), 2);
        assert_eq!(mem.regions()[0].name, "RAM");
        assert_eq!(mem.regions()[0].base, 0x00000000);
        assert_eq!(mem.regions()[0].size, 4096);
        assert_eq!(mem.regions()[0].data.len(), 4096);
        assert!(!mem.regions()[0].read_only);
        assert_eq!(mem.regions()[1].name, "ROM");
        assert!(mem.regions()[1].read_only);
    }

    #[test]
    fn test_pre_populated_data() {
        let mut rom_data = vec![0u8; 64];
        rom_data[0] = 0xAA;
        rom_data[63] = 0xBB;
        let mem = SparseMemory::new(vec![
            MemoryRegion::with_data(0x1000, rom_data, "ROM", true),
        ]);
        assert_eq!(mem.read_byte(0x1000), 0xAA);
        assert_eq!(mem.read_byte(0x103F), 0xBB);
    }

    #[test]
    fn test_zero_initialized() {
        let mem = make_test_sparse_memory();
        for i in 0..16u32 {
            assert_eq!(mem.read_byte(i), 0);
        }
    }

    #[test]
    fn test_read_write_byte() {
        let mut mem = make_test_sparse_memory();
        mem.write_byte(0x0000, 0x42);
        mem.write_byte(0x0001, 0xFF);
        mem.write_byte(0x0FFF, 0x99);
        assert_eq!(mem.read_byte(0x0000), 0x42);
        assert_eq!(mem.read_byte(0x0001), 0xFF);
        assert_eq!(mem.read_byte(0x0FFF), 0x99);
    }

    #[test]
    fn test_read_only_write_silently_ignored() {
        let mut mem = make_test_sparse_memory();
        assert_eq!(mem.read_byte(0xFFFF0000), 0);
        mem.write_byte(0xFFFF0000, 0xDE);
        assert_eq!(mem.read_byte(0xFFFF0000), 0);
    }

    #[test]
    fn test_little_endian_word() {
        let mut mem = make_test_sparse_memory();
        mem.write_word(0x0100, 0xDEADBEEF);
        assert_eq!(mem.read_byte(0x0100), 0xEF);
        assert_eq!(mem.read_byte(0x0101), 0xBE);
        assert_eq!(mem.read_byte(0x0102), 0xAD);
        assert_eq!(mem.read_byte(0x0103), 0xDE);
        assert_eq!(mem.read_word(0x0100), 0xDEADBEEF);
    }

    #[test]
    fn test_write_word_read_only() {
        let mut mem = make_test_sparse_memory();
        mem.write_word(0xFFFF0000, 0x12345678);
        assert_eq!(mem.read_word(0xFFFF0000), 0x00000000);
    }

    #[test]
    fn test_word_round_trip() {
        let mut mem = make_test_sparse_memory();
        let cases: [(u32, u32); 6] = [
            (0x0000, 0x00000000),
            (0x0004, 0xFFFFFFFF),
            (0x0008, 0x00000001),
            (0x000C, 0x80000000),
            (0x0010, 0x7FFFFFFF),
            (0x0014, 0x01020304),
        ];
        for (addr, val) in cases {
            mem.write_word(addr, val);
            assert_eq!(mem.read_word(addr), val, "at 0x{:04X}", addr);
        }
    }

    #[test]
    fn test_load_bytes() {
        let mut mem = make_test_sparse_memory();
        let data = [0x01u8, 0x02, 0x03, 0x04, 0x05];
        mem.load_bytes(0x0200, &data);
        for (i, &expected) in data.iter().enumerate() {
            assert_eq!(mem.read_byte(0x0200 + i as u32), expected);
        }
    }

    #[test]
    fn test_load_bytes_into_read_only_region() {
        let mut mem = make_test_sparse_memory();
        mem.load_bytes(0xFFFF0000, &[0xAA, 0xBB, 0xCC, 0xDD]);
        assert_eq!(mem.read_byte(0xFFFF0000), 0xAA);
        assert_eq!(mem.read_byte(0xFFFF0003), 0xDD);
        mem.write_byte(0xFFFF0000, 0x00);
        assert_eq!(mem.read_byte(0xFFFF0000), 0xAA);
    }

    #[test]
    fn test_dump() {
        let mut mem = make_test_sparse_memory();
        mem.write_byte(0x0010, 0xAA);
        mem.write_byte(0x0011, 0xBB);
        mem.write_byte(0x0012, 0xCC);
        let dumped = mem.dump(0x0010, 3);
        assert_eq!(dumped, vec![0xAA, 0xBB, 0xCC]);
    }

    #[test]
    fn test_dump_is_copy() {
        let mut mem = make_test_sparse_memory();
        mem.write_byte(0x0000, 0xFF);
        let mut dumped = mem.dump(0x0000, 4);
        dumped[0] = 0x00;
        assert_eq!(mem.read_byte(0x0000), 0xFF);
    }

    #[test]
    #[should_panic(expected = "unmapped")]
    fn test_read_byte_unmapped_panics() {
        let mem = make_test_sparse_memory();
        mem.read_byte(0x80000000);
    }

    #[test]
    #[should_panic(expected = "unmapped")]
    fn test_write_byte_unmapped_panics() {
        let mut mem = make_test_sparse_memory();
        mem.write_byte(0x80000000, 0xFF);
    }

    #[test]
    #[should_panic(expected = "unmapped")]
    fn test_read_word_unmapped_panics() {
        let mem = make_test_sparse_memory();
        mem.read_word(0x80000000);
    }

    #[test]
    #[should_panic(expected = "unmapped")]
    fn test_write_word_unmapped_panics() {
        let mut mem = make_test_sparse_memory();
        mem.write_word(0x80000000, 0xDEAD);
    }

    #[test]
    #[should_panic(expected = "unmapped")]
    fn test_read_word_crosses_boundary_panics() {
        let mem = make_test_sparse_memory();
        mem.read_word(0x0FFE);
    }

    #[test]
    fn test_multiple_regions() {
        let mut mem = SparseMemory::new(vec![
            MemoryRegion::new(0x00000000, 1024, "RAM", false),
            MemoryRegion::new(0x10000000, 256, "SRAM", false),
            MemoryRegion::new(0xFFFF0000, 128, "IO", false),
        ]);
        mem.write_byte(0x00000000, 0x11);
        mem.write_byte(0x10000000, 0x22);
        mem.write_byte(0xFFFF0000, 0x33);
        assert_eq!(mem.read_byte(0x00000000), 0x11);
        assert_eq!(mem.read_byte(0x10000000), 0x22);
        assert_eq!(mem.read_byte(0xFFFF0000), 0x33);
    }

    #[test]
    fn test_high_address_region() {
        let mut mem = SparseMemory::new(vec![
            MemoryRegion::new(0xFFFB0000, 0x50000, "HIGH_IO", false),
        ]);
        mem.write_byte(0xFFFB0000, 0x01);
        mem.write_byte(0xFFFFFFFE, 0xFE);
        mem.write_word(0xFFFFFFFC, 0xCAFEBABE);
        assert_eq!(mem.read_byte(0xFFFB0000), 0x01);
        assert_eq!(mem.read_word(0xFFFFFFFC), 0xCAFEBABE);
    }

    #[test]
    fn test_load_program() {
        let mut mem = SparseMemory::new(vec![
            MemoryRegion::new(0x00000000, 0x10000, "RAM", false),
        ]);
        let program: [u8; 16] = [
            0x93, 0x00, 0xA0, 0x02,
            0x13, 0x01, 0x30, 0x00,
            0xB3, 0x01, 0x21, 0x00,
            0x73, 0x00, 0x00, 0x00,
        ];
        mem.load_bytes(0x0000, &program);
        assert_eq!(mem.read_word(0x0000), 0x02A00093);
    }

    #[test]
    #[should_panic(expected = "unmapped")]
    fn test_load_bytes_unmapped_panics() {
        let mut mem = make_test_sparse_memory();
        mem.load_bytes(0x80000000, &[0x01, 0x02]);
    }

    #[test]
    #[should_panic(expected = "unmapped")]
    fn test_dump_unmapped_panics() {
        let mem = make_test_sparse_memory();
        mem.dump(0x80000000, 4);
    }

    #[test]
    #[should_panic(expected = "unmapped")]
    fn test_empty_regions() {
        let mem = SparseMemory::new(vec![]);
        mem.read_byte(0x0000);
    }

    #[test]
    fn test_region_count() {
        let mem = SparseMemory::new(vec![
            MemoryRegion::new(0, 16, "A", false),
            MemoryRegion::new(0x1000, 16, "B", false),
            MemoryRegion::new(0x2000, 16, "C", false),
        ]);
        assert_eq!(mem.region_count(), 3);
    }
}
