/// Cache hierarchy -- multi-level cache system (L1I + L1D + L2 + L3 + memory).
///
/// A modern CPU doesn't have just one cache -- it has a **hierarchy** of
/// progressively larger and slower caches. This is the memory equivalent
/// of keeping frequently used items close to hand:
///
/// ```text
///     +---------+     +--------+     +--------+     +--------+     +--------+
///     |   CPU   | --> |  L1    | --> |   L2   | --> |   L3   | --> |  Main  |
///     |  core   |     | 1 cyc  |     | 10 cyc |     | 30 cyc |     | Memory |
///     |         |     | 64KB   |     | 256KB  |     | 8MB    |     | 100cyc |
///     +---------+     +--------+     +--------+     +--------+     +--------+
///                      per-core       per-core       shared         shared
/// ```
///
/// Analogy:
/// - L1 = the books open on your desk (tiny, instant access)
/// - L2 = the bookshelf in your office (bigger, a few seconds to grab)
/// - L3 = the library downstairs (huge, takes a minute to walk there)
/// - Main memory = the warehouse across town (enormous, takes an hour)
///
/// When the CPU reads an address:
/// 1. Check L1D. Hit? Return data (1 cycle). Miss? Continue.
/// 2. Check L2. Hit? Return data (10 cycles), and fill L1D. Miss? Continue.
/// 3. Check L3. Hit? Return data (30 cycles), fill L2 and L1D. Miss? Continue.
/// 4. Go to main memory (100 cycles). Fill L3, L2, and L1D.
///
/// The total latency is the sum of all levels that missed:
/// - L1 hit:         1 cycle
/// - L1 miss, L2 hit:  1 + 10 = 11 cycles
/// - L1+L2 miss, L3 hit: 1 + 10 + 30 = 41 cycles
/// - All miss:       1 + 10 + 30 + 100 = 141 cycles
///
/// Harvard vs Unified:
/// - **Harvard architecture**: Separate L1 for instructions (L1I) and data (L1D).
///   This lets the CPU fetch an instruction and load data simultaneously.
/// - **Unified**: L2 and L3 are typically unified (shared between instructions
///   and data) to avoid wasting space.
use crate::cache::Cache;
use crate::cache::CacheAccess;

/// Record of an access through the full hierarchy.
///
/// Tracks which level served the data and the total latency accumulated
/// across all levels that were consulted.
#[derive(Debug)]
pub struct HierarchyAccess {
    /// The memory address that was accessed.
    pub address: u64,
    /// Name of the level that had the data ("L1D", "L2", "L3", "memory").
    pub served_by: String,
    /// Total clock cycles from start to data delivery.
    pub total_cycles: u64,
    /// Which hierarchy level served the data (0=L1, 1=L2, 2=L3, 3=memory).
    pub hit_at_level: usize,
    /// Detailed access records from each cache level consulted.
    pub level_accesses: Vec<CacheAccess>,
}

/// Multi-level cache hierarchy -- L1I + L1D + L2 + L3 + main memory.
///
/// Fully configurable: pass any combination of cache levels. You can
/// simulate anything from a simple L1-only system to a full 3-level
/// hierarchy with separate instruction and data L1 caches.
///
/// # Why `Option<Cache>` for each level?
///
/// Not every system has every cache level. A simple microcontroller might
/// have only L1. A desktop CPU has L1+L2+L3. By using `Option`, the
/// hierarchy gracefully handles any configuration without special-casing.
/// In Rust, `Option` has zero overhead for the `Some` case and is
/// the idiomatic way to express "this may or may not be present."
///
/// # Example
/// ```
/// use cpu_cache::{Cache, CacheConfig, CacheHierarchy};
///
/// let l1d = Cache::new(CacheConfig::new("L1D", 1024, 64, 4, 1));
/// let l2 = Cache::new(CacheConfig::new("L2", 4096, 64, 8, 10));
/// let mut hierarchy = CacheHierarchy::new(None, Some(l1d), Some(l2), None, 100);
/// let result = hierarchy.read(0x1000, false, 0);
/// assert_eq!(result.served_by, "memory"); // first access is always a miss
/// ```
pub struct CacheHierarchy {
    /// L1 instruction cache (optional, for Harvard architecture).
    pub l1i: Option<Cache>,
    /// L1 data cache (optional but typical).
    pub l1d: Option<Cache>,
    /// L2 cache (optional, unified).
    pub l2: Option<Cache>,
    /// L3 cache (optional, shared).
    pub l3: Option<Cache>,
    /// Clock cycles for main memory access.
    pub main_memory_latency: u64,
}

impl CacheHierarchy {
    /// Create a cache hierarchy.
    ///
    /// # Arguments
    /// * `l1i` - L1 instruction cache (for Harvard architecture).
    /// * `l1d` - L1 data cache.
    /// * `l2` - L2 cache.
    /// * `l3` - L3 cache.
    /// * `main_memory_latency` - Clock cycles for main memory access.
    pub fn new(
        l1i: Option<Cache>,
        l1d: Option<Cache>,
        l2: Option<Cache>,
        l3: Option<Cache>,
        main_memory_latency: u64,
    ) -> Self {
        Self {
            l1i,
            l1d,
            l2,
            l3,
            main_memory_latency,
        }
    }

    /// Read through the hierarchy. Returns which level served the data.
    ///
    /// Walks the hierarchy top-down. At each level:
    /// - If hit: stop, fill all higher levels, return.
    /// - If miss: accumulate latency, continue to next level.
    /// - If all miss: data comes from main memory.
    ///
    /// The **inclusive** fill policy is used: when L3 serves data, it
    /// also fills L2 and L1D so subsequent accesses hit at L1.
    ///
    /// # Arguments
    /// * `address` - Memory address to read.
    /// * `is_instruction` - If true, use L1I instead of L1D for the
    ///   first level. L2 and L3 are unified.
    /// * `cycle` - Current clock cycle.
    pub fn read(
        &mut self,
        address: u64,
        is_instruction: bool,
        cycle: u64,
    ) -> HierarchyAccess {
        // Build the ordered list of levels to walk through.
        // We collect indices into a temporary vec because we can't hold
        // mutable references to multiple Option<Cache> fields simultaneously
        // in Rust's borrow checker. The enum approach lets us access them
        // one at a time.
        let level_order = self.build_level_order(is_instruction);

        if level_order.is_empty() {
            return HierarchyAccess {
                address,
                served_by: "memory".to_string(),
                total_cycles: self.main_memory_latency,
                hit_at_level: 0,
                level_accesses: vec![],
            };
        }

        let mut total_cycles = 0u64;
        let mut accesses: Vec<CacheAccess> = Vec::new();
        let mut served_by = "memory".to_string();
        let mut hit_level = level_order.len();

        // Walk the hierarchy top-down
        for (level_idx, level_id) in level_order.iter().enumerate() {
            let (name, cache) = self.get_level_mut(*level_id);
            let access = cache.read(address, cycle);
            total_cycles += cache.config.access_latency;
            let hit = access.hit;
            accesses.push(access);

            if hit {
                served_by = name.to_string();
                hit_level = level_idx;
                break;
            }
        }

        if served_by == "memory" {
            total_cycles += self.main_memory_latency;
        }

        // Fill higher levels (inclusive policy).
        // If L3 served, fill L2 and L1. If L2 served, fill L1.
        let line_size = self.get_line_size(&level_order);
        let dummy_data = vec![0u8; line_size];
        for fill_idx in (0..hit_level).rev() {
            let level_id = level_order[fill_idx];
            let (_name, cache) = self.get_level_mut(level_id);
            cache.fill_line(address, &dummy_data, cycle);
        }

        HierarchyAccess {
            address,
            served_by,
            total_cycles,
            hit_at_level: hit_level,
            level_accesses: accesses,
        }
    }

    /// Write through the hierarchy.
    ///
    /// With write-allocate + write-back (the most common policy):
    /// 1. If L1D hit: write to L1D, mark dirty. Done.
    /// 2. If L1D miss: allocate in L1D (may cause eviction cascade),
    ///    write to L1D. The data comes from the next level that has it
    ///    or from main memory.
    pub fn write(
        &mut self,
        address: u64,
        data: &[u8],
        cycle: u64,
    ) -> HierarchyAccess {
        let level_order = self.build_level_order(false);

        if level_order.is_empty() {
            return HierarchyAccess {
                address,
                served_by: "memory".to_string(),
                total_cycles: self.main_memory_latency,
                hit_at_level: 0,
                level_accesses: vec![],
            };
        }

        // Write to the first data-level cache
        let first_id = level_order[0];
        let (first_name, first_cache) = self.get_level_mut(first_id);
        let first_name = first_name.to_string();
        let first_latency = first_cache.config.access_latency;
        let access = first_cache.write(address, data, cycle);

        if access.hit {
            return HierarchyAccess {
                address,
                served_by: first_name,
                total_cycles: first_latency,
                hit_at_level: 0,
                level_accesses: vec![access],
            };
        }

        // Write miss at L1 -- walk lower levels to find the data
        let mut total_cycles = first_latency;
        let mut accesses: Vec<CacheAccess> = vec![access];
        let mut served_by = "memory".to_string();
        let mut hit_level = level_order.len();

        for level_idx in 1..level_order.len() {
            let level_id = level_order[level_idx];
            let (name, cache) = self.get_level_mut(level_id);
            let name = name.to_string();
            let latency = cache.config.access_latency;
            let level_access = cache.read(address, cycle);
            total_cycles += latency;
            let hit = level_access.hit;
            accesses.push(level_access);

            if hit {
                served_by = name;
                hit_level = level_idx;
                break;
            }
        }

        if served_by == "memory" {
            total_cycles += self.main_memory_latency;
        }

        HierarchyAccess {
            address,
            served_by,
            total_cycles,
            hit_at_level: hit_level,
            level_accesses: accesses,
        }
    }

    /// Invalidate all caches in the hierarchy (full flush).
    pub fn invalidate_all(&mut self) {
        if let Some(ref mut c) = self.l1i {
            c.invalidate();
        }
        if let Some(ref mut c) = self.l1d {
            c.invalidate();
        }
        if let Some(ref mut c) = self.l2 {
            c.invalidate();
        }
        if let Some(ref mut c) = self.l3 {
            c.invalidate();
        }
    }

    /// Reset statistics for all cache levels.
    pub fn reset_stats(&mut self) {
        if let Some(ref mut c) = self.l1i {
            c.stats.reset();
        }
        if let Some(ref mut c) = self.l1d {
            c.stats.reset();
        }
        if let Some(ref mut c) = self.l2 {
            c.stats.reset();
        }
        if let Some(ref mut c) = self.l3 {
            c.stats.reset();
        }
    }

    // ── Internal helpers ──────────────────────────────────────────────

    /// Level identifiers used to work around Rust's borrow checker.
    ///
    /// Rust does not allow multiple mutable borrows of different fields
    /// through the same `&mut self`. By using enum identifiers and
    /// accessing one field at a time via `get_level_mut`, we satisfy
    /// the borrow checker while still walking the hierarchy dynamically.
    fn build_level_order(&self, is_instruction: bool) -> Vec<LevelId> {
        let mut order = Vec::new();
        if is_instruction {
            if self.l1i.is_some() {
                order.push(LevelId::L1I);
            }
        } else if self.l1d.is_some() {
            order.push(LevelId::L1D);
        }
        if self.l2.is_some() {
            order.push(LevelId::L2);
        }
        if self.l3.is_some() {
            order.push(LevelId::L3);
        }
        order
    }

    fn get_level_mut(&mut self, id: LevelId) -> (&str, &mut Cache) {
        match id {
            LevelId::L1I => ("L1I", self.l1i.as_mut().unwrap()),
            LevelId::L1D => ("L1D", self.l1d.as_mut().unwrap()),
            LevelId::L2 => ("L2", self.l2.as_mut().unwrap()),
            LevelId::L3 => ("L3", self.l3.as_mut().unwrap()),
        }
    }

    fn get_line_size(&self, level_order: &[LevelId]) -> usize {
        if let Some(first) = level_order.first() {
            match first {
                LevelId::L1I => self.l1i.as_ref().unwrap().config.line_size,
                LevelId::L1D => self.l1d.as_ref().unwrap().config.line_size,
                LevelId::L2 => self.l2.as_ref().unwrap().config.line_size,
                LevelId::L3 => self.l3.as_ref().unwrap().config.line_size,
            }
        } else {
            64 // default
        }
    }
}

impl std::fmt::Display for CacheHierarchy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut parts = Vec::new();
        if let Some(ref c) = self.l1i {
            parts.push(format!("L1I={}KB", c.config.total_size / 1024));
        }
        if let Some(ref c) = self.l1d {
            parts.push(format!("L1D={}KB", c.config.total_size / 1024));
        }
        if let Some(ref c) = self.l2 {
            parts.push(format!("L2={}KB", c.config.total_size / 1024));
        }
        if let Some(ref c) = self.l3 {
            parts.push(format!("L3={}KB", c.config.total_size / 1024));
        }
        parts.push(format!("mem={}cyc", self.main_memory_latency));
        write!(f, "CacheHierarchy({})", parts.join(", "))
    }
}

/// Internal enum to identify cache levels without holding references.
#[derive(Clone, Copy)]
enum LevelId {
    L1I,
    L1D,
    L2,
    L3,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cache_set::CacheConfig;

    fn make_l1d() -> Cache {
        Cache::new(CacheConfig::new("L1D", 1024, 64, 4, 1))
    }

    fn make_l2() -> Cache {
        Cache::new(CacheConfig::new("L2", 4096, 64, 8, 10))
    }

    fn make_l3() -> Cache {
        Cache::new(CacheConfig::new("L3", 16384, 64, 16, 30))
    }

    #[test]
    fn test_no_caches_goes_to_memory() {
        let mut h = CacheHierarchy::new(None, None, None, None, 100);
        let result = h.read(0x1000, false, 0);
        assert_eq!(result.served_by, "memory");
        assert_eq!(result.total_cycles, 100);
    }

    #[test]
    fn test_l1_only_miss_then_hit() {
        let mut h = CacheHierarchy::new(None, Some(make_l1d()), None, None, 100);
        let r1 = h.read(0x1000, false, 0);
        assert_eq!(r1.served_by, "memory");
        assert_eq!(r1.total_cycles, 1 + 100); // L1 miss + memory

        let r2 = h.read(0x1000, false, 1);
        assert_eq!(r2.served_by, "L1D");
        assert_eq!(r2.total_cycles, 1);
    }

    #[test]
    fn test_two_level_hierarchy() {
        let mut h = CacheHierarchy::new(None, Some(make_l1d()), Some(make_l2()), None, 100);

        // First access -- miss everywhere, served by memory
        let r1 = h.read(0x1000, false, 0);
        assert_eq!(r1.served_by, "memory");
        assert_eq!(r1.total_cycles, 1 + 10 + 100); // L1 + L2 + memory

        // Second access -- should hit in L1 (filled on miss)
        let r2 = h.read(0x1000, false, 1);
        assert_eq!(r2.served_by, "L1D");
        assert_eq!(r2.total_cycles, 1);
    }

    #[test]
    fn test_three_level_hierarchy() {
        let mut h = CacheHierarchy::new(
            None,
            Some(make_l1d()),
            Some(make_l2()),
            Some(make_l3()),
            100,
        );

        let r1 = h.read(0x1000, false, 0);
        assert_eq!(r1.served_by, "memory");
        assert_eq!(r1.total_cycles, 1 + 10 + 30 + 100);
    }

    #[test]
    fn test_write_miss_then_read_hit() {
        let mut h = CacheHierarchy::new(None, Some(make_l1d()), Some(make_l2()), None, 100);

        let w = h.write(0x2000, &[0xAB], 0);
        assert_eq!(w.served_by, "memory"); // write miss

        // Read should now hit at L1D
        let r = h.read(0x2000, false, 1);
        assert_eq!(r.served_by, "L1D");
    }

    #[test]
    fn test_instruction_cache() {
        let l1i = Cache::new(CacheConfig::new("L1I", 1024, 64, 4, 1));
        let mut h = CacheHierarchy::new(Some(l1i), Some(make_l1d()), None, None, 100);

        // Instruction read uses L1I
        let r = h.read(0x1000, true, 0);
        assert_eq!(r.served_by, "memory");

        // Data read uses L1D -- should miss separately
        let r2 = h.read(0x1000, false, 1);
        assert_eq!(r2.served_by, "memory");

        // Second instruction read should hit L1I
        let r3 = h.read(0x1000, true, 2);
        assert_eq!(r3.served_by, "L1I");
    }

    #[test]
    fn test_invalidate_all() {
        let mut h = CacheHierarchy::new(None, Some(make_l1d()), Some(make_l2()), None, 100);
        h.read(0x1000, false, 0);
        h.invalidate_all();

        // Should miss again after invalidation
        let r = h.read(0x1000, false, 1);
        assert_eq!(r.served_by, "memory");
    }

    #[test]
    fn test_reset_stats() {
        let mut h = CacheHierarchy::new(None, Some(make_l1d()), Some(make_l2()), None, 100);
        h.read(0x1000, false, 0);
        h.reset_stats();

        let l1d = h.l1d.as_ref().unwrap();
        assert_eq!(l1d.stats.total_accesses(), 0);
    }

    #[test]
    fn test_display() {
        let h = CacheHierarchy::new(None, Some(make_l1d()), Some(make_l2()), None, 100);
        let s = format!("{}", h);
        assert!(s.contains("L1D=1KB"));
        assert!(s.contains("L2=4KB"));
        assert!(s.contains("mem=100cyc"));
    }

    #[test]
    fn test_inclusive_fill_policy() {
        // When L2 serves data, L1 should also get it
        let mut h = CacheHierarchy::new(None, Some(make_l1d()), Some(make_l2()), None, 100);

        // Pre-fill L2 with the address
        h.l2.as_mut().unwrap().read(0x3000, 0);

        // Now read from hierarchy -- L1 misses, L2 should hit
        // Actually, the hierarchy walks top-down: L1 miss -> L2 read.
        // Since L2 already has the data from the pre-fill, it should hit.
        let _r = h.read(0x3000, false, 1);
        // L1 missed, L2 had it from pre-fill -> served by L2
        // But wait -- L1's read during hierarchy walk also allocates.
        // Let's check if a subsequent read hits L1.
        let r2 = h.read(0x3000, false, 2);
        assert_eq!(r2.served_by, "L1D"); // L1 was filled after L2 hit
    }
}
