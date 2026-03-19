/// Branch Target Buffer (BTB) -- caching where branches go.
///
/// The branch predictor answers "WILL this branch be taken?"
/// The BTB answers "WHERE does it go?"
///
/// Both are needed for high-performance fetch. Without a BTB, even a perfect
/// direction predictor would cause a 1-cycle bubble: the predictor says "taken"
/// in the fetch stage, but the target address isn't known until decode (when
/// the instruction's immediate field is extracted). With a BTB, the target
/// is available in the SAME cycle as the prediction, enabling zero-bubble
/// fetch redirection.
///
/// How the BTB fits into the pipeline:
///
/// ```text
///     Cycle 1 (Fetch):
///         1. Read PC
///         2. Direction predictor: "taken" or "not taken"?
///         3. BTB lookup: if "taken", where does it go?
///         4. Redirect fetch to target (BTB hit) or PC+4 (not taken / BTB miss)
/// ```
///
/// BTB organization (this implementation):
///     - Direct-mapped cache indexed by (pc % size)
///     - Each entry stores: valid bit, tag (full PC), target, branch type
///     - On lookup: check valid bit and tag match
///     - On miss: return None (fall through to PC+4)
///
/// Real-world BTB sizes:
///     - Intel Skylake: 4096 entries (L1 BTB) + 4096 entries (L2 BTB)
///     - ARM Cortex-A72: 64 entries (micro BTB) + 4096 entries (main BTB)
///     - AMD Zen 2: 512 entries (L1 BTB) + 7168 entries (L2 BTB)

/// A single entry in the Branch Target Buffer.
///
/// Each entry is like a cache line: it stores the branch's PC (as the tag),
/// the target address, and some metadata. The tag is necessary because
/// multiple branches can map to the same BTB index (aliasing).
#[derive(Debug, Clone)]
pub struct BTBEntry {
    /// Whether this entry contains valid data. Starts false.
    pub valid: bool,
    /// The PC (program counter) of the branch instruction.
    /// Used to detect aliasing -- two branches mapping to the same index.
    pub tag: u64,
    /// The branch target address (where the branch goes if taken).
    pub target: u64,
    /// The kind of branch: "conditional", "unconditional", "call", or "return".
    pub branch_type: String,
}

impl BTBEntry {
    /// Create a new invalid BTB entry.
    fn new() -> Self {
        Self {
            valid: false,
            tag: 0,
            target: 0,
            branch_type: String::new(),
        }
    }
}

/// Branch Target Buffer -- works alongside any BranchPredictor to provide
/// target addresses.
///
/// The BTB is a separate structure from the direction predictor. In a real CPU,
/// both are consulted in parallel during the fetch stage:
///
/// 1. Direction predictor says: "taken" or "not taken"
/// 2. BTB says: "if taken, the target is 0x1234" (or miss)
///
/// # Example
/// ```
/// use branch_predictor::BranchTargetBuffer;
///
/// let mut btb = BranchTargetBuffer::new(256);
///
/// // First lookup -- miss (branch never seen before)
/// assert!(btb.lookup(0x100).is_none());
///
/// // After the branch executes, update the BTB
/// btb.update(0x100, 0x200, "conditional");
///
/// // Now the lookup hits
/// assert_eq!(btb.lookup(0x100), Some(0x200));
/// ```
pub struct BranchTargetBuffer {
    /// Number of entries in the BTB.
    size: usize,
    /// The BTB storage. Pre-allocated as a `Vec` of entries.
    ///
    /// We use `Vec<BTBEntry>` rather than `HashMap` because the BTB is a
    /// direct-mapped structure with a fixed number of slots. Pre-allocating
    /// all entries mirrors the hardware: in a real BTB, every SRAM cell
    /// exists from power-on, just with the valid bit cleared. The `Vec`
    /// gives us O(1) indexed access without hashing overhead.
    entries: Vec<BTBEntry>,
    /// Total number of BTB lookups performed.
    lookups: u64,
    /// Number of BTB hits (target found and tag matched).
    hits: u64,
    /// Number of BTB misses (entry invalid or tag mismatch).
    misses: u64,
}

impl BranchTargetBuffer {
    /// Create a new BTB with the given number of entries.
    ///
    /// # Arguments
    /// * `size` - Number of entries. Should be a power of 2.
    ///   Common sizes: 64, 256, 512, 1024, 4096.
    pub fn new(size: usize) -> Self {
        let entries = (0..size).map(|_| BTBEntry::new()).collect();
        Self {
            size,
            entries,
            lookups: 0,
            hits: 0,
            misses: 0,
        }
    }

    /// Compute the BTB index for a given PC.
    ///
    /// Direct-mapped: index = pc % size.
    fn index(&self, pc: u64) -> usize {
        (pc % self.size as u64) as usize
    }

    /// Look up the predicted target for a branch at `pc`.
    ///
    /// Returns `Some(target)` on a hit, or `None` on a miss.
    /// A miss occurs when:
    /// - The entry at this index is not valid (never written)
    /// - The entry's tag doesn't match the PC (aliasing conflict)
    pub fn lookup(&mut self, pc: u64) -> Option<u64> {
        self.lookups += 1;
        let idx = self.index(pc);
        let entry = &self.entries[idx];

        if entry.valid && entry.tag == pc {
            self.hits += 1;
            Some(entry.target)
        } else {
            self.misses += 1;
            None
        }
    }

    /// Record a branch target after execution.
    ///
    /// Writes the target and metadata into the BTB. If another branch was
    /// occupying this index (aliasing), it gets evicted.
    pub fn update(&mut self, pc: u64, target: u64, branch_type: &str) {
        let idx = self.index(pc);
        self.entries[idx] = BTBEntry {
            valid: true,
            tag: pc,
            target,
            branch_type: branch_type.to_string(),
        };
    }

    /// Inspect the BTB entry for a given PC (for testing/debugging).
    ///
    /// Returns `Some(&BTBEntry)` if valid and tag matches, `None` otherwise.
    pub fn get_entry(&self, pc: u64) -> Option<&BTBEntry> {
        let idx = self.index(pc);
        let entry = &self.entries[idx];
        if entry.valid && entry.tag == pc {
            Some(entry)
        } else {
            None
        }
    }

    /// Total number of BTB lookups performed.
    pub fn lookups(&self) -> u64 {
        self.lookups
    }

    /// Number of BTB hits (target found).
    pub fn hits(&self) -> u64 {
        self.hits
    }

    /// Number of BTB misses (target not found).
    pub fn misses(&self) -> u64 {
        self.misses
    }

    /// BTB hit rate as a percentage (0.0 to 100.0).
    pub fn hit_rate(&self) -> f64 {
        if self.lookups == 0 {
            return 0.0;
        }
        (self.hits as f64 / self.lookups as f64) * 100.0
    }

    /// Reset all BTB state -- entries and statistics.
    pub fn reset(&mut self) {
        self.entries = (0..self.size).map(|_| BTBEntry::new()).collect();
        self.lookups = 0;
        self.hits = 0;
        self.misses = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_btb_all_invalid() {
        let btb = BranchTargetBuffer::new(256);
        for entry in &btb.entries {
            assert!(!entry.valid);
        }
    }

    #[test]
    fn test_lookup_miss_on_empty() {
        let mut btb = BranchTargetBuffer::new(256);
        assert!(btb.lookup(0x100).is_none());
        assert_eq!(btb.misses(), 1);
    }

    #[test]
    fn test_update_then_lookup_hit() {
        let mut btb = BranchTargetBuffer::new(256);
        btb.update(0x100, 0x200, "conditional");
        assert_eq!(btb.lookup(0x100), Some(0x200));
        assert_eq!(btb.hits(), 1);
    }

    #[test]
    fn test_tag_mismatch_is_miss() {
        let mut btb = BranchTargetBuffer::new(4); // tiny BTB for aliasing
        btb.update(0x100, 0x200, "conditional"); // index = 0x100 % 4 = 0
        // 0x104 also maps to index 0 (0x104 % 4 = 0), but tag doesn't match
        assert!(btb.lookup(0x104).is_none());
    }

    #[test]
    fn test_aliasing_eviction() {
        let mut btb = BranchTargetBuffer::new(4);
        btb.update(0x100, 0x200, "conditional"); // index 0
        btb.update(0x104, 0x300, "conditional"); // also index 0, overwrites

        // 0x100 is evicted
        assert!(btb.lookup(0x100).is_none());
        // 0x104 is present
        assert_eq!(btb.lookup(0x104), Some(0x300));
    }

    #[test]
    fn test_get_entry() {
        let mut btb = BranchTargetBuffer::new(256);
        assert!(btb.get_entry(0x100).is_none());

        btb.update(0x100, 0x200, "call");
        let entry = btb.get_entry(0x100).unwrap();
        assert!(entry.valid);
        assert_eq!(entry.tag, 0x100);
        assert_eq!(entry.target, 0x200);
        assert_eq!(entry.branch_type, "call");
    }

    #[test]
    fn test_branch_types() {
        let mut btb = BranchTargetBuffer::new(256);
        // Use non-aliasing addresses (different pc % 256)
        btb.update(0x01, 0x200, "conditional");
        btb.update(0x02, 0x300, "unconditional");
        btb.update(0x03, 0x400, "call");
        btb.update(0x04, 0x500, "return");

        assert_eq!(btb.get_entry(0x01).unwrap().branch_type, "conditional");
        assert_eq!(btb.get_entry(0x02).unwrap().branch_type, "unconditional");
        assert_eq!(btb.get_entry(0x03).unwrap().branch_type, "call");
        assert_eq!(btb.get_entry(0x04).unwrap().branch_type, "return");
    }

    #[test]
    fn test_hit_rate() {
        let mut btb = BranchTargetBuffer::new(256);
        btb.update(0x100, 0x200, "conditional");

        btb.lookup(0x100); // hit
        btb.lookup(0x100); // hit
        btb.lookup(0x200); // miss

        assert_eq!(btb.lookups(), 3);
        assert_eq!(btb.hits(), 2);
        assert_eq!(btb.misses(), 1);
        assert!((btb.hit_rate() - 66.666_666_666_666_66).abs() < 0.01);
    }

    #[test]
    fn test_hit_rate_no_lookups() {
        let btb = BranchTargetBuffer::new(256);
        assert!((btb.hit_rate() - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_reset() {
        let mut btb = BranchTargetBuffer::new(256);
        btb.update(0x100, 0x200, "conditional");
        btb.lookup(0x100);
        btb.reset();

        assert_eq!(btb.lookups(), 0);
        assert_eq!(btb.hits(), 0);
        assert!(btb.lookup(0x100).is_none()); // entry gone after reset
    }

    #[test]
    fn test_update_overwrites_target() {
        let mut btb = BranchTargetBuffer::new(256);
        btb.update(0x100, 0x200, "conditional");
        btb.update(0x100, 0x400, "unconditional"); // update target

        assert_eq!(btb.lookup(0x100), Some(0x400));
        assert_eq!(btb.get_entry(0x100).unwrap().branch_type, "unconditional");
    }

    #[test]
    fn test_multiple_independent_entries() {
        let mut btb = BranchTargetBuffer::new(256);
        // Use non-aliasing addresses (different pc % 256)
        btb.update(0x01, 0x200, "conditional");
        btb.update(0x02, 0x400, "call");

        assert_eq!(btb.lookup(0x01), Some(0x200));
        assert_eq!(btb.lookup(0x02), Some(0x400));
    }
}
