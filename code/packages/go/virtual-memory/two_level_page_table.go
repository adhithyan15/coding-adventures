package virtualmemory

// =============================================================================
// Two-Level Page Table (Sv32)
// =============================================================================
//
// A single flat page table for a 32-bit address space would need 2^20 entries
// (over one million). Even at 4 bytes each, that is 4 MB per process — wasteful
// when most processes use only a tiny fraction of the address space.
//
// The solution: a HIERARCHICAL page table with two levels of smaller tables:
//
//     Level 1 (Page Directory): 1024 entries, each pointing to a Level 2 table.
//     Level 2 (Page Table):     1024 entries, each holding a PTE.
//
// Only Level 2 tables that are actually needed get allocated.
//
// Address Splitting (Sv32)
// ========================
//
// A 32-bit virtual address is split into three fields:
//
//     +------------+------------+----------------+
//     | VPN[1]     | VPN[0]     | Page Offset    |
//     | bits 31-22 | bits 21-12 | bits 11-0      |
//     | (10 bits)  | (10 bits)  | (12 bits)      |
//     +------------+------------+----------------+
//
//     VPN[1]: Index into the page directory (Level 1). 1024 entries.
//     VPN[0]: Index into the page table (Level 2). 1024 entries.
//     Offset: Byte position within the 4 KB page.
//
//     Total: 10 + 10 + 12 = 32 bits -> 4 GB address space.

const (
	// L1Bits is the number of bits for the Level 1 index.
	L1Bits = 10
	// L2Bits is the number of bits for the Level 2 index.
	L2Bits = 10
	// L1Entries is the number of entries in the page directory.
	L1Entries = 1 << L1Bits // 1024
	// L2Entries is the number of entries per page table.
	L2Entries = 1 << L2Bits // 1024
	// L1Shift is how far to shift to extract the L1 index (22 = 12 + 10).
	L1Shift = PageOffsetBits + L2Bits
	// L2Shift is how far to shift to extract the L2 index (= 12).
	L2Shift = PageOffsetBits
	// IndexMask masks a 10-bit index: 0x3FF = 1023.
	IndexMask = 0x3FF
)

// TwoLevelPageTable implements RISC-V Sv32 with a 10-bit directory
// and 10-bit page tables. The directory is a fixed-size array of 1024
// pointers to PageTable objects. Nil means the 4 MB region is unmapped.
type TwoLevelPageTable struct {
	directory [L1Entries]*PageTable
}

// NewTwoLevelPageTable creates an empty two-level page table.
// All directory entries start as nil (no regions mapped).
func NewTwoLevelPageTable() *TwoLevelPageTable {
	result, _ := StartNew[*TwoLevelPageTable]("virtual-memory.NewTwoLevelPageTable", nil,
		func(op *Operation[*TwoLevelPageTable], rf *ResultFactory[*TwoLevelPageTable]) *OperationResult[*TwoLevelPageTable] {
			return rf.Generate(true, false, &TwoLevelPageTable{})
		}).GetResult()
	return result
}

// splitAddress splits a 32-bit virtual address into L1 index, L2 index,
// and page offset.
//
// Example: address 0x00812ABC
//
//	L1 = (0x00812ABC >> 22) & 0x3FF = 2
//	L2 = (0x00812ABC >> 12) & 0x3FF = 18
//	offset = 0x00812ABC & 0xFFF = 0xABC = 2748
func splitAddress(virtualAddr int) (l1Index, l2Index, offset int) {
	l1Index = (virtualAddr >> L1Shift) & IndexMask
	l2Index = (virtualAddr >> L2Shift) & IndexMask
	offset = virtualAddr & OffsetMask
	return
}

// Map creates a mapping from a virtual address to a physical frame.
// Creates the Level 2 page table if it doesn't exist yet (lazy allocation).
func (pt *TwoLevelPageTable) Map(virtualAddr, physicalFrame int, writable, executable, user bool) {
	_, _ = StartNew[struct{}]("virtual-memory.TwoLevelPageTable.Map", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			op.AddProperty("virtualAddr", virtualAddr)
			op.AddProperty("physicalFrame", physicalFrame)
			l1, l2, _ := splitAddress(virtualAddr)

			// Create Level 2 table on demand — only allocate page table structures
			// for address space regions that are actually used.
			if pt.directory[l1] == nil {
				pt.directory[l1] = NewPageTable()
			}

			pt.directory[l1].MapPage(l2, physicalFrame, writable, executable, user)
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
}

// Unmap removes the mapping for the page containing the given virtual address.
// Returns the removed PTE and true, or nil and false if not mapped.
func (pt *TwoLevelPageTable) Unmap(virtualAddr int) (*PageTableEntry, bool) {
	type unmapResult struct {
		pte *PageTableEntry
		ok  bool
	}
	res, _ := StartNew[unmapResult]("virtual-memory.TwoLevelPageTable.Unmap", unmapResult{nil, false},
		func(op *Operation[unmapResult], rf *ResultFactory[unmapResult]) *OperationResult[unmapResult] {
			op.AddProperty("virtualAddr", virtualAddr)
			l1, l2, _ := splitAddress(virtualAddr)

			if pt.directory[l1] == nil {
				return rf.Generate(true, false, unmapResult{nil, false})
			}

			pte, ok := pt.directory[l1].UnmapPage(l2)
			return rf.Generate(true, false, unmapResult{pte, ok})
		}).GetResult()
	return res.pte, res.ok
}

// TranslateResult holds the result of a page table translation.
type TranslateResult struct {
	PhysicalAddr int
	PTE          *PageTableEntry
}

// Translate walks both levels of the page table to translate a virtual address
// to a physical address.
//
// Returns nil if the virtual address is not mapped.
func (pt *TwoLevelPageTable) Translate(virtualAddr int) *TranslateResult {
	result, _ := StartNew[*TranslateResult]("virtual-memory.TwoLevelPageTable.Translate", nil,
		func(op *Operation[*TranslateResult], rf *ResultFactory[*TranslateResult]) *OperationResult[*TranslateResult] {
			op.AddProperty("virtualAddr", virtualAddr)
			l1, l2, offset := splitAddress(virtualAddr)

			// Step 1: Look up Level 2 table in the directory.
			if pt.directory[l1] == nil {
				return rf.Generate(true, false, nil) // This 4 MB region is completely unmapped.
			}

			// Step 2: Look up the PTE in the Level 2 table.
			pte, ok := pt.directory[l1].Lookup(l2)
			if !ok {
				return rf.Generate(true, false, nil) // This specific page is not mapped.
			}

			// Step 3: Compute physical address.
			// physical_addr = (frame_number << 12) | offset
			physicalAddr := (pte.FrameNumber << PageOffsetBits) | offset

			return rf.Generate(true, false, &TranslateResult{
				PhysicalAddr: physicalAddr,
				PTE:          pte,
			})
		}).GetResult()
	return result
}

// LookupVPN looks up a PTE by virtual page number (not full address).
func (pt *TwoLevelPageTable) LookupVPN(vpn int) (*PageTableEntry, bool) {
	type lookupResult struct {
		pte *PageTableEntry
		ok  bool
	}
	res, _ := StartNew[lookupResult]("virtual-memory.TwoLevelPageTable.LookupVPN", lookupResult{nil, false},
		func(op *Operation[lookupResult], rf *ResultFactory[lookupResult]) *OperationResult[lookupResult] {
			op.AddProperty("vpn", vpn)
			l1 := (vpn >> L2Bits) & IndexMask
			l2 := vpn & IndexMask

			if pt.directory[l1] == nil {
				return rf.Generate(true, false, lookupResult{nil, false})
			}

			pte, ok := pt.directory[l1].Lookup(l2)
			return rf.Generate(true, false, lookupResult{pte, ok})
		}).GetResult()
	return res.pte, res.ok
}

// MapVPN maps a virtual page number to a physical frame.
func (pt *TwoLevelPageTable) MapVPN(vpn, physicalFrame int, writable, executable, user bool) {
	_, _ = StartNew[struct{}]("virtual-memory.TwoLevelPageTable.MapVPN", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			op.AddProperty("vpn", vpn)
			op.AddProperty("physicalFrame", physicalFrame)
			virtualAddr := vpn << PageOffsetBits
			pt.Map(virtualAddr, physicalFrame, writable, executable, user)
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
}

// Directory returns the raw directory for iteration (used during fork/clone).
func (pt *TwoLevelPageTable) Directory() [L1Entries]*PageTable {
	result, _ := StartNew[[L1Entries]*PageTable]("virtual-memory.TwoLevelPageTable.Directory", [L1Entries]*PageTable{},
		func(op *Operation[[L1Entries]*PageTable], rf *ResultFactory[[L1Entries]*PageTable]) *OperationResult[[L1Entries]*PageTable] {
			return rf.Generate(true, false, pt.directory)
		}).GetResult()
	return result
}
