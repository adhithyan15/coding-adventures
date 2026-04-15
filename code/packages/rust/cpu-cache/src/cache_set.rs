/// Cache set -- a group of cache lines that share the same set index.
///
/// A cache set is like a row of labeled boxes on a shelf. When the CPU
/// accesses memory, the address tells us *which shelf* (set) to look at.
/// Within that shelf, we check each box (way) to see if our data is there.
///
/// In a **4-way set-associative** cache, each set has 4 lines (ways).
/// When all 4 are full and we need to bring in new data, we must **evict**
/// one. The LRU (Least Recently Used) policy picks the line that hasn't
/// been accessed for the longest time -- the logic being "if you haven't
/// used it lately, you probably won't need it soon."
///
/// Associativity is a key design tradeoff:
/// - **Direct-mapped** (1-way): Fast lookup, but high conflict misses.
///   Like a parking lot where each car is assigned exactly one spot -- if
///   two cars map to the same spot, one must leave even if other spots
///   are empty.
/// - **Fully associative** (N-way = total lines): No conflicts, but
///   expensive to search every line on every access.
/// - **Set-associative** (2/4/8/16-way): The sweet spot. Each address
///   maps to a set, and within that set, any way can hold it.
///
/// ```text
///     Set 0: [ Way 0 ] [ Way 1 ] [ Way 2 ] [ Way 3 ]
///     Set 1: [ Way 0 ] [ Way 1 ] [ Way 2 ] [ Way 3 ]
///     Set 2: [ Way 0 ] [ Way 1 ] [ Way 2 ] [ Way 3 ]
///     ...
/// ```
use crate::cache_line::CacheLine;

/// Configuration for a cache level -- the knobs you turn to get L1/L2/L3.
///
/// By adjusting these parameters, the exact same `Cache` struct can simulate
/// anything from a tiny 1KB direct-mapped L1 to a massive 32MB 16-way L3.
///
/// Real-world examples:
/// - ARM Cortex-A78: L1D = 64KB, 4-way, 64B lines, 1 cycle
/// - Intel Alder Lake: L1D = 48KB, 12-way, 64B lines, 5 cycles
/// - Apple M4: L1D = 128KB, 8-way, 64B lines, ~3 cycles
///
/// # Panics
/// Panics if the configuration is invalid (non-power-of-2 line_size,
/// total_size not divisible by line_size * associativity, etc.).
#[derive(Debug, Clone)]
pub struct CacheConfig {
    /// Human-readable name for this cache level ("L1D", "L2", etc.)
    pub name: String,
    /// Total capacity in bytes (e.g., 65536 for 64KB).
    pub total_size: usize,
    /// Bytes per cache line. Must be a power of 2.
    pub line_size: usize,
    /// Number of ways per set. 1 = direct-mapped.
    pub associativity: usize,
    /// Clock cycles to access this level on a hit.
    pub access_latency: u64,
    /// "write-back" (defer writes) or "write-through" (immediate).
    pub write_policy: WritePolicy,
}

/// Write policy for the cache -- determines when dirty data is sent to
/// the next level.
///
/// - **WriteBack**: Only write to the cache on a store. The dirty bit
///   tracks modified lines. Data is written to the next level only on
///   eviction. This is the most common policy on modern CPUs because
///   it reduces memory bus traffic.
///
/// - **WriteThrough**: Every write goes to both the cache and the next
///   level. Simpler but generates more memory traffic. Used in some
///   embedded systems where simplicity matters more than bandwidth.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WritePolicy {
    WriteBack,
    WriteThrough,
}

impl CacheConfig {
    /// Create and validate a new cache configuration.
    ///
    /// # Arguments
    /// * `name` - Human-readable label (e.g. "L1D", "L2").
    /// * `total_size` - Total capacity in bytes.
    /// * `line_size` - Bytes per cache line (must be power of 2).
    /// * `associativity` - Number of ways per set.
    /// * `access_latency` - Hit latency in clock cycles.
    ///
    /// # Panics
    /// Panics if the configuration violates hardware constraints:
    /// - `total_size` must be positive.
    /// - `line_size` must be a positive power of 2.
    /// - `associativity` must be positive.
    /// - `total_size` must be divisible by `line_size * associativity`.
    pub fn new(
        name: &str,
        total_size: usize,
        line_size: usize,
        associativity: usize,
        access_latency: u64,
    ) -> Self {
        assert!(total_size > 0, "total_size must be positive, got {total_size}");
        assert!(
            line_size > 0 && (line_size & (line_size - 1)) == 0,
            "line_size must be a positive power of 2, got {line_size}"
        );
        assert!(
            associativity > 0,
            "associativity must be positive, got {associativity}"
        );
        assert!(
            total_size % (line_size * associativity) == 0,
            "total_size ({total_size}) must be divisible by line_size * associativity ({})",
            line_size * associativity
        );

        Self {
            name: name.to_string(),
            total_size,
            line_size,
            associativity,
            access_latency,
            write_policy: WritePolicy::WriteBack,
        }
    }

    /// Create a config with a specific write policy.
    pub fn with_write_policy(mut self, policy: WritePolicy) -> Self {
        self.write_policy = policy;
        self
    }

    /// Total number of cache lines = total_size / line_size.
    pub fn num_lines(&self) -> usize {
        self.total_size / self.line_size
    }

    /// Number of sets = num_lines / associativity.
    pub fn num_sets(&self) -> usize {
        self.num_lines() / self.associativity
    }
}

/// One set in the cache -- contains N ways (lines).
///
/// Implements LRU (Least Recently Used) replacement: when all ways are
/// full and we need to bring in new data, evict the line that was
/// accessed least recently.
///
/// Think of it like a desk with N book slots. When all slots are full
/// and you need a new book, you put away the one you haven't read in
/// the longest time.
pub struct CacheSet {
    /// The lines (ways) in this set. We use a `Vec` rather than a fixed
    /// array because the associativity is determined at runtime. In Rust,
    /// arrays must have a compile-time-known size; `Vec` gives us runtime
    /// flexibility at the cost of one heap allocation per set.
    pub(crate) lines: Vec<CacheLine>,
}

impl CacheSet {
    /// Create a cache set with the given number of ways.
    ///
    /// # Arguments
    /// * `associativity` - Number of ways (lines) in this set.
    /// * `line_size` - Bytes per cache line.
    pub fn new(associativity: usize, line_size: usize) -> Self {
        let lines = (0..associativity)
            .map(|_| CacheLine::new(line_size))
            .collect();
        Self { lines }
    }

    /// Check if a tag is present in this set.
    ///
    /// Searches all ways for a valid line with a matching tag. This is
    /// what happens in hardware with a parallel tag comparator -- all
    /// ways are checked simultaneously.
    ///
    /// Returns `(hit, way_index)`: hit is true if found; way_index is the
    /// index of the matching line (or `None` if miss).
    pub fn lookup(&self, tag: u64) -> (bool, Option<usize>) {
        for (i, line) in self.lines.iter().enumerate() {
            if line.valid && line.tag == tag {
                return (true, Some(i));
            }
        }
        (false, None)
    }

    /// Access this set for a given tag. Returns `(hit, line_index)`.
    ///
    /// On a hit, updates the line's LRU timestamp so it becomes the
    /// most recently used. On a miss, returns the index of the LRU victim
    /// line (the caller decides what to do -- typically allocate new data).
    ///
    /// # Why return an index instead of `&mut CacheLine`?
    ///
    /// In Rust, returning a mutable reference to an element inside `self`
    /// would borrow `self` mutably for the lifetime of that reference,
    /// preventing the caller from doing anything else with the set.
    /// By returning an index, the caller can access the line later
    /// through `self.lines[index]` in a separate borrow scope.
    pub fn access(&mut self, tag: u64, cycle: u64) -> (bool, usize) {
        let (hit, way_index) = self.lookup(tag);
        if hit {
            let idx = way_index.unwrap();
            self.lines[idx].touch(cycle);
            return (true, idx);
        }
        // Miss -- return the LRU line index (candidate for eviction)
        let lru_index = self.find_lru();
        (false, lru_index)
    }

    /// Bring new data into this set after a cache miss.
    ///
    /// First tries to find an invalid (empty) way. If all ways are
    /// valid, evicts the LRU line. Returns the evicted line if it was
    /// dirty (the caller must write it back to the next level).
    ///
    /// Think of it like clearing a desk slot for a new book:
    /// 1. If there's an empty slot, use it (no eviction needed).
    /// 2. If all slots are full, pick the least-recently-read book.
    /// 3. If that book had notes scribbled in it (dirty), you need
    ///    to save those notes before putting the book away.
    ///
    /// Returns `Some(CacheLine)` if a dirty line was evicted (needs
    /// writeback), or `None` if no dirty writeback is needed.
    pub fn allocate(&mut self, tag: u64, data: &[u8], cycle: u64) -> Option<CacheLine> {
        // Step 1: Look for an invalid (empty) way
        for line in self.lines.iter_mut() {
            if !line.valid {
                line.fill(tag, data, cycle);
                return None; // no eviction needed
            }
        }

        // Step 2: All ways full -- evict the LRU line
        let lru_index = self.find_lru();
        let victim = &self.lines[lru_index];

        // Step 3: Check if the victim is dirty (needs writeback)
        let evicted = if victim.dirty {
            // Clone the victim before overwriting so we can return it
            // for writeback. In Rust, we clone because the line is about
            // to be overwritten -- we can't return a reference to data
            // that's about to change.
            Some(victim.clone())
        } else {
            None
        };

        // Step 4: Overwrite the victim with new data
        self.lines[lru_index].fill(tag, data, cycle);

        evicted
    }

    /// Find the least recently used way index.
    ///
    /// LRU replacement is simple: each line records its last access
    /// time (cycle count). The line with the smallest timestamp is
    /// the one that hasn't been touched for the longest time.
    ///
    /// In real hardware, true LRU is expensive for high associativity
    /// (tracking N! orderings). CPUs often use pseudo-LRU (tree-PLRU)
    /// or RRIP as approximations. For simulation, true LRU is fine.
    ///
    /// Special case: invalid lines are always preferred over valid ones
    /// (an empty slot is "older" than any real data).
    fn find_lru(&self) -> usize {
        let mut best_index = 0;
        let mut best_time = u64::MAX;
        for (i, line) in self.lines.iter().enumerate() {
            // Invalid lines are always the best candidates
            if !line.valid {
                return i;
            }
            if line.last_access < best_time {
                best_time = line.last_access;
                best_index = i;
            }
        }
        best_index
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── CacheConfig tests ─────────────────────────────────────────────

    #[test]
    fn test_valid_config() {
        let config = CacheConfig::new("L1D", 1024, 64, 4, 1);
        assert_eq!(config.name, "L1D");
        assert_eq!(config.total_size, 1024);
        assert_eq!(config.line_size, 64);
        assert_eq!(config.associativity, 4);
        assert_eq!(config.access_latency, 1);
        assert_eq!(config.num_lines(), 16);
        assert_eq!(config.num_sets(), 4);
    }

    #[test]
    #[should_panic(expected = "total_size must be positive")]
    fn test_zero_total_size() {
        CacheConfig::new("bad", 0, 64, 4, 1);
    }

    #[test]
    #[should_panic(expected = "line_size must be a positive power of 2")]
    fn test_non_power_of_2_line_size() {
        CacheConfig::new("bad", 1024, 48, 4, 1);
    }

    #[test]
    #[should_panic(expected = "associativity must be positive")]
    fn test_zero_associativity() {
        CacheConfig::new("bad", 1024, 64, 0, 1);
    }

    #[test]
    #[should_panic(expected = "must be divisible")]
    fn test_indivisible_config() {
        // 1000 is not divisible by 64 * 4 = 256
        CacheConfig::new("bad", 1000, 64, 4, 1);
    }

    #[test]
    fn test_write_policy_builder() {
        let config = CacheConfig::new("L1D", 1024, 64, 4, 1)
            .with_write_policy(WritePolicy::WriteThrough);
        assert_eq!(config.write_policy, WritePolicy::WriteThrough);
    }

    #[test]
    fn test_direct_mapped_config() {
        // Direct-mapped: associativity = 1
        let config = CacheConfig::new("DM", 256, 64, 1, 1);
        assert_eq!(config.num_lines(), 4);
        assert_eq!(config.num_sets(), 4);
    }

    // ── CacheSet tests ────────────────────────────────────────────────

    #[test]
    fn test_new_set_is_empty() {
        let set = CacheSet::new(4, 64);
        assert_eq!(set.lines.len(), 4);
        for line in &set.lines {
            assert!(!line.valid);
        }
    }

    #[test]
    fn test_lookup_miss_on_empty() {
        let set = CacheSet::new(4, 64);
        let (hit, idx) = set.lookup(42);
        assert!(!hit);
        assert!(idx.is_none());
    }

    #[test]
    fn test_allocate_into_empty_slot() {
        let mut set = CacheSet::new(4, 64);
        let data = vec![0xAA; 64];
        let evicted = set.allocate(42, &data, 100);
        assert!(evicted.is_none()); // no eviction from empty set

        let (hit, idx) = set.lookup(42);
        assert!(hit);
        assert_eq!(idx, Some(0));
        assert_eq!(set.lines[0].tag, 42);
    }

    #[test]
    fn test_allocate_fills_sequentially() {
        let mut set = CacheSet::new(4, 64);
        let data = vec![0; 64];
        for tag in 0..4 {
            let evicted = set.allocate(tag, &data, tag);
            assert!(evicted.is_none());
        }
        // All 4 ways should be full now
        for (i, line) in set.lines.iter().enumerate() {
            assert!(line.valid);
            assert_eq!(line.tag, i as u64);
        }
    }

    #[test]
    fn test_lru_eviction() {
        let mut set = CacheSet::new(2, 64);
        let data = vec![0; 64];

        // Fill both ways
        set.allocate(10, &data, 1); // way 0, lru=1
        set.allocate(20, &data, 2); // way 1, lru=2

        // Allocate a third -- should evict way 0 (lru=1, oldest)
        let evicted = set.allocate(30, &data, 3);
        // Way 0 was clean, so no dirty eviction
        assert!(evicted.is_none());

        // Tag 10 should be gone, tag 30 should be present
        let (hit10, _) = set.lookup(10);
        let (hit30, _) = set.lookup(30);
        assert!(!hit10);
        assert!(hit30);
    }

    #[test]
    fn test_dirty_eviction_returns_victim() {
        let mut set = CacheSet::new(2, 64);
        let data = vec![0; 64];

        // Fill both ways
        set.allocate(10, &data, 1);
        set.allocate(20, &data, 2);

        // Mark way 0 as dirty
        set.lines[0].dirty = true;

        // Allocate a third -- should evict dirty way 0
        let evicted = set.allocate(30, &data, 3);
        assert!(evicted.is_some());
        let victim = evicted.unwrap();
        assert!(victim.dirty);
        assert_eq!(victim.tag, 10);
    }

    #[test]
    fn test_access_hit_updates_lru() {
        let mut set = CacheSet::new(4, 64);
        let data = vec![0; 64];

        set.allocate(10, &data, 1);
        set.allocate(20, &data, 2);

        // Access tag 10 at a later cycle -- should update its LRU
        let (hit, idx) = set.access(10, 50);
        assert!(hit);
        assert_eq!(set.lines[idx].last_access, 50);
    }

    #[test]
    fn test_access_miss_returns_lru_index() {
        let mut set = CacheSet::new(2, 64);
        let data = vec![0; 64];

        set.allocate(10, &data, 1);
        set.allocate(20, &data, 2);

        // Miss on tag 99 -- should return index of LRU (way 0, lru=1)
        let (hit, idx) = set.access(99, 3);
        assert!(!hit);
        assert_eq!(idx, 0); // way 0 has the oldest timestamp
    }
}
