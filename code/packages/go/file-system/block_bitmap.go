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
	result, _ := StartNew[*BlockBitmap]("file-system.NewBlockBitmap", nil,
		func(op *Operation[*BlockBitmap], rf *ResultFactory[*BlockBitmap]) *OperationResult[*BlockBitmap] {
			op.AddProperty("totalBlocks", totalBlocks)
			return rf.Generate(true, false, &BlockBitmap{
				bitmap:      make([]byte, totalBlocks),
				totalBlocks: totalBlocks,
			})
		}).GetResult()
	return result
}

// Allocate finds the first free block, marks it as used, and returns its
// index. Returns -1 if the disk is full (all blocks are used).
func (bm *BlockBitmap) Allocate() int {
	result, _ := StartNew[int]("file-system.BlockBitmap.Allocate", -1,
		func(op *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			for i := 0; i < bm.totalBlocks; i++ {
				if bm.bitmap[i] == 0 {
					bm.bitmap[i] = 1
					return rf.Generate(true, false, i)
				}
			}
			return rf.Generate(true, false, -1) // Disk full
		}).GetResult()
	return result
}

// Free marks a block as free (available for reuse).
// Returns an error if the block number is out of range.
func (bm *BlockBitmap) Free(blockNum int) error {
	_, err := StartNew[struct{}]("file-system.BlockBitmap.Free", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			op.AddProperty("blockNum", blockNum)
			if blockNum < 0 || blockNum >= bm.totalBlocks {
				return rf.Fail(struct{}{}, fmt.Errorf("block number %d out of range [0, %d)", blockNum, bm.totalBlocks))
			}
			bm.bitmap[blockNum] = 0
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
	return err
}

// IsFree checks whether a block is free (unallocated).
// Returns an error if the block number is out of range.
func (bm *BlockBitmap) IsFree(blockNum int) (bool, error) {
	return StartNew[bool]("file-system.BlockBitmap.IsFree", false,
		func(op *Operation[bool], rf *ResultFactory[bool]) *OperationResult[bool] {
			op.AddProperty("blockNum", blockNum)
			if blockNum < 0 || blockNum >= bm.totalBlocks {
				return rf.Fail(false, fmt.Errorf("block number %d out of range [0, %d)", blockNum, bm.totalBlocks))
			}
			return rf.Generate(true, false, bm.bitmap[blockNum] == 0)
		}).GetResult()
}

// FreeCount returns the number of free (unallocated) blocks.
func (bm *BlockBitmap) FreeCount() int {
	result, _ := StartNew[int]("file-system.BlockBitmap.FreeCount", 0,
		func(op *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			count := 0
			for _, b := range bm.bitmap {
				if b == 0 {
					count++
				}
			}
			return rf.Generate(true, false, count)
		}).GetResult()
	return result
}

// TotalBlockCount returns the total number of blocks tracked by this bitmap.
func (bm *BlockBitmap) TotalBlockCount() int {
	result, _ := StartNew[int]("file-system.BlockBitmap.TotalBlockCount", 0,
		func(op *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			return rf.Generate(true, false, bm.totalBlocks)
		}).GetResult()
	return result
}
