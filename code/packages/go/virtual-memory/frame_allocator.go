package virtualmemory

import (
	"errors"
	"fmt"
)

// =============================================================================
// PhysicalFrameAllocator — bitmap-based frame manager
// =============================================================================
//
// Physical memory (RAM) is divided into fixed-size chunks called FRAMES.
// Each frame is 4 KB (same size as a virtual page).
//
// For a machine with 16 MB of RAM:
//     16 MB / 4 KB = 4096 frames
//     Frame 0:    bytes 0x00000000 - 0x00000FFF
//     Frame 1:    bytes 0x00001000 - 0x00001FFF
//     ...
//
// The allocator tracks which frames are free/used via a BITMAP:
//     bitmap[i] = false  ->  frame i is free
//     bitmap[i] = true   ->  frame i is allocated
//
// Allocation scans linearly for the first free frame — O(n) in the worst
// case. Real OS kernels use buddy allocators for O(1) allocation.

// ErrDoubleFree is returned when freeing a frame that is already free.
var ErrDoubleFree = errors.New("double-free detected")

// ErrOutOfRange is returned when a frame number is out of range.
var ErrOutOfRange = errors.New("frame number out of range")

// PhysicalFrameAllocator manages physical frames using a bitmap.
type PhysicalFrameAllocator struct {
	bitmap      []bool // true = allocated, false = free
	totalFrames int
}

// NewPhysicalFrameAllocator creates an allocator with all frames free.
func NewPhysicalFrameAllocator(totalFrames int) *PhysicalFrameAllocator {
	result, _ := StartNew[*PhysicalFrameAllocator]("virtual-memory.NewPhysicalFrameAllocator", nil,
		func(op *Operation[*PhysicalFrameAllocator], rf *ResultFactory[*PhysicalFrameAllocator]) *OperationResult[*PhysicalFrameAllocator] {
			op.AddProperty("totalFrames", totalFrames)
			return rf.Generate(true, false, &PhysicalFrameAllocator{
				bitmap:      make([]bool, totalFrames),
				totalFrames: totalFrames,
			})
		}).GetResult()
	return result
}

// Allocate finds the first free frame, marks it as used, and returns its number.
// Returns -1 if all frames are in use (out of memory).
func (a *PhysicalFrameAllocator) Allocate() int {
	result, _ := StartNew[int]("virtual-memory.Allocate", -1,
		func(op *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			for i := 0; i < a.totalFrames; i++ {
				if !a.bitmap[i] {
					a.bitmap[i] = true
					return rf.Generate(true, false, i)
				}
			}
			return rf.Generate(true, false, -1) // No free frames.
		}).GetResult()
	return result
}

// Free marks a frame as free. Returns an error if the frame is out of range
// or already free (double-free).
func (a *PhysicalFrameAllocator) Free(frame int) error {
	_, err := StartNew[struct{}]("virtual-memory.Free", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			op.AddProperty("frame", frame)
			if frame < 0 || frame >= a.totalFrames {
				return rf.Fail(struct{}{}, fmt.Errorf("%w: frame %d not in [0, %d)", ErrOutOfRange, frame, a.totalFrames))
			}
			if !a.bitmap[frame] {
				return rf.Fail(struct{}{}, fmt.Errorf("%w: frame %d is already free", ErrDoubleFree, frame))
			}
			a.bitmap[frame] = false
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
	return err
}

// IsAllocated checks whether a frame is currently in use.
func (a *PhysicalFrameAllocator) IsAllocated(frame int) bool {
	result, _ := StartNew[bool]("virtual-memory.IsAllocated", false,
		func(op *Operation[bool], rf *ResultFactory[bool]) *OperationResult[bool] {
			op.AddProperty("frame", frame)
			if frame < 0 || frame >= a.totalFrames {
				return rf.Generate(true, false, false)
			}
			return rf.Generate(true, false, a.bitmap[frame])
		}).GetResult()
	return result
}

// FreeCount returns the number of unallocated frames.
func (a *PhysicalFrameAllocator) FreeCount() int {
	result, _ := StartNew[int]("virtual-memory.FreeCount", 0,
		func(op *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			count := 0
			for _, used := range a.bitmap {
				if !used {
					count++
				}
			}
			return rf.Generate(true, false, count)
		}).GetResult()
	return result
}

// AllocatedCount returns the number of allocated frames.
func (a *PhysicalFrameAllocator) AllocatedCount() int {
	result, _ := StartNew[int]("virtual-memory.AllocatedCount", 0,
		func(op *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			return rf.Generate(true, false, a.totalFrames-a.FreeCount())
		}).GetResult()
	return result
}

// TotalFrames returns the total number of frames managed.
func (a *PhysicalFrameAllocator) TotalFrames() int {
	result, _ := StartNew[int]("virtual-memory.TotalFrames", 0,
		func(op *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			return rf.Generate(true, false, a.totalFrames)
		}).GetResult()
	return result
}
