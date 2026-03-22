package filesystem

// ---------------------------------------------------------------------------
// Superblock --- the file system's identity card
// ---------------------------------------------------------------------------
// The superblock is always stored at block 0 --- the very first block on
// disk. It is the first thing the OS reads when mounting a file system.
// Without a valid superblock, the OS cannot interpret the rest of the disk.
//
// Analogy: The superblock is like the cover page of a book. It tells you
// the title (magic number), the number of pages (total blocks), and the
// table of contents (where to find inodes and data).
//
// Layout within block 0 (conceptual):
//
//	+---------+-----------+-------------+-------------+-------------+-------------+
//	| magic   | blockSize | totalBlocks | totalInodes | freeBlocks  | freeInodes  |
//	| (4 B)   | (4 B)     | (4 B)       | (4 B)       | (4 B)       | (4 B)       |
//	+---------+-----------+-------------+-------------+-------------+-------------+

// SuperblockMagic is the magic number 0x45585432, which is the ASCII
// encoding of "EXT2". Used to verify that a disk contains our file system.
const SuperblockMagic = 0x45585432

// Superblock stores the essential metadata for the entire file system.
type Superblock struct {
	// Magic identifies this disk as containing our file system.
	// Must be SuperblockMagic (0x45585432 = "EXT2").
	Magic int

	// BlockSize is the size of every block on disk, in bytes.
	BlockSize int

	// TotalBlocks is the total number of blocks on the disk.
	TotalBlocks int

	// TotalInodes is the maximum number of files/directories.
	TotalInodes int

	// FreeBlocks is how many data blocks are currently unallocated.
	FreeBlocks int

	// FreeInodes is how many inodes are currently unallocated.
	FreeInodes int

	// RootInodeNum is the inode number of the root directory (always 0).
	RootInodeNum int
}

// NewSuperblock creates a superblock with the given block and inode counts.
// All blocks and inodes start as free.
func NewSuperblock(totalBlocks, totalInodes int) *Superblock {
	return &Superblock{
		Magic:        SuperblockMagic,
		BlockSize:    BlockSize,
		TotalBlocks:  totalBlocks,
		TotalInodes:  totalInodes,
		FreeBlocks:   totalBlocks,
		FreeInodes:   totalInodes,
		RootInodeNum: RootInode,
	}
}

// IsValid checks whether this superblock has the correct magic number.
// Returns true if the magic number matches SuperblockMagic, meaning this
// disk was formatted with our file system.
func (sb *Superblock) IsValid() bool {
	return sb.Magic == SuperblockMagic
}
