package virtualmemory

// PageTable is a single-level page table: a map from virtual page number (VPN)
// to PageTableEntry (PTE).
//
// This is the simplest possible page table implementation. Every virtual page
// that is mapped has an entry; unmapped pages simply have no entry in the map.
//
// Why a map instead of an array?
// A 32-bit address space has 2^20 = 1,048,576 possible virtual pages. A flat
// array of that many entries would consume ~4 MB per process even if only a
// handful of pages are mapped. A map only stores entries for pages that are
// actually in use — much more memory-efficient.
//
// Real hardware uses multi-level page tables (see TwoLevelPageTable) to achieve
// similar space savings with structures the CPU can walk efficiently.
type PageTable struct {
	entries map[int]*PageTableEntry
}

// NewPageTable creates an empty page table.
func NewPageTable() *PageTable {
	result, _ := StartNew[*PageTable]("virtual-memory.NewPageTable", nil,
		func(op *Operation[*PageTable], rf *ResultFactory[*PageTable]) *OperationResult[*PageTable] {
			return rf.Generate(true, false, &PageTable{
				entries: make(map[int]*PageTableEntry),
			})
		}).GetResult()
	return result
}

// MapPage creates a mapping from a virtual page number to a physical frame.
//
// This is called when the OS allocates a new page for a process. It creates
// a PTE with the given permissions and marks it as present (meaning the page
// is currently in physical memory).
//
// Parameters:
//   - vpn: Virtual page number (upper 20 bits of the virtual address)
//   - frame: Physical frame number to map to
//   - writable: Whether the process can write to this page
//   - executable: Whether the CPU can fetch instructions from this page
//   - user: Whether user-mode code can access this page
func (pt *PageTable) MapPage(vpn, frame int, writable, executable, user bool) {
	_, _ = StartNew[struct{}]("virtual-memory.PageTable.MapPage", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			op.AddProperty("vpn", vpn)
			op.AddProperty("frame", frame)
			pt.entries[vpn] = &PageTableEntry{
				FrameNumber:    frame,
				Present:        true,
				Writable:       writable,
				Executable:     executable,
				UserAccessible: user,
			}
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
}

// UnmapPage removes a mapping for the given virtual page number.
// Returns the removed PTE (so the caller can free the physical frame)
// and a boolean indicating whether the VPN was actually mapped.
func (pt *PageTable) UnmapPage(vpn int) (*PageTableEntry, bool) {
	type unmapResult struct {
		pte *PageTableEntry
		ok  bool
	}
	res, _ := StartNew[unmapResult]("virtual-memory.PageTable.UnmapPage", unmapResult{nil, false},
		func(op *Operation[unmapResult], rf *ResultFactory[unmapResult]) *OperationResult[unmapResult] {
			op.AddProperty("vpn", vpn)
			pte, ok := pt.entries[vpn]
			if !ok {
				return rf.Generate(true, false, unmapResult{nil, false})
			}
			delete(pt.entries, vpn)
			return rf.Generate(true, false, unmapResult{pte, true})
		}).GetResult()
	return res.pte, res.ok
}

// Lookup finds the PTE for a virtual page number.
// Returns the PTE and true if found, or nil and false if the VPN is not mapped.
func (pt *PageTable) Lookup(vpn int) (*PageTableEntry, bool) {
	type lookupResult struct {
		pte *PageTableEntry
		ok  bool
	}
	res, _ := StartNew[lookupResult]("virtual-memory.PageTable.Lookup", lookupResult{nil, false},
		func(op *Operation[lookupResult], rf *ResultFactory[lookupResult]) *OperationResult[lookupResult] {
			op.AddProperty("vpn", vpn)
			pte, ok := pt.entries[vpn]
			return rf.Generate(true, false, lookupResult{pte, ok})
		}).GetResult()
	return res.pte, res.ok
}

// Entries returns the internal map for iteration (e.g., during fork/clone).
func (pt *PageTable) Entries() map[int]*PageTableEntry {
	result, _ := StartNew[map[int]*PageTableEntry]("virtual-memory.PageTable.Entries", nil,
		func(op *Operation[map[int]*PageTableEntry], rf *ResultFactory[map[int]*PageTableEntry]) *OperationResult[map[int]*PageTableEntry] {
			return rf.Generate(true, false, pt.entries)
		}).GetResult()
	return result
}

// MappedCount returns the number of currently mapped pages.
func (pt *PageTable) MappedCount() int {
	result, _ := StartNew[int]("virtual-memory.PageTable.MappedCount", 0,
		func(op *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			return rf.Generate(true, false, len(pt.entries))
		}).GetResult()
	return result
}
