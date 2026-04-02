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
	result, _ := StartNew[*OpenFileTable]("file-system.NewOpenFileTable", nil,
		func(op *Operation[*OpenFileTable], rf *ResultFactory[*OpenFileTable]) *OperationResult[*OpenFileTable] {
			return rf.Generate(true, false, &OpenFileTable{
				entries: make(map[int]*OpenFile),
				nextFD:  3,
			})
		}).GetResult()
	return result
}

// Open creates a new open file entry and returns its file descriptor.
func (oft *OpenFileTable) Open(inodeNumber int, flags int) int {
	result, _ := StartNew[int]("file-system.OpenFileTable.Open", -1,
		func(op *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			op.AddProperty("inodeNumber", inodeNumber)
			op.AddProperty("flags", flags)
			fd := oft.nextFD
			oft.entries[fd] = &OpenFile{
				InodeNumber: inodeNumber,
				Offset:      0,
				Flags:       flags,
				RefCount:    1,
			}
			oft.nextFD++
			return rf.Generate(true, false, fd)
		}).GetResult()
	return result
}

// Close decrements the ref count on the entry. If ref count drops to 0,
// the entry is removed. Returns true if the fd existed, false otherwise.
func (oft *OpenFileTable) Close(fd int) bool {
	result, _ := StartNew[bool]("file-system.OpenFileTable.Close", false,
		func(op *Operation[bool], rf *ResultFactory[bool]) *OperationResult[bool] {
			op.AddProperty("fd", fd)
			entry, ok := oft.entries[fd]
			if !ok {
				return rf.Generate(true, false, false)
			}
			entry.RefCount--
			if entry.RefCount <= 0 {
				delete(oft.entries, fd)
			}
			return rf.Generate(true, false, true)
		}).GetResult()
	return result
}

// Get retrieves the OpenFile entry for a file descriptor.
// Returns nil if the fd does not exist.
func (oft *OpenFileTable) Get(fd int) *OpenFile {
	result, _ := StartNew[*OpenFile]("file-system.OpenFileTable.Get", nil,
		func(op *Operation[*OpenFile], rf *ResultFactory[*OpenFile]) *OperationResult[*OpenFile] {
			op.AddProperty("fd", fd)
			return rf.Generate(true, false, oft.entries[fd])
		}).GetResult()
	return result
}

// Dup duplicates a file descriptor. Creates a new fd pointing to the same
// OpenFile entry (incrementing ref count). Returns -1 if fd is invalid.
func (oft *OpenFileTable) Dup(fd int) int {
	result, _ := StartNew[int]("file-system.OpenFileTable.Dup", -1,
		func(op *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			op.AddProperty("fd", fd)
			entry, ok := oft.entries[fd]
			if !ok {
				return rf.Generate(true, false, -1)
			}
			entry.RefCount++
			newFD := oft.nextFD
			oft.entries[newFD] = entry
			oft.nextFD++
			return rf.Generate(true, false, newFD)
		}).GetResult()
	return result
}

// Dup2 duplicates old_fd to new_fd. If new_fd is already open, it is
// closed first. Returns new_fd on success, -1 if old_fd is invalid.
func (oft *OpenFileTable) Dup2(oldFD, newFD int) int {
	result, _ := StartNew[int]("file-system.OpenFileTable.Dup2", -1,
		func(op *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			op.AddProperty("oldFD", oldFD)
			op.AddProperty("newFD", newFD)
			oldEntry, ok := oft.entries[oldFD]
			if !ok {
				return rf.Generate(true, false, -1)
			}

			// Close new_fd if it's already open
			if _, exists := oft.entries[newFD]; exists {
				oft.Close(newFD)
			}

			oldEntry.RefCount++
			oft.entries[newFD] = oldEntry
			return rf.Generate(true, false, newFD)
		}).GetResult()
	return result
}

// FileDescriptorTable is a per-process mapping of local fds to system-wide
// fds. Each process has its own table, allowing different processes to have
// fd 3 point to different files.
type FileDescriptorTable struct {
	mappings map[int]int // local_fd -> global_fd
}

// NewFileDescriptorTable creates an empty per-process fd table.
func NewFileDescriptorTable() *FileDescriptorTable {
	result, _ := StartNew[*FileDescriptorTable]("file-system.NewFileDescriptorTable", nil,
		func(op *Operation[*FileDescriptorTable], rf *ResultFactory[*FileDescriptorTable]) *OperationResult[*FileDescriptorTable] {
			return rf.Generate(true, false, &FileDescriptorTable{
				mappings: make(map[int]int),
			})
		}).GetResult()
	return result
}

// Add creates a mapping from a local fd to a system-wide fd.
func (fdt *FileDescriptorTable) Add(localFD, globalFD int) {
	_, _ = StartNew[struct{}]("file-system.FileDescriptorTable.Add", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			op.AddProperty("localFD", localFD)
			op.AddProperty("globalFD", globalFD)
			fdt.mappings[localFD] = globalFD
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
}

// Remove removes a local fd mapping and returns the global fd.
// Returns -1 if no mapping existed.
func (fdt *FileDescriptorTable) Remove(localFD int) int {
	result, _ := StartNew[int]("file-system.FileDescriptorTable.Remove", -1,
		func(op *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			op.AddProperty("localFD", localFD)
			globalFD, ok := fdt.mappings[localFD]
			if !ok {
				return rf.Generate(true, false, -1)
			}
			delete(fdt.mappings, localFD)
			return rf.Generate(true, false, globalFD)
		}).GetResult()
	return result
}

// GetGlobal looks up the system-wide fd for a local fd.
// Returns -1 if no mapping exists.
func (fdt *FileDescriptorTable) GetGlobal(localFD int) int {
	result, _ := StartNew[int]("file-system.FileDescriptorTable.GetGlobal", -1,
		func(op *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			op.AddProperty("localFD", localFD)
			globalFD, ok := fdt.mappings[localFD]
			if !ok {
				return rf.Generate(true, false, -1)
			}
			return rf.Generate(true, false, globalFD)
		}).GetResult()
	return result
}

// Clone creates an independent copy of this table (used during fork).
func (fdt *FileDescriptorTable) Clone() *FileDescriptorTable {
	result, _ := StartNew[*FileDescriptorTable]("file-system.FileDescriptorTable.Clone", nil,
		func(op *Operation[*FileDescriptorTable], rf *ResultFactory[*FileDescriptorTable]) *OperationResult[*FileDescriptorTable] {
			newTable := NewFileDescriptorTable()
			for k, v := range fdt.mappings {
				newTable.mappings[k] = v
			}
			return rf.Generate(true, false, newTable)
		}).GetResult()
	return result
}
