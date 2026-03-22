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
	return &PhysicalFrameAllocator{
		bitmap:      make([]bool, totalFrames),
		totalFrames: totalFrames,
	}
}

// Allocate finds the first free frame, marks it as used, and returns its number.
// Returns -1 if all frames are in use (out of memory).
func (a *PhysicalFrameAllocator) Allocate() int {
	for i := 0; i < a.totalFrames; i++ {
		if !a.bitmap[i] {
			a.bitmap[i] = true
			return i
		}
	}
	return -1 // No free frames.
}

// Free marks a frame as free. Returns an error if the frame is out of range
// or already free (double-free).
func (a *PhysicalFrameAllocator) Free(frame int) error {
	if frame < 0 || frame >= a.totalFrames {
		return fmt.Errorf("%w: frame %d not in [0, %d)", ErrOutOfRange, frame, a.totalFrames)
	}
	if !a.bitmap[frame] {
		return fmt.Errorf("%w: frame %d is already free", ErrDoubleFree, frame)
	}
	a.bitmap[frame] = false
	return nil
}

// IsAllocated checks whether a frame is currently in use.
func (a *PhysicalFrameAllocator) IsAllocated(frame int) bool {
	if frame < 0 || frame >= a.totalFrames {
		return false
	}
	return a.bitmap[frame]
}

// FreeCount returns the number of unallocated frames.
func (a *PhysicalFrameAllocator) FreeCount() int {
	count := 0
	for _, used := range a.bitmap {
		if !used {
			count++
		}
	}
	return count
}

// AllocatedCount returns the number of allocated frames.
func (a *PhysicalFrameAllocator) AllocatedCount() int {
	return a.totalFrames - a.FreeCount()
}

// TotalFrames returns the total number of frames managed.
func (a *PhysicalFrameAllocator) TotalFrames() int {
	return a.totalFrames
}
