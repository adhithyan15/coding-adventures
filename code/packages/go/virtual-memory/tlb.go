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
	result, _ := StartNew[*TLB]("virtual-memory.NewTLB", nil,
		func(op *Operation[*TLB], rf *ResultFactory[*TLB]) *OperationResult[*TLB] {
			op.AddProperty("capacity", capacity)
			return rf.Generate(true, false, &TLB{
				entries:     make(map[int]*tlbEntry),
				capacity:    capacity,
				accessOrder: make([]int, 0),
			})
		}).GetResult()
	return result
}

// Lookup checks the TLB for a cached translation.
//
// On a hit: returns the frame number, PTE, and true. Increments Hits.
// On a miss: returns 0, nil, and false. Increments Misses.
func (t *TLB) Lookup(vpn int) (int, *PageTableEntry, bool) {
	type lookupResult struct {
		frame int
		pte   *PageTableEntry
		ok    bool
	}
	res, _ := StartNew[lookupResult]("virtual-memory.TLB.Lookup", lookupResult{0, nil, false},
		func(op *Operation[lookupResult], rf *ResultFactory[lookupResult]) *OperationResult[lookupResult] {
			op.AddProperty("vpn", vpn)
			entry, ok := t.entries[vpn]
			if ok {
				t.Hits++
				// Move to end of access order (most recently used).
				t.removeFromOrder(vpn)
				t.accessOrder = append(t.accessOrder, vpn)
				return rf.Generate(true, false, lookupResult{entry.frame, entry.pte, true})
			}

			t.Misses++
			return rf.Generate(true, false, lookupResult{0, nil, false})
		}).GetResult()
	return res.frame, res.pte, res.ok
}

// Insert adds a translation to the TLB. If full, evicts the LRU entry.
func (t *TLB) Insert(vpn, frame int, pte *PageTableEntry) {
	_, _ = StartNew[struct{}]("virtual-memory.TLB.Insert", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			op.AddProperty("vpn", vpn)
			op.AddProperty("frame", frame)
			// Update existing entry.
			if _, ok := t.entries[vpn]; ok {
				t.entries[vpn] = &tlbEntry{frame: frame, pte: pte}
				t.removeFromOrder(vpn)
				t.accessOrder = append(t.accessOrder, vpn)
				return rf.Generate(true, false, struct{}{})
			}

			// Evict LRU if full.
			if len(t.entries) >= t.capacity {
				t.evictLRU()
			}

			t.entries[vpn] = &tlbEntry{frame: frame, pte: pte}
			t.accessOrder = append(t.accessOrder, vpn)
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
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
	_, _ = StartNew[struct{}]("virtual-memory.TLB.Invalidate", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			op.AddProperty("vpn", vpn)
			delete(t.entries, vpn)
			t.removeFromOrder(vpn)
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
}

// Flush removes ALL entries. Called on context switch because the new
// process has a different page table. This prevents security violations
// where one process could access another's memory via stale TLB entries.
func (t *TLB) Flush() {
	_, _ = StartNew[struct{}]("virtual-memory.TLB.Flush", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			t.entries = make(map[int]*tlbEntry)
			t.accessOrder = t.accessOrder[:0]
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
}

// HitRate returns the ratio of hits to total lookups.
// Returns 0.0 if no lookups have been performed.
func (t *TLB) HitRate() float64 {
	result, _ := StartNew[float64]("virtual-memory.TLB.HitRate", 0,
		func(op *Operation[float64], rf *ResultFactory[float64]) *OperationResult[float64] {
			total := t.Hits + t.Misses
			if total == 0 {
				return rf.Generate(true, false, 0.0)
			}
			return rf.Generate(true, false, float64(t.Hits)/float64(total))
		}).GetResult()
	return result
}

// Size returns the current number of entries.
func (t *TLB) Size() int {
	result, _ := StartNew[int]("virtual-memory.TLB.Size", 0,
		func(op *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			return rf.Generate(true, false, len(t.entries))
		}).GetResult()
	return result
}

// Capacity returns the maximum number of entries.
func (t *TLB) Capacity() int {
	result, _ := StartNew[int]("virtual-memory.TLB.Capacity", 0,
		func(op *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			return rf.Generate(true, false, t.capacity)
		}).GetResult()
	return result
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
