package virtualmemory

// =============================================================================
// TLB — Translation Lookaside Buffer
// =============================================================================
//
// The TLB is a small, fast cache that stores recent virtual-to-physical address
// translations. Without a TLB, every memory access would require walking the
// page table — 2-3 additional memory accesses just to find the physical address.
//
// Because programs exhibit strong LOCALITY (they access the same pages
// repeatedly), a small TLB of 32-256 entries achieves hit rates above 95%.
//
//     CPU wants to access virtual address 0x1ABC
//         |
//         v
//     TLB: "Do I have VPN 0x1 cached?"
//         |               |
//         YES (hit!)      NO (miss)
//         |               |
//         v               v
//     Return frame    Walk page table
//     from cache      (2-3 memory accesses)
//         |               |
//         v               v
//     Physical addr   Found -> cache in TLB

// tlbEntry stores a cached VPN -> (frame, PTE) mapping.
type tlbEntry struct {
	frame int
	pte   *PageTableEntry
}

// TLB caches recent virtual-to-physical translations for fast lookup.
type TLB struct {
	entries     map[int]*tlbEntry // VPN -> entry
	capacity    int
	accessOrder []int // LRU eviction: front = oldest, back = newest
	Hits        int   // Number of successful lookups
	Misses      int   // Number of failed lookups
}

// NewTLB creates a TLB with the given capacity. Real TLBs have 32-256 entries.
func NewTLB(capacity int) *TLB {
	return &TLB{
		entries:     make(map[int]*tlbEntry),
		capacity:    capacity,
		accessOrder: make([]int, 0),
	}
}

// Lookup checks the TLB for a cached translation.
//
// On a hit: returns the frame number, PTE, and true. Increments Hits.
// On a miss: returns 0, nil, and false. Increments Misses.
func (t *TLB) Lookup(vpn int) (int, *PageTableEntry, bool) {
	entry, ok := t.entries[vpn]
	if ok {
		t.Hits++
		// Move to end of access order (most recently used).
		t.removeFromOrder(vpn)
		t.accessOrder = append(t.accessOrder, vpn)
		return entry.frame, entry.pte, true
	}

	t.Misses++
	return 0, nil, false
}

// Insert adds a translation to the TLB. If full, evicts the LRU entry.
func (t *TLB) Insert(vpn, frame int, pte *PageTableEntry) {
	// Update existing entry.
	if _, ok := t.entries[vpn]; ok {
		t.entries[vpn] = &tlbEntry{frame: frame, pte: pte}
		t.removeFromOrder(vpn)
		t.accessOrder = append(t.accessOrder, vpn)
		return
	}

	// Evict LRU if full.
	if len(t.entries) >= t.capacity {
		t.evictLRU()
	}

	t.entries[vpn] = &tlbEntry{frame: frame, pte: pte}
	t.accessOrder = append(t.accessOrder, vpn)
}

// evictLRU removes the least recently used entry.
func (t *TLB) evictLRU() {
	if len(t.accessOrder) == 0 {
		return
	}
	victim := t.accessOrder[0]
	t.accessOrder = t.accessOrder[1:]
	delete(t.entries, victim)
}

// Invalidate removes a single entry from the TLB.
// Called when a specific mapping changes (e.g., page unmapped or remapped).
func (t *TLB) Invalidate(vpn int) {
	delete(t.entries, vpn)
	t.removeFromOrder(vpn)
}

// Flush removes ALL entries. Called on context switch because the new
// process has a different page table. This prevents security violations
// where one process could access another's memory via stale TLB entries.
func (t *TLB) Flush() {
	t.entries = make(map[int]*tlbEntry)
	t.accessOrder = t.accessOrder[:0]
}

// HitRate returns the ratio of hits to total lookups.
// Returns 0.0 if no lookups have been performed.
func (t *TLB) HitRate() float64 {
	total := t.Hits + t.Misses
	if total == 0 {
		return 0.0
	}
	return float64(t.Hits) / float64(total)
}

// Size returns the current number of entries.
func (t *TLB) Size() int {
	return len(t.entries)
}

// Capacity returns the maximum number of entries.
func (t *TLB) Capacity() int {
	return t.capacity
}

// removeFromOrder removes a VPN from the access order list.
func (t *TLB) removeFromOrder(vpn int) {
	for i, v := range t.accessOrder {
		if v == vpn {
			t.accessOrder = append(t.accessOrder[:i], t.accessOrder[i+1:]...)
			return
		}
	}
}
