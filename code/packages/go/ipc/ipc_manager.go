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
	return &IPCManager{
		pipes:        make(map[int]*Pipe),
		nextPipeID:   0,
		nextFD:       3,
		messageQueues: make(map[string]*MessageQueue),
		sharedRegions: make(map[string]*SharedMemoryRegion),
	}
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
	pipeID := m.nextPipeID
	m.nextPipeID++

	readFD := m.nextFD
	m.nextFD++
	writeFD := m.nextFD
	m.nextFD++

	m.pipes[pipeID] = NewPipe(capacity)
	return pipeID, readFD, writeFD
}

// GetPipe looks up a pipe by its ID. Returns nil if not found.
func (m *IPCManager) GetPipe(pipeID int) *Pipe {
	return m.pipes[pipeID]
}

// ClosePipeRead closes the read end of a pipe.
func (m *IPCManager) ClosePipeRead(pipeID int) {
	if p, ok := m.pipes[pipeID]; ok {
		p.CloseRead()
	}
}

// ClosePipeWrite closes the write end of a pipe.
func (m *IPCManager) ClosePipeWrite(pipeID int) {
	if p, ok := m.pipes[pipeID]; ok {
		p.CloseWrite()
	}
}

// DestroyPipe removes a pipe from the manager entirely.
// Returns true if the pipe existed and was removed.
func (m *IPCManager) DestroyPipe(pipeID int) bool {
	if _, ok := m.pipes[pipeID]; ok {
		delete(m.pipes, pipeID)
		return true
	}
	return false
}

// ====================================================================
// Message queue management
// ====================================================================

// CreateMessageQueue creates (or retrieves) a message queue by name.
//
// If a queue with this name already exists, returns the existing one
// (like msgget with IPC_CREAT in Unix — idempotent creation).
func (m *IPCManager) CreateMessageQueue(name string, maxMessages, maxMessageSize int) *MessageQueue {
	if mq, ok := m.messageQueues[name]; ok {
		return mq
	}
	mq := NewMessageQueue(maxMessages, maxMessageSize)
	m.messageQueues[name] = mq
	return mq
}

// GetMessageQueue looks up a message queue by name. Returns nil if not found.
func (m *IPCManager) GetMessageQueue(name string) *MessageQueue {
	return m.messageQueues[name]
}

// DeleteMessageQueue removes a message queue. Returns true if it existed.
func (m *IPCManager) DeleteMessageQueue(name string) bool {
	if _, ok := m.messageQueues[name]; ok {
		delete(m.messageQueues, name)
		return true
	}
	return false
}

// ====================================================================
// Shared memory management
// ====================================================================

// CreateSharedMemory creates (or retrieves) a shared memory region by name.
//
// If a region with this name already exists, returns the existing one
// (like shmget with IPC_CREAT).
func (m *IPCManager) CreateSharedMemory(name string, size int, ownerPID int) *SharedMemoryRegion {
	if r, ok := m.sharedRegions[name]; ok {
		return r
	}
	r := NewSharedMemoryRegion(name, size, ownerPID)
	m.sharedRegions[name] = r
	return r
}

// GetSharedMemory looks up a shared memory region by name. Returns nil if not found.
func (m *IPCManager) GetSharedMemory(name string) *SharedMemoryRegion {
	return m.sharedRegions[name]
}

// DeleteSharedMemory removes a shared memory region. Returns true if it existed.
func (m *IPCManager) DeleteSharedMemory(name string) bool {
	if _, ok := m.sharedRegions[name]; ok {
		delete(m.sharedRegions, name)
		return true
	}
	return false
}

// ====================================================================
// Listing operations
// ====================================================================

// ListPipes returns a slice of all active pipe IDs.
func (m *IPCManager) ListPipes() []int {
	ids := make([]int, 0, len(m.pipes))
	for id := range m.pipes {
		ids = append(ids, id)
	}
	return ids
}

// ListMessageQueues returns a slice of all message queue names.
func (m *IPCManager) ListMessageQueues() []string {
	names := make([]string, 0, len(m.messageQueues))
	for name := range m.messageQueues {
		names = append(names, name)
	}
	return names
}

// ListSharedRegions returns a slice of all shared memory region names.
func (m *IPCManager) ListSharedRegions() []string {
	names := make([]string, 0, len(m.sharedRegions))
	for name := range m.sharedRegions {
		names = append(names, name)
	}
	return names
}
