/**
 * Branch Target Buffer (BTB) — caching where branches go.
 *
 * The branch predictor answers "WILL this branch be taken?"
 * The BTB answers "WHERE does it go?"
 *
 * Both are needed for high-performance fetch. Without a BTB, even a perfect
 * direction predictor would cause a 1-cycle bubble: the predictor says "taken"
 * in the fetch stage, but the target address isn't known until decode (when
 * the instruction's immediate field is extracted). With a BTB, the target
 * is available in the SAME cycle as the prediction, enabling zero-bubble
 * fetch redirection.
 *
 * How the BTB fits into the pipeline:
 *
 *     Cycle 1 (Fetch):
 *         1. Read PC
 *         2. Direction predictor: "taken" or "not taken"?
 *         3. BTB lookup: if "taken", where does it go?
 *         4. Redirect fetch to target (BTB hit) or PC+4 (not taken / BTB miss)
 *
 *     Cycle 2+ (Decode, Execute, ...):
 *         Branch is decoded and eventually resolved.
 *         If BTB was wrong -> flush pipeline and update BTB.
 *
 * BTB organization (this implementation):
 *     - Direct-mapped cache indexed by (pc % size)
 *     - Each entry stores: valid bit, tag (full PC), target, branch type
 *     - On lookup: check valid bit and tag match
 *     - On miss: return null (fall through to PC+4)
 *
 *     In real hardware, BTBs are often 2-way or 4-way set-associative to
 *     reduce aliasing conflicts. We use direct-mapped for simplicity.
 *
 * Branch types tracked:
 *     - "conditional": if/else branches (beq, bne, blt, etc.)
 *     - "unconditional": always-taken jumps (j, jal)
 *     - "call": function calls (jal ra, call)
 *     - "return": function returns (jr ra, ret) — often use a separate
 *       Return Address Stack (RAS) instead of the BTB
 *
 * Real-world BTB sizes:
 *     - Intel Skylake: 4096 entries (L1 BTB) + 4096 entries (L2 BTB)
 *     - ARM Cortex-A72: 64 entries (micro BTB) + 4096 entries (main BTB)
 *     - AMD Zen 2: 512 entries (L1 BTB) + 7168 entries (L2 BTB)
 */

// ─── BTBEntry ─────────────────────────────────────────────────────────────────
//
// Each entry in the BTB is like a cache line. It stores:
//   valid  — is this entry occupied? (starts false, set true on first update)
//   tag    — the full PC of the branch (for disambiguation on aliasing)
//   target — the branch target address (the whole point of the BTB)
//   branchType — metadata about what kind of branch this is
//
// The tag is necessary because multiple branches can map to the same BTB index
// (aliasing). Without a tag, we'd return the wrong target for aliased branches.
// With a tag, we detect the alias and return a miss instead.

/**
 * Branch Target Buffer entry — caches the target address of a branch.
 *
 * The branch predictor tells you WHETHER a branch is taken.
 * The BTB tells you WHERE it goes.
 *
 * Without a BTB, even if you correctly predict "taken", you don't know
 * the target address until the decode stage, causing a 1-cycle bubble.
 * With a BTB, you can redirect fetch in the SAME cycle as prediction.
 *
 * @property valid - Whether this entry contains valid data. Starts false.
 * @property tag - The PC (program counter) of the branch instruction. Used to
 *     detect aliasing — two branches mapping to the same BTB index.
 * @property target - The branch target address (where the branch goes if taken).
 * @property branchType - The kind of branch — "conditional", "unconditional",
 *     "call", or "return". Used by the frontend to make additional
 *     predictions (e.g., using a Return Address Stack for "return" branches).
 */
export interface BTBEntry {
  readonly valid: boolean;
  readonly tag: number;
  readonly target: number;
  readonly branchType: string;
}

/**
 * Create a default (empty) BTB entry.
 */
export function createBTBEntry(
  valid = false,
  tag = 0,
  target = 0,
  branchType = "",
): BTBEntry {
  return { valid, tag, target, branchType };
}

// ─── BranchTargetBuffer ──────────────────────────────────────────────────────
//
// The BTB is organized as a direct-mapped cache:
//
//   index = pc % size
//
//   Lookup:
//     1. Compute index from PC
//     2. Read entry at that index
//     3. If entry.valid AND entry.tag == pc -> HIT -> return entry.target
//     4. Otherwise -> MISS -> return null
//
//   Update:
//     1. Compute index from PC
//     2. Write new entry at that index (overwrites any previous occupant)
//
// Eviction policy: the new entry always replaces the old one (direct-mapped).
// This means a BTB miss is guaranteed when:
//   - First encounter of a branch (compulsory miss)
//   - Two frequently-used branches alias to the same index (conflict miss)
//
// The stats tracking records hits and misses separately from direction
// prediction accuracy. A BTB miss doesn't necessarily mean a misprediction —
// the direction predictor might still predict "not taken" correctly even
// without a BTB entry.

/**
 * BTB — works alongside any BranchPredictor to provide target addresses.
 *
 * The BTB is a separate structure from the direction predictor. In a real CPU,
 * both are consulted in parallel during the fetch stage:
 *
 * 1. Direction predictor says: "taken" or "not taken"
 * 2. BTB says: "if taken, the target is 0x1234" (or miss)
 *
 * If the direction predictor says "taken" but the BTB misses, the CPU must
 * wait for the decode stage to compute the target — a 1-cycle penalty.
 *
 * @param size - Number of entries in the BTB. Should be a power of 2.
 *     Common sizes: 64, 256, 512, 1024, 4096.
 *
 * @example
 * ```ts
 * const btb = new BranchTargetBuffer(256);
 *
 * // First lookup — miss (branch never seen before)
 * let target = btb.lookup(0x100);
 * // target === null
 *
 * // After the branch executes, update the BTB
 * btb.update(0x100, 0x200, "conditional");
 *
 * // Now the lookup hits
 * target = btb.lookup(0x100);
 * // target === 0x200
 * ```
 */
export class BranchTargetBuffer {
  private _size: number;

  // ── BTB storage ───────────────────────────────────────────────────
  // Pre-allocate all entries as invalid. In hardware, this is a SRAM
  // array with valid bits cleared on reset.
  private _entries: BTBEntry[];

  // ── Statistics ────────────────────────────────────────────────────
  private _lookups = 0;
  private _hits = 0;
  private _misses = 0;

  constructor(size = 256) {
    this._size = size;
    this._entries = Array.from({ length: size }, () => createBTBEntry());
  }

  /**
   * Compute the BTB index for a given PC.
   *
   * Direct-mapped: index = pc % size.
   *
   * @param pc - The program counter of the branch instruction.
   * @returns An integer in [0, size) indexing into the BTB array.
   */
  private _index(pc: number): number {
    return pc % this._size;
  }

  /**
   * Look up the predicted target for a branch at `pc`.
   *
   * Returns the cached target address on a hit, or null on a miss.
   * A miss occurs when:
   * - The entry at this index is not valid (never written)
   * - The entry's tag doesn't match the PC (aliasing conflict)
   *
   * @param pc - The program counter of the branch instruction.
   * @returns The predicted target address, or null if the BTB doesn't know.
   */
  lookup(pc: number): number | null {
    this._lookups += 1;
    const index = this._index(pc);
    const entry = this._entries[index];

    // Check valid bit AND tag match (just like a cache)
    if (entry.valid && entry.tag === pc) {
      this._hits += 1;
      return entry.target;
    }

    this._misses += 1;
    return null;
  }

  /**
   * Record a branch target after execution.
   *
   * Writes the target and metadata into the BTB. If another branch was
   * occupying this index (aliasing), it gets evicted — this is the
   * direct-mapped eviction policy.
   *
   * @param pc - The program counter of the branch instruction.
   * @param target - The actual target address of the branch.
   * @param branchType - The kind of branch — "conditional", "unconditional",
   *     "call", or "return".
   */
  update(pc: number, target: number, branchType = "conditional"): void {
    const index = this._index(pc);
    this._entries[index] = createBTBEntry(true, pc, target, branchType);
  }

  /**
   * Inspect the BTB entry for a given PC (for testing/debugging).
   *
   * Returns the entry if it's valid and the tag matches, null otherwise.
   *
   * @param pc - The program counter to look up.
   * @returns The BTBEntry if found, null otherwise.
   */
  getEntry(pc: number): BTBEntry | null {
    const index = this._index(pc);
    const entry = this._entries[index];
    if (entry.valid && entry.tag === pc) {
      return entry;
    }
    return null;
  }

  /** Total number of BTB lookups performed. */
  get lookups(): number {
    return this._lookups;
  }

  /** Number of BTB hits (target found). */
  get hits(): number {
    return this._hits;
  }

  /** Number of BTB misses (target not found). */
  get misses(): number {
    return this._misses;
  }

  /** BTB hit rate as a percentage (0.0 to 100.0). */
  get hitRate(): number {
    if (this._lookups === 0) {
      return 0.0;
    }
    return (this._hits / this._lookups) * 100.0;
  }

  /** Reset all BTB state — entries and statistics. */
  reset(): void {
    this._entries = Array.from({ length: this._size }, () => createBTBEntry());
    this._lookups = 0;
    this._hits = 0;
    this._misses = 0;
  }
}
