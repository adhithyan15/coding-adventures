//! A flat, little-endian, bounds-checked address space.
//!
//! The simulator is itself a sandbox: every access is range-checked, so a buggy
//! emitted program faults cleanly ([`Trap::MemoryFault`]) instead of touching
//! host memory. The address space holds three regions the harness lays out:
//!
//! ```text
//!   0x0000…              code region (loaded function bytes)
//!   heap_base …          bump heap (backs __twig_alloc_bytes)
//!   … stack_top          stack (rsp starts near the top, grows down)
//! ```
//!
//! x86 is little-endian, matching the host byte order we read/write with.

use crate::trap::Trap;

/// A flat byte-addressable memory.
#[derive(Debug, Clone)]
pub struct Memory {
    bytes: Vec<u8>,
    /// Next free heap offset (monotonic bump allocator for `__twig_alloc_bytes`).
    heap_next: u64,
    heap_end: u64,
}

impl Memory {
    /// Create a `size`-byte address space with a heap window `[heap_base, heap_end)`.
    pub fn new(size: usize, heap_base: u64, heap_end: u64) -> Memory {
        Memory { bytes: vec![0; size], heap_next: heap_base, heap_end }
    }

    /// Total size in bytes.
    pub fn size(&self) -> u64 { self.bytes.len() as u64 }

    /// Copy `data` to `addr` (used to load the code region). Panics only on a
    /// programming error in the harness (load past the end), not on guest input.
    pub fn write_block(&mut self, addr: u64, data: &[u8]) -> Result<(), Trap> {
        let end = addr.checked_add(data.len() as u64).ok_or(Trap::MemoryFault(addr))?;
        if end > self.size() {
            return Err(Trap::MemoryFault(addr));
        }
        self.bytes[addr as usize..end as usize].copy_from_slice(data);
        Ok(())
    }

    fn check(&self, addr: u64, width: u64) -> Result<usize, Trap> {
        let end = addr.checked_add(width).ok_or(Trap::MemoryFault(addr))?;
        if end > self.size() {
            return Err(Trap::MemoryFault(addr));
        }
        Ok(addr as usize)
    }

    /// Read a `width`-byte (1/2/4/8) little-endian value, zero-extended to u64.
    pub fn load(&self, addr: u64, width: u8) -> Result<u64, Trap> {
        let i = self.check(addr, width as u64)?;
        let mut v = 0u64;
        for b in 0..width as usize {
            v |= (self.bytes[i + b] as u64) << (8 * b);
        }
        Ok(v)
    }

    /// Write the low `width` bytes of `val` little-endian to `addr`.
    pub fn store(&mut self, addr: u64, width: u8, val: u64) -> Result<(), Trap> {
        let i = self.check(addr, width as u64)?;
        for b in 0..width as usize {
            self.bytes[i + b] = (val >> (8 * b)) as u8;
        }
        Ok(())
    }

    /// The bump-allocator behind `__twig_alloc_bytes(n)` — reserve `n` zeroed
    /// bytes (memory is zero-initialised) and return the base pointer, or a fault
    /// when the heap window is exhausted. 8-byte aligned so array headers/elements
    /// are naturally aligned.
    pub fn alloc(&mut self, n: u64) -> Result<u64, Trap> {
        let base = (self.heap_next + 7) & !7;
        let end = base.checked_add(n).ok_or(Trap::MemoryFault(base))?;
        if end > self.heap_end {
            return Err(Trap::MemoryFault(base));
        }
        self.heap_next = end;
        Ok(base)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_store_little_endian() {
        let mut m = Memory::new(64, 32, 48);
        m.store(0, 8, 0x1122_3344_5566_7788).unwrap();
        assert_eq!(m.load(0, 8).unwrap(), 0x1122_3344_5566_7788);
        assert_eq!(m.load(0, 1).unwrap(), 0x88); // low byte first
        assert_eq!(m.load(0, 4).unwrap(), 0x5566_7788);
    }

    #[test]
    fn out_of_range_faults() {
        let m = Memory::new(16, 0, 0);
        assert!(matches!(m.load(12, 8), Err(Trap::MemoryFault(_))));
    }

    #[test]
    fn bump_alloc_is_aligned_and_bounded() {
        let mut m = Memory::new(128, 32, 64);
        let a = m.alloc(3).unwrap();
        assert_eq!(a, 32);
        let b = m.alloc(8).unwrap();
        assert_eq!(b, 40, "8-byte aligned bump past the 3-byte alloc");
        assert!(m.alloc(1000).is_err(), "heap window exhausted → fault");
    }
}
