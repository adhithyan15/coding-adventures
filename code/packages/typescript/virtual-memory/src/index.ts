/**
 * # Virtual Memory Subsystem
 *
 * Virtual memory is one of the most important abstractions in computer science.
 * It gives every process the illusion that it has the entire memory space to
 * itself — starting at address 0, stretching to some large upper limit — even
 * though the physical machine has limited RAM shared among many processes.
 *
 * ## Analogy: The Apartment Building
 *
 * Imagine an apartment building. Each tenant thinks their apartment number
 * starts at "Room 1" — they have Room 1 (bedroom), Room 2 (kitchen), Room 3
 * (bathroom). But the building manager knows the truth: Tenant A's "Room 1"
 * is actually physical room 401, Tenant B's "Room 1" is physical room 712.
 * The tenants never need to know their real room numbers. They just say
 * "go to my Room 1" and the building manager (the MMU) translates.
 *
 * ## How addresses are split
 *
 * A 32-bit virtual address is split into two parts:
 *
 * ```
 * ┌──────────────────────────┬────────────────┐
 * │ Virtual Page Number (VPN)│ Page Offset    │
 * │ bits 31–12 (20 bits)     │ bits 11–0      │
 * │                          │ (12 bits)      │
 * └──────────────────────────┴────────────────┘
 * ```
 *
 * The offset (12 bits) addresses each byte within a 4 KB page (2^12 = 4096).
 * The VPN identifies which page. The physical address is formed by replacing
 * the VPN with the physical frame number:
 *
 *   physical_address = (frame_number << 12) | offset
 *
 * @module
 */

// ============================================================================
// Constants
// ============================================================================

/**
 * PAGE_SIZE is 4096 bytes (4 KB). This has been the standard page size since
 * the Intel 386 in 1985, and RISC-V uses it too. It is a compromise between
 * small pages (less internal fragmentation, but bigger page tables) and large
 * pages (smaller tables, but more waste when a process uses only part of a page).
 */
export const PAGE_SIZE = 4096;

/**
 * PAGE_OFFSET_BITS is 12 because 2^12 = 4096 = PAGE_SIZE. The lower 12 bits
 * of any address identify the byte offset within a page. The upper bits
 * identify which page.
 */
export const PAGE_OFFSET_BITS = 12;

/**
 * PAGE_OFFSET_MASK isolates the lower 12 bits of an address: 0xFFF.
 * Usage: offset = address & PAGE_OFFSET_MASK
 */
export const PAGE_OFFSET_MASK = PAGE_SIZE - 1; // 0xFFF

/**
 * DEFAULT_TLB_CAPACITY: Real TLBs have 32-256 entries. We use 64 for
 * simulation — large enough to demonstrate locality effects, small enough
 * to test eviction behavior.
 */
export const DEFAULT_TLB_CAPACITY = 64;

// ============================================================================
// PageTableEntry
// ============================================================================

/**
 * Flags for configuring page permissions. These mirror the RISC-V Sv32 PTE
 * format:
 *
 * ```
 * ┌────────────────────┬───┬───┬───┬───┬───┬───┬───┬───┐
 * │ PPN (frame number) │ D │ A │ G │ U │ X │ W │ R │ V │
 * │ bits 31–10         │ 7 │ 6 │ 5 │ 4 │ 3 │ 2 │ 1 │ 0 │
 * └────────────────────┴───┴───┴───┴───┴───┴───┴───┴───┘
 * ```
 *
 * V = Valid (present), R = Readable, W = Writable, X = Executable,
 * U = User-accessible, A = Accessed, D = Dirty
 */
export interface PageFlags {
  writable?: boolean;
  executable?: boolean;
  user_accessible?: boolean;
}

/**
 * A PageTableEntry describes the mapping for one virtual page. Each field
 * corresponds to a hardware bit in a real page table entry:
 *
 * - **frame_number**: Which physical frame this page maps to. Only meaningful
 *   when present is true.
 *
 * - **present**: Is this page currently in physical memory? If false, accessing
 *   it triggers a page fault (interrupt 14). A page might not be present because
 *   it was never allocated, was swapped to disk, or is a lazy allocation.
 *
 * - **dirty**: Has this page been written to since it was loaded? If true, it
 *   must be written back to disk before the frame can be reused.
 *
 * - **accessed**: Has this page been read or written recently? Used by page
 *   replacement algorithms (Clock/LRU) to decide which page to evict.
 *
 * - **writable**: Can this page be written to? Code pages are read-only.
 *   Stack/heap pages are writable. Copy-on-write pages start read-only.
 *
 * - **executable**: Can this page contain executable code? Data pages should
 *   not be executable (NX bit — prevents code injection attacks).
 *
 * - **user_accessible**: Can user-mode code access this page? Kernel pages
 *   are not user-accessible to prevent user programs from reading/writing
 *   kernel memory.
 */
export class PageTableEntry {
  frame_number: number;
  present: boolean;
  dirty: boolean;
  accessed: boolean;
  writable: boolean;
  executable: boolean;
  user_accessible: boolean;

  constructor(
    frame_number: number,
    flags: {
      present?: boolean;
      dirty?: boolean;
      accessed?: boolean;
      writable?: boolean;
      executable?: boolean;
      user_accessible?: boolean;
    } = {}
  ) {
    this.frame_number = frame_number;
    this.present = flags.present ?? true;
    this.dirty = flags.dirty ?? false;
    this.accessed = flags.accessed ?? false;
    this.writable = flags.writable ?? false;
    this.executable = flags.executable ?? false;
    this.user_accessible = flags.user_accessible ?? false;
  }

  /**
   * Create a deep copy of this PTE. Used during COW cloning — each process
   * needs its own PTE so that modifying flags in one process does not affect
   * the other.
   */
  clone(): PageTableEntry {
    return new PageTableEntry(this.frame_number, {
      present: this.present,
      dirty: this.dirty,
      accessed: this.accessed,
      writable: this.writable,
      executable: this.executable,
      user_accessible: this.user_accessible,
    });
  }
}

// ============================================================================
// PageTable (Single-Level)
// ============================================================================

/**
 * The simplest page table: a hash map from virtual page number to PTE.
 *
 * ```
 * PageTable:
 *   entries: Map<vpn, PTE>
 *
 *   VPN 0 → PTE { frame=4, present=true, ... }
 *   VPN 1 → PTE { frame=2, present=true, ... }
 *   VPN 2 → PTE { frame=0, present=true, ... }
 * ```
 *
 * This is simple but does not match how real hardware works. Real CPUs walk
 * multi-level page tables in hardware — they do not have hash map circuits.
 * We use this as a building block for the two-level page table.
 */
export class PageTable {
  /** Internal storage: maps virtual page numbers to their page table entries. */
  private entries: Map<number, PageTableEntry> = new Map();

  /**
   * Map a virtual page to a physical frame with the given flags.
   *
   * @param vpn - The virtual page number (0, 1, 2, ...).
   * @param frame_number - The physical frame number this page maps to.
   * @param flags - Permission flags for this page.
   */
  map_page(
    vpn: number,
    frame_number: number,
    flags: PageFlags = {}
  ): void {
    const pte = new PageTableEntry(frame_number, {
      present: true,
      writable: flags.writable ?? false,
      executable: flags.executable ?? false,
      user_accessible: flags.user_accessible ?? false,
    });
    this.entries.set(vpn, pte);
  }

  /**
   * Remove a mapping for a virtual page.
   *
   * @param vpn - The virtual page number to unmap.
   * @returns The PTE that was removed, or undefined if none existed.
   */
  unmap_page(vpn: number): PageTableEntry | undefined {
    const pte = this.entries.get(vpn);
    this.entries.delete(vpn);
    return pte;
  }

  /**
   * Look up the PTE for a virtual page number.
   *
   * @param vpn - The virtual page number to look up.
   * @returns The PTE if mapped, undefined otherwise.
   */
  lookup(vpn: number): PageTableEntry | undefined {
    return this.entries.get(vpn);
  }

  /** How many pages are currently mapped in this table. */
  mapped_count(): number {
    return this.entries.size;
  }

  /**
   * Get all mapped VPNs. Used during clone/destroy operations to iterate
   * over every mapping.
   */
  get_all_vpns(): number[] {
    return Array.from(this.entries.keys());
  }

  /**
   * Insert a PTE directly (used by TwoLevelPageTable internals).
   */
  insert(vpn: number, pte: PageTableEntry): void {
    this.entries.set(vpn, pte);
  }
}

// ============================================================================
// TwoLevelPageTable (Sv32)
// ============================================================================

/**
 * RISC-V's Sv32 scheme splits the 20-bit VPN into two 10-bit indices:
 *
 * ```
 * 32-bit virtual address:
 * ┌────────────┬────────────┬────────────────┐
 * │ VPN[1]     │ VPN[0]     │ Page Offset    │
 * │ bits 31–22 │ bits 21–12 │ bits 11–0      │
 * │ (10 bits)  │ (10 bits)  │ (12 bits)      │
 * └────────────┴────────────┴────────────────┘
 *
 * VPN[1] indexes into the PAGE DIRECTORY (1024 entries)
 * VPN[0] indexes into a PAGE TABLE      (1024 entries)
 * ```
 *
 * Why two levels? A single flat page table for a 32-bit address space would
 * need 2^20 = 1,048,576 entries (4 MB per process). With two levels, we only
 * allocate second-level tables for regions actually in use. Most processes
 * only need a handful of second-level tables.
 *
 * The directory has 1024 slots. Each slot can hold a PageTable (second-level)
 * or be undefined (meaning that entire 4 MB region is unmapped).
 * 1024 entries x 4 MB each = 4 GB total addressable.
 */
export class TwoLevelPageTable {
  /**
   * The page directory: an array of 1024 optional second-level page tables.
   * directory[i] is either a PageTable or undefined.
   */
  private directory: (PageTable | undefined)[] = new Array(1024).fill(
    undefined
  );

  /**
   * Extract the two VPN indices from a virtual address.
   *
   * Given address 0x00012ABC:
   *   vpn = 0x00012ABC >> 12 = 0x12 = 18
   *   vpn1 = (18 >> 10) & 0x3FF = 0  (which 4MB region)
   *   vpn0 = 18 & 0x3FF = 18         (which page within that region)
   *   offset = 0x00012ABC & 0xFFF = 0xABC
   */
  static splitAddress(vaddr: number): {
    vpn1: number;
    vpn0: number;
    offset: number;
  } {
    // Use >>> 0 to ensure unsigned 32-bit arithmetic.
    // JavaScript bitwise operators work on signed 32-bit integers,
    // so without >>> 0, addresses with bit 31 set would be negative.
    const addr = vaddr >>> 0;
    const vpn = addr >>> PAGE_OFFSET_BITS;
    const vpn1 = (vpn >>> 10) & 0x3ff;
    const vpn0 = vpn & 0x3ff;
    const offset = addr & PAGE_OFFSET_MASK;
    return { vpn1, vpn0, offset };
  }

  /**
   * Map a virtual address to a physical frame.
   *
   * Creates the second-level page table on demand if it does not exist yet.
   * This is the key advantage of two-level page tables: memory for page table
   * entries is only allocated for address regions that are actually used.
   *
   * @param vaddr - The virtual address (only the page-aligned part matters).
   * @param frame_number - The physical frame to map to.
   * @param flags - Permission flags for this mapping.
   */
  map(vaddr: number, frame_number: number, flags: PageFlags = {}): void {
    const { vpn1, vpn0 } = TwoLevelPageTable.splitAddress(vaddr);

    // Create second-level table on demand — this is what makes two-level
    // page tables memory-efficient. We only allocate tables for regions
    // that have at least one mapping.
    if (this.directory[vpn1] === undefined) {
      this.directory[vpn1] = new PageTable();
    }

    this.directory[vpn1]!.map_page(vpn0, frame_number, flags);
  }

  /**
   * Translate a virtual address to a physical address + PTE.
   *
   * This is the page table "walk" that hardware performs on every memory
   * access (when the TLB misses):
   *
   * 1. Use VPN[1] to index into the page directory
   * 2. If the directory entry is empty, the page is unmapped
   * 3. Use VPN[0] to index into the second-level page table
   * 4. If the PTE is not present, trigger a page fault
   * 5. Combine the frame number with the offset to get the physical address
   *
   * @returns An object with phys_addr and pte, or undefined if unmapped.
   */
  translate(
    vaddr: number
  ): { phys_addr: number; pte: PageTableEntry } | undefined {
    const { vpn1, vpn0, offset } = TwoLevelPageTable.splitAddress(vaddr);

    const table = this.directory[vpn1];
    if (table === undefined) {
      return undefined;
    }

    const pte = table.lookup(vpn0);
    if (pte === undefined || !pte.present) {
      return undefined;
    }

    // Physical address = (frame_number << 12) | offset
    // Use >>> 0 for unsigned result in case frame_number is large.
    const phys_addr = ((pte.frame_number << PAGE_OFFSET_BITS) | offset) >>> 0;
    return { phys_addr, pte };
  }

  /**
   * Remove a mapping for a virtual address.
   *
   * @returns The PTE that was removed, or undefined if none existed.
   */
  unmap(vaddr: number): PageTableEntry | undefined {
    const { vpn1, vpn0 } = TwoLevelPageTable.splitAddress(vaddr);

    const table = this.directory[vpn1];
    if (table === undefined) {
      return undefined;
    }

    return table.unmap_page(vpn0);
  }

  /**
   * Look up the PTE for a virtual address without computing the physical
   * address. Useful for checking flags or modifying the entry in place.
   */
  lookupPTE(vaddr: number): PageTableEntry | undefined {
    const { vpn1, vpn0 } = TwoLevelPageTable.splitAddress(vaddr);
    const table = this.directory[vpn1];
    if (table === undefined) return undefined;
    return table.lookup(vpn0);
  }

  /**
   * Iterate over all mappings in the page table. Yields [vaddr, PTE] pairs
   * where vaddr is the base address of each mapped page.
   *
   * Used by clone_address_space and destroy_address_space.
   */
  allMappings(): Array<{ vaddr: number; pte: PageTableEntry }> {
    const result: Array<{ vaddr: number; pte: PageTableEntry }> = [];
    for (let vpn1 = 0; vpn1 < 1024; vpn1++) {
      const table = this.directory[vpn1];
      if (table === undefined) continue;
      for (const vpn0 of table.get_all_vpns()) {
        const pte = table.lookup(vpn0);
        if (pte !== undefined) {
          // Reconstruct the virtual address from vpn1 and vpn0:
          // vpn = (vpn1 << 10) | vpn0, vaddr = vpn << 12
          const vaddr = (((vpn1 << 10) | vpn0) << PAGE_OFFSET_BITS) >>> 0;
          result.push({ vaddr, pte });
        }
      }
    }
    return result;
  }
}

// ============================================================================
// TLB (Translation Lookaside Buffer)
// ============================================================================

/**
 * The TLB is a small, fast cache that remembers recent virtual-to-physical
 * address translations. Without a TLB, every memory access would require
 * 2-3 additional memory accesses just to walk the page table.
 *
 * How it works:
 * - On every memory access, the CPU checks the TLB first (fast path).
 * - If the translation is cached (TLB hit), use it immediately.
 * - If not (TLB miss), walk the page table (slow path) and cache the result.
 *
 * A good TLB has >95% hit rate. Programs tend to access the same pages
 * repeatedly (temporal locality), so a small TLB captures most translations.
 *
 * The TLB is keyed by (pid, vpn) so that different processes can have the
 * same VPN without conflict. On context switch, the TLB is flushed to
 * prevent process B from seeing process A's translations.
 */
export class TLB {
  /** Maximum number of entries the TLB can hold. */
  readonly capacity: number;

  /**
   * The cache: maps "pid:vpn" string keys to { frame, pte } values.
   * We use a Map to maintain insertion order for LRU eviction.
   */
  private entries: Map<string, { frame: number; pte: PageTableEntry }> =
    new Map();

  /** Number of successful lookups (translation was cached). */
  hits: number = 0;

  /** Number of failed lookups (required a page table walk). */
  misses: number = 0;

  constructor(capacity: number = DEFAULT_TLB_CAPACITY) {
    this.capacity = capacity;
  }

  /** Create a cache key from pid and vpn. */
  private key(pid: number, vpn: number): string {
    return `${pid}:${vpn}`;
  }

  /**
   * Look up a cached translation.
   *
   * @returns The cached frame number and PTE, or undefined on miss.
   */
  lookup(
    pid: number,
    vpn: number
  ): { frame: number; pte: PageTableEntry } | undefined {
    const k = this.key(pid, vpn);
    const entry = this.entries.get(k);
    if (entry !== undefined) {
      this.hits++;
      // Move to end for LRU: delete and re-insert to make it "most recent"
      this.entries.delete(k);
      this.entries.set(k, entry);
      return entry;
    }
    this.misses++;
    return undefined;
  }

  /**
   * Insert a translation into the TLB cache.
   *
   * If the TLB is at capacity, evict the least-recently-used entry.
   * The Map maintains insertion order, so the first entry is the oldest
   * (or least recently accessed, since lookup() moves entries to the end).
   */
  insert(pid: number, vpn: number, frame: number, pte: PageTableEntry): void {
    const k = this.key(pid, vpn);

    // If already present, delete so re-insert moves it to the end (most recent)
    if (this.entries.has(k)) {
      this.entries.delete(k);
    }

    // Evict the LRU entry if at capacity
    if (this.entries.size >= this.capacity) {
      // Map.keys().next() gives us the first (oldest/LRU) key
      const firstKey = this.entries.keys().next().value;
      if (firstKey !== undefined) {
        this.entries.delete(firstKey);
      }
    }

    this.entries.set(k, { frame, pte });
  }

  /**
   * Invalidate a specific entry. Called when a mapping changes (e.g., after
   * a page fault resolves or a page is unmapped).
   */
  invalidate(pid: number, vpn: number): void {
    this.entries.delete(this.key(pid, vpn));
  }

  /**
   * Flush ALL entries. Called on context switch because the new process has
   * a different page table. If we did not flush, the new process might get
   * the old process's translations — a security hole!
   *
   * This is why context switches are expensive: the TLB must be rebuilt
   * from scratch for the new process.
   */
  flush(): void {
    this.entries.clear();
  }

  /**
   * Calculate the hit rate: hits / (hits + misses).
   *
   * A TLB hit rate above 95% is good. Programs exhibit temporal locality —
   * they access the same pages repeatedly — so even a small TLB captures
   * most translations.
   *
   * @returns A number between 0 and 1, or 0 if no lookups have been performed.
   */
  hit_rate(): number {
    const total = this.hits + this.misses;
    if (total === 0) return 0;
    return this.hits / total;
  }

  /** How many entries are currently cached. */
  size(): number {
    return this.entries.size;
  }
}

// ============================================================================
// PhysicalFrameAllocator
// ============================================================================

/**
 * The PhysicalFrameAllocator manages which physical frames are free and which
 * are in use. It uses a boolean array (conceptually a bitmap) where each
 * element represents one frame: false = free, true = in use.
 *
 * For 16 MB RAM with 4 KB frames: 16 MB / 4 KB = 4096 frames.
 *
 * ```
 * Frame bitmap example (16 frames):
 * [T, T, T, F, F, T, F, F, F, T, T, F, F, F, F, F]
 *  ^  ^  ^         ^  ^
 *  kernel frames   process frames
 * ```
 *
 * allocate() scans linearly for the first free frame — simple but O(n).
 * Real allocators use free lists or buddy systems for O(1) allocation.
 */
export class PhysicalFrameAllocator {
  /** Total number of physical frames available. */
  readonly total_frames: number;

  /**
   * Bitmap of frame allocation status.
   * frames[i] = true means frame i is allocated.
   * frames[i] = false means frame i is free.
   */
  private frames: boolean[];

  /** How many frames are currently free. Maintained for O(1) queries. */
  private _free_count: number;

  constructor(total_frames: number) {
    this.total_frames = total_frames;
    this.frames = new Array(total_frames).fill(false);
    this._free_count = total_frames;
  }

  /**
   * Allocate a physical frame.
   *
   * Scans the bitmap linearly for the first free frame, marks it as used,
   * and returns its number. Returns undefined if no frames are available
   * (out of memory!).
   */
  allocate(): number | undefined {
    for (let i = 0; i < this.total_frames; i++) {
      if (!this.frames[i]) {
        this.frames[i] = true;
        this._free_count--;
        return i;
      }
    }
    return undefined; // Out of memory!
  }

  /**
   * Free a previously allocated frame.
   *
   * @throws Error if the frame is already free (double-free is a bug).
   * @throws Error if the frame number is out of range.
   */
  free(frame_number: number): void {
    if (frame_number < 0 || frame_number >= this.total_frames) {
      throw new Error(
        `Frame number ${frame_number} out of range [0, ${this.total_frames})`
      );
    }
    if (!this.frames[frame_number]) {
      throw new Error(
        `Double-free: frame ${frame_number} is already free`
      );
    }
    this.frames[frame_number] = false;
    this._free_count++;
  }

  /** Check if a frame is currently allocated. */
  is_allocated(frame_number: number): boolean {
    if (frame_number < 0 || frame_number >= this.total_frames) {
      throw new Error(
        `Frame number ${frame_number} out of range [0, ${this.total_frames})`
      );
    }
    return this.frames[frame_number];
  }

  /** How many frames are currently free. */
  free_count(): number {
    return this._free_count;
  }
}

// ============================================================================
// Page Replacement Policies
// ============================================================================

/**
 * A ReplacementPolicy decides which page to evict when physical memory is
 * full and a new frame is needed. The goal is to evict the page least likely
 * to be used soon.
 *
 * Three classic policies are implemented:
 * - **FIFO**: Evict the oldest page (simplest, but can be pathological).
 * - **LRU**: Evict the least recently accessed page (optimal in practice,
 *   but expensive to maintain).
 * - **Clock**: A practical approximation of LRU using "use bits" and a
 *   sweeping clock hand (what most real operating systems use).
 */
export interface ReplacementPolicy {
  /** Record that a frame was accessed (read or written). */
  record_access(frame: number): void;
  /** Select a frame to evict. Returns undefined if no frames are tracked. */
  select_victim(): number | undefined;
  /** Start tracking a newly allocated frame. */
  add_frame(frame: number): void;
  /** Stop tracking a frame (it was freed explicitly). */
  remove_frame(frame: number): void;
}

// ============================================================================
// FIFO Policy
// ============================================================================

/**
 * FIFO (First-In, First-Out): evict the oldest page — the one that has been
 * in memory the longest.
 *
 * ```
 * Queue: [A, B, C, D]  (A is oldest)
 * Need to evict -> evict A
 * Queue becomes: [B, C, D, E]  (E is newly loaded)
 * ```
 *
 * FIFO is simple but can be pathological: it might evict a frequently used
 * page just because it was loaded a long time ago. A classic example is
 * Belady's anomaly, where adding more frames can actually increase page faults
 * under FIFO.
 */
export class FIFOPolicy implements ReplacementPolicy {
  /** Queue of frame numbers, ordered by arrival time. */
  private queue: number[] = [];

  record_access(_frame: number): void {
    // FIFO ignores access patterns — it only cares about arrival order.
    // This is its weakness: a page accessed every cycle is treated the same
    // as a page accessed once and never again.
  }

  /**
   * Select the oldest frame for eviction.
   * @returns The frame number of the oldest frame, or undefined if empty.
   */
  select_victim(): number | undefined {
    if (this.queue.length === 0) return undefined;
    return this.queue.shift()!;
  }

  /** Add a newly allocated frame to the back of the queue. */
  add_frame(frame: number): void {
    this.queue.push(frame);
  }

  /** Remove a frame from tracking (it was freed explicitly). */
  remove_frame(frame: number): void {
    const idx = this.queue.indexOf(frame);
    if (idx !== -1) {
      this.queue.splice(idx, 1);
    }
  }
}

// ============================================================================
// LRU Policy
// ============================================================================

/**
 * LRU (Least Recently Used): evict the page that has not been accessed for
 * the longest time. Based on temporal locality — if a page was used recently,
 * it will probably be used again soon.
 *
 * ```
 * Access order: [C, A, D, B]  (C is least recently used)
 * Process accesses A -> [C, D, B, A]
 * Need to evict -> evict C
 * ```
 *
 * LRU is near-optimal in practice but expensive: every memory access must
 * update the access order. Hardware approximates it with the accessed bit.
 * We use a Map (which maintains insertion order) with delete-and-reinsert
 * to move accessed entries to the end.
 */
export class LRUPolicy implements ReplacementPolicy {
  /**
   * Map of frame numbers to a monotonically increasing timestamp.
   * We use a Map to maintain insertion order. The first entry is the LRU.
   */
  private access_order: Map<number, number> = new Map();

  /** Monotonically increasing counter for timestamps. */
  private timestamp: number = 0;

  /**
   * Record an access to a frame. Moves it to the "most recently used" position
   * by deleting and reinserting.
   */
  record_access(frame: number): void {
    if (this.access_order.has(frame)) {
      this.access_order.delete(frame);
    }
    this.access_order.set(frame, this.timestamp++);
  }

  /**
   * Select the least recently used frame for eviction.
   * The first entry in the Map is the oldest (LRU).
   */
  select_victim(): number | undefined {
    const first = this.access_order.keys().next().value;
    if (first === undefined) return undefined;
    this.access_order.delete(first);
    return first;
  }

  /** Add a newly allocated frame, marking it as most recently used. */
  add_frame(frame: number): void {
    this.access_order.set(frame, this.timestamp++);
  }

  /** Remove a frame from tracking. */
  remove_frame(frame: number): void {
    this.access_order.delete(frame);
  }
}

// ============================================================================
// Clock Policy
// ============================================================================

/**
 * Clock (Second-Chance): a practical approximation of LRU that uses a "use bit"
 * for each frame. Frames are arranged in a circular buffer. A "clock hand"
 * sweeps around looking for a frame to evict:
 *
 * ```
 *     ┌───┐
 * ┌───┤ A │◄── use=1 -> clear, move on
 * │   └───┘
 * │     │
 * ┌───┐ │ ┌───┐
 * │ D │ └─┤ B │◄── use=0 -> EVICT THIS ONE
 * └───┘   └───┘
 * │         │
 * │   ┌───┐ │
 * └───┤ C ├─┘
 *     └───┘
 * ```
 *
 * The "second chance" name comes from the fact that a page with its use bit
 * set gets one more pass before eviction. If the page is accessed again before
 * the hand comes back around, its bit will be set again and it survives.
 *
 * This is what most real operating systems use because it is simple, efficient,
 * and provides a good approximation of LRU behavior.
 */
export class ClockPolicy implements ReplacementPolicy {
  /** Circular list of frame numbers. */
  private frames: number[] = [];

  /** Use bit for each frame: true means "recently accessed". */
  private use_bits: Map<number, boolean> = new Map();

  /** The clock hand position — index into the frames array. */
  private hand: number = 0;

  /**
   * Record an access: set the use bit for this frame. When the clock hand
   * reaches this frame, it will get a "second chance" instead of being evicted.
   */
  record_access(frame: number): void {
    if (this.use_bits.has(frame)) {
      this.use_bits.set(frame, true);
    }
  }

  /**
   * Select a victim frame for eviction using the clock algorithm:
   *
   * 1. Look at the frame under the clock hand.
   * 2. If its use bit is 0 (not recently accessed), evict it.
   * 3. If its use bit is 1 (recently accessed), clear the bit (second chance)
   *    and advance the hand to the next frame.
   * 4. Repeat until a victim is found.
   *
   * In the worst case, the hand goes around the entire circle twice:
   * once to clear all use bits, once to find a frame with use=0.
   */
  select_victim(): number | undefined {
    if (this.frames.length === 0) return undefined;

    // We scan at most 2 * frames.length to guarantee finding a victim
    // (one pass to clear all use bits, one pass to find use=0).
    const maxScans = this.frames.length * 2;
    for (let i = 0; i < maxScans; i++) {
      const frame = this.frames[this.hand];
      if (!this.use_bits.get(frame)) {
        // Use bit is clear — evict this frame
        this.frames.splice(this.hand, 1);
        this.use_bits.delete(frame);
        // Adjust hand if we removed the last element
        if (this.frames.length > 0) {
          this.hand = this.hand % this.frames.length;
        } else {
          this.hand = 0;
        }
        return frame;
      }
      // Use bit is set — give it a second chance
      this.use_bits.set(frame, false);
      this.hand = (this.hand + 1) % this.frames.length;
    }

    // Should never reach here if frames.length > 0, but just in case:
    return undefined;
  }

  /** Add a newly allocated frame with its use bit set. */
  add_frame(frame: number): void {
    this.frames.push(frame);
    this.use_bits.set(frame, true);
  }

  /** Remove a frame from tracking. */
  remove_frame(frame: number): void {
    const idx = this.frames.indexOf(frame);
    if (idx !== -1) {
      this.frames.splice(idx, 1);
      this.use_bits.delete(frame);
      // Adjust hand if needed
      if (this.frames.length > 0) {
        if (idx < this.hand) {
          this.hand--;
        }
        this.hand = this.hand % this.frames.length;
      } else {
        this.hand = 0;
      }
    }
  }
}

// ============================================================================
// MMU (Memory Management Unit)
// ============================================================================

/**
 * The MMU is the central component that ties everything together. It manages:
 *
 * - **Page tables**: One per process, mapping virtual addresses to physical frames.
 * - **TLB**: A shared translation cache for fast lookups.
 * - **Frame allocator**: Tracks which physical frames are free.
 * - **Replacement policy**: Decides which page to evict when memory is full.
 * - **Reference counts**: Tracks how many processes share each physical frame
 *   (for copy-on-write support).
 *
 * Every memory access by every process goes through the MMU's translate()
 * method. This is the single most performance-critical path in the entire
 * operating system.
 */
export class MMU {
  /** One page table per process, keyed by PID. */
  private page_tables: Map<number, TwoLevelPageTable> = new Map();

  /** Translation cache shared by all processes. */
  readonly tlb: TLB;

  /** Manages physical frame allocation. */
  readonly frame_allocator: PhysicalFrameAllocator;

  /** Which algorithm to use when evicting pages. */
  private replacement_policy: ReplacementPolicy;

  /**
   * Reference counts for physical frames. When a frame's refcount drops
   * to 0, it can be freed. This is essential for copy-on-write: after
   * fork(), both parent and child share the same frame (refcount=2).
   * When one process writes (triggering COW), a new frame is allocated
   * and the old frame's refcount drops to 1.
   */
  private frame_refcounts: Map<number, number> = new Map();

  /** The currently active process ID (for TLB context). */
  private current_pid: number | undefined;

  constructor(
    total_frames: number,
    policy: ReplacementPolicy,
    tlb_capacity: number = DEFAULT_TLB_CAPACITY
  ) {
    this.frame_allocator = new PhysicalFrameAllocator(total_frames);
    this.replacement_policy = policy;
    this.tlb = new TLB(tlb_capacity);
  }

  /**
   * Create a new, empty address space for a process.
   * Called by exec() or when creating a new process from scratch.
   *
   * @param pid - The process ID.
   */
  create_address_space(pid: number): void {
    this.page_tables.set(pid, new TwoLevelPageTable());
  }

  /**
   * Destroy a process's address space, freeing all owned frames.
   * Called when a process exits.
   *
   * For each mapped page:
   * 1. Decrement the frame's reference count.
   * 2. If the refcount drops to 0, free the physical frame.
   * 3. Delete the page table.
   *
   * @param pid - The process ID.
   */
  destroy_address_space(pid: number): void {
    const pt = this.page_tables.get(pid);
    if (pt === undefined) return;

    for (const { pte } of pt.allMappings()) {
      if (pte.present) {
        this.decrement_refcount(pte.frame_number);
      }
    }

    this.page_tables.delete(pid);
  }

  /**
   * Map a virtual page to a newly allocated physical frame.
   *
   * This is the kernel's way of giving a process memory. The kernel calls
   * map_page() for each page the process needs (stack, heap, code, etc.).
   *
   * @param pid - The process ID.
   * @param vaddr - The virtual address (page-aligned).
   * @param flags - Permission flags.
   * @returns The allocated frame number, or undefined if out of memory.
   */
  map_page(pid: number, vaddr: number, flags: PageFlags = {}): number | undefined {
    const pt = this.page_tables.get(pid);
    if (pt === undefined) {
      throw new Error(`No address space for PID ${pid}`);
    }

    // Allocate a physical frame
    let frame = this.frame_allocator.allocate();
    if (frame === undefined) {
      // Try to evict a page to free a frame
      frame = this.evict_page();
      if (frame === undefined) return undefined;
    }

    pt.map(vaddr, frame, flags);
    this.set_refcount(frame, 1);
    this.replacement_policy.add_frame(frame);

    return frame;
  }

  /**
   * Translate a virtual address to a physical address.
   *
   * This is the core operation of the MMU. Every memory access goes through
   * this path:
   *
   * 1. Split the address into VPN and offset.
   * 2. Check the TLB (fast path).
   * 3. On TLB miss, walk the page table (slow path).
   * 4. On page fault (not present), call handle_page_fault.
   * 5. Update accessed/dirty bits.
   * 6. Cache in TLB for next time.
   * 7. Compute physical address = (frame << 12) | offset.
   *
   * @param pid - The process ID.
   * @param vaddr - The virtual address to translate.
   * @param write - Whether this is a write access (sets dirty bit).
   * @returns The physical address.
   * @throws Error if the address is unmapped and cannot be faulted in.
   */
  translate(pid: number, vaddr: number, write: boolean = false): number {
    const addr = vaddr >>> 0;
    const vpn = addr >>> PAGE_OFFSET_BITS;
    const offset = addr & PAGE_OFFSET_MASK;

    // Step 1: Check the TLB (fast path)
    const tlbResult = this.tlb.lookup(pid, vpn);
    if (tlbResult !== undefined) {
      // TLB hit! Update accessed/dirty bits on the PTE
      tlbResult.pte.accessed = true;
      if (write) {
        // Check write permission
        if (!tlbResult.pte.writable) {
          // Could be a COW page — handle the fault
          const phys = this.handle_cow_or_fault(pid, vaddr, write);
          return phys;
        }
        tlbResult.pte.dirty = true;
      }
      this.replacement_policy.record_access(tlbResult.frame);
      return ((tlbResult.frame << PAGE_OFFSET_BITS) | offset) >>> 0;
    }

    // Step 2: TLB miss — walk the page table
    const pt = this.page_tables.get(pid);
    if (pt === undefined) {
      throw new Error(`No address space for PID ${pid}`);
    }

    let result = pt.translate(addr);

    // Step 3: Page fault — page not present
    if (result === undefined) {
      const phys = this.handle_page_fault(pid, vaddr);
      return phys;
    }

    const { pte } = result;

    // Step 4: Check write permission
    if (write && !pte.writable) {
      const phys = this.handle_cow_or_fault(pid, vaddr, write);
      return phys;
    }

    // Step 5: Update accessed/dirty bits
    pte.accessed = true;
    if (write) {
      pte.dirty = true;
    }

    // Step 6: Cache in TLB
    this.tlb.insert(pid, vpn, pte.frame_number, pte);
    this.replacement_policy.record_access(pte.frame_number);

    // Step 7: Compute physical address
    return ((pte.frame_number << PAGE_OFFSET_BITS) | offset) >>> 0;
  }

  /**
   * Handle a page fault: the process accessed a virtual page that is not
   * currently mapped to a physical frame.
   *
   * This allocates a new frame, maps it, and returns the physical address.
   * In a real OS, this would also load the page from disk if it was swapped
   * out, but we simulate demand paging by just allocating a zeroed frame.
   *
   * @param pid - The process ID.
   * @param vaddr - The faulting virtual address.
   * @returns The physical address after the fault is resolved.
   */
  handle_page_fault(pid: number, vaddr: number): number {
    const addr = vaddr >>> 0;
    const offset = addr & PAGE_OFFSET_MASK;

    // Allocate a frame
    let frame = this.frame_allocator.allocate();
    if (frame === undefined) {
      frame = this.evict_page();
      if (frame === undefined) {
        throw new Error("Out of memory: no frames available and no pages to evict");
      }
    }

    const pt = this.page_tables.get(pid);
    if (pt === undefined) {
      throw new Error(`No address space for PID ${pid}`);
    }

    // Map the page with default permissions (writable, user-accessible)
    pt.map(addr, frame, { writable: true, user_accessible: true });
    this.set_refcount(frame, 1);
    this.replacement_policy.add_frame(frame);

    // Update the PTE
    const pte = pt.lookupPTE(addr);
    if (pte) {
      pte.accessed = true;
    }

    // Cache in TLB
    const vpn = addr >>> PAGE_OFFSET_BITS;
    if (pte) {
      this.tlb.insert(pid, vpn, frame, pte);
    }

    return ((frame << PAGE_OFFSET_BITS) | offset) >>> 0;
  }

  /**
   * Handle a write to a read-only page. This could be a COW (copy-on-write)
   * page shared with another process. If so, make a private copy.
   */
  private handle_cow_or_fault(
    pid: number,
    vaddr: number,
    _write: boolean
  ): number {
    const addr = vaddr >>> 0;
    const offset = addr & PAGE_OFFSET_MASK;
    const vpn = addr >>> PAGE_OFFSET_BITS;

    const pt = this.page_tables.get(pid);
    if (pt === undefined) {
      throw new Error(`No address space for PID ${pid}`);
    }

    const pte = pt.lookupPTE(addr);
    if (pte === undefined || !pte.present) {
      // Not a COW fault — it is a genuine page fault
      return this.handle_page_fault(pid, vaddr);
    }

    // This is a COW fault: the page is shared (refcount > 1)
    const refcount = this.get_refcount(pte.frame_number);
    if (refcount > 1) {
      // Allocate a new frame for this process's private copy
      let new_frame = this.frame_allocator.allocate();
      if (new_frame === undefined) {
        new_frame = this.evict_page();
        if (new_frame === undefined) {
          throw new Error("Out of memory during COW fault");
        }
      }

      // Decrement the old frame's refcount
      this.decrement_refcount(pte.frame_number);

      // Update the PTE to point to the new frame
      pte.frame_number = new_frame;
      pte.writable = true;
      pte.dirty = true;
      pte.accessed = true;
      this.set_refcount(new_frame, 1);
      this.replacement_policy.add_frame(new_frame);
    } else {
      // Sole owner — just make it writable
      pte.writable = true;
      pte.dirty = true;
      pte.accessed = true;
    }

    // Invalidate stale TLB entry and cache the new one
    this.tlb.invalidate(pid, vpn);
    this.tlb.insert(pid, vpn, pte.frame_number, pte);
    this.replacement_policy.record_access(pte.frame_number);

    return ((pte.frame_number << PAGE_OFFSET_BITS) | offset) >>> 0;
  }

  /**
   * Clone an address space from one process to another (fork with COW).
   *
   * Instead of copying physical frames (which would be very expensive),
   * we share them and mark all pages as read-only in both processes.
   * When either process writes, a page fault triggers a copy of just
   * that one page (copy-on-write).
   *
   * ```
   * Before: Parent VPN 0 -> Frame 5 (RW, refcount=1)
   * After:  Parent VPN 0 -> Frame 5 (RO, refcount=2)
   *         Child  VPN 0 -> Frame 5 (RO, refcount=2)
   * ```
   *
   * @param from_pid - The source process ID.
   * @param to_pid - The destination process ID.
   */
  clone_address_space(from_pid: number, to_pid: number): void {
    const src_pt = this.page_tables.get(from_pid);
    if (src_pt === undefined) {
      throw new Error(`No address space for PID ${from_pid}`);
    }

    // Create a fresh page table for the child
    const dst_pt = new TwoLevelPageTable();
    this.page_tables.set(to_pid, dst_pt);

    // Copy all mappings, sharing frames with COW
    for (const { vaddr, pte } of src_pt.allMappings()) {
      if (!pte.present) continue;

      // Mark the source page as read-only (COW)
      pte.writable = false;

      // Create a clone PTE for the child, also read-only
      const child_pte = pte.clone();
      child_pte.writable = false;

      // Insert into child's page table
      const { vpn1, vpn0 } = TwoLevelPageTable.splitAddress(vaddr);
      // We need to get/create the second-level table in dst_pt
      dst_pt.map(vaddr, child_pte.frame_number, {
        executable: child_pte.executable,
        user_accessible: child_pte.user_accessible,
      });
      // Override the PTE the map() call created with our cloned one
      // to preserve all the original flags (dirty, accessed, etc.)
      const newPte = dst_pt.lookupPTE(vaddr);
      if (newPte) {
        newPte.writable = false; // COW: read-only
        newPte.dirty = child_pte.dirty;
        newPte.accessed = child_pte.accessed;
      }

      // Increment the frame's reference count
      this.increment_refcount(pte.frame_number);

      // Invalidate parent's TLB entry (permissions changed to RO)
      const vpn = vaddr >>> PAGE_OFFSET_BITS;
      this.tlb.invalidate(from_pid, vpn);
    }
  }

  /**
   * Context switch: change the active process.
   *
   * The TLB must be flushed because it contains translations for the old
   * process. Without flushing, the new process might use stale translations
   * and access the old process's memory — a critical security vulnerability.
   *
   * This is one reason context switches are expensive: the new process
   * starts with a cold TLB and every memory access triggers a TLB miss
   * until the working set is re-cached.
   *
   * @param new_pid - The PID of the process being switched to.
   */
  context_switch(new_pid: number): void {
    this.tlb.flush();
    this.current_pid = new_pid;
  }

  /**
   * Get the page table for a process (for testing/inspection).
   */
  get_page_table(pid: number): TwoLevelPageTable | undefined {
    return this.page_tables.get(pid);
  }

  // ---- Reference count helpers ----

  private get_refcount(frame: number): number {
    return this.frame_refcounts.get(frame) ?? 0;
  }

  private set_refcount(frame: number, count: number): void {
    this.frame_refcounts.set(frame, count);
  }

  private increment_refcount(frame: number): void {
    this.set_refcount(frame, this.get_refcount(frame) + 1);
  }

  private decrement_refcount(frame: number): void {
    const count = this.get_refcount(frame) - 1;
    if (count <= 0) {
      this.frame_refcounts.delete(frame);
      if (this.frame_allocator.is_allocated(frame)) {
        this.frame_allocator.free(frame);
        this.replacement_policy.remove_frame(frame);
      }
    } else {
      this.set_refcount(frame, count);
    }
  }

  /**
   * Evict a page selected by the replacement policy, freeing its frame.
   * Returns the freed frame number, or undefined if no eviction possible.
   */
  private evict_page(): number | undefined {
    const victim_frame = this.replacement_policy.select_victim();
    if (victim_frame === undefined) return undefined;

    // Find and unmap the victim page from whichever process owns it
    for (const [pid, pt] of this.page_tables) {
      for (const { vaddr, pte } of pt.allMappings()) {
        if (pte.frame_number === victim_frame && pte.present) {
          pte.present = false;
          const vpn = vaddr >>> PAGE_OFFSET_BITS;
          this.tlb.invalidate(pid, vpn);
          // Don't free via refcount — we are reusing the frame
          this.frame_refcounts.delete(victim_frame);
          // The frame is already allocated; we just transfer ownership
          return victim_frame;
        }
      }
    }

    // Frame was tracked by policy but not found in any page table
    // (shouldn't happen, but handle gracefully)
    return victim_frame;
  }
}
