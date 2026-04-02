package oskernel

// =========================================================================
// Memory Manager -- region-based memory allocation
// =========================================================================
//
// Our memory manager uses the simplest possible scheme: region-based
// allocation. There is no paging, no virtual memory, no MMU. Each process
// gets a fixed region of physical memory assigned at creation time.
//
// Each region has permissions (read, write, execute) and an owner (PID).
// A process can only access memory within its own region. This is not
// enforced in our simulation, but the data structure tracks it for
// educational value.

// MemoryPermission represents read/write/execute flags for a memory region.
type MemoryPermission uint8

const (
	// PermRead allows reading from the region.
	PermRead MemoryPermission = 1 << iota // 0x01

	// PermWrite allows writing to the region.
	PermWrite // 0x02

	// PermExecute allows executing code from the region.
	PermExecute // 0x04
)

// MemoryRegion describes a contiguous block of memory with permissions.
type MemoryRegion struct {
	// Base is the starting address of this region.
	Base uint32

	// Size is the region size in bytes.
	Size uint32

	// Permissions are the R/W/X flags for this region.
	Permissions MemoryPermission

	// Owner is the PID that owns this region, or -1 for kernel-owned.
	Owner int

	// Name is a human-readable label (e.g., "kernel code", "PID 1 memory").
	Name string
}

// MemoryManager tracks all allocated memory regions.
type MemoryManager struct {
	Regions []MemoryRegion
}

// NewMemoryManager creates a memory manager with pre-defined regions.
func NewMemoryManager(regions []MemoryRegion) *MemoryManager {
	result, _ := StartNew[*MemoryManager]("os-kernel.NewMemoryManager", nil,
		func(op *Operation[*MemoryManager], rf *ResultFactory[*MemoryManager]) *OperationResult[*MemoryManager] {
			copied := make([]MemoryRegion, len(regions))
			copy(copied, regions)
			return rf.Generate(true, false, &MemoryManager{Regions: copied})
		}).GetResult()
	return result
}

// FindRegion returns the memory region containing the given address, or nil
// if the address is not in any region.
func (mm *MemoryManager) FindRegion(address uint32) *MemoryRegion {
	result, _ := StartNew[*MemoryRegion]("os-kernel.MemoryManager.FindRegion", nil,
		func(op *Operation[*MemoryRegion], rf *ResultFactory[*MemoryRegion]) *OperationResult[*MemoryRegion] {
			for i := range mm.Regions {
				r := &mm.Regions[i]
				if address >= r.Base && address < r.Base+r.Size {
					return rf.Generate(true, false, r)
				}
			}
			return rf.Generate(true, false, nil)
		}).GetResult()
	return result
}

// CheckAccess verifies that the given PID can access the given address
// with the given permissions. Returns true if allowed.
//
// Rules:
//   - Kernel (owner == -1) regions are accessible by all
//   - A process can access its own regions
//   - The requested permission must be a subset of the region's permissions
func (mm *MemoryManager) CheckAccess(pid int, address uint32, perm MemoryPermission) bool {
	result, _ := StartNew[bool]("os-kernel.MemoryManager.CheckAccess", false,
		func(op *Operation[bool], rf *ResultFactory[bool]) *OperationResult[bool] {
			region := mm.FindRegion(address)
			if region == nil {
				return rf.Generate(true, false, false)
			}
			// Check owner (kernel regions are accessible by all)
			if region.Owner != -1 && region.Owner != pid {
				return rf.Generate(true, false, false)
			}
			// Check permissions
			return rf.Generate(true, false, (region.Permissions&perm) == perm)
		}).GetResult()
	return result
}

// AllocateRegion adds a new memory region for the given PID.
func (mm *MemoryManager) AllocateRegion(pid int, base, size uint32, perm MemoryPermission, name string) {
	_, _ = StartNew[struct{}]("os-kernel.MemoryManager.AllocateRegion", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			mm.Regions = append(mm.Regions, MemoryRegion{
				Base:        base,
				Size:        size,
				Permissions: perm,
				Owner:       pid,
				Name:        name,
			})
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
}

// RegionCount returns the number of tracked regions.
func (mm *MemoryManager) RegionCount() int {
	result, _ := StartNew[int]("os-kernel.MemoryManager.RegionCount", 0,
		func(op *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			return rf.Generate(true, false, len(mm.Regions))
		}).GetResult()
	return result
}
