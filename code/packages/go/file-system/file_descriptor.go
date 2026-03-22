package filesystem

// ---------------------------------------------------------------------------
// File descriptors --- the process's view of open files
// ---------------------------------------------------------------------------
// When a process calls open("/data/log.txt"), it gets a small integer --- a
// file descriptor (fd). File descriptors abstract away inodes and blocks.
//
// There are two levels of indirection:
//
//	Process A                          System-Wide
//	+---------------------+           +------------------------------+
//	| FileDescriptorTable |           | OpenFileTable                |
//	| (per-process)       |           | (shared by all processes)    |
//	|                     |           |                              |
//	| fd 0 -> global 0    |           | entry 0: stdin               |
//	| fd 1 -> global 1    |           | entry 1: stdout              |
//	| fd 2 -> global 2    |           | entry 2: stderr              |
//	| fd 3 -> global 5    |           | entry 5: inode=23, offset=42 |
//	+---------------------+           +------------------------------+

// OpenFile is a system-wide entry representing one opening of a file.
// Multiple file descriptors can point to the same OpenFile entry (they
// share the same offset and flags).
type OpenFile struct {
	// InodeNumber identifies which file this entry refers to.
	InodeNumber int

	// Offset is the current read/write position within the file.
	Offset int

	// Flags records how the file was opened (O_RDONLY, O_WRONLY, O_RDWR, etc.).
	Flags int

	// RefCount tracks how many file descriptors point to this entry.
	// When it drops to 0, the entry is removed.
	RefCount int
}

// OpenFileTable is the system-wide table of all open files.
// File descriptors 0, 1, 2 are reserved for stdin, stdout, stderr.
type OpenFileTable struct {
	entries map[int]*OpenFile
	nextFD  int
}

// NewOpenFileTable creates a new system-wide open file table.
// File descriptors start at 3 (0-2 reserved for stdio).
func NewOpenFileTable() *OpenFileTable {
	return &OpenFileTable{
		entries: make(map[int]*OpenFile),
		nextFD:  3,
	}
}

// Open creates a new open file entry and returns its file descriptor.
func (oft *OpenFileTable) Open(inodeNumber int, flags int) int {
	fd := oft.nextFD
	oft.entries[fd] = &OpenFile{
		InodeNumber: inodeNumber,
		Offset:      0,
		Flags:       flags,
		RefCount:    1,
	}
	oft.nextFD++
	return fd
}

// Close decrements the ref count on the entry. If ref count drops to 0,
// the entry is removed. Returns true if the fd existed, false otherwise.
func (oft *OpenFileTable) Close(fd int) bool {
	entry, ok := oft.entries[fd]
	if !ok {
		return false
	}
	entry.RefCount--
	if entry.RefCount <= 0 {
		delete(oft.entries, fd)
	}
	return true
}

// Get retrieves the OpenFile entry for a file descriptor.
// Returns nil if the fd does not exist.
func (oft *OpenFileTable) Get(fd int) *OpenFile {
	return oft.entries[fd]
}

// Dup duplicates a file descriptor. Creates a new fd pointing to the same
// OpenFile entry (incrementing ref count). Returns -1 if fd is invalid.
func (oft *OpenFileTable) Dup(fd int) int {
	entry, ok := oft.entries[fd]
	if !ok {
		return -1
	}
	entry.RefCount++
	newFD := oft.nextFD
	oft.entries[newFD] = entry
	oft.nextFD++
	return newFD
}

// Dup2 duplicates old_fd to new_fd. If new_fd is already open, it is
// closed first. Returns new_fd on success, -1 if old_fd is invalid.
func (oft *OpenFileTable) Dup2(oldFD, newFD int) int {
	oldEntry, ok := oft.entries[oldFD]
	if !ok {
		return -1
	}

	// Close new_fd if it's already open
	if _, exists := oft.entries[newFD]; exists {
		oft.Close(newFD)
	}

	oldEntry.RefCount++
	oft.entries[newFD] = oldEntry
	return newFD
}

// FileDescriptorTable is a per-process mapping of local fds to system-wide
// fds. Each process has its own table, allowing different processes to have
// fd 3 point to different files.
type FileDescriptorTable struct {
	mappings map[int]int // local_fd -> global_fd
}

// NewFileDescriptorTable creates an empty per-process fd table.
func NewFileDescriptorTable() *FileDescriptorTable {
	return &FileDescriptorTable{
		mappings: make(map[int]int),
	}
}

// Add creates a mapping from a local fd to a system-wide fd.
func (fdt *FileDescriptorTable) Add(localFD, globalFD int) {
	fdt.mappings[localFD] = globalFD
}

// Remove removes a local fd mapping and returns the global fd.
// Returns -1 if no mapping existed.
func (fdt *FileDescriptorTable) Remove(localFD int) int {
	globalFD, ok := fdt.mappings[localFD]
	if !ok {
		return -1
	}
	delete(fdt.mappings, localFD)
	return globalFD
}

// GetGlobal looks up the system-wide fd for a local fd.
// Returns -1 if no mapping exists.
func (fdt *FileDescriptorTable) GetGlobal(localFD int) int {
	globalFD, ok := fdt.mappings[localFD]
	if !ok {
		return -1
	}
	return globalFD
}

// Clone creates an independent copy of this table (used during fork).
func (fdt *FileDescriptorTable) Clone() *FileDescriptorTable {
	newTable := NewFileDescriptorTable()
	for k, v := range fdt.mappings {
		newTable.mappings[k] = v
	}
	return newTable
}
