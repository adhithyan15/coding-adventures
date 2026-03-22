package filesystem

import (
	"fmt"
)

// ---------------------------------------------------------------------------
// BlockBitmap --- tracking free and used blocks
// ---------------------------------------------------------------------------
// A block bitmap uses a single byte per block to track whether each data
// block on disk is free (0) or used (1). We use a byte array rather than
// actual bits for clarity --- a production file system would pack 8 blocks
// into each byte, but our approach makes the code much more readable.
//
// Operations:
//   - Allocate(): find first free block, mark it used, return its index
//   - Free(n):    mark block n as free
//   - IsFree(n):  check if block n is available
//   - FreeCount(): count how many blocks are free

// BlockBitmap tracks which data blocks are free (0) vs. allocated (1).
type BlockBitmap struct {
	bitmap      []byte
	totalBlocks int
}

// NewBlockBitmap creates a bitmap with all blocks initially free.
func NewBlockBitmap(totalBlocks int) *BlockBitmap {
	return &BlockBitmap{
		bitmap:      make([]byte, totalBlocks),
		totalBlocks: totalBlocks,
	}
}

// Allocate finds the first free block, marks it as used, and returns its
// index. Returns -1 if the disk is full (all blocks are used).
func (bm *BlockBitmap) Allocate() int {
	for i := 0; i < bm.totalBlocks; i++ {
		if bm.bitmap[i] == 0 {
			bm.bitmap[i] = 1
			return i
		}
	}
	return -1 // Disk full
}

// Free marks a block as free (available for reuse).
// Returns an error if the block number is out of range.
func (bm *BlockBitmap) Free(blockNum int) error {
	if blockNum < 0 || blockNum >= bm.totalBlocks {
		return fmt.Errorf("block number %d out of range [0, %d)", blockNum, bm.totalBlocks)
	}
	bm.bitmap[blockNum] = 0
	return nil
}

// IsFree checks whether a block is free (unallocated).
// Returns an error if the block number is out of range.
func (bm *BlockBitmap) IsFree(blockNum int) (bool, error) {
	if blockNum < 0 || blockNum >= bm.totalBlocks {
		return false, fmt.Errorf("block number %d out of range [0, %d)", blockNum, bm.totalBlocks)
	}
	return bm.bitmap[blockNum] == 0, nil
}

// FreeCount returns the number of free (unallocated) blocks.
func (bm *BlockBitmap) FreeCount() int {
	count := 0
	for _, b := range bm.bitmap {
		if b == 0 {
			count++
		}
	}
	return count
}

// TotalBlockCount returns the total number of blocks tracked by this bitmap.
func (bm *BlockBitmap) TotalBlockCount() int {
	return bm.totalBlocks
}
