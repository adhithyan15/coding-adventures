// Package cpusimulator provides the core framework for a CPU simulation.
//
// === Sparse Memory: Simulating a 32-bit Address Space ===
//
// A real 32-bit CPU can address 4 GB of memory (2^32 bytes). But most of
// that address space is empty — a typical embedded system might have:
//
//   0x00000000 - 0x000FFFFF: 1 MB of RAM (for code and data)
//   0xFFFB0000 - 0xFFFFFFFF: 320 KB of I/O registers (for peripherals)
//
// Everything in between is unmapped — accessing it would trigger a bus fault
// on real hardware. Allocating a contiguous 4 GB byte array to simulate this
// would be wasteful and impractical.
//
// SparseMemory solves this by mapping only the regions that actually exist.
// Each region is a named slice of bytes at a specific base address. Reads and
// writes are dispatched to the correct region by checking address ranges.
//
// === How it works ===
//
// Think of SparseMemory as a building with multiple floors, where each floor
// has a different purpose:
//
//   Floor 0 (0x00000000): RAM      — read/write, for code and data
//   Floor N (0xFFFB0000): I/O Regs — some read-only, some read/write
//
// When the CPU reads address 0x00001234, we find which "floor" contains that
// address (RAM, at base 0x00000000), compute the offset within the floor
// (0x1234), and read from that floor's backing byte slice at that offset.
//
// === Memory-mapped I/O ===
//
// In embedded systems and operating systems, hardware devices (UART, timers,
// interrupt controllers) appear as memory addresses. Writing to address
// 0xFFFF0000 might send a byte over a serial port. Reading from 0xFFFB0004
// might return the current timer count. SparseMemory naturally supports this
// pattern — each device gets its own MemoryRegion.
//
// === Read-only regions ===
//
// Some regions should never be written to. For example:
//   - ROM (bootloader code burned into hardware)
//   - Memory-mapped status registers (read-only by design)
//
// When a region is marked ReadOnly, writes are silently ignored. This matches
// the behavior of real hardware where writing to ROM has no effect. In a debug
// build, you might want to panic instead — but for simulation, silent ignore
// is safer and matches hardware semantics.
package cpusimulator

import "fmt"

// MemoryRegion defines a contiguous block of addressable memory.
//
// Each region has a base address, a size, and a backing byte slice.
// The region occupies addresses [Base, Base+Size). Any access within
// this range is translated to an offset into the Data slice:
//
//   offset = address - Base
//   value  = Data[offset]
//
// Example:
//
//   MemoryRegion{Base: 0x1000, Size: 256, Data: make([]byte, 256), Name: "SRAM"}
//
//   ReadByte(0x1000) -> Data[0]
//   ReadByte(0x10FF) -> Data[255]
//   ReadByte(0x1100) -> ERROR: outside this region
type MemoryRegion struct {
	// Base is the starting address of this region in the 32-bit address space.
	Base uint32

	// Size is the number of bytes in this region.
	// The region covers addresses [Base, Base+Size).
	Size uint32

	// Data is the backing storage for this region.
	// Must have exactly Size bytes allocated.
	Data []byte

	// Name is a human-readable label for debugging (e.g., "RAM", "ROM", "UART").
	Name string

	// ReadOnly controls whether writes are permitted.
	// When true, WriteByte and WriteWord silently discard the value.
	// This models ROM, flash memory, and read-only status registers.
	ReadOnly bool
}

// SparseMemory maps address ranges to backing byte slices, enabling a
// full 32-bit address space without allocating 4 GB.
//
// === Region lookup ===
//
// On every access, SparseMemory searches through its regions to find one
// that contains the target address. This is a linear scan — O(N) where N
// is the number of regions. For the small number of regions in a typical
// system (2-10), this is negligible compared to the cost of instruction
// execution. A real memory management unit (MMU) uses page tables and TLBs
// for O(1) lookup, but that complexity is not needed here.
//
// === Unmapped addresses ===
//
// If no region contains the target address, the access panics. On real
// hardware this would be a bus fault or data abort exception. Panicking
// in the simulator makes bugs immediately visible rather than silently
// corrupting state.
type SparseMemory struct {
	// Regions is the list of mapped memory regions.
	// Regions must not overlap — the behavior is undefined if they do.
	Regions []MemoryRegion
}

// NewSparseMemory creates a SparseMemory from a list of region definitions.
//
// Each region's Data slice is allocated if nil, or reused if pre-populated
// (useful for loading ROM images). Regions are stored in the order given.
//
// Example — a simple embedded system memory map:
//
//   regions := []MemoryRegion{
//       {Base: 0x00000000, Size: 0x100000, Name: "RAM"},
//       {Base: 0xFFFB0000, Size: 0x50000, Name: "I/O", ReadOnly: true},
//   }
//   mem := NewSparseMemory(regions)
func NewSparseMemory(regions []MemoryRegion) *SparseMemory {
	result, _ := StartNew[*SparseMemory]("cpu-simulator.NewSparseMemory", nil,
		func(op *Operation[*SparseMemory], rf *ResultFactory[*SparseMemory]) *OperationResult[*SparseMemory] {
			op.AddProperty("num_regions", len(regions))
			copied := make([]MemoryRegion, len(regions))
			for i, r := range regions {
				copied[i] = r
				if copied[i].Data == nil {
					copied[i].Data = make([]byte, r.Size)
				}
			}
			return rf.Generate(true, false, &SparseMemory{Regions: copied})
		}).GetResult()
	return result
}

// findRegion locates the MemoryRegion that contains the given address range
// [address, address+numBytes). Returns the region and the offset within it.
//
// Panics if no region contains the full range. This models a bus fault —
// the CPU tried to access memory that does not physically exist.
//
// === Why panic instead of returning an error? ===
//
// In a CPU simulator, an unmapped memory access is always a bug in the
// program being simulated (or in the simulator itself). Returning an error
// would require every caller to check it, adding noise without value.
// Panicking with a clear message makes debugging straightforward.
func (m *SparseMemory) findRegion(address uint32, numBytes int) (*MemoryRegion, int) {
	end := uint64(address) + uint64(numBytes)
	for i := range m.Regions {
		r := &m.Regions[i]
		regionEnd := uint64(r.Base) + uint64(r.Size)
		if uint64(address) >= uint64(r.Base) && end <= regionEnd {
			offset := int(address - r.Base)
			return r, offset
		}
	}
	panic(fmt.Sprintf("SparseMemory: unmapped address 0x%08X (accessing %d bytes)", address, numBytes))
}

// ReadByte reads a single byte from the sparse address space.
//
// The address is looked up across all regions. If found, the byte at
// the corresponding offset within the region's Data slice is returned.
// If no region contains the address, the function panics (bus fault).
func (m *SparseMemory) ReadByte(address uint32) byte {
	result, _ := StartNew[byte]("cpu-simulator.SparseMemory.ReadByte", 0,
		func(op *Operation[byte], rf *ResultFactory[byte]) *OperationResult[byte] {
			op.AddProperty("address", address)
			region, offset := m.findRegion(address, 1)
			return rf.Generate(true, false, region.Data[offset])
		}).PanicOnUnexpected().GetResult()
	return result
}

// WriteByte writes a single byte to the sparse address space.
//
// If the target region is read-only, the write is silently ignored.
// This matches real hardware behavior — writing to ROM has no effect,
// and the CPU does not receive an error signal.
func (m *SparseMemory) WriteByte(address uint32, value byte) {
	_, _ = StartNew[struct{}]("cpu-simulator.SparseMemory.WriteByte", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			op.AddProperty("address", address)
			region, offset := m.findRegion(address, 1)
			if region.ReadOnly {
				return rf.Generate(true, false, struct{}{}) // writes to read-only regions are silently discarded
			}
			region.Data[offset] = value
			return rf.Generate(true, false, struct{}{})
		}).PanicOnUnexpected().GetResult()
}

// ReadWord reads a 32-bit word (4 bytes) from the sparse address space
// in little-endian byte order.
//
// === Little-endian byte order ===
//
// Little-endian means the least significant byte is stored at the lowest
// address. For the value 0xDEADBEEF stored at address 0x1000:
//
//	Address  Byte
//	0x1000   0xEF  (least significant)
//	0x1001   0xBE
//	0x1002   0xAD
//	0x1003   0xDE  (most significant)
//
// This matches RISC-V, ARM (in default config), and x86 byte ordering.
func (m *SparseMemory) ReadWord(address uint32) uint32 {
	result, _ := StartNew[uint32]("cpu-simulator.SparseMemory.ReadWord", 0,
		func(op *Operation[uint32], rf *ResultFactory[uint32]) *OperationResult[uint32] {
			op.AddProperty("address", address)
			region, offset := m.findRegion(address, 4)
			return rf.Generate(true, false, uint32(region.Data[offset])|
				(uint32(region.Data[offset+1])<<8)|
				(uint32(region.Data[offset+2])<<16)|
				(uint32(region.Data[offset+3])<<24))
		}).PanicOnUnexpected().GetResult()
	return result
}

// WriteWord writes a 32-bit word (4 bytes) to the sparse address space
// in little-endian byte order.
//
// If the target region is read-only, the write is silently ignored.
func (m *SparseMemory) WriteWord(address uint32, value uint32) {
	_, _ = StartNew[struct{}]("cpu-simulator.SparseMemory.WriteWord", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			op.AddProperty("address", address)
			region, offset := m.findRegion(address, 4)
			if region.ReadOnly {
				return rf.Generate(true, false, struct{}{})
			}
			region.Data[offset] = byte(value & 0xFF)
			region.Data[offset+1] = byte((value >> 8) & 0xFF)
			region.Data[offset+2] = byte((value >> 16) & 0xFF)
			region.Data[offset+3] = byte((value >> 24) & 0xFF)
			return rf.Generate(true, false, struct{}{})
		}).PanicOnUnexpected().GetResult()
}

// LoadBytes copies a sequence of bytes into the sparse address space
// starting at the given address.
//
// This is typically used to load a program binary into simulated RAM
// or to initialize ROM contents. The entire byte range must fall within
// a single memory region.
//
// Note: LoadBytes bypasses the ReadOnly check. This allows pre-loading
// ROM contents during system initialization (before the CPU starts
// executing). Once the CPU is running, normal WriteByte/WriteWord
// calls will respect the ReadOnly flag.
func (m *SparseMemory) LoadBytes(address uint32, data []byte) {
	_, _ = StartNew[struct{}]("cpu-simulator.SparseMemory.LoadBytes", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			op.AddProperty("address", address)
			op.AddProperty("data_size", len(data))
			region, offset := m.findRegion(address, len(data))
			copy(region.Data[offset:], data)
			return rf.Generate(true, false, struct{}{})
		}).PanicOnUnexpected().GetResult()
}

// Dump returns a copy of bytes from the sparse address space.
//
// The entire range [start, start+length) must fall within a single
// memory region. The returned slice is a copy — modifying it does not
// affect the simulated memory.
//
// Useful for inspecting memory contents during debugging:
//
//	bytes := mem.Dump(0x1000, 16) // inspect 16 bytes at 0x1000
func (m *SparseMemory) Dump(start uint32, length int) []byte {
	result, _ := StartNew[[]byte]("cpu-simulator.SparseMemory.Dump", nil,
		func(op *Operation[[]byte], rf *ResultFactory[[]byte]) *OperationResult[[]byte] {
			op.AddProperty("start", start)
			op.AddProperty("length", length)
			region, offset := m.findRegion(start, length)
			data := make([]byte, length)
			copy(data, region.Data[offset:offset+length])
			return rf.Generate(true, false, data)
		}).PanicOnUnexpected().GetResult()
	return result
}

// RegionCount returns the number of mapped regions.
// Useful for testing and diagnostics.
func (m *SparseMemory) RegionCount() int {
	result, _ := StartNew[int]("cpu-simulator.SparseMemory.RegionCount", 0,
		func(op *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			return rf.Generate(true, false, len(m.Regions))
		}).GetResult()
	return result
}
