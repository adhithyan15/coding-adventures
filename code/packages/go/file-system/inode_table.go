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
	result, _ := StartNew[*InodeTable]("file-system.NewInodeTable", nil,
		func(op *Operation[*InodeTable], rf *ResultFactory[*InodeTable]) *OperationResult[*InodeTable] {
			op.AddProperty("maxInodes", maxInodes)
			return rf.Generate(true, false, &InodeTable{
				inodes:    make([]*Inode, maxInodes),
				maxInodes: maxInodes,
			})
		}).GetResult()
	return result
}

// Allocate finds the first free slot, creates an Inode with the given type,
// and stores it. Returns the new inode, or nil if all slots are occupied.
func (it *InodeTable) Allocate(fileType FileType) *Inode {
	result, _ := StartNew[*Inode]("file-system.InodeTable.Allocate", nil,
		func(op *Operation[*Inode], rf *ResultFactory[*Inode]) *OperationResult[*Inode] {
			for i := 0; i < it.maxInodes; i++ {
				if it.inodes[i] == nil {
					inode := NewInode(i, fileType)
					it.inodes[i] = inode
					return rf.Generate(true, false, inode)
				}
			}
			return rf.Generate(false, false, nil) // All inodes used
		}).GetResult()
	return result
}

// Free releases an inode slot, making it available for reuse.
// Returns an error if the inode number is out of range.
func (it *InodeTable) Free(inodeNum int) error {
	_, err := StartNew[struct{}]("file-system.InodeTable.Free", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			op.AddProperty("inodeNum", inodeNum)
			if inodeNum < 0 || inodeNum >= it.maxInodes {
				return rf.Fail(struct{}{}, fmt.Errorf("inode number %d out of range [0, %d)", inodeNum, it.maxInodes))
			}
			it.inodes[inodeNum] = nil
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
	return err
}

// Get returns the inode at the given slot, or nil if the slot is free.
// Returns an error if the inode number is out of range.
func (it *InodeTable) Get(inodeNum int) (*Inode, error) {
	return StartNew[*Inode]("file-system.InodeTable.Get", nil,
		func(op *Operation[*Inode], rf *ResultFactory[*Inode]) *OperationResult[*Inode] {
			op.AddProperty("inodeNum", inodeNum)
			if inodeNum < 0 || inodeNum >= it.maxInodes {
				return rf.Fail(nil, fmt.Errorf("inode number %d out of range [0, %d)", inodeNum, it.maxInodes))
			}
			return rf.Generate(true, false, it.inodes[inodeNum])
		}).GetResult()
}

// FreeCount returns the number of free (unallocated) inode slots.
func (it *InodeTable) FreeCount() int {
	result, _ := StartNew[int]("file-system.InodeTable.FreeCount", 0,
		func(op *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			count := 0
			for _, inode := range it.inodes {
				if inode == nil {
					count++
				}
			}
			return rf.Generate(true, false, count)
		}).GetResult()
	return result
}

// MaxInodeCount returns the maximum number of inodes this table can hold.
func (it *InodeTable) MaxInodeCount() int {
	result, _ := StartNew[int]("file-system.InodeTable.MaxInodeCount", 0,
		func(op *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			return rf.Generate(true, false, it.maxInodes)
		}).GetResult()
	return result
}
