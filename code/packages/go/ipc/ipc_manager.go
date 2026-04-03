package ipc

// IPCManager is the central coordinator for all IPC mechanisms.
//
// In a real OS kernel, the IPC manager maintains global tables of all active
// pipes, message queues, and shared memory segments. It is the single point
// of creation, lookup, and destruction.
//
//	+------------------+---------------------------------------------+
//	| Table            | Description                                 |
//	+==================+=============================================+
//	| pipes            | All active pipes, indexed by pipe ID.       |
//	+------------------+---------------------------------------------+
//	| messageQueues    | All message queues, keyed by name (the      |
//	|                  | "well-known key" processes agree on).       |
//	+------------------+---------------------------------------------+
//	| sharedRegions    | All shared memory segments, keyed by name.  |
//	+------------------+---------------------------------------------+
//
// Pipe creation returns a triple: (pipeID, readFD, writeFD). The readFD and
// writeFD are logical file descriptor numbers. In a full OS, the caller maps
// these to real entries in the process's file descriptor table.
type IPCManager struct {
	pipes        map[int]*Pipe
	nextPipeID   int
	nextFD       int // starts at 3 (0=stdin, 1=stdout, 2=stderr)
	messageQueues map[string]*MessageQueue
	sharedRegions map[string]*SharedMemoryRegion
}

// NewIPCManager creates a new IPC manager with empty tables.
func NewIPCManager() *IPCManager {
	result, _ := StartNew[*IPCManager]("ipc.NewIPCManager", nil,
		func(op *Operation[*IPCManager], rf *ResultFactory[*IPCManager]) *OperationResult[*IPCManager] {
			return rf.Generate(true, false, &IPCManager{
				pipes:        make(map[int]*Pipe),
				nextPipeID:   0,
				nextFD:       3,
				messageQueues: make(map[string]*MessageQueue),
				sharedRegions: make(map[string]*SharedMemoryRegion),
			})
		}).GetResult()
	return result
}

// ====================================================================
// Pipe management
// ====================================================================

// CreatePipe creates a new pipe with the given capacity.
//
// Returns (pipeID, readFD, writeFD):
//   - pipeID: unique identifier for the pipe
//   - readFD: logical file descriptor for the read end
//   - writeFD: logical file descriptor for the write end
//
// In a real OS, after fork(), both parent and child have copies of readFD
// and writeFD. Typically the parent closes one end and the child closes the
// other to establish a one-way channel.
func (m *IPCManager) CreatePipe(capacity int) (int, int, int) {
	type tripleResult struct {
		pipeID  int
		readFD  int
		writeFD int
	}
	result, _ := StartNew[tripleResult]("ipc.IPCManager.CreatePipe", tripleResult{},
		func(op *Operation[tripleResult], rf *ResultFactory[tripleResult]) *OperationResult[tripleResult] {
			op.AddProperty("capacity", capacity)
			pipeID := m.nextPipeID
			m.nextPipeID++

			readFD := m.nextFD
			m.nextFD++
			writeFD := m.nextFD
			m.nextFD++

			m.pipes[pipeID] = NewPipe(capacity)
			return rf.Generate(true, false, tripleResult{pipeID, readFD, writeFD})
		}).GetResult()
	return result.pipeID, result.readFD, result.writeFD
}

// GetPipe looks up a pipe by its ID. Returns nil if not found.
func (m *IPCManager) GetPipe(pipeID int) *Pipe {
	result, _ := StartNew[*Pipe]("ipc.IPCManager.GetPipe", nil,
		func(op *Operation[*Pipe], rf *ResultFactory[*Pipe]) *OperationResult[*Pipe] {
			op.AddProperty("pipeID", pipeID)
			return rf.Generate(true, false, m.pipes[pipeID])
		}).GetResult()
	return result
}

// ClosePipeRead closes the read end of a pipe.
func (m *IPCManager) ClosePipeRead(pipeID int) {
	_, _ = StartNew[struct{}]("ipc.IPCManager.ClosePipeRead", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			op.AddProperty("pipeID", pipeID)
			if p, ok := m.pipes[pipeID]; ok {
				p.CloseRead()
			}
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
}

// ClosePipeWrite closes the write end of a pipe.
func (m *IPCManager) ClosePipeWrite(pipeID int) {
	_, _ = StartNew[struct{}]("ipc.IPCManager.ClosePipeWrite", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			op.AddProperty("pipeID", pipeID)
			if p, ok := m.pipes[pipeID]; ok {
				p.CloseWrite()
			}
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
}

// DestroyPipe removes a pipe from the manager entirely.
// Returns true if the pipe existed and was removed.
func (m *IPCManager) DestroyPipe(pipeID int) bool {
	result, _ := StartNew[bool]("ipc.IPCManager.DestroyPipe", false,
		func(op *Operation[bool], rf *ResultFactory[bool]) *OperationResult[bool] {
			op.AddProperty("pipeID", pipeID)
			if _, ok := m.pipes[pipeID]; ok {
				delete(m.pipes, pipeID)
				return rf.Generate(true, false, true)
			}
			return rf.Generate(true, false, false)
		}).GetResult()
	return result
}

// ====================================================================
// Message queue management
// ====================================================================

// CreateMessageQueue creates (or retrieves) a message queue by name.
//
// If a queue with this name already exists, returns the existing one
// (like msgget with IPC_CREAT in Unix — idempotent creation).
func (m *IPCManager) CreateMessageQueue(name string, maxMessages, maxMessageSize int) *MessageQueue {
	result, _ := StartNew[*MessageQueue]("ipc.IPCManager.CreateMessageQueue", nil,
		func(op *Operation[*MessageQueue], rf *ResultFactory[*MessageQueue]) *OperationResult[*MessageQueue] {
			op.AddProperty("name", name)
			op.AddProperty("maxMessages", maxMessages)
			op.AddProperty("maxMessageSize", maxMessageSize)
			if mq, ok := m.messageQueues[name]; ok {
				return rf.Generate(true, false, mq)
			}
			mq := NewMessageQueue(maxMessages, maxMessageSize)
			m.messageQueues[name] = mq
			return rf.Generate(true, false, mq)
		}).GetResult()
	return result
}

// GetMessageQueue looks up a message queue by name. Returns nil if not found.
func (m *IPCManager) GetMessageQueue(name string) *MessageQueue {
	result, _ := StartNew[*MessageQueue]("ipc.IPCManager.GetMessageQueue", nil,
		func(op *Operation[*MessageQueue], rf *ResultFactory[*MessageQueue]) *OperationResult[*MessageQueue] {
			op.AddProperty("name", name)
			return rf.Generate(true, false, m.messageQueues[name])
		}).GetResult()
	return result
}

// DeleteMessageQueue removes a message queue. Returns true if it existed.
func (m *IPCManager) DeleteMessageQueue(name string) bool {
	result, _ := StartNew[bool]("ipc.IPCManager.DeleteMessageQueue", false,
		func(op *Operation[bool], rf *ResultFactory[bool]) *OperationResult[bool] {
			op.AddProperty("name", name)
			if _, ok := m.messageQueues[name]; ok {
				delete(m.messageQueues, name)
				return rf.Generate(true, false, true)
			}
			return rf.Generate(true, false, false)
		}).GetResult()
	return result
}

// ====================================================================
// Shared memory management
// ====================================================================

// CreateSharedMemory creates (or retrieves) a shared memory region by name.
//
// If a region with this name already exists, returns the existing one
// (like shmget with IPC_CREAT).
func (m *IPCManager) CreateSharedMemory(name string, size int, ownerPID int) *SharedMemoryRegion {
	result, _ := StartNew[*SharedMemoryRegion]("ipc.IPCManager.CreateSharedMemory", nil,
		func(op *Operation[*SharedMemoryRegion], rf *ResultFactory[*SharedMemoryRegion]) *OperationResult[*SharedMemoryRegion] {
			op.AddProperty("name", name)
			op.AddProperty("size", size)
			op.AddProperty("ownerPID", ownerPID)
			if r, ok := m.sharedRegions[name]; ok {
				return rf.Generate(true, false, r)
			}
			r := NewSharedMemoryRegion(name, size, ownerPID)
			m.sharedRegions[name] = r
			return rf.Generate(true, false, r)
		}).GetResult()
	return result
}

// GetSharedMemory looks up a shared memory region by name. Returns nil if not found.
func (m *IPCManager) GetSharedMemory(name string) *SharedMemoryRegion {
	result, _ := StartNew[*SharedMemoryRegion]("ipc.IPCManager.GetSharedMemory", nil,
		func(op *Operation[*SharedMemoryRegion], rf *ResultFactory[*SharedMemoryRegion]) *OperationResult[*SharedMemoryRegion] {
			op.AddProperty("name", name)
			return rf.Generate(true, false, m.sharedRegions[name])
		}).GetResult()
	return result
}

// DeleteSharedMemory removes a shared memory region. Returns true if it existed.
func (m *IPCManager) DeleteSharedMemory(name string) bool {
	result, _ := StartNew[bool]("ipc.IPCManager.DeleteSharedMemory", false,
		func(op *Operation[bool], rf *ResultFactory[bool]) *OperationResult[bool] {
			op.AddProperty("name", name)
			if _, ok := m.sharedRegions[name]; ok {
				delete(m.sharedRegions, name)
				return rf.Generate(true, false, true)
			}
			return rf.Generate(true, false, false)
		}).GetResult()
	return result
}

// ====================================================================
// Listing operations
// ====================================================================

// ListPipes returns a slice of all active pipe IDs.
func (m *IPCManager) ListPipes() []int {
	result, _ := StartNew[[]int]("ipc.IPCManager.ListPipes", nil,
		func(op *Operation[[]int], rf *ResultFactory[[]int]) *OperationResult[[]int] {
			ids := make([]int, 0, len(m.pipes))
			for id := range m.pipes {
				ids = append(ids, id)
			}
			return rf.Generate(true, false, ids)
		}).GetResult()
	return result
}

// ListMessageQueues returns a slice of all message queue names.
func (m *IPCManager) ListMessageQueues() []string {
	result, _ := StartNew[[]string]("ipc.IPCManager.ListMessageQueues", nil,
		func(op *Operation[[]string], rf *ResultFactory[[]string]) *OperationResult[[]string] {
			names := make([]string, 0, len(m.messageQueues))
			for name := range m.messageQueues {
				names = append(names, name)
			}
			return rf.Generate(true, false, names)
		}).GetResult()
	return result
}

// ListSharedRegions returns a slice of all shared memory region names.
func (m *IPCManager) ListSharedRegions() []string {
	result, _ := StartNew[[]string]("ipc.IPCManager.ListSharedRegions", nil,
		func(op *Operation[[]string], rf *ResultFactory[[]string]) *OperationResult[[]string] {
			names := make([]string, 0, len(m.sharedRegions))
			for name := range m.sharedRegions {
				names = append(names, name)
			}
			return rf.Generate(true, false, names)
		}).GetResult()
	return result
}
