package filesystem

// ---------------------------------------------------------------------------
// FileType --- what kind of object an inode represents
// ---------------------------------------------------------------------------
// In Unix, the mantra is "everything is a file." Directories, devices,
// pipes, and sockets are all represented as inodes with different types.
// The kernel dispatches operations differently based on the type.

// FileType indicates what kind of file system object an inode represents.
type FileType int

const (
	// FileTypeRegular is an ordinary file (text, binary, image, etc.).
	FileTypeRegular FileType = 1

	// FileTypeDirectory contains directory entries (name -> inode pairs).
	FileTypeDirectory FileType = 2

	// FileTypeSymlink is a symbolic link (stores a path to another file).
	FileTypeSymlink FileType = 3

	// FileTypeCharDevice is a character device (e.g., keyboard, serial port).
	FileTypeCharDevice FileType = 4

	// FileTypeBlockDevice is a block device (e.g., hard disk, SSD).
	FileTypeBlockDevice FileType = 5

	// FileTypePipe is a named pipe / FIFO for inter-process communication.
	FileTypePipe FileType = 6

	// FileTypeSocket is a Unix domain socket for local IPC.
	FileTypeSocket FileType = 7
)

// ---------------------------------------------------------------------------
// Inode --- the heart of the file system
// ---------------------------------------------------------------------------
// An inode (index node) stores everything about a file *except its name*.
// Names live in directories, not in files. This separation is what makes
// hard links possible: one file can have multiple names, all pointing to
// the same inode.
//
// Block pointer structure:
//
//	Inode
//	+-------------------+
//	| DirectBlks[0]  -----> Data Block (bytes 0-511)
//	| DirectBlks[1]  -----> Data Block (bytes 512-1023)
//	| ...               |
//	| DirectBlks[11] -----> Data Block (bytes 5632-6143)
//	|                   |
//	| IndirectBlock  -----> +--------------------+
//	|                   |   | ptr[0] -> Data     | (bytes 6144-6655)
//	|                   |   | ...                |
//	|                   |   | ptr[127] -> Data   | (bytes 71168-71679)
//	+-------------------+   +--------------------+

// Inode is a fixed-size record storing all metadata for one file or directory.
type Inode struct {
	// InodeNumber is the unique identifier (0 through MaxInodes-1).
	// Inode 0 is always the root directory "/".
	InodeNumber int

	// Type indicates what kind of object this inode represents.
	Type FileType

	// Size is the file size in bytes. For directories, this is the total
	// serialized size of all directory entries.
	Size int

	// Permissions are the octal permission bits (e.g., 0755 = rwxr-xr-x).
	Permissions int

	// OwnerPID is the PID of the process that created this file.
	OwnerPID int

	// LinkCount is the number of directory entries pointing to this inode.
	// When it reaches 0, the inode and its data blocks are freed.
	LinkCount int

	// DirectBlks are the 12 direct block pointers. Each points to a data
	// block containing file data. A value of -1 means "not allocated."
	DirectBlks [DirectBlocks]int

	// IndirectBlock is the block number of an indirect block. The indirect
	// block contains up to 128 four-byte block numbers pointing to data
	// blocks. A value of -1 means "no indirect block allocated."
	IndirectBlock int

	// CreatedAt is the creation timestamp (seconds since epoch).
	CreatedAt int

	// ModifiedAt is the last modification timestamp.
	ModifiedAt int
}

// NewInode creates a new inode with sensible defaults. All direct block
// pointers are initialized to -1 (unallocated).
func NewInode(inodeNumber int, fileType FileType) *Inode {
	result, _ := StartNew[*Inode]("file-system.NewInode", nil,
		func(op *Operation[*Inode], rf *ResultFactory[*Inode]) *OperationResult[*Inode] {
			op.AddProperty("inodeNumber", inodeNumber)
			inode := &Inode{
				InodeNumber:   inodeNumber,
				Type:          fileType,
				Permissions:   0o755,
				LinkCount:     1,
				IndirectBlock: -1,
			}
			// Initialize all direct block pointers to -1 (unallocated)
			for i := range inode.DirectBlks {
				inode.DirectBlks[i] = -1
			}
			return rf.Generate(true, false, inode)
		}).GetResult()
	return result
}
