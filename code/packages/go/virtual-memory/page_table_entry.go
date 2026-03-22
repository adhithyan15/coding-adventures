// Package virtualmemory implements a complete virtual memory subsystem.
//
// Virtual memory is one of the most important abstractions in computer science.
// It gives every process the illusion that it has the entire memory space to
// itself — starting at address 0, stretching to some large upper limit — even
// though the physical machine has limited RAM shared among many processes.
//
// This package provides:
//   - Page tables (single-level and two-level Sv32)
//   - TLB (Translation Lookaside Buffer) for caching translations
//   - Physical frame allocator (bitmap-based)
//   - Page replacement policies (FIFO, LRU, Clock)
//   - MMU (Memory Management Unit) that ties everything together
package virtualmemory

// =============================================================================
// Constants — Page Geometry
// =============================================================================
//
// These constants define the fundamental page geometry for a 32-bit virtual
// address space with 4 KB pages.
//
// Why 4 KB pages?
// It is a compromise. Smaller pages reduce internal fragmentation (wasted
// space within a page) but require larger page tables (more entries to track).
// Larger pages mean smaller tables but more waste. 4 KB has been the standard
// since the Intel 386 in 1985, and RISC-V uses it too.

const (
	// PageSize is the size of each page/frame in bytes: 4 KB = 2^12 = 4096.
	// Every page in virtual memory and every frame in physical memory is
	// exactly this size.
	PageSize = 4096

	// PageOffsetBits is the number of bits used for the page offset: 12 bits.
	// Since 2^12 = 4096, twelve bits can address every byte within a 4 KB page.
	//
	// Given a 32-bit virtual address:
	//   - Bits 11-0  (12 bits): page offset (which byte within the page)
	//   - Bits 31-12 (20 bits): virtual page number (which page)
	//
	// To extract these:
	//   vpn    = address >> 12        // shift right to drop the offset
	//   offset = address & 0xFFF      // mask the lower 12 bits
	PageOffsetBits = 12

	// VPNBits is the number of bits in the Virtual Page Number: 20 bits.
	// With 20 bits, we can address 2^20 = 1,048,576 distinct virtual pages.
	// At 4 KB per page, that gives us 4 GB of virtual address space.
	VPNBits = 20

	// OffsetMask masks the lower 12 bits to extract the page offset.
	OffsetMask = 0xFFF
)

// =============================================================================
// PageTableEntry
// =============================================================================

// PageTableEntry holds the metadata for a single virtual-to-physical page
// mapping. Each field corresponds to a hardware bit in the PTE.
//
// In real CPUs, these are packed into a single 32-bit or 64-bit integer:
//
//	+--------------------+---+---+---+---+---+---+---+---+
//	| PPN (frame number) | D | A | G | U | X | W | R | V |
//	| bits 31-10         | 7 | 6 | 5 | 4 | 3 | 2 | 1 | 0 |
//	+--------------------+---+---+---+---+---+---+---+---+
//	V = Valid (Present)    R = Readable
//	W = Writable           X = Executable
//	U = User-accessible    G = Global (ignored here)
//	A = Accessed           D = Dirty
//
// We use named boolean fields for clarity in this educational implementation.
type PageTableEntry struct {
	// FrameNumber identifies which physical frame this virtual page maps to.
	// Only meaningful when Present is true. A frame is a 4 KB chunk of
	// physical RAM. If FrameNumber is 42, the page occupies physical bytes
	// 42*4096 through 42*4096+4095.
	FrameNumber int

	// Present indicates whether this page is currently in physical memory.
	// If false, accessing it triggers a page fault (interrupt 14).
	// A page might not be present because:
	//   - It was never allocated (new mapping)
	//   - It was swapped to disk
	//   - It is a lazy allocation (allocated on first access)
	Present bool

	// Dirty indicates whether this page has been written to since it was
	// loaded. When evicting a dirty page, its contents must be written to
	// disk first. Clean pages can simply be discarded.
	Dirty bool

	// Accessed indicates whether this page was read or written recently.
	// Used by page replacement algorithms (Clock, LRU) to decide which
	// page to evict. The Clock algorithm clears this periodically.
	Accessed bool

	// Writable controls whether writes are allowed. Code pages are
	// typically read-only. Stack and heap pages are writable.
	// Copy-on-write pages start read-only and become writable after a fault.
	Writable bool

	// Executable controls whether instructions can be fetched from this page.
	// Data pages should NOT be executable (NX bit) to prevent code injection.
	Executable bool

	// UserAccessible controls whether user-mode code can access this page.
	// Kernel pages set this to false, preventing user programs from reading
	// or writing kernel memory.
	UserAccessible bool
}

// NewPageTableEntry creates a PTE with sensible defaults: not present,
// writable, user-accessible.
func NewPageTableEntry() PageTableEntry {
	return PageTableEntry{
		Writable:       true,
		UserAccessible: true,
	}
}

// Copy creates a deep copy of this PTE. Used during copy-on-write operations
// when forking a process.
func (pte PageTableEntry) Copy() PageTableEntry {
	return PageTableEntry{
		FrameNumber:    pte.FrameNumber,
		Present:        pte.Present,
		Dirty:          pte.Dirty,
		Accessed:       pte.Accessed,
		Writable:       pte.Writable,
		Executable:     pte.Executable,
		UserAccessible: pte.UserAccessible,
	}
}
