package filesystem

import (
	"encoding/binary"
	"strings"
)

// ---------------------------------------------------------------------------
// VFS --- Virtual File System, the main API
// ---------------------------------------------------------------------------
// The VFS is the single entry point for all file operations. User programs
// never interact with inodes, block bitmaps, or directory entries directly.
// Instead, they call VFS methods like Open(), Read(), Write(), MkDir(),
// and Unlink(), and the VFS translates these high-level requests into
// low-level block I/O.
//
// Architecture:
//
//	User Program
//	|   vfs.Open("/data/log.txt", O_RDWR)
//	|   vfs.Write(fd, []byte("hello"))
//	|   vfs.Close(fd)
//	v
//	VFS (this file)
//	|   +-- Path Resolution:  "/" -> inode 0 -> "data" -> inode 5
//	|   +-- Inode Table:      metadata for every file/directory
//	|   +-- Block Bitmap:     which data blocks are free/used
//	|   +-- Open File Table:  system-wide table of open files
//	|   +-- Superblock:       file system metadata
//	v
//	In-Memory Block Storage ([]byte)

// VFS is the Virtual File System --- the main API for file operations.
type VFS struct {
	storage       []byte
	superblock    *Superblock
	inodeTable    *InodeTable
	blockBitmap   *BlockBitmap
	openFileTable *OpenFileTable
	formatted     bool
}

// NewVFS creates a new VFS with the given number of blocks and inodes.
func NewVFS(totalBlocks, totalInodes int) *VFS {
	result, _ := StartNew[*VFS]("file-system.NewVFS", nil,
		func(op *Operation[*VFS], rf *ResultFactory[*VFS]) *OperationResult[*VFS] {
			op.AddProperty("totalBlocks", totalBlocks)
			op.AddProperty("totalInodes", totalInodes)
			return rf.Generate(true, false, &VFS{
				storage:       make([]byte, totalBlocks*BlockSize),
				superblock:    NewSuperblock(totalBlocks, totalInodes),
				inodeTable:    NewInodeTable(totalInodes),
				blockBitmap:   NewBlockBitmap(totalBlocks),
				openFileTable: NewOpenFileTable(),
			})
		}).GetResult()
	return result
}

// NewDefaultVFS creates a VFS with default parameters (512 blocks, 128 inodes).
func NewDefaultVFS() *VFS {
	result, _ := StartNew[*VFS]("file-system.NewDefaultVFS", nil,
		func(op *Operation[*VFS], rf *ResultFactory[*VFS]) *OperationResult[*VFS] {
			return rf.Generate(true, false, NewVFS(MaxBlocks, MaxInodes))
		}).GetResult()
	return result
}

// GetSuperblock returns the file system's superblock (for inspection).
func (vfs *VFS) GetSuperblock() *Superblock {
	result, _ := StartNew[*Superblock]("file-system.VFS.GetSuperblock", nil,
		func(op *Operation[*Superblock], rf *ResultFactory[*Superblock]) *OperationResult[*Superblock] {
			return rf.Generate(true, false, vfs.superblock)
		}).GetResult()
	return result
}

// =======================================================================
// Format --- initialize a blank file system
// =======================================================================

// Format initializes the file system: creates the superblock and root
// directory. This is the equivalent of mkfs.ext2.
//
// Steps:
//  1. Allocate inode 0 as the root directory (type = DIRECTORY).
//  2. Allocate one data block for the root directory's entries.
//  3. Write the initial directory entries ("." and "..") to that block.
//  4. Update the superblock's free counts.
func (vfs *VFS) Format() error {
	_, err := StartNew[struct{}]("file-system.VFS.Format", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			// Step 1: allocate inode 0 for root directory
			rootInode := vfs.inodeTable.Allocate(FileTypeDirectory)
			if rootInode == nil {
				return rf.Fail(struct{}{}, errNoFreeInodes)
			}

			// Step 2: allocate a data block for root's directory entries
			rootBlock := vfs.blockBitmap.Allocate()
			if rootBlock < 0 {
				return rf.Fail(struct{}{}, errDiskFull)
			}
			rootInode.DirectBlks[0] = rootBlock

			// Step 3: create "." and ".." entries
			dotEntry, _ := NewDirectoryEntry(".", 0)
			dotDotEntry, _ := NewDirectoryEntry("..", 0)
			entries := []*DirectoryEntry{dotEntry, dotDotEntry}
			vfs.writeDirEntries(rootInode, entries)

			// Step 4: update superblock
			vfs.superblock.FreeBlocks = vfs.blockBitmap.FreeCount()
			vfs.superblock.FreeInodes = vfs.inodeTable.FreeCount()
			vfs.formatted = true

			return rf.Generate(true, false, struct{}{})
		}).GetResult()
	return err
}

// =======================================================================
// Open --- open a file by path
// =======================================================================

// Open opens a file by path. Creates the file if O_CREAT is set and it
// doesn't exist. Returns a file descriptor (fd >= 3) on success, -1 on error.
func (vfs *VFS) Open(path string, flags int) int {
	result, _ := StartNew[int]("file-system.VFS.Open", -1,
		func(op *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			op.AddProperty("path", path)
			op.AddProperty("flags", flags)
			return rf.Generate(true, false, vfs.openImpl(path, flags))
		}).GetResult()
	return result
}

func (vfs *VFS) openImpl(path string, flags int) int {
	inode := vfs.ResolvePath(path)

	if inode == nil {
		// File doesn't exist
		if flags&O_CREAT == 0 {
			return -1 // Not found and O_CREAT not set
		}

		// Create the file
		parentPath, filename := splitPath(path)
		if filename == "" {
			return -1
		}

		parentInode := vfs.ResolvePath(parentPath)
		if parentInode == nil {
			return -1
		}
		if parentInode.Type != FileTypeDirectory {
			return -1
		}

		newInode := vfs.inodeTable.Allocate(FileTypeRegular)
		if newInode == nil {
			return -1
		}

		// Add directory entry in parent
		entries := vfs.readDirEntries(parentInode)
		newEntry, _ := NewDirectoryEntry(filename, newInode.InodeNumber)
		entries = append(entries, newEntry)
		vfs.writeDirEntries(parentInode, entries)

		vfs.superblock.FreeInodes = vfs.inodeTable.FreeCount()
		inode = newInode
	}

	// Handle O_TRUNC
	if flags&O_TRUNC != 0 && inode.Type == FileTypeRegular {
		vfs.truncateInode(inode)
	}

	// Create open file entry
	fd := vfs.openFileTable.Open(inode.InodeNumber, flags)

	// Handle O_APPEND
	if flags&O_APPEND != 0 {
		openFile := vfs.openFileTable.Get(fd)
		if openFile != nil {
			openFile.Offset = inode.Size
		}
	}

	return fd
}

// =======================================================================
// Close --- close a file descriptor
// =======================================================================

// Close closes a file descriptor. Returns 0 on success, -1 on error.
func (vfs *VFS) Close(fd int) int {
	result, _ := StartNew[int]("file-system.VFS.Close", -1,
		func(op *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			op.AddProperty("fd", fd)
			if vfs.openFileTable.Close(fd) {
				return rf.Generate(true, false, 0)
			}
			return rf.Generate(true, false, -1)
		}).GetResult()
	return result
}

// =======================================================================
// Read --- read bytes from an open file
// =======================================================================

// Read reads up to count bytes from the file at the current offset.
// Returns the data read (may be shorter than count if EOF is reached).
func (vfs *VFS) Read(fd int, count int) []byte {
	result, _ := StartNew[[]byte]("file-system.VFS.Read", nil,
		func(op *Operation[[]byte], rf *ResultFactory[[]byte]) *OperationResult[[]byte] {
			op.AddProperty("fd", fd)
			op.AddProperty("count", count)
			return rf.Generate(true, false, vfs.readImpl(fd, count))
		}).GetResult()
	return result
}

func (vfs *VFS) readImpl(fd int, count int) []byte {
	openFile := vfs.openFileTable.Get(fd)
	if openFile == nil {
		return nil
	}

	// Check read permission
	accessMode := openFile.Flags & 0x3
	if accessMode == O_WRONLY {
		return nil
	}

	inode, _ := vfs.inodeTable.Get(openFile.InodeNumber)
	if inode == nil {
		return nil
	}

	remaining := inode.Size - openFile.Offset
	if remaining <= 0 {
		return nil
	}
	bytesToRead := count
	if bytesToRead > remaining {
		bytesToRead = remaining
	}

	result := make([]byte, 0, bytesToRead)
	bytesRead := 0

	for bytesRead < bytesToRead {
		blockIndex := openFile.Offset / BlockSize
		byteWithinBlock := openFile.Offset % BlockSize

		availableInBlock := BlockSize - byteWithinBlock
		chunkSize := bytesToRead - bytesRead
		if chunkSize > availableInBlock {
			chunkSize = availableInBlock
		}

		blockNum := vfs.getBlockNumber(inode, blockIndex)
		if blockNum < 0 {
			break
		}

		blockData := vfs.readBlock(blockNum)
		result = append(result, blockData[byteWithinBlock:byteWithinBlock+chunkSize]...)

		openFile.Offset += chunkSize
		bytesRead += chunkSize
	}

	return result
}

// =======================================================================
// Write --- write bytes to an open file
// =======================================================================

// Write writes data to the file at the current offset. Allocates new
// blocks as needed. Returns number of bytes written, or -1 on error.
func (vfs *VFS) Write(fd int, data []byte) int {
	result, _ := StartNew[int]("file-system.VFS.Write", -1,
		func(op *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			op.AddProperty("fd", fd)
			op.AddProperty("dataLen", len(data))
			return rf.Generate(true, false, vfs.writeImpl(fd, data))
		}).GetResult()
	return result
}

func (vfs *VFS) writeImpl(fd int, data []byte) int {
	openFile := vfs.openFileTable.Get(fd)
	if openFile == nil {
		return -1
	}

	accessMode := openFile.Flags & 0x3
	if accessMode == O_RDONLY {
		return -1
	}

	inode, _ := vfs.inodeTable.Get(openFile.InodeNumber)
	if inode == nil {
		return -1
	}

	// Handle O_APPEND
	if openFile.Flags&O_APPEND != 0 {
		openFile.Offset = inode.Size
	}

	bytesWritten := 0
	total := len(data)

	for bytesWritten < total {
		blockIndex := openFile.Offset / BlockSize
		byteWithinBlock := openFile.Offset % BlockSize

		blockNum := vfs.getBlockNumber(inode, blockIndex)
		if blockNum < 0 {
			blockNum = vfs.allocateBlockForInode(inode, blockIndex)
			if blockNum < 0 {
				break // Disk full
			}
		}

		blockData := make([]byte, BlockSize)
		copy(blockData, vfs.readBlock(blockNum))

		availableInBlock := BlockSize - byteWithinBlock
		chunkSize := total - bytesWritten
		if chunkSize > availableInBlock {
			chunkSize = availableInBlock
		}

		copy(blockData[byteWithinBlock:], data[bytesWritten:bytesWritten+chunkSize])
		vfs.writeBlock(blockNum, blockData)

		openFile.Offset += chunkSize
		bytesWritten += chunkSize

		if openFile.Offset > inode.Size {
			inode.Size = openFile.Offset
		}
	}

	vfs.superblock.FreeBlocks = vfs.blockBitmap.FreeCount()
	return bytesWritten
}

// =======================================================================
// Lseek --- reposition the file offset
// =======================================================================

// Lseek repositions the read/write offset. Returns the new offset, or -1.
func (vfs *VFS) Lseek(fd int, offset int, whence int) int {
	result, _ := StartNew[int]("file-system.VFS.Lseek", -1,
		func(op *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			op.AddProperty("fd", fd)
			op.AddProperty("offset", offset)
			op.AddProperty("whence", whence)
			return rf.Generate(true, false, vfs.lseekImpl(fd, offset, whence))
		}).GetResult()
	return result
}

func (vfs *VFS) lseekImpl(fd int, offset int, whence int) int {
	openFile := vfs.openFileTable.Get(fd)
	if openFile == nil {
		return -1
	}

	inode, _ := vfs.inodeTable.Get(openFile.InodeNumber)
	if inode == nil {
		return -1
	}

	var newOffset int
	switch whence {
	case SeekSet:
		newOffset = offset
	case SeekCur:
		newOffset = openFile.Offset + offset
	case SeekEnd:
		newOffset = inode.Size + offset
	default:
		return -1
	}

	if newOffset < 0 {
		return -1
	}

	openFile.Offset = newOffset
	return newOffset
}

// =======================================================================
// Stat --- get file metadata
// =======================================================================

// Stat returns the inode for the given path, or nil if not found.
func (vfs *VFS) Stat(path string) *Inode {
	result, _ := StartNew[*Inode]("file-system.VFS.Stat", nil,
		func(op *Operation[*Inode], rf *ResultFactory[*Inode]) *OperationResult[*Inode] {
			op.AddProperty("path", path)
			return rf.Generate(true, false, vfs.ResolvePath(path))
		}).GetResult()
	return result
}

// =======================================================================
// MkDir --- create a directory
// =======================================================================

// MkDir creates a new directory at the given path. Returns 0 on success, -1
// on error. Creates "." and ".." entries automatically.
func (vfs *VFS) MkDir(path string, permissions int) int {
	result, _ := StartNew[int]("file-system.VFS.MkDir", -1,
		func(op *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			op.AddProperty("path", path)
			op.AddProperty("permissions", permissions)
			return rf.Generate(true, false, vfs.mkDirImpl(path, permissions))
		}).GetResult()
	return result
}

func (vfs *VFS) mkDirImpl(path string, permissions int) int {
	if vfs.ResolvePath(path) != nil {
		return -1 // Already exists
	}

	parentPath, dirname := splitPath(path)
	if dirname == "" {
		return -1
	}

	parentInode := vfs.ResolvePath(parentPath)
	if parentInode == nil {
		return -1
	}
	if parentInode.Type != FileTypeDirectory {
		return -1
	}

	newInode := vfs.inodeTable.Allocate(FileTypeDirectory)
	if newInode == nil {
		return -1
	}
	newInode.Permissions = permissions

	newBlock := vfs.blockBitmap.Allocate()
	if newBlock < 0 {
		vfs.inodeTable.Free(newInode.InodeNumber)
		return -1
	}
	newInode.DirectBlks[0] = newBlock

	// Create "." and ".." entries
	dotEntry, _ := NewDirectoryEntry(".", newInode.InodeNumber)
	dotDotEntry, _ := NewDirectoryEntry("..", parentInode.InodeNumber)
	dirEntries := []*DirectoryEntry{dotEntry, dotDotEntry}
	vfs.writeDirEntries(newInode, dirEntries)

	newInode.LinkCount = 2

	// Add entry in parent
	parentEntries := vfs.readDirEntries(parentInode)
	newDirEntry, _ := NewDirectoryEntry(dirname, newInode.InodeNumber)
	parentEntries = append(parentEntries, newDirEntry)
	vfs.writeDirEntries(parentInode, parentEntries)

	parentInode.LinkCount++

	vfs.superblock.FreeBlocks = vfs.blockBitmap.FreeCount()
	vfs.superblock.FreeInodes = vfs.inodeTable.FreeCount()

	return 0
}

// =======================================================================
// ReadDir --- list directory entries
// =======================================================================

// ReadDir returns the directory entries at the given path.
// Returns nil if the path doesn't exist or is not a directory.
func (vfs *VFS) ReadDir(path string) []*DirectoryEntry {
	result, _ := StartNew[[]*DirectoryEntry]("file-system.VFS.ReadDir", nil,
		func(op *Operation[[]*DirectoryEntry], rf *ResultFactory[[]*DirectoryEntry]) *OperationResult[[]*DirectoryEntry] {
			op.AddProperty("path", path)
			inode := vfs.ResolvePath(path)
			if inode == nil {
				return rf.Generate(true, false, nil)
			}
			if inode.Type != FileTypeDirectory {
				return rf.Generate(true, false, nil)
			}
			return rf.Generate(true, false, vfs.readDirEntries(inode))
		}).GetResult()
	return result
}

// =======================================================================
// Unlink --- remove a file
// =======================================================================

// Unlink removes a file. Returns 0 on success, -1 on error.
// Does not work on directories (use RmDir instead).
func (vfs *VFS) Unlink(path string) int {
	result, _ := StartNew[int]("file-system.VFS.Unlink", -1,
		func(op *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			op.AddProperty("path", path)
			return rf.Generate(true, false, vfs.unlinkImpl(path))
		}).GetResult()
	return result
}

func (vfs *VFS) unlinkImpl(path string) int {
	parentPath, filename := splitPath(path)
	if filename == "" {
		return -1
	}

	parentInode := vfs.ResolvePath(parentPath)
	if parentInode == nil {
		return -1
	}

	entries := vfs.readDirEntries(parentInode)
	var targetEntry *DirectoryEntry
	for _, e := range entries {
		if e.Name == filename {
			targetEntry = e
			break
		}
	}
	if targetEntry == nil {
		return -1
	}

	targetInode, _ := vfs.inodeTable.Get(targetEntry.InodeNumber)
	if targetInode == nil {
		return -1
	}

	if targetInode.Type == FileTypeDirectory {
		return -1 // Use rmdir for directories
	}

	// Remove entry from parent
	newEntries := make([]*DirectoryEntry, 0, len(entries)-1)
	for _, e := range entries {
		if e.Name != filename {
			newEntries = append(newEntries, e)
		}
	}
	vfs.writeDirEntries(parentInode, newEntries)

	targetInode.LinkCount--
	if targetInode.LinkCount <= 0 {
		vfs.freeInodeBlocks(targetInode)
		vfs.inodeTable.Free(targetInode.InodeNumber)
	}

	vfs.superblock.FreeBlocks = vfs.blockBitmap.FreeCount()
	vfs.superblock.FreeInodes = vfs.inodeTable.FreeCount()

	return 0
}

// =======================================================================
// ResolvePath --- walk the directory tree
// =======================================================================

// ResolvePath resolves an absolute path to its inode.
//
// Algorithm:
//  1. Start at root inode (inode 0).
//  2. Split path by "/" and iterate over each component.
//  3. For each component, verify current inode is a directory.
//  4. Search directory entries for the component name.
//  5. If found, move to that entry's inode and continue.
//  6. If not found, return nil.
func (vfs *VFS) ResolvePath(path string) *Inode {
	result, _ := StartNew[*Inode]("file-system.VFS.ResolvePath", nil,
		func(op *Operation[*Inode], rf *ResultFactory[*Inode]) *OperationResult[*Inode] {
			op.AddProperty("path", path)
			return rf.Generate(true, false, vfs.resolvePathImpl(path))
		}).GetResult()
	return result
}

func (vfs *VFS) resolvePathImpl(path string) *Inode {
	if path == "" || path[0] != '/' {
		return nil
	}

	currentInode, _ := vfs.inodeTable.Get(0)
	if currentInode == nil {
		return nil
	}

	// Split and filter empty components
	parts := strings.Split(path, "/")
	components := make([]string, 0)
	for _, p := range parts {
		if p != "" {
			components = append(components, p)
		}
	}

	if len(components) == 0 {
		return currentInode // Just "/"
	}

	for _, component := range components {
		if currentInode.Type != FileTypeDirectory {
			return nil
		}

		entries := vfs.readDirEntries(currentInode)
		found := false
		for _, entry := range entries {
			if entry.Name == component {
				currentInode, _ = vfs.inodeTable.Get(entry.InodeNumber)
				if currentInode == nil {
					return nil
				}
				found = true
				break
			}
		}
		if !found {
			return nil
		}
	}

	return currentInode
}

// =======================================================================
// Internal helpers --- block I/O
// =======================================================================

func (vfs *VFS) readBlock(blockNum int) []byte {
	start := blockNum * BlockSize
	result := make([]byte, BlockSize)
	copy(result, vfs.storage[start:start+BlockSize])
	return result
}

func (vfs *VFS) writeBlock(blockNum int, data []byte) {
	start := blockNum * BlockSize
	// Pad with zeros if shorter than BlockSize
	padded := make([]byte, BlockSize)
	copy(padded, data)
	copy(vfs.storage[start:start+BlockSize], padded)
}

// =======================================================================
// Internal helpers --- directory I/O
// =======================================================================

func (vfs *VFS) readDirEntries(inode *Inode) []*DirectoryEntry {
	var rawData []byte

	for i := 0; i < DirectBlocks; i++ {
		if inode.DirectBlks[i] == -1 {
			break
		}
		rawData = append(rawData, vfs.readBlock(inode.DirectBlks[i])...)
	}

	if inode.IndirectBlock != -1 {
		indirectData := vfs.readBlock(inode.IndirectBlock)
		for j := 0; j < BlockSize; j += 4 {
			ptr := int(int32(binary.LittleEndian.Uint32(indirectData[j : j+4])))
			if ptr == -1 || ptr == 0 {
				break
			}
			rawData = append(rawData, vfs.readBlock(ptr)...)
		}
	}

	// Trim null bytes and parse
	text := strings.TrimRight(string(rawData), "\x00")
	var entries []*DirectoryEntry
	for _, line := range strings.Split(text, "\n") {
		line = strings.TrimSpace(line)
		if line != "" && strings.Contains(line, ":") {
			entry, err := DeserializeDirectoryEntry(line)
			if err == nil {
				entries = append(entries, entry)
			}
		}
	}
	return entries
}

func (vfs *VFS) writeDirEntries(inode *Inode, entries []*DirectoryEntry) {
	var sb strings.Builder
	for _, entry := range entries {
		sb.WriteString(entry.Serialize())
	}
	data := []byte(sb.String())
	inode.Size = len(data)

	offset := 0
	blockIndex := 0
	for offset < len(data) {
		end := offset + BlockSize
		if end > len(data) {
			end = len(data)
		}
		chunk := data[offset:end]

		blockNum := vfs.getBlockNumber(inode, blockIndex)
		if blockNum < 0 {
			blockNum = vfs.allocateBlockForInode(inode, blockIndex)
			if blockNum < 0 {
				return // Disk full
			}
		}

		vfs.writeBlock(blockNum, chunk)
		offset += BlockSize
		blockIndex++
	}
}

// =======================================================================
// Internal helpers --- block allocation and lookup
// =======================================================================

func (vfs *VFS) getBlockNumber(inode *Inode, blockIndex int) int {
	if blockIndex < DirectBlocks {
		return inode.DirectBlks[blockIndex]
	}

	maxIndirect := BlockSize / 4
	if blockIndex < DirectBlocks+maxIndirect {
		if inode.IndirectBlock == -1 {
			return -1
		}
		indirectData := vfs.readBlock(inode.IndirectBlock)
		ptrOffset := (blockIndex - DirectBlocks) * 4
		ptr := int(int32(binary.LittleEndian.Uint32(indirectData[ptrOffset : ptrOffset+4])))
		return ptr
	}
	return -1 // Beyond addressing capability
}

func (vfs *VFS) allocateBlockForInode(inode *Inode, blockIndex int) int {
	newBlock := vfs.blockBitmap.Allocate()
	if newBlock < 0 {
		return -1
	}

	if blockIndex < DirectBlocks {
		inode.DirectBlks[blockIndex] = newBlock
		return newBlock
	}

	maxIndirect := BlockSize / 4
	if blockIndex < DirectBlocks+maxIndirect {
		if inode.IndirectBlock == -1 {
			indirectBlock := vfs.blockBitmap.Allocate()
			if indirectBlock < 0 {
				vfs.blockBitmap.Free(newBlock)
				return -1
			}
			inode.IndirectBlock = indirectBlock
			// Initialize with -1 pointers
			initData := make([]byte, BlockSize)
			for i := 0; i < BlockSize; i += 4 {
				binary.LittleEndian.PutUint32(initData[i:], uint32(0xFFFFFFFF)) // -1 as int32
			}
			vfs.writeBlock(indirectBlock, initData)
		}

		indirectData := make([]byte, BlockSize)
		copy(indirectData, vfs.readBlock(inode.IndirectBlock))
		ptrOffset := (blockIndex - DirectBlocks) * 4
		binary.LittleEndian.PutUint32(indirectData[ptrOffset:], uint32(newBlock))
		vfs.writeBlock(inode.IndirectBlock, indirectData)

		return newBlock
	}

	vfs.blockBitmap.Free(newBlock)
	return -1
}

func (vfs *VFS) truncateInode(inode *Inode) {
	vfs.freeInodeBlocks(inode)
	inode.Size = 0
}

func (vfs *VFS) freeInodeBlocks(inode *Inode) {
	// Free direct blocks
	for i := 0; i < DirectBlocks; i++ {
		if inode.DirectBlks[i] != -1 {
			vfs.blockBitmap.Free(inode.DirectBlks[i])
			inode.DirectBlks[i] = -1
		}
	}

	// Free indirect block and its pointers
	if inode.IndirectBlock != -1 {
		indirectData := vfs.readBlock(inode.IndirectBlock)
		for j := 0; j < BlockSize; j += 4 {
			ptr := int(int32(binary.LittleEndian.Uint32(indirectData[j : j+4])))
			if ptr != -1 && ptr != 0 {
				vfs.blockBitmap.Free(ptr)
			}
		}
		vfs.blockBitmap.Free(inode.IndirectBlock)
		inode.IndirectBlock = -1
	}
}

// =======================================================================
// Path utilities
// =======================================================================

func splitPath(path string) (parentPath string, basename string) {
	path = strings.TrimRight(path, "/")
	if path == "" || path == "/" {
		return "/", ""
	}
	lastSlash := strings.LastIndex(path, "/")
	if lastSlash == 0 {
		return "/", path[1:]
	}
	return path[:lastSlash], path[lastSlash+1:]
}

// Sentinel errors
var (
	errNoFreeInodes = &fsError{"no free inodes"}
	errDiskFull     = &fsError{"disk full"}
)

type fsError struct {
	msg string
}

func (e *fsError) Error() string {
	return e.msg
}
