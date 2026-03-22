package virtualmemory

import (
	"errors"
	"testing"
)

func TestFrameAllocator(t *testing.T) {
	t.Run("all free initially", func(t *testing.T) {
		a := NewPhysicalFrameAllocator(16)
		if a.FreeCount() != 16 {
			t.Errorf("FreeCount = %d, want 16", a.FreeCount())
		}
		if a.AllocatedCount() != 0 {
			t.Errorf("AllocatedCount = %d, want 0", a.AllocatedCount())
		}
	})

	t.Run("total frames", func(t *testing.T) {
		a := NewPhysicalFrameAllocator(256)
		if a.TotalFrames() != 256 {
			t.Errorf("TotalFrames = %d, want 256", a.TotalFrames())
		}
	})

	t.Run("allocate sequential", func(t *testing.T) {
		a := NewPhysicalFrameAllocator(8)
		for i := 0; i < 3; i++ {
			frame := a.Allocate()
			if frame != i {
				t.Errorf("Allocate() = %d, want %d", frame, i)
			}
		}
	})

	t.Run("allocate updates counts", func(t *testing.T) {
		a := NewPhysicalFrameAllocator(4)
		a.Allocate()
		if a.FreeCount() != 3 || a.AllocatedCount() != 1 {
			t.Error("counts not updated correctly")
		}
	})

	t.Run("allocate all", func(t *testing.T) {
		a := NewPhysicalFrameAllocator(4)
		for i := 0; i < 4; i++ {
			a.Allocate()
		}
		if a.FreeCount() != 0 {
			t.Error("all frames should be allocated")
		}
	})

	t.Run("allocate when full", func(t *testing.T) {
		a := NewPhysicalFrameAllocator(2)
		a.Allocate()
		a.Allocate()
		if a.Allocate() != -1 {
			t.Error("should return -1 when full")
		}
	})

	t.Run("free and reallocate", func(t *testing.T) {
		a := NewPhysicalFrameAllocator(4)
		for i := 0; i < 4; i++ {
			a.Allocate()
		}
		a.Free(1)
		if a.FreeCount() != 1 {
			t.Error("should have 1 free")
		}
		frame := a.Allocate()
		if frame != 1 {
			t.Errorf("reallocated frame = %d, want 1", frame)
		}
	})

	t.Run("is allocated", func(t *testing.T) {
		a := NewPhysicalFrameAllocator(4)
		if a.IsAllocated(0) {
			t.Error("should be free")
		}
		a.Allocate()
		if !a.IsAllocated(0) {
			t.Error("should be allocated")
		}
		a.Free(0)
		if a.IsAllocated(0) {
			t.Error("should be free after free()")
		}
	})

	t.Run("is allocated out of range", func(t *testing.T) {
		a := NewPhysicalFrameAllocator(4)
		if a.IsAllocated(100) || a.IsAllocated(-1) {
			t.Error("out of range should return false")
		}
	})

	t.Run("free out of range", func(t *testing.T) {
		a := NewPhysicalFrameAllocator(4)
		err := a.Free(10)
		if err == nil {
			t.Error("should return error")
		}
		if !errors.Is(err, ErrOutOfRange) {
			t.Errorf("want ErrOutOfRange, got %v", err)
		}
	})

	t.Run("double free", func(t *testing.T) {
		a := NewPhysicalFrameAllocator(4)
		a.Allocate()
		a.Free(0)
		err := a.Free(0)
		if err == nil {
			t.Error("should return error")
		}
		if !errors.Is(err, ErrDoubleFree) {
			t.Errorf("want ErrDoubleFree, got %v", err)
		}
	})
}
