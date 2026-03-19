package branchpredictor

// ─── Branch Target Buffer (BTB) ──────────────────────────────────────────────
//
// The branch predictor answers "WILL this branch be taken?"
// The BTB answers "WHERE does it go?"
//
// Both are needed for high-performance fetch. Without a BTB, even a perfect
// direction predictor would cause a 1-cycle bubble: the predictor says "taken"
// in the fetch stage, but the target address isn't known until decode. With a
// BTB, the target is available in the SAME cycle as the prediction, enabling
// zero-bubble fetch redirection.
//
// How the BTB fits into the pipeline:
//
//	Cycle 1 (Fetch):
//	  1. Read PC
//	  2. Direction predictor: "taken" or "not taken"?
//	  3. BTB lookup: if "taken", where does it go?
//	  4. Redirect fetch to target (BTB hit) or PC+4 (not taken / BTB miss)
//
// BTB organization (this implementation):
//   - Direct-mapped cache indexed by (pc % size)
//   - Each entry stores: valid bit, tag (full PC), target, branch type
//   - On lookup: check valid bit and tag match
//   - On miss: return NoTarget (-1)
//
// Real-world BTB sizes:
//
//	Intel Skylake: 4096 entries (L1 BTB) + 4096 entries (L2 BTB)
//	ARM Cortex-A72: 64 entries (micro BTB) + 4096 entries (main BTB)
//	AMD Zen 2: 512 entries (L1 BTB) + 7168 entries (L2 BTB)

// BTBEntry represents a single entry in the Branch Target Buffer.
//
// Each entry is like a cache line, storing:
//   - Valid:      is this entry occupied?
//   - Tag:        the full PC of the branch (for disambiguation on aliasing)
//   - Target:     the branch target address (the whole point of the BTB)
//   - BranchType: metadata about the kind of branch
type BTBEntry struct {
	Valid      bool
	Tag        int
	Target     int
	BranchType string
}

// BranchTargetBuffer caches branch target addresses alongside a direction
// predictor to provide zero-bubble fetch redirection.
//
// The BTB is a separate structure from the direction predictor. In a real CPU,
// both are consulted in parallel during the fetch stage:
//
//  1. Direction predictor says: "taken" or "not taken"
//  2. BTB says: "if taken, the target is 0x1234" (or miss)
type BranchTargetBuffer struct {
	size    int
	entries []BTBEntry

	// Statistics
	Lookups int
	Hits    int
	Misses  int
}

// NewBranchTargetBuffer creates a new BTB with the given number of entries.
// size should be a power of 2. Common sizes: 64, 256, 512, 1024, 4096.
func NewBranchTargetBuffer(size int) *BranchTargetBuffer {
	entries := make([]BTBEntry, size)
	return &BranchTargetBuffer{
		size:    size,
		entries: entries,
	}
}

// Lookup returns the cached target for a branch at pc, or NoTarget on a miss.
//
// A miss occurs when:
//   - The entry at this index is not valid (never written)
//   - The entry's tag doesn't match the PC (aliasing conflict)
func (b *BranchTargetBuffer) Lookup(pc int) int {
	b.Lookups++
	index := pc % b.size
	entry := b.entries[index]

	// Check valid bit AND tag match (just like a cache)
	if entry.Valid && entry.Tag == pc {
		b.Hits++
		return entry.Target
	}

	b.Misses++
	return NoTarget
}

// Update records a branch target after execution.
//
// Writes the target and metadata into the BTB. If another branch was
// occupying this index (aliasing), it gets evicted -- direct-mapped policy.
func (b *BranchTargetBuffer) Update(pc int, target int, branchType string) {
	index := pc % b.size
	b.entries[index] = BTBEntry{
		Valid:      true,
		Tag:        pc,
		Target:     target,
		BranchType: branchType,
	}
}

// GetEntry returns the BTB entry for a given PC if it exists, or nil otherwise.
func (b *BranchTargetBuffer) GetEntry(pc int) *BTBEntry {
	index := pc % b.size
	entry := b.entries[index]
	if entry.Valid && entry.Tag == pc {
		return &entry
	}
	return nil
}

// HitRate returns the BTB hit rate as a percentage (0.0 to 100.0).
func (b *BranchTargetBuffer) HitRate() float64 {
	if b.Lookups == 0 {
		return 0.0
	}
	return (float64(b.Hits) / float64(b.Lookups)) * 100.0
}

// Reset clears all BTB state -- entries and statistics.
func (b *BranchTargetBuffer) Reset() {
	b.entries = make([]BTBEntry, b.size)
	b.Lookups = 0
	b.Hits = 0
	b.Misses = 0
}
