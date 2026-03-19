/// Cache line -- the smallest unit of data in a cache.
///
/// In a real CPU, data is not moved one byte at a time between memory and the
/// cache. Instead, it moves in fixed-size chunks called **cache lines** (also
/// called cache blocks). A typical cache line is 64 bytes.
///
/// Analogy: Think of a warehouse that ships goods in standard containers.
/// You can't order a single screw -- you get the whole container (cache line)
/// that includes the screw you need plus 63 other bytes of nearby data.
/// This works well because of **spatial locality**: if you accessed byte N,
/// you'll likely access bytes N+1, N+2, ... soon.
///
/// Each cache line stores:
///
/// ```text
///     +-------+-------+-----+------+---------------------------+
///     | valid | dirty | tag | LRU  |     data (64 bytes)       |
///     +-------+-------+-----+------+---------------------------+
/// ```
///
/// - **valid**: Is this line holding real data? After a reset, all lines are
///   invalid (empty boxes). A line becomes valid when data is loaded into it.
///
/// - **dirty**: Has the data been modified since it was loaded from memory?
///   In a write-back cache, writes go only to the cache (not memory). The
///   dirty bit tracks whether the line needs to be written back to memory
///   when evicted. (Like editing a document locally -- you need to save it
///   back to the server before closing.)
///
/// - **tag**: The high bits of the memory address. Since many addresses map
///   to the same cache set (like many apartments on the same floor), the tag
///   distinguishes WHICH address is actually stored here.
///
/// - **data**: The actual bytes -- a `Vec<u8>` because the line size is
///   configured at runtime (could be 32, 64, or 128 bytes). We use `Vec<u8>`
///   instead of `&[u8]` because each cache line **owns** its data. A borrow
///   (`&[u8]`) would require a lifetime parameter tying the line to whatever
///   originally provided the data -- but in a cache, data gets overwritten
///   on eviction, so we need ownership semantics.
///
/// - **last_access**: A timestamp (cycle count) recording when this line was
///   last read or written. Used by the LRU replacement policy to decide
///   which line to evict when the set is full.

/// A single cache line -- one slot in the cache.
///
/// # Example
/// ```
/// use cache::CacheLine;
///
/// let mut line = CacheLine::new(64);
/// assert!(!line.valid);
/// line.fill(42, &vec![0xAB; 64], 100);
/// assert!(line.valid);
/// assert_eq!(line.tag, 42);
/// assert_eq!(line.last_access, 100);
/// ```
#[derive(Debug, Clone)]
pub struct CacheLine {
    /// Is this line holding real data?
    pub valid: bool,
    /// Has this line been modified? (write-back policy tracking)
    pub dirty: bool,
    /// High bits of the address -- identifies which memory block is cached.
    pub tag: u64,
    /// The actual bytes stored in this cache line.
    ///
    /// We use `Vec<u8>` (owned heap allocation) rather than `&[u8]` (borrowed
    /// slice) because the cache line must own its data independently. In Rust's
    /// ownership model, a borrow would require the original data source to
    /// outlive every cache line -- but cache lines get evicted and refilled
    /// with entirely new data throughout a simulation. Ownership via `Vec`
    /// lets each line manage its own data lifetime.
    pub data: Vec<u8>,
    /// Cycle count of last access -- used for LRU replacement.
    pub last_access: u64,
}

impl CacheLine {
    /// Create a new invalid cache line with the given size.
    ///
    /// # Arguments
    /// * `line_size` - Number of bytes per cache line. Defaults to 64 in
    ///   typical usage, which is standard on modern x86 and ARM CPUs.
    pub fn new(line_size: usize) -> Self {
        Self {
            valid: false,
            dirty: false,
            tag: 0,
            data: vec![0u8; line_size],
            last_access: 0,
        }
    }

    /// Load data into this cache line, marking it valid.
    ///
    /// This is called when a cache miss brings data from a lower level
    /// (L2, L3, or main memory) into this line.
    ///
    /// # Arguments
    /// * `tag` - The tag bits for the address being cached.
    /// * `data` - The bytes to store (should match line_size).
    /// * `cycle` - Current clock cycle (for LRU tracking).
    pub fn fill(&mut self, tag: u64, data: &[u8], cycle: u64) {
        self.valid = true;
        self.dirty = false; // freshly loaded data is clean
        self.tag = tag;
        self.data = data.to_vec(); // defensive copy via to_vec()
        self.last_access = cycle;
    }

    /// Update the last access time -- called on every hit.
    ///
    /// This is the heartbeat of LRU: the most recently used line
    /// gets the highest timestamp, so it's the *last* to be evicted.
    pub fn touch(&mut self, cycle: u64) {
        self.last_access = cycle;
    }

    /// Mark this line as invalid (empty).
    ///
    /// Used during cache flushes or coherence protocol invalidations.
    /// The data is not zeroed -- it's just marked as not-present.
    pub fn invalidate(&mut self) {
        self.valid = false;
        self.dirty = false;
    }

    /// Number of bytes in this cache line.
    pub fn line_size(&self) -> usize {
        self.data.len()
    }
}

impl std::fmt::Display for CacheLine {
    /// Compact representation for debugging.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let v = if self.valid { 'V' } else { '-' };
        let d = if self.dirty { 'D' } else { '-' };
        write!(
            f,
            "CacheLine({}{}, tag=0x{:X}, lru={})",
            v, d, self.tag, self.last_access
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_line_is_invalid() {
        let line = CacheLine::new(64);
        assert!(!line.valid);
        assert!(!line.dirty);
        assert_eq!(line.tag, 0);
        assert_eq!(line.last_access, 0);
        assert_eq!(line.data.len(), 64);
        assert_eq!(line.line_size(), 64);
    }

    #[test]
    fn test_fill_makes_valid_and_clean() {
        let mut line = CacheLine::new(64);
        let data = vec![0xAB; 64];
        line.fill(42, &data, 100);
        assert!(line.valid);
        assert!(!line.dirty);
        assert_eq!(line.tag, 42);
        assert_eq!(line.last_access, 100);
        assert_eq!(line.data, data);
    }

    #[test]
    fn test_fill_is_defensive_copy() {
        let mut line = CacheLine::new(4);
        let mut data = vec![1, 2, 3, 4];
        line.fill(1, &data, 0);
        // Modifying the original data should not affect the line
        data[0] = 99;
        assert_eq!(line.data[0], 1);
    }

    #[test]
    fn test_touch_updates_lru() {
        let mut line = CacheLine::new(64);
        line.fill(1, &vec![0; 64], 10);
        assert_eq!(line.last_access, 10);
        line.touch(50);
        assert_eq!(line.last_access, 50);
    }

    #[test]
    fn test_invalidate() {
        let mut line = CacheLine::new(64);
        line.fill(1, &vec![0; 64], 10);
        line.dirty = true;
        line.invalidate();
        assert!(!line.valid);
        assert!(!line.dirty);
    }

    #[test]
    fn test_display() {
        let mut line = CacheLine::new(64);
        assert_eq!(format!("{}", line), "CacheLine(--, tag=0x0, lru=0)");
        line.fill(0xFF, &vec![0; 64], 42);
        assert_eq!(format!("{}", line), "CacheLine(V-, tag=0xFF, lru=42)");
        line.dirty = true;
        assert_eq!(format!("{}", line), "CacheLine(VD, tag=0xFF, lru=42)");
    }

    #[test]
    fn test_different_line_sizes() {
        let line32 = CacheLine::new(32);
        assert_eq!(line32.line_size(), 32);
        let line128 = CacheLine::new(128);
        assert_eq!(line128.line_size(), 128);
    }
}
