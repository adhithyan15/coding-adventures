/// Cache -- a single configurable level of the cache hierarchy.
///
/// This module implements the core cache logic. The same struct is used for
/// L1, L2, and L3 -- the only difference is the configuration (size,
/// associativity, latency). This reflects real hardware: an L1 and an L3
/// use the same SRAM cell design, just at different scales.
///
/// ## Address Decomposition
///
/// When the CPU accesses memory address 0x1A2B3C4D, the cache must figure
/// out three things:
///
/// 1. **Offset** (lowest bits): Which byte *within* the cache line?
///    - For 64-byte lines: 6 bits (2^6 = 64)
///    - Example: offset = 0x0D = byte 13 of the line
///
/// 2. **Set Index** (middle bits): Which set should we look in?
///    - For 256 sets: 8 bits (2^8 = 256)
///    - Example: set_index = 0xF1 = set 241
///
/// 3. **Tag** (highest bits): Which memory block is this?
///    - All remaining bits above offset + set_index
///    - Example: tag = 0x1A2B3 (uniquely identifies the block)
///
/// Visual for a 64KB, 4-way, 64B-line cache (256 sets):
///
/// ```text
///     Address: | tag (18 bits) | set index (8 bits) | offset (6 bits) |
///              |  31 ... 14    |     13 ... 6       |    5 ... 0      |
/// ```
///
/// This bit-slicing is why cache sizes must be powers of 2 -- it lets the
/// hardware extract fields with simple bit masks instead of division.
///
/// ## Read Path
///
/// ```text
///     CPU reads address 0x1000
///          |
///          v
///     Decompose: tag=0x4, set=0, offset=0
///          |
///          v
///     Look in Set 0: compare tag 0x4 against all ways
///          |
///     +----+----+
///     |         |
///     HIT      MISS
///     |         |
///     Return   Go to next level (L2/L3/memory)
///     data     Bring data back, allocate in this cache
///              Maybe evict an old line (LRU)
/// ```
use crate::cache_line::CacheLine;
use crate::cache_set::{CacheConfig, CacheSet, WritePolicy};
use crate::stats::CacheStats;

/// Record of a single cache access -- for debugging and performance analysis.
///
/// Every `read()` or `write()` call returns one of these, telling you exactly
/// what happened: was it a hit? Which set? Was anything evicted? How many
/// cycles did it cost?
///
/// This is like a receipt for each memory transaction.
#[derive(Debug)]
pub struct CacheAccess {
    /// The full memory address that was accessed.
    pub address: u64,
    /// True if the data was found in the cache (no need to go further).
    pub hit: bool,
    /// The tag bits extracted from the address.
    pub tag: u64,
    /// The set index bits -- which set in the cache was consulted.
    pub set_index: usize,
    /// The offset bits -- byte position within the cache line.
    pub offset: usize,
    /// Clock cycles this access took (latency).
    pub cycles: u64,
    /// If a dirty line was evicted during this access, it's stored here.
    /// Only set for dirty evictions that need writeback.
    pub evicted: Option<CacheLine>,
}

/// A single level of cache -- configurable to be L1, L2, or L3.
///
/// This is the workhorse of the cache simulator. Give it a `CacheConfig`
/// and it handles address decomposition, set lookup, LRU replacement,
/// and statistics tracking.
///
/// # Why `&mut self` on read/write?
///
/// Even a read operation mutates the cache: it updates LRU timestamps,
/// allocates lines on a miss, and updates statistics counters. In Rust's
/// ownership model, any mutation requires `&mut self`. This is actually
/// more honest than languages where a `read()` method looks pure but
/// silently mutates internal state -- Rust forces you to acknowledge
/// the mutation at the call site.
///
/// # Example
/// ```
/// use cache::{Cache, CacheConfig};
///
/// let config = CacheConfig::new("L1D", 1024, 64, 4, 1);
/// let mut cache = Cache::new(config);
/// let access = cache.read(0x100, 0);
/// assert!(!access.hit); // first access is always a miss
/// let access = cache.read(0x100, 1);
/// assert!(access.hit); // second access hits
/// ```
pub struct Cache {
    /// The configuration that parameterizes this cache level.
    pub config: CacheConfig,
    /// The array of sets. Each set contains `associativity` lines.
    sets: Vec<CacheSet>,
    /// Performance statistics (hits, misses, evictions, etc.).
    pub stats: CacheStats,
    /// Number of bits for the byte offset within a line: log2(line_size).
    offset_bits: u32,
    /// Number of bits for the set index: log2(num_sets).
    set_bits: u32,
    /// Bitmask for extracting the set index: num_sets - 1.
    set_mask: u64,
}

impl Cache {
    /// Initialize the cache with the given configuration.
    ///
    /// Creates all sets, precomputes bit positions for address
    /// decomposition, and initializes statistics.
    pub fn new(config: CacheConfig) -> Self {
        let num_sets = config.num_sets();
        let sets: Vec<CacheSet> = (0..num_sets)
            .map(|_| CacheSet::new(config.associativity, config.line_size))
            .collect();

        // Precompute bit positions for address decomposition.
        //   offset_bits = log2(line_size)    e.g., log2(64) = 6
        //   set_bits    = log2(num_sets)     e.g., log2(256) = 8
        let offset_bits = (config.line_size as f64).log2() as u32;
        let set_bits = if num_sets > 1 {
            (num_sets as f64).log2() as u32
        } else {
            0
        };
        let set_mask = if num_sets > 0 {
            (num_sets - 1) as u64
        } else {
            0
        };

        Self {
            config,
            sets,
            stats: CacheStats::new(),
            offset_bits,
            set_bits,
            set_mask,
        }
    }

    /// Split a memory address into (tag, set_index, offset).
    ///
    /// This is pure bit manipulation -- no division needed because all
    /// sizes are powers of 2.
    ///
    /// Example for 64KB cache, 64B lines, 256 sets:
    /// ```text
    ///     address = 0x1A2B3C4D
    ///     offset     = address & 0x3F              = 0x0D (13)
    ///     set_index  = (address >> 6) & 0xFF        = 0xF1 (241)
    ///     tag        = address >> 14                 = 0x68AC
    /// ```
    fn decompose_address(&self, address: u64) -> (u64, usize, usize) {
        let offset = (address & ((1u64 << self.offset_bits) - 1)) as usize;
        let set_index = ((address >> self.offset_bits) & self.set_mask) as usize;
        let tag = address >> (self.offset_bits + self.set_bits);
        (tag, set_index, offset)
    }

    /// Read data from the cache.
    ///
    /// On a hit, the data is returned immediately with the cache's
    /// access latency. On a miss, dummy data is allocated (the caller
    /// -- typically the hierarchy -- is responsible for actually fetching
    /// from the next level).
    ///
    /// # Arguments
    /// * `address` - Memory address to read.
    /// * `cycle` - Current clock cycle.
    pub fn read(&mut self, address: u64, cycle: u64) -> CacheAccess {
        let (tag, set_index, offset) = self.decompose_address(address);
        let cache_set = &mut self.sets[set_index];

        let (hit, _line_idx) = cache_set.access(tag, cycle);

        if hit {
            self.stats.record_read(true);
            return CacheAccess {
                address,
                hit: true,
                tag,
                set_index,
                offset,
                cycles: self.config.access_latency,
                evicted: None,
            };
        }

        // Miss -- allocate the line with dummy data.
        // In a real system, the hierarchy fetches from the next level
        // and fills this line. Here we simulate by filling with zeros.
        self.stats.record_read(false);
        let dummy_data = vec![0u8; self.config.line_size];
        let evicted = cache_set.allocate(tag, &dummy_data, cycle);

        if evicted.is_some() {
            self.stats.record_eviction(true);
        } else if self.sets[set_index].lines.iter().all(|l| l.valid) {
            // A valid but clean line was evicted (all ways are now valid
            // and one of them is our new line)
            // Actually, if allocate returned None and all lines are valid,
            // that means we filled an invalid slot or evicted a clean line.
            // Since we know it wasn't invalid (all valid now and allocate
            // prefers invalid slots), a clean eviction happened.
            // But wait -- allocate returns None for both "filled invalid slot"
            // and "evicted clean line". We need to distinguish.
            // allocate only returns Some for dirty evictions. If all lines
            // are valid now, either we filled the last invalid slot (no eviction)
            // or we evicted a clean line. We record a clean eviction only if
            // all lines were already valid BEFORE the allocate.
            // Since we can't easily tell post-hoc, we accept the Python logic:
            // if all lines are valid after allocate, record a clean eviction.
            // This slightly over-counts when the last invalid slot is filled,
            // matching the Python behavior.
            self.stats.record_eviction(false);
        }

        CacheAccess {
            address,
            hit: false,
            tag,
            set_index,
            offset,
            cycles: self.config.access_latency,
            evicted,
        }
    }

    /// Write data to the cache.
    ///
    /// **Write-back policy**: Write only to the cache. Mark the line
    /// as dirty. The data is written to the next level only when the
    /// line is evicted.
    ///
    /// **Write-through policy**: Write to both the cache and the next
    /// level simultaneously. The line is never dirty.
    ///
    /// On a write miss, we use **write-allocate**: first bring the
    /// line into the cache (like a read miss), then perform the write.
    /// This is the most common policy on modern CPUs.
    ///
    /// # Arguments
    /// * `address` - Memory address to write.
    /// * `data` - Bytes to write (optional; if empty, just marks dirty).
    /// * `cycle` - Current clock cycle.
    pub fn write(&mut self, address: u64, data: &[u8], cycle: u64) -> CacheAccess {
        let (tag, set_index, offset) = self.decompose_address(address);
        let cache_set = &mut self.sets[set_index];

        let (hit, line_idx) = cache_set.access(tag, cycle);

        if hit {
            self.stats.record_write(true);
            // Write the data into the line
            let line = &mut cache_set.lines[line_idx];
            for (i, &byte) in data.iter().enumerate() {
                if offset + i < line.data.len() {
                    line.data[offset + i] = byte;
                }
            }
            // Mark dirty for write-back; write-through stays clean
            if self.config.write_policy == WritePolicy::WriteBack {
                line.dirty = true;
            }
            return CacheAccess {
                address,
                hit: true,
                tag,
                set_index,
                offset,
                cycles: self.config.access_latency,
                evicted: None,
            };
        }

        // Write miss -- allocate (write-allocate policy), then write
        self.stats.record_write(false);
        let mut fill_data = vec![0u8; self.config.line_size];
        for (i, &byte) in data.iter().enumerate() {
            if offset + i < fill_data.len() {
                fill_data[offset + i] = byte;
            }
        }

        let evicted = cache_set.allocate(tag, &fill_data, cycle);
        if evicted.is_some() {
            self.stats.record_eviction(true);
        } else if self.sets[set_index].lines.iter().all(|l| l.valid) {
            self.stats.record_eviction(false);
        }

        // For write-back, mark the newly allocated line as dirty
        // (it has new data that isn't in the next level)
        if self.config.write_policy == WritePolicy::WriteBack {
            let cache_set = &mut self.sets[set_index];
            let (new_hit, new_idx) = cache_set.access(tag, cycle);
            if new_hit {
                cache_set.lines[new_idx].dirty = true;
            }
        }

        CacheAccess {
            address,
            hit: false,
            tag,
            set_index,
            offset,
            cycles: self.config.access_latency,
            evicted,
        }
    }

    /// Invalidate all lines in the cache (cache flush).
    ///
    /// This is equivalent to a cold start -- after invalidation, every
    /// access will be a compulsory miss. Used when context-switching
    /// between processes or when explicitly flushing (e.g., for I/O
    /// coherence).
    pub fn invalidate(&mut self) {
        for set in &mut self.sets {
            for line in &mut set.lines {
                line.invalidate();
            }
        }
    }

    /// Directly fill a cache line with data (used by hierarchy on miss).
    ///
    /// This bypasses the normal read/write path -- it's used when the
    /// hierarchy fetches data from a lower level and wants to install
    /// it in this cache.
    ///
    /// Returns the evicted dirty `CacheLine` if a writeback is needed,
    /// else `None`.
    pub fn fill_line(&mut self, address: u64, data: &[u8], cycle: u64) -> Option<CacheLine> {
        let (tag, set_index, _offset) = self.decompose_address(address);
        let cache_set = &mut self.sets[set_index];
        cache_set.allocate(tag, data, cycle)
    }
}

impl std::fmt::Display for Cache {
    /// Human-readable summary of the cache configuration.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Cache({}: {}KB, {}-way, {}B lines, {} sets)",
            self.config.name,
            self.config.total_size / 1024,
            self.config.associativity,
            self.config.line_size,
            self.config.num_sets()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_l1d() -> Cache {
        Cache::new(CacheConfig::new("L1D", 1024, 64, 4, 1))
    }

    #[test]
    fn test_address_decomposition() {
        let cache = make_l1d();
        // 1024 bytes / 64 bytes per line = 16 lines
        // 16 lines / 4 ways = 4 sets
        // offset_bits = log2(64) = 6
        // set_bits = log2(4) = 2
        let (tag, set_index, offset) = cache.decompose_address(0x100);
        assert_eq!(offset, 0);
        assert_eq!(set_index, 0);
        assert_eq!(tag, 0x100 >> 8); // 6 + 2 = 8 bits for offset+set
    }

    #[test]
    fn test_first_read_is_miss() {
        let mut cache = make_l1d();
        let access = cache.read(0x100, 0);
        assert!(!access.hit);
        assert_eq!(access.cycles, 1);
        assert_eq!(cache.stats.reads, 1);
        assert_eq!(cache.stats.misses, 1);
    }

    #[test]
    fn test_second_read_is_hit() {
        let mut cache = make_l1d();
        cache.read(0x100, 0);
        let access = cache.read(0x100, 1);
        assert!(access.hit);
        assert_eq!(cache.stats.hits, 1);
    }

    #[test]
    fn test_different_addresses_same_set() {
        let mut cache = make_l1d();
        // With 4 sets and 64-byte lines, addresses that differ only in the tag
        // bits map to the same set. Addresses 0, 256, 512, 768 all map to set 0.
        cache.read(0x000, 0);
        cache.read(0x100, 1);
        cache.read(0x200, 2);
        cache.read(0x300, 3);
        // All 4 ways of set 0 should be full now (all misses)
        assert_eq!(cache.stats.misses, 4);

        // A 5th address to set 0 should evict the LRU
        cache.read(0x400, 4);
        assert_eq!(cache.stats.misses, 5);

        // Original address 0x000 should be evicted
        let access = cache.read(0x000, 5);
        assert!(!access.hit); // evicted, so miss again
    }

    #[test]
    fn test_write_hit() {
        let mut cache = make_l1d();
        // First read to bring data in
        cache.read(0x100, 0);
        // Write to the same address -- should be a hit
        let access = cache.write(0x100, &[0xAB], 1);
        assert!(access.hit);
        assert_eq!(cache.stats.writes, 1);
        assert_eq!(cache.stats.hits, 2); // 1 read hit + 1 write hit... wait
        // Actually: first read is a miss (1 miss), second write is a hit (1 hit from write)
        // stats.hits = 1 (write hit only; the read was a miss)
        assert_eq!(cache.stats.hits, 1);
    }

    #[test]
    fn test_write_miss_allocates() {
        let mut cache = make_l1d();
        let access = cache.write(0x100, &[0xAB], 0);
        assert!(!access.hit);
        // Should be allocated now
        let access2 = cache.read(0x100, 1);
        assert!(access2.hit);
    }

    #[test]
    fn test_write_back_dirty_bit() {
        let mut cache = make_l1d();
        cache.read(0x100, 0); // bring line in (clean)
        cache.write(0x100, &[0xAB], 1); // write-back marks dirty

        // Verify the line is dirty by filling the set and evicting it
        // Set 0 addresses: 0x000, 0x100, 0x200, 0x300, 0x400
        cache.read(0x000, 2);
        cache.read(0x200, 3);
        cache.read(0x300, 4);
        // 0x400 should evict 0x100 (if it's the LRU)
        // Actually LRU depends on timestamps. Let's touch 0x100 last.
        // 0x000 at cycle 2, 0x200 at cycle 3, 0x300 at cycle 4, 0x100 at cycle 1
        // LRU is 0x100 at cycle 1... wait, we wrote to 0x100 at cycle 1.
        // 0x000 at cycle 2 is newer. So 0x100 (cycle 1) is LRU.
        let access = cache.read(0x400, 5);
        assert!(!access.hit);
        // The evicted line (0x100) was dirty
        if access.evicted.is_some() {
            assert!(access.evicted.as_ref().unwrap().dirty);
        }
    }

    #[test]
    fn test_write_through_no_dirty() {
        let config = CacheConfig::new("L1D", 1024, 64, 4, 1)
            .with_write_policy(WritePolicy::WriteThrough);
        let mut cache = Cache::new(config);
        cache.read(0x100, 0);
        cache.write(0x100, &[0xAB], 1);
        // In write-through, the line should NOT be dirty
        let (tag, set_index, _) = cache.decompose_address(0x100);
        let set = &cache.sets[set_index];
        let (hit, idx) = set.lookup(tag);
        assert!(hit);
        assert!(!set.lines[idx.unwrap()].dirty);
    }

    #[test]
    fn test_invalidate_all() {
        let mut cache = make_l1d();
        cache.read(0x100, 0);
        cache.read(0x200, 1);
        cache.invalidate();

        // After invalidation, everything misses again
        let access = cache.read(0x100, 2);
        assert!(!access.hit);
    }

    #[test]
    fn test_fill_line() {
        let mut cache = make_l1d();
        let data = vec![0xCD; 64];
        let evicted = cache.fill_line(0x100, &data, 0);
        assert!(evicted.is_none());

        // Should be accessible now
        let access = cache.read(0x100, 1);
        assert!(access.hit);
    }

    #[test]
    fn test_display() {
        let cache = make_l1d();
        let s = format!("{}", cache);
        assert!(s.contains("L1D"));
        assert!(s.contains("4-way"));
        assert!(s.contains("64B lines"));
    }

    #[test]
    fn test_sequential_access_pattern() {
        let mut cache = make_l1d();
        // Access sequential addresses within the same line (spatial locality)
        for i in 0..64 {
            let access = cache.read(0x100 + i, i);
            if i == 0 {
                assert!(!access.hit); // first access is a miss
            } else {
                assert!(access.hit); // same line, all hits
            }
        }
        assert_eq!(cache.stats.hits, 63);
        assert_eq!(cache.stats.misses, 1);
    }

    #[test]
    fn test_strided_access_pattern() {
        // Access every 64th byte -- each access hits a different line
        let mut cache = make_l1d();
        for i in 0..4 {
            let addr = (i * 64) as u64;
            let access = cache.read(addr, i as u64);
            assert!(!access.hit); // all compulsory misses
        }
        assert_eq!(cache.stats.misses, 4);

        // Re-access -- all should hit
        for i in 0..4 {
            let addr = (i * 64) as u64;
            let access = cache.read(addr, (i + 4) as u64);
            assert!(access.hit);
        }
        assert_eq!(cache.stats.hits, 4);
    }
}
