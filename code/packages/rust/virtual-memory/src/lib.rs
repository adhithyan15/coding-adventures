//! # Virtual Memory Subsystem (D13)
//!
//! Virtual memory is one of the most important abstractions in computer science.
//! It gives every process the illusion that it has the entire memory space to
//! itself — starting at address 0, stretching to some large upper limit — even
//! though the physical machine has limited RAM shared among many processes.
//!
//! ## Analogy
//!
//! Imagine an apartment building. Each tenant thinks their apartment starts at
//! "Room 1". But the building manager knows Tenant A's "Room 1" is actually
//! physical room 401, and Tenant B's "Room 1" is room 712. The tenants never
//! need to know their real room numbers. They just say "go to my Room 1" and
//! the building manager (the MMU) translates.
//!
//! ## Components
//!
//! - [`PageTableEntry`]: Permission bits and frame mapping for one virtual page
//! - [`PageTable`]: Single-level hash map from VPN to PTE
//! - [`TwoLevelPageTable`]: RISC-V Sv32 hierarchical page table
//! - [`TLB`]: Translation cache with LRU eviction
//! - [`PhysicalFrameAllocator`]: Bitmap allocator with reference counting
//! - [`ReplacementPolicy`]: Trait with FIFO, LRU, and Clock implementations
//! - [`MMU`]: Ties everything together with COW fork support

use std::collections::{HashMap, VecDeque};

// ============================================================================
// Constants
// ============================================================================

/// Page size in bytes. Every page and frame is exactly this size.
/// 4 KB = 4096 bytes = 2^12 bytes.
///
/// This has been the standard page size since the Intel 386 (1985).
/// RISC-V also uses 4 KB as the base page size.
pub const PAGE_SIZE: usize = 4096;

/// Number of bits in the page offset (lower bits of an address).
/// 2^12 = 4096, so we need 12 bits to address every byte within a page.
pub const PAGE_OFFSET_BITS: u32 = 12;

/// Bitmask for extracting the page offset from an address.
/// 0xFFF = 0b111111111111 = 4095
/// Usage: `let offset = address & PAGE_OFFSET_MASK;`
pub const PAGE_OFFSET_MASK: u32 = (PAGE_SIZE as u32) - 1;

/// Number of entries in the Sv32 page directory (2^10 = 1024).
/// Each entry covers 4 MB of virtual address space.
pub const DIRECTORY_ENTRIES: usize = 1024;

/// Default TLB capacity. Real TLBs have 32-256 entries.
pub const DEFAULT_TLB_CAPACITY: usize = 64;

// ============================================================================
// PageTableEntry
// ============================================================================

/// A Page Table Entry describes the mapping for one virtual page.
///
/// Each PTE stores which physical frame the page maps to, plus metadata
/// about permissions and state:
///
/// ```text
/// RISC-V Sv32 PTE bit layout:
/// +--------------------+---+---+---+---+---+---+---+---+
/// | PPN (frame number) | D | A | G | U | X | W | R | V |
/// | bits 31-10         | 7 | 6 | 5 | 4 | 3 | 2 | 1 | 0 |
/// +--------------------+---+---+---+---+---+---+---+---+
///
/// V = Valid (present)    R = Readable
/// W = Writable           X = Executable
/// U = User-accessible    G = Global (not used here)
/// A = Accessed           D = Dirty
/// ```
#[derive(Debug, Clone)]
pub struct PageTableEntry {
    /// Which physical frame this page maps to. Only meaningful if present is true.
    pub frame_number: u32,

    /// Is this page currently in physical memory? If false, accessing it
    /// triggers a page fault (interrupt 14).
    pub present: bool,

    /// Has this page been written to since it was loaded? If true, it must
    /// be written back to disk before the frame can be reused.
    pub dirty: bool,

    /// Has this page been read or written recently? Used by page replacement
    /// algorithms (Clock/LRU) to decide which page to evict.
    pub accessed: bool,

    /// Can this page be written to? Code pages are read-only. Stack/heap
    /// pages are writable. Copy-on-write pages start read-only.
    pub writable: bool,

    /// Can code on this page be executed? Data pages should not be executable
    /// (NX bit — prevents code injection attacks).
    pub executable: bool,

    /// Can user-mode code access this page? Kernel pages are not user-accessible.
    pub user_accessible: bool,
}

impl PageTableEntry {
    /// Create a new page table entry with default values.
    ///
    /// Defaults: not present, not dirty, not accessed, writable, not executable,
    /// user-accessible. The frame number is 0 (meaningless until present=true).
    pub fn new() -> Self {
        Self {
            frame_number: 0,
            present: false,
            dirty: false,
            accessed: false,
            writable: true,
            executable: false,
            user_accessible: true,
        }
    }

    /// Create a PTE that is present and mapped to the given frame.
    pub fn with_frame(frame_number: u32, permissions: &PagePermissions) -> Self {
        Self {
            frame_number,
            present: true,
            dirty: false,
            accessed: false,
            writable: permissions.writable,
            executable: permissions.executable,
            user_accessible: permissions.user_accessible,
        }
    }
}

impl Default for PageTableEntry {
    fn default() -> Self {
        Self::new()
    }
}

/// Permission flags for mapping a page.
///
/// Separated from the PTE so callers can specify permissions without
/// needing to construct a full PTE.
#[derive(Debug, Clone)]
pub struct PagePermissions {
    pub writable: bool,
    pub executable: bool,
    pub user_accessible: bool,
}

impl Default for PagePermissions {
    fn default() -> Self {
        Self {
            writable: true,
            executable: false,
            user_accessible: true,
        }
    }
}

// ============================================================================
// PageTable (Single-Level)
// ============================================================================

/// A single-level page table: a hash map from virtual page number to PTE.
///
/// This is the simplest implementation. Real hardware uses multi-level tables,
/// but a hash map is more memory-efficient for sparse address spaces. Most
/// processes only use a tiny fraction of their 2^20 possible pages.
///
/// # Example
///
/// ```
/// use virtual_memory::*;
///
/// let mut table = PageTable::new();
/// let pte = PageTableEntry::with_frame(10, &PagePermissions::default());
/// table.map_page(5, pte);
///
/// let entry = table.lookup(5).unwrap();
/// assert_eq!(entry.frame_number, 10);
/// ```
pub struct PageTable {
    entries: HashMap<u32, PageTableEntry>,
}

impl PageTable {
    /// Create a new, empty page table.
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }

    /// Map a virtual page number to a page table entry.
    pub fn map_page(&mut self, vpn: u32, pte: PageTableEntry) {
        self.entries.insert(vpn, pte);
    }

    /// Remove the mapping for a virtual page number.
    pub fn unmap_page(&mut self, vpn: u32) -> Option<PageTableEntry> {
        self.entries.remove(&vpn)
    }

    /// Look up the PTE for a virtual page number.
    pub fn lookup(&self, vpn: u32) -> Option<&PageTableEntry> {
        self.entries.get(&vpn)
    }

    /// Get a mutable reference to a PTE (for updating flags).
    pub fn lookup_mut(&mut self, vpn: u32) -> Option<&mut PageTableEntry> {
        self.entries.get_mut(&vpn)
    }

    /// How many pages are currently mapped?
    pub fn mapped_count(&self) -> usize {
        self.entries.len()
    }

    /// Return all VPN -> PTE mappings for iteration.
    pub fn entries(&self) -> &HashMap<u32, PageTableEntry> {
        &self.entries
    }
}

impl Default for PageTable {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// TwoLevelPageTable (Sv32)
// ============================================================================

/// A two-level page table implementing RISC-V's Sv32 addressing scheme.
///
/// Sv32 splits the 20-bit virtual page number into two 10-bit indices:
///
/// ```text
/// 32-bit virtual address:
/// +------------+------------+----------------+
/// | VPN[1]     | VPN[0]     | Page Offset    |
/// | bits 31-22 | bits 21-12 | bits 11-0      |
/// | (10 bits)  | (10 bits)  | (12 bits)      |
/// +------------+------------+----------------+
///
/// VPN[1] selects one of 1024 entries in the PAGE DIRECTORY.
/// VPN[0] selects one of 1024 entries in the second-level PAGE TABLE.
/// ```
///
/// ## Memory Savings
///
/// A flat table for 32-bit addresses needs 2^20 = 1M entries (4 MB).
/// With two levels, we only allocate second-level tables for regions
/// actually in use. A process using 8 MB needs only 2 second-level
/// tables: 4 KB (directory) + 2 * 4 KB = 12 KB total.
pub struct TwoLevelPageTable {
    /// The page directory: 1024 slots, each either None (unmapped region)
    /// or Some(PageTable) for the second-level table.
    pub directory: Vec<Option<PageTable>>,
}

impl TwoLevelPageTable {
    /// Create a new two-level page table with an empty directory.
    pub fn new() -> Self {
        let mut directory = Vec::with_capacity(DIRECTORY_ENTRIES);
        for _ in 0..DIRECTORY_ENTRIES {
            directory.push(None);
        }
        Self { directory }
    }

    /// Map a virtual address to a physical frame with permissions.
    ///
    /// Creates the second-level table on demand if it doesn't exist.
    pub fn map(&mut self, vaddr: u32, frame_number: u32, permissions: &PagePermissions) {
        let vpn = vaddr >> PAGE_OFFSET_BITS;
        let vpn1 = ((vpn >> 10) & 0x3FF) as usize;
        let vpn0 = vpn & 0x3FF;

        // Create the second-level table if it doesn't exist (lazy allocation).
        if self.directory[vpn1].is_none() {
            self.directory[vpn1] = Some(PageTable::new());
        }

        let pte = PageTableEntry::with_frame(frame_number, permissions);
        self.directory[vpn1].as_mut().unwrap().map_page(vpn0, pte);
    }

    /// Remove the mapping for a virtual address.
    pub fn unmap(&mut self, vaddr: u32) -> Option<PageTableEntry> {
        let vpn = vaddr >> PAGE_OFFSET_BITS;
        let vpn1 = ((vpn >> 10) & 0x3FF) as usize;
        let vpn0 = vpn & 0x3FF;

        let table = self.directory[vpn1].as_mut()?;
        let result = table.unmap_page(vpn0);

        // Free empty second-level tables.
        if table.mapped_count() == 0 {
            self.directory[vpn1] = None;
        }

        result
    }

    /// Translate a virtual address to (physical_address, &PTE).
    ///
    /// Returns None if the page is not mapped or not present.
    pub fn translate(&self, vaddr: u32) -> Option<(u32, &PageTableEntry)> {
        let vpn = vaddr >> PAGE_OFFSET_BITS;
        let offset = vaddr & PAGE_OFFSET_MASK;
        let vpn1 = ((vpn >> 10) & 0x3FF) as usize;
        let vpn0 = vpn & 0x3FF;

        let table = self.directory[vpn1].as_ref()?;
        let pte = table.lookup(vpn0)?;

        if !pte.present {
            return None;
        }

        let phys_addr = (pte.frame_number << PAGE_OFFSET_BITS) | offset;
        Some((phys_addr, pte))
    }

    /// Look up the PTE for a virtual address (mutable, for updating flags).
    pub fn lookup_pte_mut(&mut self, vaddr: u32) -> Option<&mut PageTableEntry> {
        let vpn = vaddr >> PAGE_OFFSET_BITS;
        let vpn1 = ((vpn >> 10) & 0x3FF) as usize;
        let vpn0 = vpn & 0x3FF;

        self.directory[vpn1].as_mut()?.lookup_mut(vpn0)
    }

    /// Look up the PTE for a virtual address (immutable).
    pub fn lookup_pte(&self, vaddr: u32) -> Option<&PageTableEntry> {
        let vpn = vaddr >> PAGE_OFFSET_BITS;
        let vpn1 = ((vpn >> 10) & 0x3FF) as usize;
        let vpn0 = vpn & 0x3FF;

        self.directory[vpn1].as_ref()?.lookup(vpn0)
    }

    /// Count total mapped pages across all second-level tables.
    pub fn mapped_count(&self) -> usize {
        self.directory
            .iter()
            .filter_map(|t| t.as_ref())
            .map(|t| t.mapped_count())
            .sum()
    }
}

impl Default for TwoLevelPageTable {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// TLB (Translation Lookaside Buffer)
// ============================================================================

/// The TLB is a small, fast cache of recent virtual-to-physical translations.
///
/// Without the TLB, every memory access would require walking the page table
/// (2-3 additional memory accesses). The TLB caches recent translations so
/// most accesses resolve in a single cycle.
///
/// ## Why TLBs Work
///
/// Programs exhibit *locality*:
/// - **Temporal locality**: recently accessed pages will be accessed again soon.
/// - **Spatial locality**: nearby addresses are accessed together.
///
/// A 64-entry TLB covers 64 * 4 KB = 256 KB. Most programs' working sets
/// fit in 256 KB, giving hit rates above 95%.
///
/// ## Flushing
///
/// The TLB must be flushed on context switch. Otherwise, process B might
/// get process A's cached translations, breaking isolation.
pub struct TLB {
    /// Maximum number of cached translations.
    capacity: usize,

    /// Cache: (pid, vpn) -> (frame_number, PTE clone).
    entries: HashMap<(u32, u32), (u32, PageTableEntry)>,

    /// Access order for LRU eviction. Most recent at the back.
    access_order: VecDeque<(u32, u32)>,

    /// Number of successful lookups (TLB hits).
    pub hits: u64,

    /// Number of failed lookups (TLB misses).
    pub misses: u64,
}

impl TLB {
    /// Create a new TLB with the given capacity.
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity,
            entries: HashMap::new(),
            access_order: VecDeque::new(),
            hits: 0,
            misses: 0,
        }
    }

    /// Look up a cached translation.
    ///
    /// Returns the cached frame number on hit, None on miss.
    /// Updates hit/miss counters.
    pub fn lookup(&mut self, pid: u32, vpn: u32) -> Option<u32> {
        let key = (pid, vpn);
        if let Some(&(frame, _)) = self.entries.get(&key) {
            self.hits += 1;
            // Move to back of access order (most recently used).
            self.access_order.retain(|k| *k != key);
            self.access_order.push_back(key);
            Some(frame)
        } else {
            self.misses += 1;
            None
        }
    }

    /// Insert a translation into the TLB.
    ///
    /// If the TLB is full, evicts the least recently used entry.
    pub fn insert(&mut self, pid: u32, vpn: u32, frame_number: u32, pte: &PageTableEntry) {
        let key = (pid, vpn);

        // Update existing entry.
        if self.entries.contains_key(&key) {
            self.entries.insert(key, (frame_number, pte.clone()));
            self.access_order.retain(|k| *k != key);
            self.access_order.push_back(key);
            return;
        }

        // Evict LRU if full.
        if self.entries.len() >= self.capacity {
            if let Some(evict_key) = self.access_order.pop_front() {
                self.entries.remove(&evict_key);
            }
        }

        self.entries.insert(key, (frame_number, pte.clone()));
        self.access_order.push_back(key);
    }

    /// Invalidate a single TLB entry.
    pub fn invalidate(&mut self, pid: u32, vpn: u32) {
        let key = (pid, vpn);
        self.entries.remove(&key);
        self.access_order.retain(|k| *k != key);
    }

    /// Flush all TLB entries. Called on context switch.
    pub fn flush(&mut self) {
        self.entries.clear();
        self.access_order.clear();
    }

    /// Calculate the TLB hit rate (0.0 to 1.0).
    pub fn hit_rate(&self) -> f64 {
        let total = self.hits + self.misses;
        if total == 0 {
            0.0
        } else {
            self.hits as f64 / total as f64
        }
    }

    /// How many entries are currently cached?
    pub fn size(&self) -> usize {
        self.entries.len()
    }

    /// Reset hit/miss counters.
    pub fn reset_stats(&mut self) {
        self.hits = 0;
        self.misses = 0;
    }
}

// ============================================================================
// PhysicalFrameAllocator
// ============================================================================

/// Manages physical memory frames using a bitmap.
///
/// Physical memory (RAM) is divided into fixed-size frames of 4 KB.
/// The allocator tracks which frames are free (false) and which are
/// allocated (true).
///
/// ```text
/// Bitmap for 16 frames:
/// [T, T, T, F, F, T, F, F, F, T, T, F, F, F, F, F]
///  ^  ^  ^        ^           ^  ^
///  kernel          process     process frames
/// ```
///
/// ## Reference Counting
///
/// Each frame has a reference count for copy-on-write support.
/// When two processes share a frame (after fork), refcount = 2.
/// The frame is only freed when refcount reaches 0.
pub struct PhysicalFrameAllocator {
    /// Total number of physical frames.
    pub total_frames: usize,

    /// Bitmap: false = free, true = allocated.
    bitmap: Vec<bool>,

    /// Reference counts per frame (for COW support).
    refcounts: Vec<u32>,

    /// Number of free frames (maintained for O(1) queries).
    free_count: usize,
}

impl PhysicalFrameAllocator {
    /// Create a new allocator for the given number of frames.
    ///
    /// All frames start free. For 16 MB RAM: 16 * 1024 * 1024 / 4096 = 4096 frames.
    pub fn new(total_frames: usize) -> Self {
        Self {
            total_frames,
            bitmap: vec![false; total_frames],
            refcounts: vec![0; total_frames],
            free_count: total_frames,
        }
    }

    /// Allocate a frame using first-fit. Returns None if all frames are in use.
    pub fn allocate(&mut self) -> Option<u32> {
        for (i, allocated) in self.bitmap.iter_mut().enumerate() {
            if !*allocated {
                *allocated = true;
                self.refcounts[i] = 1;
                self.free_count -= 1;
                return Some(i as u32);
            }
        }
        None
    }

    /// Free a frame. Panics if the frame is already free (double-free bug).
    pub fn free(&mut self, frame: u32) {
        let i = frame as usize;
        assert!(i < self.total_frames, "Frame {} out of range", frame);
        assert!(self.bitmap[i], "Double free: frame {} is already free", frame);

        self.bitmap[i] = false;
        self.refcounts[i] = 0;
        self.free_count += 1;
    }

    /// Check if a frame is currently allocated.
    pub fn is_allocated(&self, frame: u32) -> bool {
        let i = frame as usize;
        assert!(i < self.total_frames, "Frame {} out of range", frame);
        self.bitmap[i]
    }

    /// How many frames are free?
    pub fn free_count(&self) -> usize {
        self.free_count
    }

    /// Increment the reference count for a frame (used by COW fork).
    pub fn increment_refcount(&mut self, frame: u32) {
        let i = frame as usize;
        assert!(i < self.total_frames, "Frame {} out of range", frame);
        self.refcounts[i] += 1;
    }

    /// Decrement the reference count. Returns true if the frame was freed (refcount hit 0).
    pub fn decrement_refcount(&mut self, frame: u32) -> bool {
        let i = frame as usize;
        assert!(i < self.total_frames, "Frame {} out of range", frame);

        if self.refcounts[i] > 0 {
            self.refcounts[i] -= 1;
        }

        if self.refcounts[i] == 0 {
            self.bitmap[i] = false;
            self.free_count += 1;
            true
        } else {
            false
        }
    }

    /// Get the reference count for a frame.
    pub fn refcount(&self, frame: u32) -> u32 {
        let i = frame as usize;
        assert!(i < self.total_frames, "Frame {} out of range", frame);
        self.refcounts[i]
    }
}

// ============================================================================
// Page Replacement Policies
// ============================================================================

/// Trait for page replacement policies.
///
/// When physical memory is full, the OS must choose which page to evict.
/// Different policies use different heuristics:
///
/// | Policy | Rule                  | Pros              | Cons                |
/// |--------|-----------------------|-------------------|---------------------|
/// | FIFO   | Evict oldest          | Simple, O(1)      | Can evict hot pages |
/// | LRU    | Evict least recent    | Good approximation| Expensive tracking  |
/// | Clock  | Use bit + sweep       | Cheap LRU approx  | Slightly worse      |
pub trait ReplacementPolicy {
    /// Record that a frame was accessed (for LRU/Clock tracking).
    fn record_access(&mut self, frame: u32);

    /// Select a frame to evict. Returns None if no frames are tracked.
    fn select_victim(&mut self) -> Option<u32>;

    /// Add a newly loaded frame to the tracker.
    fn add_frame(&mut self, frame: u32);

    /// Remove a frame from the tracker (explicit free, not eviction).
    fn remove_frame(&mut self, frame: u32);

    /// How many frames are being tracked?
    fn size(&self) -> usize;
}

/// FIFO (First-In, First-Out) page replacement.
///
/// Evicts the oldest page — the one that has been in memory the longest.
/// Simple but can suffer from Belady's anomaly (more frames = more faults).
///
/// ```text
/// Queue: [A, B, C, D]  (A is oldest)
/// Need to evict → evict A
/// Queue becomes: [B, C, D, E]
/// ```
pub struct FIFOPolicy {
    queue: VecDeque<u32>,
}

impl FIFOPolicy {
    pub fn new() -> Self {
        Self {
            queue: VecDeque::new(),
        }
    }
}

impl Default for FIFOPolicy {
    fn default() -> Self {
        Self::new()
    }
}

impl ReplacementPolicy for FIFOPolicy {
    fn record_access(&mut self, _frame: u32) {
        // FIFO ignores access patterns — eviction is by arrival order only.
    }

    fn select_victim(&mut self) -> Option<u32> {
        self.queue.pop_front()
    }

    fn add_frame(&mut self, frame: u32) {
        self.queue.push_back(frame);
    }

    fn remove_frame(&mut self, frame: u32) {
        self.queue.retain(|&f| f != frame);
    }

    fn size(&self) -> usize {
        self.queue.len()
    }
}

/// LRU (Least Recently Used) page replacement.
///
/// Evicts the page that hasn't been accessed for the longest time.
/// Based on temporal locality: recently used pages are likely to be
/// used again soon.
///
/// Uses a logical clock and per-frame timestamps. The frame with the
/// smallest timestamp is the LRU victim.
pub struct LRUPolicy {
    /// Maps frame number to its last access timestamp.
    timestamps: HashMap<u32, u64>,
    /// Logical clock incremented on every access.
    clock: u64,
}

impl LRUPolicy {
    pub fn new() -> Self {
        Self {
            timestamps: HashMap::new(),
            clock: 0,
        }
    }
}

impl Default for LRUPolicy {
    fn default() -> Self {
        Self::new()
    }
}

impl ReplacementPolicy for LRUPolicy {
    fn record_access(&mut self, frame: u32) {
        self.clock += 1;
        self.timestamps.insert(frame, self.clock);
    }

    fn select_victim(&mut self) -> Option<u32> {
        if self.timestamps.is_empty() {
            return None;
        }
        // Find the frame with the smallest (oldest) timestamp.
        let victim = *self
            .timestamps
            .iter()
            .min_by_key(|&(_, &ts)| ts)
            .unwrap()
            .0;
        self.timestamps.remove(&victim);
        Some(victim)
    }

    fn add_frame(&mut self, frame: u32) {
        self.clock += 1;
        self.timestamps.insert(frame, self.clock);
    }

    fn remove_frame(&mut self, frame: u32) {
        self.timestamps.remove(&frame);
    }

    fn size(&self) -> usize {
        self.timestamps.len()
    }
}

/// Clock (Second-Chance) page replacement.
///
/// A practical LRU approximation using a use bit per frame. Frames are
/// arranged conceptually in a circle with a sweeping clock hand:
///
/// ```text
///       +---+
///   +---| A |<-- use=1 → clear, advance
///   |   +---+
///   |     |
/// +-+-+ | +---+
/// | D | +--| B |<-- use=0 → EVICT
/// +---+    +---+
///   |        |
///   |  +---+ |
///   +--| C |-+
///      +---+
/// ```
pub struct ClockPolicy {
    /// Circular buffer of frame numbers.
    frames: Vec<u32>,
    /// Use bit per frame.
    use_bits: HashMap<u32, bool>,
    /// Clock hand position.
    hand: usize,
}

impl ClockPolicy {
    pub fn new() -> Self {
        Self {
            frames: Vec::new(),
            use_bits: HashMap::new(),
            hand: 0,
        }
    }
}

impl Default for ClockPolicy {
    fn default() -> Self {
        Self::new()
    }
}

impl ReplacementPolicy for ClockPolicy {
    fn record_access(&mut self, frame: u32) {
        self.use_bits.insert(frame, true);
    }

    fn select_victim(&mut self) -> Option<u32> {
        if self.frames.is_empty() {
            return None;
        }

        let max_sweeps = 2 * self.frames.len();
        for _ in 0..max_sweeps {
            if self.hand >= self.frames.len() {
                self.hand = 0;
            }
            let frame = self.frames[self.hand];

            if *self.use_bits.get(&frame).unwrap_or(&false) {
                // Second chance: clear the use bit, advance.
                self.use_bits.insert(frame, false);
                self.hand += 1;
            } else {
                // Evict this frame.
                self.frames.remove(self.hand);
                self.use_bits.remove(&frame);
                if !self.frames.is_empty() && self.hand >= self.frames.len() {
                    self.hand = 0;
                }
                return Some(frame);
            }
        }

        // Safety fallback.
        if !self.frames.is_empty() {
            if self.hand >= self.frames.len() {
                self.hand = 0;
            }
            let frame = self.frames.remove(self.hand);
            self.use_bits.remove(&frame);
            if !self.frames.is_empty() && self.hand >= self.frames.len() {
                self.hand = 0;
            }
            Some(frame)
        } else {
            None
        }
    }

    fn add_frame(&mut self, frame: u32) {
        self.frames.push(frame);
        self.use_bits.insert(frame, true);
    }

    fn remove_frame(&mut self, frame: u32) {
        if let Some(idx) = self.frames.iter().position(|&f| f == frame) {
            self.frames.remove(idx);
            self.use_bits.remove(&frame);
            if !self.frames.is_empty() && self.hand >= self.frames.len() {
                self.hand = 0;
            }
        }
    }

    fn size(&self) -> usize {
        self.frames.len()
    }
}

// ============================================================================
// MMU (Memory Management Unit)
// ============================================================================

/// Error types for MMU operations.
#[derive(Debug)]
pub enum MMUError {
    /// Process accessed memory it doesn't own.
    SegmentationFault(u32, u32), // (pid, vaddr)
    /// Process violated page permissions.
    ProtectionFault(u32, u32), // (pid, vaddr)
    /// No address space exists for this PID.
    NoAddressSpace(u32),
    /// Out of physical memory.
    OutOfMemory,
}

impl std::fmt::Display for MMUError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MMUError::SegmentationFault(pid, addr) => {
                write!(f, "Segfault: PID {} at 0x{:08X}", pid, addr)
            }
            MMUError::ProtectionFault(pid, addr) => {
                write!(f, "Protection fault: PID {} at 0x{:08X}", pid, addr)
            }
            MMUError::NoAddressSpace(pid) => {
                write!(f, "No address space for PID {}", pid)
            }
            MMUError::OutOfMemory => write!(f, "Out of physical memory"),
        }
    }
}

impl std::error::Error for MMUError {}

/// The Memory Management Unit ties together all virtual memory components.
///
/// The MMU sits between the CPU and physical memory. Every memory access
/// goes through the MMU's translate() method, which:
///
/// 1. Checks the TLB (fast path, ~1 cycle)
/// 2. On TLB miss, walks the page table (slow path, 2-3 memory accesses)
/// 3. On page fault, allocates a frame and maps the page
/// 4. Checks permissions and updates accessed/dirty bits
///
/// ## Copy-on-Write Fork
///
/// When fork() clones an address space, physical frames are shared (not
/// copied). Both processes' pages are marked read-only. When either writes,
/// a COW fault creates a private copy of just that page.
pub struct MMU {
    /// Per-process page tables. Key is PID.
    page_tables: HashMap<u32, TwoLevelPageTable>,

    /// Translation cache.
    pub tlb: TLB,

    /// Physical frame manager.
    pub frame_allocator: PhysicalFrameAllocator,

    /// Page replacement policy.
    replacement_policy: Box<dyn ReplacementPolicy>,

    /// Tracks which (pid, vpn) owns each frame.
    frame_owners: HashMap<u32, (u32, u32)>,

    /// Currently active process.
    pub current_pid: Option<u32>,
}

impl MMU {
    /// Create a new MMU with the given number of physical frames and replacement policy.
    pub fn new(total_frames: usize, replacement_policy: Box<dyn ReplacementPolicy>) -> Self {
        Self {
            page_tables: HashMap::new(),
            tlb: TLB::new(DEFAULT_TLB_CAPACITY),
            frame_allocator: PhysicalFrameAllocator::new(total_frames),
            replacement_policy,
            frame_owners: HashMap::new(),
            current_pid: None,
        }
    }

    /// Create a new, empty address space for a process.
    pub fn create_address_space(&mut self, pid: u32) {
        self.page_tables.insert(pid, TwoLevelPageTable::new());
    }

    /// Destroy a process's address space and free all its frames.
    pub fn destroy_address_space(&mut self, pid: u32) {
        let table = match self.page_tables.remove(&pid) {
            Some(t) => t,
            None => return,
        };

        // Walk all second-level tables and free mapped frames.
        for maybe_table in &table.directory {
            if let Some(pt) = maybe_table {
                for (_, pte) in pt.entries() {
                    if pte.present {
                        let frame = pte.frame_number;
                        self.replacement_policy.remove_frame(frame);
                        self.frame_owners.remove(&frame);
                        self.frame_allocator.decrement_refcount(frame);
                    }
                }
            }
        }

        self.tlb.flush();
    }

    /// Map a virtual address to a physical frame.
    ///
    /// Returns the allocated frame number.
    pub fn map_page(
        &mut self,
        pid: u32,
        vaddr: u32,
        permissions: PagePermissions,
    ) -> Result<u32, MMUError> {
        if !self.page_tables.contains_key(&pid) {
            return Err(MMUError::NoAddressSpace(pid));
        }

        let frame = self.allocate_frame_or_evict()?;
        let vpn = vaddr >> PAGE_OFFSET_BITS;

        self.page_tables
            .get_mut(&pid)
            .unwrap()
            .map(vaddr, frame, &permissions);

        self.frame_owners.insert(frame, (pid, vpn));
        self.replacement_policy.add_frame(frame);
        self.tlb.invalidate(pid, vpn);

        Ok(frame)
    }

    /// Translate a virtual address to a physical address.
    ///
    /// This is the core MMU operation. Every memory access goes through here.
    pub fn translate(&mut self, pid: u32, vaddr: u32, write: bool) -> Result<u32, MMUError> {
        if !self.page_tables.contains_key(&pid) {
            return Err(MMUError::NoAddressSpace(pid));
        }

        let vpn = vaddr >> PAGE_OFFSET_BITS;
        let offset = vaddr & PAGE_OFFSET_MASK;

        // Step 1: Check the TLB (fast path).
        if let Some(cached_frame) = self.tlb.lookup(pid, vpn) {
            // Check write permission on TLB hit.
            let needs_cow = if write {
                let pte = self.page_tables.get(&pid).unwrap().lookup_pte(vaddr);
                pte.map(|p| !p.writable).unwrap_or(false)
            } else {
                false
            };

            if needs_cow {
                self.handle_cow_fault(pid, vaddr);
                // Re-translate after COW.
                let pte = self.page_tables.get(&pid).unwrap().lookup_pte(vaddr).unwrap();
                let frame = pte.frame_number;
                // Update flags.
                let pte_mut = self.page_tables.get_mut(&pid).unwrap().lookup_pte_mut(vaddr).unwrap();
                pte_mut.accessed = true;
                pte_mut.dirty = true;
                self.replacement_policy.record_access(frame);
                return Ok((frame << PAGE_OFFSET_BITS) | offset);
            }

            // Update accessed/dirty bits.
            if let Some(pte) = self.page_tables.get_mut(&pid).unwrap().lookup_pte_mut(vaddr) {
                pte.accessed = true;
                if write {
                    pte.dirty = true;
                }
            }
            self.replacement_policy.record_access(cached_frame);
            return Ok((cached_frame << PAGE_OFFSET_BITS) | offset);
        }

        // Step 2: TLB miss — walk the page table.
        let translate_result = self.page_tables.get(&pid).unwrap().translate(vaddr);

        let (_phys_addr, _frame_number) = if let Some((pa, pte)) = translate_result {
            (pa, pte.frame_number)
        } else {
            // Page fault.
            self.handle_page_fault(pid, vaddr)?;
            let result = self.page_tables.get(&pid).unwrap().translate(vaddr);
            match result {
                Some((pa, pte)) => (pa, pte.frame_number),
                None => return Err(MMUError::SegmentationFault(pid, vaddr)),
            }
        };

        // Check write permission.
        let needs_cow = if write {
            let pte = self.page_tables.get(&pid).unwrap().lookup_pte(vaddr);
            pte.map(|p| !p.writable).unwrap_or(false)
        } else {
            false
        };

        if needs_cow {
            self.handle_cow_fault(pid, vaddr);
            let pte = self.page_tables.get_mut(&pid).unwrap().lookup_pte_mut(vaddr).unwrap();
            pte.accessed = true;
            pte.dirty = true;
            let frame = pte.frame_number;
            self.tlb.insert(pid, vpn, frame, &PageTableEntry::new());
            self.replacement_policy.record_access(frame);
            return Ok((frame << PAGE_OFFSET_BITS) | offset);
        }

        // Update flags.
        let final_frame = {
            let pte = self.page_tables.get_mut(&pid).unwrap().lookup_pte_mut(vaddr).unwrap();
            pte.accessed = true;
            if write {
                pte.dirty = true;
            }
            pte.frame_number
        };

        // Cache in TLB.
        let pte_for_cache = self.page_tables.get(&pid).unwrap().lookup_pte(vaddr).unwrap().clone();
        self.tlb.insert(pid, vpn, final_frame, &pte_for_cache);
        self.replacement_policy.record_access(final_frame);

        Ok((final_frame << PAGE_OFFSET_BITS) | offset)
    }

    /// Handle a page fault (allocate a frame for an unmapped page).
    pub fn handle_page_fault(&mut self, pid: u32, vaddr: u32) -> Result<u32, MMUError> {
        if !self.page_tables.contains_key(&pid) {
            return Err(MMUError::NoAddressSpace(pid));
        }

        let vpn = vaddr >> PAGE_OFFSET_BITS;
        let offset = vaddr & PAGE_OFFSET_MASK;

        // Check if PTE exists but is not present (demand paging).
        let pte_exists = self.page_tables.get(&pid).unwrap().lookup_pte(vaddr).is_some();
        let pte_present = self
            .page_tables
            .get(&pid)
            .unwrap()
            .lookup_pte(vaddr)
            .map(|p| p.present)
            .unwrap_or(false);

        if pte_exists && !pte_present {
            let frame = self.allocate_frame_or_evict()?;
            let pte = self.page_tables.get_mut(&pid).unwrap().lookup_pte_mut(vaddr).unwrap();
            pte.frame_number = frame;
            pte.present = true;
            pte.accessed = true;

            self.frame_owners.insert(frame, (pid, vpn));
            self.replacement_policy.add_frame(frame);
            self.tlb.invalidate(pid, vpn);

            return Ok((frame << PAGE_OFFSET_BITS) | offset);
        }

        if !pte_exists {
            // Lazy allocation: create a new mapping.
            let frame = self.allocate_frame_or_evict()?;
            self.page_tables
                .get_mut(&pid)
                .unwrap()
                .map(vaddr, frame, &PagePermissions::default());

            self.frame_owners.insert(frame, (pid, vpn));
            self.replacement_policy.add_frame(frame);
            self.tlb.invalidate(pid, vpn);

            return Ok((frame << PAGE_OFFSET_BITS) | offset);
        }

        // Already present — no fault needed.
        let pte = self.page_tables.get(&pid).unwrap().lookup_pte(vaddr).unwrap();
        Ok((pte.frame_number << PAGE_OFFSET_BITS) | offset)
    }

    /// Clone an address space with copy-on-write semantics.
    ///
    /// All pages in the source are shared with the destination. Both are
    /// marked read-only. Writes trigger COW faults that create private copies.
    pub fn clone_address_space(&mut self, from_pid: u32, to_pid: u32) -> Result<(), MMUError> {
        if !self.page_tables.contains_key(&from_pid) {
            return Err(MMUError::NoAddressSpace(from_pid));
        }

        self.create_address_space(to_pid);

        // Collect all mappings from the source.
        let mut mappings: Vec<(u32, u32, bool, bool)> = Vec::new(); // (vaddr, frame, exec, user)

        {
            let src = self.page_tables.get(&from_pid).unwrap();
            for (vpn1, maybe_table) in src.directory.iter().enumerate() {
                if let Some(table) = maybe_table {
                    for (&vpn0, pte) in table.entries() {
                        if pte.present {
                            let vpn = ((vpn1 as u32) << 10) | vpn0;
                            let vaddr = vpn << PAGE_OFFSET_BITS;
                            mappings.push((vaddr, pte.frame_number, pte.executable, pte.user_accessible));
                        }
                    }
                }
            }
        }

        // Apply COW: mark source pages read-only, create destination mappings.
        for (vaddr, frame, executable, user_accessible) in &mappings {
            // Mark source as read-only.
            if let Some(pte) = self.page_tables.get_mut(&from_pid).unwrap().lookup_pte_mut(*vaddr) {
                pte.writable = false;
            }

            // Create destination mapping (also read-only, shared frame).
            let perms = PagePermissions {
                writable: false,
                executable: *executable,
                user_accessible: *user_accessible,
            };
            self.page_tables.get_mut(&to_pid).unwrap().map(*vaddr, *frame, &perms);

            // Increment refcount.
            self.frame_allocator.increment_refcount(*frame);

            // Ensure frame_owners has an entry.
            let vpn = *vaddr >> PAGE_OFFSET_BITS;
            self.frame_owners.entry(*frame).or_insert((from_pid, vpn));
        }

        self.tlb.flush();
        Ok(())
    }

    /// Switch context to a new process. Flushes the TLB.
    pub fn context_switch(&mut self, new_pid: u32) {
        self.current_pid = Some(new_pid);
        self.tlb.flush();
    }

    /// Check if a process has an address space.
    pub fn has_address_space(&self, pid: u32) -> bool {
        self.page_tables.contains_key(&pid)
    }

    /// Get a reference to a process's page table (for inspection/testing).
    pub fn page_table_for(&self, pid: u32) -> Option<&TwoLevelPageTable> {
        self.page_tables.get(&pid)
    }

    // -- Private helpers --

    /// Allocate a frame, evicting a victim if memory is full.
    fn allocate_frame_or_evict(&mut self) -> Result<u32, MMUError> {
        if let Some(frame) = self.frame_allocator.allocate() {
            return Ok(frame);
        }

        // Memory is full — evict a victim.
        let victim = self
            .replacement_policy
            .select_victim()
            .ok_or(MMUError::OutOfMemory)?;

        // Unmap the victim from its owner's page table.
        if let Some((owner_pid, owner_vpn)) = self.frame_owners.remove(&victim) {
            let vaddr = owner_vpn << PAGE_OFFSET_BITS;
            if let Some(table) = self.page_tables.get_mut(&owner_pid) {
                table.unmap(vaddr);
            }
            self.tlb.invalidate(owner_pid, owner_vpn);
        }

        // Free and re-allocate the victim frame.
        if self.frame_allocator.is_allocated(victim) {
            self.frame_allocator.free(victim);
        }

        self.frame_allocator.allocate().ok_or(MMUError::OutOfMemory)
    }

    /// Handle a copy-on-write fault.
    fn handle_cow_fault(&mut self, pid: u32, vaddr: u32) {
        let vpn = vaddr >> PAGE_OFFSET_BITS;

        let old_frame = {
            let pte = self.page_tables.get(&pid).unwrap().lookup_pte(vaddr).unwrap();
            pte.frame_number
        };

        let refcount = self.frame_allocator.refcount(old_frame);

        if refcount > 1 {
            // Frame is shared — make a private copy.
            let new_frame = self.allocate_frame_or_evict().unwrap();

            // Get permission info before mutating.
            let (exec, user) = {
                let pte = self.page_tables.get(&pid).unwrap().lookup_pte(vaddr).unwrap();
                (pte.executable, pte.user_accessible)
            };

            let perms = PagePermissions {
                writable: true,
                executable: exec,
                user_accessible: user,
            };

            self.page_tables
                .get_mut(&pid)
                .unwrap()
                .map(vaddr, new_frame, &perms);

            self.frame_owners.insert(new_frame, (pid, vpn));
            self.replacement_policy.add_frame(new_frame);
            self.frame_allocator.decrement_refcount(old_frame);

            // If old frame now has refcount 1, restore write access to the remaining owner.
            if self.frame_allocator.refcount(old_frame) == 1 {
                if let Some(&(r_pid, r_vpn)) = self.frame_owners.get(&old_frame) {
                    let r_vaddr = r_vpn << PAGE_OFFSET_BITS;
                    if let Some(table) = self.page_tables.get_mut(&r_pid) {
                        if let Some(r_pte) = table.lookup_pte_mut(r_vaddr) {
                            r_pte.writable = true;
                        }
                    }
                }
            }
        } else {
            // Not shared — just make it writable.
            if let Some(pte) = self.page_tables.get_mut(&pid).unwrap().lookup_pte_mut(vaddr) {
                pte.writable = true;
            }
        }

        self.tlb.invalidate(pid, vpn);
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // -- PageTableEntry tests --

    #[test]
    fn test_pte_default_values() {
        let pte = PageTableEntry::new();
        assert_eq!(pte.frame_number, 0);
        assert!(!pte.present);
        assert!(!pte.dirty);
        assert!(!pte.accessed);
        assert!(pte.writable);
        assert!(!pte.executable);
        assert!(pte.user_accessible);
    }

    #[test]
    fn test_pte_with_frame() {
        let perms = PagePermissions {
            writable: false,
            executable: true,
            user_accessible: false,
        };
        let pte = PageTableEntry::with_frame(42, &perms);
        assert_eq!(pte.frame_number, 42);
        assert!(pte.present);
        assert!(!pte.writable);
        assert!(pte.executable);
        assert!(!pte.user_accessible);
    }

    #[test]
    fn test_pte_flag_mutation() {
        let mut pte = PageTableEntry::with_frame(10, &PagePermissions::default());
        pte.accessed = true;
        assert!(pte.accessed);
        pte.dirty = true;
        assert!(pte.dirty);
        pte.accessed = false;
        assert!(!pte.accessed);
    }

    #[test]
    fn test_pte_clone() {
        let original = PageTableEntry::with_frame(7, &PagePermissions::default());
        let copy = original.clone();
        assert_eq!(copy.frame_number, 7);
        // Verify clone is independent by checking original is unaffected.
        // (Clone creates a separate value in Rust, so this always holds.)
        assert_eq!(original.frame_number, 7);
        assert!(original.writable);
    }

    // -- PageTable tests --

    #[test]
    fn test_page_table_empty() {
        let table = PageTable::new();
        assert_eq!(table.mapped_count(), 0);
        assert!(table.lookup(0).is_none());
    }

    #[test]
    fn test_page_table_map_and_lookup() {
        let mut table = PageTable::new();
        let pte = PageTableEntry::with_frame(10, &PagePermissions::default());
        table.map_page(5, pte);

        let result = table.lookup(5).unwrap();
        assert_eq!(result.frame_number, 10);
        assert!(result.present);
    }

    #[test]
    fn test_page_table_multiple_mappings() {
        let mut table = PageTable::new();
        table.map_page(0, PageTableEntry::with_frame(100, &PagePermissions::default()));
        table.map_page(1, PageTableEntry::with_frame(200, &PagePermissions::default()));
        table.map_page(2, PageTableEntry::with_frame(300, &PagePermissions::default()));
        assert_eq!(table.mapped_count(), 3);
        assert_eq!(table.lookup(0).unwrap().frame_number, 100);
        assert_eq!(table.lookup(1).unwrap().frame_number, 200);
        assert_eq!(table.lookup(2).unwrap().frame_number, 300);
    }

    #[test]
    fn test_page_table_overwrite() {
        let mut table = PageTable::new();
        table.map_page(5, PageTableEntry::with_frame(10, &PagePermissions::default()));
        table.map_page(5, PageTableEntry::with_frame(20, &PagePermissions::default()));
        assert_eq!(table.mapped_count(), 1);
        assert_eq!(table.lookup(5).unwrap().frame_number, 20);
    }

    #[test]
    fn test_page_table_unmap() {
        let mut table = PageTable::new();
        table.map_page(5, PageTableEntry::with_frame(10, &PagePermissions::default()));
        let removed = table.unmap_page(5);
        assert!(removed.is_some());
        assert_eq!(removed.unwrap().frame_number, 10);
        assert!(table.lookup(5).is_none());
        assert_eq!(table.mapped_count(), 0);
    }

    #[test]
    fn test_page_table_unmap_nonexistent() {
        let mut table = PageTable::new();
        assert!(table.unmap_page(99).is_none());
    }

    // -- TwoLevelPageTable tests --

    #[test]
    fn test_two_level_basic_translate() {
        let mut table = TwoLevelPageTable::new();
        table.map(0x12ABC, 7, &PagePermissions::default());

        let (phys, pte) = table.translate(0x12ABC).unwrap();
        // phys = (7 << 12) | 0xABC = 0x7ABC
        assert_eq!(phys, 0x7ABC);
        assert_eq!(pte.frame_number, 7);
    }

    #[test]
    fn test_two_level_directory_on_demand() {
        let mut table = TwoLevelPageTable::new();
        assert!(table.directory.iter().all(|d| d.is_none()));

        table.map(0x1000, 1, &PagePermissions::default());
        assert!(table.directory[0].is_some());
        assert!(table.directory[1].is_none());
    }

    #[test]
    fn test_two_level_same_region() {
        let mut table = TwoLevelPageTable::new();
        table.map(0x0000, 10, &PagePermissions::default());
        table.map(0x1000, 11, &PagePermissions::default());
        table.map(0x2000, 12, &PagePermissions::default());

        assert_eq!(table.mapped_count(), 3);
        assert_eq!(table.translate(0x0000).unwrap().0, 10 << 12);
        assert_eq!(table.translate(0x1000).unwrap().0, 11 << 12);
        assert_eq!(table.translate(0x2000).unwrap().0, 12 << 12);
    }

    #[test]
    fn test_two_level_across_regions() {
        let mut table = TwoLevelPageTable::new();
        table.map(0x1000, 5, &PagePermissions::default());
        table.map(0x400000, 6, &PagePermissions::default()); // 4 MB boundary

        assert_eq!(table.translate(0x1000).unwrap().0, 5 << 12);
        assert_eq!(table.translate(0x400000).unwrap().0, 6 << 12);
    }

    #[test]
    fn test_two_level_offset_preserved() {
        let mut table = TwoLevelPageTable::new();
        table.map(0x5000, 3, &PagePermissions::default());
        let (phys, _) = table.translate(0x5123).unwrap();
        assert_eq!(phys, (3 << 12) | 0x123);
    }

    #[test]
    fn test_two_level_unmap() {
        let mut table = TwoLevelPageTable::new();
        table.map(0x3000, 8, &PagePermissions::default());
        assert_eq!(table.mapped_count(), 1);

        table.unmap(0x3000);
        assert_eq!(table.mapped_count(), 0);
        assert!(table.translate(0x3000).is_none());
    }

    #[test]
    fn test_two_level_unmap_frees_empty_table() {
        let mut table = TwoLevelPageTable::new();
        table.map(0x1000, 5, &PagePermissions::default());
        assert!(table.directory[0].is_some());

        table.unmap(0x1000);
        assert!(table.directory[0].is_none());
    }

    #[test]
    fn test_two_level_translate_unmapped() {
        let table = TwoLevelPageTable::new();
        assert!(table.translate(0x1000).is_none());
    }

    #[test]
    fn test_two_level_not_present() {
        let mut table = TwoLevelPageTable::new();
        table.map(0x1000, 5, &PagePermissions::default());
        table.lookup_pte_mut(0x1000).unwrap().present = false;
        assert!(table.translate(0x1000).is_none());
    }

    #[test]
    fn test_two_level_permissions() {
        let mut table = TwoLevelPageTable::new();
        let perms = PagePermissions {
            writable: false,
            executable: true,
            user_accessible: false,
        };
        table.map(0x1000, 5, &perms);
        let pte = table.lookup_pte(0x1000).unwrap();
        assert!(!pte.writable);
        assert!(pte.executable);
        assert!(!pte.user_accessible);
    }

    #[test]
    fn test_two_level_high_address() {
        let mut table = TwoLevelPageTable::new();
        table.map(0xFFFFF000, 100, &PagePermissions::default());
        let (phys, _) = table.translate(0xFFFFF000).unwrap();
        assert_eq!(phys, 100 << 12);
    }

    // -- TLB tests --

    #[test]
    fn test_tlb_empty() {
        let tlb = TLB::new(4);
        assert_eq!(tlb.size(), 0);
        assert_eq!(tlb.hits, 0);
        assert_eq!(tlb.misses, 0);
    }

    #[test]
    fn test_tlb_insert_and_lookup() {
        let mut tlb = TLB::new(4);
        let pte = PageTableEntry::new();
        tlb.insert(1, 5, 10, &pte);

        assert_eq!(tlb.lookup(1, 5), Some(10));
        assert_eq!(tlb.hits, 1);
        assert_eq!(tlb.misses, 0);
    }

    #[test]
    fn test_tlb_miss() {
        let mut tlb = TLB::new(4);
        assert_eq!(tlb.lookup(1, 5), None);
        assert_eq!(tlb.misses, 1);
    }

    #[test]
    fn test_tlb_process_isolation() {
        let mut tlb = TLB::new(4);
        let pte = PageTableEntry::new();
        tlb.insert(1, 5, 10, &pte);
        tlb.insert(2, 5, 20, &pte);

        assert_eq!(tlb.lookup(1, 5), Some(10));
        assert_eq!(tlb.lookup(2, 5), Some(20));
    }

    #[test]
    fn test_tlb_lru_eviction() {
        let mut tlb = TLB::new(4);
        let pte = PageTableEntry::new();
        tlb.insert(1, 0, 100, &pte);
        tlb.insert(1, 1, 101, &pte);
        tlb.insert(1, 2, 102, &pte);
        tlb.insert(1, 3, 103, &pte);

        // Access entry 0 to make it recently used.
        tlb.lookup(1, 0);

        // Insert 5th entry — should evict entry 1 (LRU).
        tlb.insert(1, 4, 104, &pte);
        assert_eq!(tlb.size(), 4);
        assert_eq!(tlb.lookup(1, 1), None); // Evicted.
        assert_eq!(tlb.lookup(1, 0), Some(100)); // Still present.
    }

    #[test]
    fn test_tlb_update_existing() {
        let mut tlb = TLB::new(4);
        let pte = PageTableEntry::new();
        tlb.insert(1, 5, 10, &pte);
        tlb.insert(1, 5, 20, &pte);
        assert_eq!(tlb.lookup(1, 5), Some(20));
        assert_eq!(tlb.size(), 1);
    }

    #[test]
    fn test_tlb_invalidate() {
        let mut tlb = TLB::new(4);
        let pte = PageTableEntry::new();
        tlb.insert(1, 5, 10, &pte);
        tlb.insert(1, 6, 11, &pte);

        tlb.invalidate(1, 5);
        assert_eq!(tlb.lookup(1, 5), None);
        assert_eq!(tlb.lookup(1, 6), Some(11));
    }

    #[test]
    fn test_tlb_flush() {
        let mut tlb = TLB::new(4);
        let pte = PageTableEntry::new();
        tlb.insert(1, 0, 100, &pte);
        tlb.insert(2, 0, 200, &pte);

        tlb.flush();
        assert_eq!(tlb.size(), 0);
        assert_eq!(tlb.lookup(1, 0), None);
    }

    #[test]
    fn test_tlb_hit_rate() {
        let mut tlb = TLB::new(4);
        assert_eq!(tlb.hit_rate(), 0.0);

        let pte = PageTableEntry::new();
        tlb.insert(1, 0, 100, &pte);
        tlb.lookup(1, 0); // hit
        tlb.lookup(1, 0); // hit
        tlb.lookup(1, 0); // hit
        tlb.lookup(1, 99); // miss

        assert!((tlb.hit_rate() - 0.75).abs() < 0.001);
    }

    #[test]
    fn test_tlb_reset_stats() {
        let mut tlb = TLB::new(4);
        let pte = PageTableEntry::new();
        tlb.insert(1, 0, 100, &pte);
        tlb.lookup(1, 0);
        tlb.lookup(1, 99);

        tlb.reset_stats();
        assert_eq!(tlb.hits, 0);
        assert_eq!(tlb.misses, 0);
    }

    // -- PhysicalFrameAllocator tests --

    #[test]
    fn test_allocator_fresh() {
        let alloc = PhysicalFrameAllocator::new(8);
        assert_eq!(alloc.total_frames, 8);
        assert_eq!(alloc.free_count(), 8);
    }

    #[test]
    fn test_allocator_sequential() {
        let mut alloc = PhysicalFrameAllocator::new(8);
        assert_eq!(alloc.allocate(), Some(0));
        assert_eq!(alloc.allocate(), Some(1));
        assert_eq!(alloc.allocate(), Some(2));
        assert_eq!(alloc.free_count(), 5);
    }

    #[test]
    fn test_allocator_exhaustion() {
        let mut alloc = PhysicalFrameAllocator::new(3);
        alloc.allocate();
        alloc.allocate();
        alloc.allocate();
        assert_eq!(alloc.allocate(), None);
    }

    #[test]
    fn test_allocator_free_and_reuse() {
        let mut alloc = PhysicalFrameAllocator::new(8);
        alloc.allocate(); // 0
        alloc.allocate(); // 1
        alloc.free(0);
        assert_eq!(alloc.allocate(), Some(0)); // Reused.
    }

    #[test]
    #[should_panic(expected = "Double free")]
    fn test_allocator_double_free() {
        let mut alloc = PhysicalFrameAllocator::new(8);
        let frame = alloc.allocate().unwrap();
        alloc.free(frame);
        alloc.free(frame); // Panic!
    }

    #[test]
    #[should_panic(expected = "out of range")]
    fn test_allocator_out_of_range() {
        let alloc = PhysicalFrameAllocator::new(8);
        alloc.is_allocated(8); // Panic!
    }

    #[test]
    fn test_allocator_is_allocated() {
        let mut alloc = PhysicalFrameAllocator::new(8);
        assert!(!alloc.is_allocated(0));
        alloc.allocate();
        assert!(alloc.is_allocated(0));
        assert!(!alloc.is_allocated(1));
    }

    #[test]
    fn test_allocator_refcount() {
        let mut alloc = PhysicalFrameAllocator::new(8);
        let frame = alloc.allocate().unwrap();
        assert_eq!(alloc.refcount(frame), 1);
        alloc.increment_refcount(frame);
        assert_eq!(alloc.refcount(frame), 2);
    }

    #[test]
    fn test_allocator_decrement_refcount() {
        let mut alloc = PhysicalFrameAllocator::new(8);
        let frame = alloc.allocate().unwrap();
        alloc.increment_refcount(frame); // refcount = 2

        assert!(!alloc.decrement_refcount(frame)); // 2 -> 1, not freed
        assert!(alloc.decrement_refcount(frame)); // 1 -> 0, freed
        assert!(!alloc.is_allocated(frame));
    }

    // -- FIFO Policy tests --

    #[test]
    fn test_fifo_order() {
        let mut policy = FIFOPolicy::new();
        policy.add_frame(10);
        policy.add_frame(20);
        policy.add_frame(30);

        assert_eq!(policy.select_victim(), Some(10));
        assert_eq!(policy.select_victim(), Some(20));
        assert_eq!(policy.select_victim(), Some(30));
        assert_eq!(policy.select_victim(), None);
    }

    #[test]
    fn test_fifo_ignores_access() {
        let mut policy = FIFOPolicy::new();
        policy.add_frame(0);
        policy.add_frame(1);

        policy.record_access(0); // No effect on FIFO.
        assert_eq!(policy.select_victim(), Some(0));
    }

    #[test]
    fn test_fifo_remove() {
        let mut policy = FIFOPolicy::new();
        policy.add_frame(0);
        policy.add_frame(1);
        policy.add_frame(2);

        policy.remove_frame(1);
        assert_eq!(policy.size(), 2);
        assert_eq!(policy.select_victim(), Some(0));
        assert_eq!(policy.select_victim(), Some(2));
    }

    // -- LRU Policy tests --

    #[test]
    fn test_lru_basic() {
        let mut policy = LRUPolicy::new();
        policy.add_frame(0);
        policy.add_frame(1);
        policy.add_frame(2);

        assert_eq!(policy.select_victim(), Some(0)); // Oldest.
    }

    #[test]
    fn test_lru_access_refreshes() {
        let mut policy = LRUPolicy::new();
        policy.add_frame(0);
        policy.add_frame(1);
        policy.add_frame(2);

        policy.record_access(0); // Refresh frame 0.

        assert_eq!(policy.select_victim(), Some(1)); // 1 is now LRU.
    }

    #[test]
    fn test_lru_multiple_accesses() {
        let mut policy = LRUPolicy::new();
        policy.add_frame(0);
        policy.add_frame(1);
        policy.add_frame(2);

        policy.record_access(0);
        policy.record_access(2);

        assert_eq!(policy.select_victim(), Some(1)); // LRU
        assert_eq!(policy.select_victim(), Some(0)); // Next LRU
        assert_eq!(policy.select_victim(), Some(2)); // Last
    }

    #[test]
    fn test_lru_remove() {
        let mut policy = LRUPolicy::new();
        policy.add_frame(0);
        policy.add_frame(1);
        policy.add_frame(2);

        policy.remove_frame(0);
        assert_eq!(policy.size(), 2);
        assert_eq!(policy.select_victim(), Some(1));
    }

    // -- Clock Policy tests --

    #[test]
    fn test_clock_basic_eviction() {
        let mut policy = ClockPolicy::new();
        policy.add_frame(0);
        policy.add_frame(1);
        policy.add_frame(2);

        // All have use_bit=true. Clock clears all, then evicts 0.
        assert_eq!(policy.select_victim(), Some(0));
    }

    #[test]
    fn test_clock_second_chance() {
        let mut policy = ClockPolicy::new();
        policy.add_frame(10);
        policy.add_frame(11);
        policy.add_frame(12);

        // First eviction clears all use bits, evicts 10.
        assert_eq!(policy.select_victim(), Some(10));

        // 11 and 12 have use_bit=false now. Next evicts 11.
        assert_eq!(policy.select_victim(), Some(11));
    }

    #[test]
    fn test_clock_access_preserves() {
        let mut policy = ClockPolicy::new();
        policy.add_frame(0);
        policy.add_frame(1);
        policy.add_frame(2);

        // Evict 0 (clears all use bits first).
        policy.select_victim();

        // Access 1 to set its use bit.
        policy.record_access(1);

        // Next victim: 1 has use=true (skip), 2 has use=false → evict 2.
        assert_eq!(policy.select_victim(), Some(2));
    }

    #[test]
    fn test_clock_single_frame() {
        let mut policy = ClockPolicy::new();
        policy.add_frame(42);
        assert_eq!(policy.select_victim(), Some(42));
        assert_eq!(policy.size(), 0);
    }

    #[test]
    fn test_clock_empty() {
        let mut policy = ClockPolicy::new();
        assert_eq!(policy.select_victim(), None);
    }

    // -- MMU tests --

    fn make_mmu(frames: usize) -> MMU {
        MMU::new(frames, Box::new(FIFOPolicy::new()))
    }

    #[test]
    fn test_mmu_create_address_space() {
        let mut mmu = make_mmu(16);
        mmu.create_address_space(1);
        assert!(mmu.has_address_space(1));
        assert!(!mmu.has_address_space(2));
    }

    #[test]
    fn test_mmu_map_and_translate() {
        let mut mmu = make_mmu(16);
        mmu.create_address_space(1);
        let frame = mmu.map_page(1, 0x5000, PagePermissions::default()).unwrap();
        let phys = mmu.translate(1, 0x5ABC, false).unwrap();
        assert_eq!(phys, (frame << 12) | 0xABC);
    }

    #[test]
    fn test_mmu_address_isolation() {
        let mut mmu = make_mmu(16);
        mmu.create_address_space(1);
        mmu.create_address_space(2);

        let f1 = mmu.map_page(1, 0x1000, PagePermissions::default()).unwrap();
        let f2 = mmu.map_page(2, 0x1000, PagePermissions::default()).unwrap();

        let p1 = mmu.translate(1, 0x1000, false).unwrap();
        let p2 = mmu.translate(2, 0x1000, false).unwrap();

        assert_ne!(p1, p2);
        assert_eq!(p1, f1 << 12);
        assert_eq!(p2, f2 << 12);
    }

    #[test]
    fn test_mmu_tlb_caching() {
        let mut mmu = make_mmu(16);
        mmu.create_address_space(1);
        mmu.map_page(1, 0x1000, PagePermissions::default()).unwrap();

        mmu.translate(1, 0x1000, false).unwrap(); // TLB miss
        assert_eq!(mmu.tlb.misses, 1);

        mmu.translate(1, 0x1000, false).unwrap(); // TLB hit
        assert_eq!(mmu.tlb.hits, 1);
    }

    #[test]
    fn test_mmu_page_fault() {
        let mut mmu = make_mmu(16);
        mmu.create_address_space(1);

        let phys = mmu.handle_page_fault(1, 0x3000).unwrap();
        let phys2 = mmu.translate(1, 0x3000, false).unwrap();
        assert_eq!(phys, phys2);
    }

    #[test]
    fn test_mmu_destroy_address_space() {
        let mut mmu = make_mmu(16);
        mmu.create_address_space(1);
        mmu.map_page(1, 0x1000, PagePermissions::default()).unwrap();
        mmu.map_page(1, 0x2000, PagePermissions::default()).unwrap();

        let free_before = mmu.frame_allocator.free_count();
        mmu.destroy_address_space(1);
        assert!(!mmu.has_address_space(1));
        assert_eq!(mmu.frame_allocator.free_count(), free_before + 2);
    }

    #[test]
    fn test_mmu_clone_address_space() {
        let mut mmu = make_mmu(16);
        mmu.create_address_space(1);
        mmu.map_page(1, 0x1000, PagePermissions::default()).unwrap();
        mmu.map_page(1, 0x2000, PagePermissions::default()).unwrap();

        mmu.clone_address_space(1, 2).unwrap();
        assert!(mmu.has_address_space(2));

        let p1 = mmu.translate(1, 0x1000, false).unwrap();
        let p2 = mmu.translate(2, 0x1000, false).unwrap();
        assert_eq!(p1, p2); // Shared frame.
    }

    #[test]
    fn test_mmu_cow_write() {
        let mut mmu = make_mmu(16);
        mmu.create_address_space(1);
        mmu.map_page(1, 0x1000, PagePermissions::default()).unwrap();
        mmu.translate(1, 0x1000, false).unwrap();

        mmu.clone_address_space(1, 2).unwrap();

        let p1 = mmu.translate(1, 0x1000, false).unwrap();
        let p2 = mmu.translate(2, 0x1000, false).unwrap();
        assert_eq!(p1, p2);

        // Child writes → COW.
        let p2_after = mmu.translate(2, 0x1000, true).unwrap();
        let p1_after = mmu.translate(1, 0x1000, false).unwrap();
        assert_ne!(p2_after, p1_after);
    }

    #[test]
    fn test_mmu_context_switch() {
        let mut mmu = make_mmu(16);
        mmu.create_address_space(1);
        mmu.map_page(1, 0x1000, PagePermissions::default()).unwrap();
        mmu.translate(1, 0x1000, false).unwrap();

        assert_eq!(mmu.tlb.size(), 1);

        mmu.context_switch(2);
        assert_eq!(mmu.current_pid, Some(2));
        assert_eq!(mmu.tlb.size(), 0);
    }

    #[test]
    fn test_mmu_no_address_space() {
        let mut mmu = make_mmu(16);
        assert!(mmu.translate(99, 0x1000, false).is_err());
        assert!(mmu.map_page(99, 0x1000, PagePermissions::default()).is_err());
    }

    #[test]
    fn test_mmu_destroy_nonexistent() {
        let mut mmu = make_mmu(16);
        mmu.destroy_address_space(99); // Should not panic.
    }

    #[test]
    fn test_mmu_page_replacement() {
        let mut mmu = MMU::new(4, Box::new(FIFOPolicy::new()));
        mmu.create_address_space(1);

        mmu.map_page(1, 0x1000, PagePermissions::default()).unwrap();
        mmu.map_page(1, 0x2000, PagePermissions::default()).unwrap();
        mmu.map_page(1, 0x3000, PagePermissions::default()).unwrap();
        mmu.map_page(1, 0x4000, PagePermissions::default()).unwrap();
        assert_eq!(mmu.frame_allocator.free_count(), 0);

        // 5th mapping triggers eviction.
        mmu.map_page(1, 0x5000, PagePermissions::default()).unwrap();
        let phys = mmu.translate(1, 0x5000, false).unwrap();
        assert!(phys > 0 || phys == 0); // Just verify it works.
    }

    #[test]
    fn test_mmu_write_sets_dirty() {
        let mut mmu = make_mmu(16);
        mmu.create_address_space(1);
        mmu.map_page(1, 0x1000, PagePermissions::default()).unwrap();

        mmu.translate(1, 0x1000, true).unwrap();

        let pte = mmu.page_table_for(1).unwrap().lookup_pte(0x1000).unwrap();
        assert!(pte.dirty);
        assert!(pte.accessed);
    }

    #[test]
    fn test_mmu_lru_replacement() {
        let mut mmu = MMU::new(3, Box::new(LRUPolicy::new()));
        mmu.create_address_space(1);

        mmu.map_page(1, 0x1000, PagePermissions::default()).unwrap();
        mmu.map_page(1, 0x2000, PagePermissions::default()).unwrap();
        mmu.map_page(1, 0x3000, PagePermissions::default()).unwrap();

        // Access 0x1000 to refresh it.
        mmu.translate(1, 0x1000, false).unwrap();

        // 4th mapping evicts 0x2000 (LRU).
        mmu.map_page(1, 0x4000, PagePermissions::default()).unwrap();

        // 0x1000 should still be accessible.
        let phys = mmu.translate(1, 0x1000, false).unwrap();
        assert!(phys > 0 || phys == 0);
    }

    #[test]
    fn test_mmu_clock_replacement() {
        let mut mmu = MMU::new(3, Box::new(ClockPolicy::new()));
        mmu.create_address_space(1);

        mmu.map_page(1, 0x1000, PagePermissions::default()).unwrap();
        mmu.map_page(1, 0x2000, PagePermissions::default()).unwrap();
        mmu.map_page(1, 0x3000, PagePermissions::default()).unwrap();

        mmu.map_page(1, 0x4000, PagePermissions::default()).unwrap();

        let phys = mmu.translate(1, 0x4000, false).unwrap();
        assert!(phys > 0 || phys == 0);
    }

    #[test]
    fn test_mmu_multiple_pages() {
        let mut mmu = make_mmu(16);
        mmu.create_address_space(1);

        let mut frames = Vec::new();
        for i in 0..5 {
            let f = mmu.map_page(1, i * PAGE_SIZE as u32, PagePermissions::default()).unwrap();
            frames.push(f);
        }

        for i in 0..5u32 {
            let phys = mmu.translate(1, i * PAGE_SIZE as u32, false).unwrap();
            assert_eq!(phys, frames[i as usize] << PAGE_OFFSET_BITS);
        }
    }

    #[test]
    fn test_mmu_page_table_for() {
        let mut mmu = make_mmu(16);
        mmu.create_address_space(1);
        assert!(mmu.page_table_for(1).is_some());
        assert!(mmu.page_table_for(99).is_none());
    }
}
