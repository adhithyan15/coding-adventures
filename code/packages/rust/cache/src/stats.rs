/// Cache statistics tracking -- measuring how well the cache is performing.
///
/// Every cache keeps a scorecard. Just like a baseball player tracks batting
/// average (hits / at-bats), a cache tracks its **hit rate** (cache hits /
/// total accesses). A high hit rate means the cache is doing its job well --
/// most memory requests are being served quickly from the cache rather than
/// going to slower main memory.
///
/// Key metrics:
/// - **Reads/Writes**: How many times the CPU asked for data or stored data.
/// - **Hits**: How many times the requested data was already in the cache.
/// - **Misses**: How many times we had to go to a slower level to get the data.
/// - **Evictions**: How many times we had to kick out old data to make room.
/// - **Writebacks**: How many evictions involved dirty data that needed to be
///   written back to the next level (only relevant for write-back caches).
///
/// Analogy: Think of a library desk (L1 cache). If you keep the right books
/// on your desk, you rarely need to walk to the shelf (L2). Your "hit rate"
/// is how often the book you need is already on your desk.

/// Tracks performance statistics for a single cache level.
///
/// Every read or write to the cache updates these counters. After running
/// a simulation, you can inspect `hit_rate()` and `miss_rate()` to see how
/// effective the cache configuration is for a given workload.
///
/// # Example
/// ```
/// use cache::CacheStats;
///
/// let mut stats = CacheStats::new();
/// stats.record_read(true);
/// stats.record_read(false);
/// assert!((stats.hit_rate() - 0.5).abs() < f64::EPSILON);
/// assert!((stats.miss_rate() - 0.5).abs() < f64::EPSILON);
/// ```
#[derive(Debug, Clone, Default)]
pub struct CacheStats {
    /// Number of read operations.
    pub reads: u64,
    /// Number of write operations.
    pub writes: u64,
    /// Number of cache hits (data found in cache).
    pub hits: u64,
    /// Number of cache misses (data not found, had to go further).
    pub misses: u64,
    /// Number of lines evicted from the cache to make room.
    pub evictions: u64,
    /// Number of dirty evictions that required a writeback to the next level.
    pub writebacks: u64,
}

impl CacheStats {
    /// Create a new, zeroed-out statistics tracker.
    pub fn new() -> Self {
        Self::default()
    }

    /// Total number of read + write operations.
    pub fn total_accesses(&self) -> u64 {
        self.reads + self.writes
    }

    /// Fraction of accesses that were cache hits (0.0 to 1.0).
    ///
    /// Returns 0.0 if no accesses have been made (avoids division by zero).
    ///
    /// A hit rate of 0.95 means 95% of memory requests were served from
    /// this cache level -- excellent for an L1 cache.
    pub fn hit_rate(&self) -> f64 {
        let total = self.total_accesses();
        if total == 0 {
            return 0.0;
        }
        self.hits as f64 / total as f64
    }

    /// Fraction of accesses that were cache misses (0.0 to 1.0).
    ///
    /// Always equals `1.0 - hit_rate()`. Provided for convenience since
    /// miss rate is the more commonly discussed metric in architecture
    /// papers ("this workload has a 5% L1 miss rate").
    pub fn miss_rate(&self) -> f64 {
        let total = self.total_accesses();
        if total == 0 {
            return 0.0;
        }
        self.misses as f64 / total as f64
    }

    /// Record a read access. Pass `hit = true` for a cache hit.
    pub fn record_read(&mut self, hit: bool) {
        self.reads += 1;
        if hit {
            self.hits += 1;
        } else {
            self.misses += 1;
        }
    }

    /// Record a write access. Pass `hit = true` for a cache hit.
    pub fn record_write(&mut self, hit: bool) {
        self.writes += 1;
        if hit {
            self.hits += 1;
        } else {
            self.misses += 1;
        }
    }

    /// Record an eviction. Pass `dirty = true` if the evicted line was dirty.
    ///
    /// A dirty eviction means the data was modified in the cache but not
    /// yet written to the next level. The cache controller must "write back"
    /// the dirty data before discarding it -- this is the extra cost of a
    /// write-back policy.
    pub fn record_eviction(&mut self, dirty: bool) {
        self.evictions += 1;
        if dirty {
            self.writebacks += 1;
        }
    }

    /// Reset all counters to zero.
    ///
    /// Useful when you want to measure stats for a specific phase of
    /// execution (e.g., "what's the hit rate during matrix multiply?"
    /// without counting the initial data loading phase).
    pub fn reset(&mut self) {
        self.reads = 0;
        self.writes = 0;
        self.hits = 0;
        self.misses = 0;
        self.evictions = 0;
        self.writebacks = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_stats_are_zero() {
        let stats = CacheStats::new();
        assert_eq!(stats.reads, 0);
        assert_eq!(stats.writes, 0);
        assert_eq!(stats.hits, 0);
        assert_eq!(stats.misses, 0);
        assert_eq!(stats.evictions, 0);
        assert_eq!(stats.writebacks, 0);
        assert_eq!(stats.total_accesses(), 0);
        assert!((stats.hit_rate() - 0.0).abs() < f64::EPSILON);
        assert!((stats.miss_rate() - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_record_read_hit() {
        let mut stats = CacheStats::new();
        stats.record_read(true);
        assert_eq!(stats.reads, 1);
        assert_eq!(stats.hits, 1);
        assert_eq!(stats.misses, 0);
        assert!((stats.hit_rate() - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_record_read_miss() {
        let mut stats = CacheStats::new();
        stats.record_read(false);
        assert_eq!(stats.reads, 1);
        assert_eq!(stats.hits, 0);
        assert_eq!(stats.misses, 1);
        assert!((stats.miss_rate() - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_record_write_hit_and_miss() {
        let mut stats = CacheStats::new();
        stats.record_write(true);
        stats.record_write(false);
        assert_eq!(stats.writes, 2);
        assert_eq!(stats.hits, 1);
        assert_eq!(stats.misses, 1);
        assert!((stats.hit_rate() - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn test_mixed_reads_and_writes() {
        let mut stats = CacheStats::new();
        stats.record_read(true);
        stats.record_read(true);
        stats.record_write(false);
        stats.record_write(true);
        assert_eq!(stats.total_accesses(), 4);
        assert_eq!(stats.hits, 3);
        assert_eq!(stats.misses, 1);
        assert!((stats.hit_rate() - 0.75).abs() < f64::EPSILON);
        assert!((stats.miss_rate() - 0.25).abs() < f64::EPSILON);
    }

    #[test]
    fn test_eviction_tracking() {
        let mut stats = CacheStats::new();
        stats.record_eviction(false);
        stats.record_eviction(true);
        stats.record_eviction(true);
        assert_eq!(stats.evictions, 3);
        assert_eq!(stats.writebacks, 2);
    }

    #[test]
    fn test_reset() {
        let mut stats = CacheStats::new();
        stats.record_read(true);
        stats.record_write(false);
        stats.record_eviction(true);
        stats.reset();
        assert_eq!(stats.total_accesses(), 0);
        assert_eq!(stats.hits, 0);
        assert_eq!(stats.misses, 0);
        assert_eq!(stats.evictions, 0);
        assert_eq!(stats.writebacks, 0);
    }
}
