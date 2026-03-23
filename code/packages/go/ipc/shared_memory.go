package ipc

import "fmt"

// SharedMemoryRegion is a named shared memory segment accessible by multiple
// processes.
//
// Pipes and message queues both copy data: the sender writes bytes, the kernel
// copies them into a buffer, and the receiver copies them out. For large data
// transfers, this double-copy is expensive.
//
// Shared memory eliminates copying entirely. Two (or more) processes map the
// same physical pages into their virtual address spaces. A write by one process
// is immediately visible to the other — no system call, no copy, no kernel
// involvement after setup.
//
// # Analogy
//
// Imagine two people in adjacent offices with a window between them and a
// whiteboard mounted in the window frame. Both can read and write on the
// whiteboard. This is the fastest possible communication, but if both write
// simultaneously, the result is garbled. Real systems use semaphores or mutexes
// to coordinate. Our simulation omits synchronization for simplicity.
//
// # Memory Layout
//
//	Process A's address space     Process B's address space
//	+-----------------------+    +-----------------------+
//	| 0x8000 +-----------+ |    | 0xC000 +-----------+ |
//	|        |Shared Data|<+----+------->|Shared Data| |
//	|        +-----------+ |    |        +-----------+ |
//	+-----------------------+    +-----------------------+
//	            |                            |
//	            +------+---------------------+
//	                   |
//	            +------v------+
//	            | Physical    |
//	            | Page Frame  |  ← same physical memory
//	            +-------------+
type SharedMemoryRegion struct {
	name        string
	size        int
	data        []byte
	ownerPID    int
	attachedPIDs map[int]bool
}

// NewSharedMemoryRegion creates a new shared memory region.
//
// The data is zero-initialized, just like freshly allocated memory in a real OS.
func NewSharedMemoryRegion(name string, size int, ownerPID int) *SharedMemoryRegion {
	return &SharedMemoryRegion{
		name:        name,
		size:        size,
		data:        make([]byte, size),
		ownerPID:    ownerPID,
		attachedPIDs: make(map[int]bool),
	}
}

// Attach maps this shared memory region into a process's address space.
//
// In a real OS, this modifies the process's page table so that a range of
// virtual addresses points to the shared physical pages. In our simulation,
// we just record the PID.
//
// Returns true if the PID was newly attached, false if already attached.
func (s *SharedMemoryRegion) Attach(pid int) bool {
	if s.attachedPIDs[pid] {
		return false
	}
	s.attachedPIDs[pid] = true
	return true
}

// Detach unmaps this shared memory region from a process.
//
// Returns true if the PID was detached, false if it was not attached.
func (s *SharedMemoryRegion) Detach(pid int) bool {
	if !s.attachedPIDs[pid] {
		return false
	}
	delete(s.attachedPIDs, pid)
	return true
}

// ReadAt reads count bytes from the shared region starting at offset.
//
// In a real OS, the process reads directly from its virtual address space
// (no system call needed after attach). We simulate this with an explicit
// method for clarity.
//
// Returns an error if the access is out of bounds.
func (s *SharedMemoryRegion) ReadAt(offset, count int) ([]byte, error) {
	if offset < 0 {
		return nil, fmt.Errorf("negative offset: %d", offset)
	}
	if offset+count > s.size {
		return nil, fmt.Errorf(
			"read beyond region bounds: offset=%d, count=%d, size=%d",
			offset, count, s.size,
		)
	}
	result := make([]byte, count)
	copy(result, s.data[offset:offset+count])
	return result, nil
}

// WriteAt writes data into the shared region starting at offset.
//
// WARNING: Shared memory has NO built-in synchronization. If process A writes
// while process B reads, B may see partially-updated data. Real programs use
// semaphores or mutexes to coordinate access.
//
// Returns the number of bytes written, or an error if out of bounds.
func (s *SharedMemoryRegion) WriteAt(offset int, data []byte) (int, error) {
	if offset < 0 {
		return 0, fmt.Errorf("negative offset: %d", offset)
	}
	if offset+len(data) > s.size {
		return 0, fmt.Errorf(
			"write beyond region bounds: offset=%d, len(data)=%d, size=%d",
			offset, len(data), s.size,
		)
	}
	copy(s.data[offset:], data)
	return len(data), nil
}

// Name returns the human-readable name of this shared memory segment.
func (s *SharedMemoryRegion) Name() string {
	return s.name
}

// Size returns the size of this segment in bytes.
func (s *SharedMemoryRegion) Size() int {
	return s.size
}

// OwnerPID returns the PID of the process that created this segment.
func (s *SharedMemoryRegion) OwnerPID() int {
	return s.ownerPID
}

// AttachedCount returns the number of processes currently attached.
func (s *SharedMemoryRegion) AttachedCount() int {
	return len(s.attachedPIDs)
}

// IsAttached checks whether a given PID is currently attached.
func (s *SharedMemoryRegion) IsAttached(pid int) bool {
	return s.attachedPIDs[pid]
}
