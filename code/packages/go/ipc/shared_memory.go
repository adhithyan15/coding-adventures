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
	result, _ := StartNew[*SharedMemoryRegion]("ipc.NewSharedMemoryRegion", nil,
		func(op *Operation[*SharedMemoryRegion], rf *ResultFactory[*SharedMemoryRegion]) *OperationResult[*SharedMemoryRegion] {
			op.AddProperty("name", name)
			op.AddProperty("size", size)
			op.AddProperty("ownerPID", ownerPID)
			return rf.Generate(true, false, &SharedMemoryRegion{
				name:        name,
				size:        size,
				data:        make([]byte, size),
				ownerPID:    ownerPID,
				attachedPIDs: make(map[int]bool),
			})
		}).GetResult()
	return result
}

// Attach maps this shared memory region into a process's address space.
//
// In a real OS, this modifies the process's page table so that a range of
// virtual addresses points to the shared physical pages. In our simulation,
// we just record the PID.
//
// Returns true if the PID was newly attached, false if already attached.
func (s *SharedMemoryRegion) Attach(pid int) bool {
	result, _ := StartNew[bool]("ipc.SharedMemoryRegion.Attach", false,
		func(op *Operation[bool], rf *ResultFactory[bool]) *OperationResult[bool] {
			op.AddProperty("pid", pid)
			if s.attachedPIDs[pid] {
				return rf.Generate(true, false, false)
			}
			s.attachedPIDs[pid] = true
			return rf.Generate(true, false, true)
		}).GetResult()
	return result
}

// Detach unmaps this shared memory region from a process.
//
// Returns true if the PID was detached, false if it was not attached.
func (s *SharedMemoryRegion) Detach(pid int) bool {
	result, _ := StartNew[bool]("ipc.SharedMemoryRegion.Detach", false,
		func(op *Operation[bool], rf *ResultFactory[bool]) *OperationResult[bool] {
			op.AddProperty("pid", pid)
			if !s.attachedPIDs[pid] {
				return rf.Generate(true, false, false)
			}
			delete(s.attachedPIDs, pid)
			return rf.Generate(true, false, true)
		}).GetResult()
	return result
}

// ReadAt reads count bytes from the shared region starting at offset.
//
// In a real OS, the process reads directly from its virtual address space
// (no system call needed after attach). We simulate this with an explicit
// method for clarity.
//
// Returns an error if the access is out of bounds.
func (s *SharedMemoryRegion) ReadAt(offset, count int) ([]byte, error) {
	type readResult struct {
		data []byte
		err  error
	}
	result, _ := StartNew[readResult]("ipc.SharedMemoryRegion.ReadAt", readResult{},
		func(op *Operation[readResult], rf *ResultFactory[readResult]) *OperationResult[readResult] {
			op.AddProperty("offset", offset)
			op.AddProperty("count", count)
			if offset < 0 {
				return rf.Generate(true, false, readResult{nil, fmt.Errorf("negative offset: %d", offset)})
			}
			if offset+count > s.size {
				return rf.Generate(true, false, readResult{nil, fmt.Errorf(
					"read beyond region bounds: offset=%d, count=%d, size=%d",
					offset, count, s.size,
				)})
			}
			data := make([]byte, count)
			copy(data, s.data[offset:offset+count])
			return rf.Generate(true, false, readResult{data, nil})
		}).GetResult()
	return result.data, result.err
}

// WriteAt writes data into the shared region starting at offset.
//
// WARNING: Shared memory has NO built-in synchronization. If process A writes
// while process B reads, B may see partially-updated data. Real programs use
// semaphores or mutexes to coordinate access.
//
// Returns the number of bytes written, or an error if out of bounds.
func (s *SharedMemoryRegion) WriteAt(offset int, data []byte) (int, error) {
	type writeResult struct {
		n   int
		err error
	}
	result, _ := StartNew[writeResult]("ipc.SharedMemoryRegion.WriteAt", writeResult{},
		func(op *Operation[writeResult], rf *ResultFactory[writeResult]) *OperationResult[writeResult] {
			op.AddProperty("offset", offset)
			if offset < 0 {
				return rf.Generate(true, false, writeResult{0, fmt.Errorf("negative offset: %d", offset)})
			}
			if offset+len(data) > s.size {
				return rf.Generate(true, false, writeResult{0, fmt.Errorf(
					"write beyond region bounds: offset=%d, len(data)=%d, size=%d",
					offset, len(data), s.size,
				)})
			}
			copy(s.data[offset:], data)
			return rf.Generate(true, false, writeResult{len(data), nil})
		}).GetResult()
	return result.n, result.err
}

// Name returns the human-readable name of this shared memory segment.
func (s *SharedMemoryRegion) Name() string {
	result, _ := StartNew[string]("ipc.SharedMemoryRegion.Name", "",
		func(op *Operation[string], rf *ResultFactory[string]) *OperationResult[string] {
			return rf.Generate(true, false, s.name)
		}).GetResult()
	return result
}

// Size returns the size of this segment in bytes.
func (s *SharedMemoryRegion) Size() int {
	result, _ := StartNew[int]("ipc.SharedMemoryRegion.Size", 0,
		func(op *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			return rf.Generate(true, false, s.size)
		}).GetResult()
	return result
}

// OwnerPID returns the PID of the process that created this segment.
func (s *SharedMemoryRegion) OwnerPID() int {
	result, _ := StartNew[int]("ipc.SharedMemoryRegion.OwnerPID", 0,
		func(op *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			return rf.Generate(true, false, s.ownerPID)
		}).GetResult()
	return result
}

// AttachedCount returns the number of processes currently attached.
func (s *SharedMemoryRegion) AttachedCount() int {
	result, _ := StartNew[int]("ipc.SharedMemoryRegion.AttachedCount", 0,
		func(op *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			return rf.Generate(true, false, len(s.attachedPIDs))
		}).GetResult()
	return result
}

// IsAttached checks whether a given PID is currently attached.
func (s *SharedMemoryRegion) IsAttached(pid int) bool {
	result, _ := StartNew[bool]("ipc.SharedMemoryRegion.IsAttached", false,
		func(op *Operation[bool], rf *ResultFactory[bool]) *OperationResult[bool] {
			op.AddProperty("pid", pid)
			return rf.Generate(true, false, s.attachedPIDs[pid])
		}).GetResult()
	return result
}
