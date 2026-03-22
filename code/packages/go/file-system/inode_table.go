package filesystem

import "fmt"

// ---------------------------------------------------------------------------
// InodeTable --- the fixed-size array of all inodes
// ---------------------------------------------------------------------------
// The inode table is an array of MaxInodes slots, each of which can hold
// one Inode or be nil (free). When a new file or directory is created, the
// file system allocates an inode from this table. When a file is deleted
// and its link count drops to 0, the inode is freed back.
//
// Analogy: The inode table is like the card catalog in a library. Each
// drawer (slot) can hold one catalog card (inode) or be empty.
//
// Allocation strategy: first-fit --- scan from slot 0 upward and return
// the first empty slot. O(n) worst case, but with 128 inodes it is instant.

// InodeTable is a fixed-size table of inodes indexed by inode number.
type InodeTable struct {
	inodes    []*Inode
	maxInodes int
}

// NewInodeTable creates a table with all slots initially free (nil).
func NewInodeTable(maxInodes int) *InodeTable {
	return &InodeTable{
		inodes:    make([]*Inode, maxInodes),
		maxInodes: maxInodes,
	}
}

// Allocate finds the first free slot, creates an Inode with the given type,
// and stores it. Returns the new inode, or nil if all slots are occupied.
func (it *InodeTable) Allocate(fileType FileType) *Inode {
	for i := 0; i < it.maxInodes; i++ {
		if it.inodes[i] == nil {
			inode := NewInode(i, fileType)
			it.inodes[i] = inode
			return inode
		}
	}
	return nil // All inodes used
}

// Free releases an inode slot, making it available for reuse.
// Returns an error if the inode number is out of range.
func (it *InodeTable) Free(inodeNum int) error {
	if inodeNum < 0 || inodeNum >= it.maxInodes {
		return fmt.Errorf("inode number %d out of range [0, %d)", inodeNum, it.maxInodes)
	}
	it.inodes[inodeNum] = nil
	return nil
}

// Get returns the inode at the given slot, or nil if the slot is free.
// Returns an error if the inode number is out of range.
func (it *InodeTable) Get(inodeNum int) (*Inode, error) {
	if inodeNum < 0 || inodeNum >= it.maxInodes {
		return nil, fmt.Errorf("inode number %d out of range [0, %d)", inodeNum, it.maxInodes)
	}
	return it.inodes[inodeNum], nil
}

// FreeCount returns the number of free (unallocated) inode slots.
func (it *InodeTable) FreeCount() int {
	count := 0
	for _, inode := range it.inodes {
		if inode == nil {
			count++
		}
	}
	return count
}

// MaxInodeCount returns the maximum number of inodes this table can hold.
func (it *InodeTable) MaxInodeCount() int {
	return it.maxInodes
}
