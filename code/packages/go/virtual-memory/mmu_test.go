package virtualmemory

import (
	"errors"
	"testing"
)

func TestMMUAddressSpace(t *testing.T) {
	t.Run("create", func(t *testing.T) {
		mmu := NewMMU(16, nil)
		if err := mmu.CreateAddressSpace(1); err != nil {
			t.Fatal(err)
		}
	})

	t.Run("create duplicate", func(t *testing.T) {
		mmu := NewMMU(16, nil)
		mmu.CreateAddressSpace(1)
		err := mmu.CreateAddressSpace(1)
		if !errors.Is(err, ErrAddressSpaceExists) {
			t.Errorf("want ErrAddressSpaceExists, got %v", err)
		}
	})

	t.Run("destroy", func(t *testing.T) {
		mmu := NewMMU(16, nil)
		mmu.CreateAddressSpace(1)
		mmu.MapPage(1, 0x1000, true, false)
		mmu.MapPage(1, 0x2000, true, false)

		if mmu.FrameAllocator().AllocatedCount() != 2 {
			t.Error("should have 2 allocated frames")
		}

		mmu.DestroyAddressSpace(1)

		if mmu.FrameAllocator().AllocatedCount() != 0 {
			t.Error("all frames should be freed")
		}
	})

	t.Run("destroy nonexistent", func(t *testing.T) {
		mmu := NewMMU(16, nil)
		err := mmu.DestroyAddressSpace(99)
		if !errors.Is(err, ErrNoAddressSpace) {
			t.Errorf("want ErrNoAddressSpace, got %v", err)
		}
	})
}

func TestMMUMapPage(t *testing.T) {
	t.Run("allocates frame", func(t *testing.T) {
		mmu := NewMMU(16, nil)
		mmu.CreateAddressSpace(1)

		frame, err := mmu.MapPage(1, 0x1000, true, false)
		if err != nil {
			t.Fatal(err)
		}
		if frame < 0 {
			t.Error("frame should be >= 0")
		}
		if !mmu.FrameAllocator().IsAllocated(frame) {
			t.Error("frame should be allocated")
		}
	})

	t.Run("no address space", func(t *testing.T) {
		mmu := NewMMU(16, nil)
		_, err := mmu.MapPage(99, 0x1000, true, false)
		if !errors.Is(err, ErrNoAddressSpace) {
			t.Errorf("want ErrNoAddressSpace, got %v", err)
		}
	})

	t.Run("multiple pages get different frames", func(t *testing.T) {
		mmu := NewMMU(16, nil)
		mmu.CreateAddressSpace(1)

		f1, _ := mmu.MapPage(1, 0x1000, true, false)
		f2, _ := mmu.MapPage(1, 0x2000, true, false)
		f3, _ := mmu.MapPage(1, 0x3000, true, false)

		if f1 == f2 || f2 == f3 || f1 == f3 {
			t.Error("each page should get a different frame")
		}
	})
}

func TestMMUTranslate(t *testing.T) {
	t.Run("mapped page", func(t *testing.T) {
		mmu := NewMMU(16, nil)
		mmu.CreateAddressSpace(1)

		frame, _ := mmu.MapPage(1, 0x5000, true, false)

		physical, err := mmu.Translate(1, 0x5ABC, false)
		if err != nil {
			t.Fatal(err)
		}
		expected := (frame << PageOffsetBits) | 0xABC
		if physical != expected {
			t.Errorf("physical = 0x%X, want 0x%X", physical, expected)
		}
	})

	t.Run("no address space", func(t *testing.T) {
		mmu := NewMMU(16, nil)
		_, err := mmu.Translate(99, 0x1000, false)
		if !errors.Is(err, ErrNoAddressSpace) {
			t.Errorf("want ErrNoAddressSpace, got %v", err)
		}
	})

	t.Run("page fault allocates", func(t *testing.T) {
		mmu := NewMMU(16, nil)
		mmu.CreateAddressSpace(1)

		physical, err := mmu.Translate(1, 0x3000, false)
		if err != nil {
			t.Fatal(err)
		}
		if physical < 0 {
			t.Error("should return valid physical address")
		}
		if mmu.FrameAllocator().AllocatedCount() < 1 {
			t.Error("should allocate at least one frame")
		}
	})

	t.Run("preserves offset", func(t *testing.T) {
		mmu := NewMMU(16, nil)
		mmu.CreateAddressSpace(1)
		frame, _ := mmu.MapPage(1, 0x1000, true, false)

		p1, _ := mmu.Translate(1, 0x1000, false)
		p2, _ := mmu.Translate(1, 0x1FFF, false)
		p3, _ := mmu.Translate(1, 0x1123, false)

		if p1 != (frame<<PageOffsetBits)|0x000 {
			t.Error("offset 0x000 wrong")
		}
		if p2 != (frame<<PageOffsetBits)|0xFFF {
			t.Error("offset 0xFFF wrong")
		}
		if p3 != (frame<<PageOffsetBits)|0x123 {
			t.Error("offset 0x123 wrong")
		}
	})
}

func TestMMUTLBIntegration(t *testing.T) {
	t.Run("first translate is miss", func(t *testing.T) {
		mmu := NewMMU(16, nil)
		mmu.CreateAddressSpace(1)
		mmu.MapPage(1, 0x1000, true, false)

		mmu.Translate(1, 0x1000, false)

		if mmu.TLB().Misses < 1 {
			t.Error("first translate should be a miss")
		}
	})

	t.Run("second translate is hit", func(t *testing.T) {
		mmu := NewMMU(16, nil)
		mmu.CreateAddressSpace(1)
		mmu.MapPage(1, 0x1000, true, false)

		mmu.Translate(1, 0x1000, false)
		missesBefore := mmu.TLB().Misses

		mmu.Translate(1, 0x1000, false)

		if mmu.TLB().Misses != missesBefore {
			t.Error("second translate should be a hit")
		}
		if mmu.TLB().Hits < 1 {
			t.Error("should increment hits")
		}
	})

	t.Run("context switch flushes TLB", func(t *testing.T) {
		mmu := NewMMU(16, nil)
		mmu.CreateAddressSpace(1)
		mmu.CreateAddressSpace(2)
		mmu.MapPage(1, 0x1000, true, false)

		mmu.Translate(1, 0x1000, false)
		mmu.ContextSwitch(2)

		if mmu.TLB().Size() != 0 {
			t.Error("context switch should flush TLB")
		}
	})

	t.Run("context switch nonexistent PID", func(t *testing.T) {
		mmu := NewMMU(16, nil)
		err := mmu.ContextSwitch(99)
		if !errors.Is(err, ErrNoAddressSpace) {
			t.Errorf("want ErrNoAddressSpace, got %v", err)
		}
	})

	t.Run("TLB miss after flush", func(t *testing.T) {
		mmu := NewMMU(16, nil)
		mmu.CreateAddressSpace(1)
		mmu.CreateAddressSpace(2)
		mmu.MapPage(1, 0x1000, true, false)

		mmu.Translate(1, 0x1000, false) // miss
		mmu.Translate(1, 0x1000, false) // hit

		missesBefore := mmu.TLB().Misses
		mmu.ContextSwitch(2)
		mmu.ContextSwitch(1) // flush again

		mmu.Translate(1, 0x1000, false) // miss again
		if mmu.TLB().Misses != missesBefore+1 {
			t.Error("should miss after flush")
		}
	})
}

func TestMMUClone(t *testing.T) {
	t.Run("creates new address space", func(t *testing.T) {
		mmu := NewMMU(16, nil)
		mmu.CreateAddressSpace(1)
		mmu.MapPage(1, 0x1000, true, false)

		if err := mmu.CloneAddressSpace(1, 2); err != nil {
			t.Fatal(err)
		}

		physical, err := mmu.Translate(2, 0x1000, false)
		if err != nil {
			t.Fatal(err)
		}
		if physical < 0 {
			t.Error("child should be able to translate")
		}
	})

	t.Run("shares frames", func(t *testing.T) {
		mmu := NewMMU(16, nil)
		mmu.CreateAddressSpace(1)
		mmu.MapPage(1, 0x1000, true, false)

		mmu.CloneAddressSpace(1, 2)

		p1, _ := mmu.Translate(1, 0x1000, false)
		p2, _ := mmu.Translate(2, 0x1000, false)

		if p1 != p2 {
			t.Errorf("should share frame: p1=0x%X, p2=0x%X", p1, p2)
		}
	})

	t.Run("nonexistent source", func(t *testing.T) {
		mmu := NewMMU(16, nil)
		err := mmu.CloneAddressSpace(99, 2)
		if !errors.Is(err, ErrNoAddressSpace) {
			t.Errorf("want ErrNoAddressSpace, got %v", err)
		}
	})

	t.Run("existing dest", func(t *testing.T) {
		mmu := NewMMU(16, nil)
		mmu.CreateAddressSpace(1)
		mmu.CreateAddressSpace(2)
		err := mmu.CloneAddressSpace(1, 2)
		if !errors.Is(err, ErrAddressSpaceExists) {
			t.Errorf("want ErrAddressSpaceExists, got %v", err)
		}
	})

	t.Run("COW write creates private copy", func(t *testing.T) {
		mmu := NewMMU(16, nil)
		mmu.CreateAddressSpace(1)
		mmu.MapPage(1, 0x1000, true, false)

		mmu.CloneAddressSpace(1, 2)

		p1Before, _ := mmu.Translate(1, 0x1000, false)
		p2Before, _ := mmu.Translate(2, 0x1000, false)
		if p1Before != p2Before {
			t.Error("should share frame before write")
		}

		p2After, _ := mmu.Translate(2, 0x1000, true) // write triggers COW
		p1After, _ := mmu.Translate(1, 0x1000, false)

		if p2After == p1After {
			t.Errorf("after COW, should have different frames: p1=0x%X, p2=0x%X", p1After, p2After)
		}
	})
}

func TestMMUPageFault(t *testing.T) {
	t.Run("allocates frame", func(t *testing.T) {
		mmu := NewMMU(16, nil)
		mmu.CreateAddressSpace(1)

		physical, err := mmu.HandlePageFault(1, 0x5000)
		if err != nil {
			t.Fatal(err)
		}

		translated, err := mmu.Translate(1, 0x5000, false)
		if err != nil {
			t.Fatal(err)
		}
		if translated != physical {
			t.Error("should translate to same address after fault")
		}
	})
}

func TestMMUEviction(t *testing.T) {
	t.Run("FIFO eviction", func(t *testing.T) {
		mmu := NewMMU(3, NewFIFOPolicy())
		mmu.CreateAddressSpace(1)

		mmu.MapPage(1, 0x1000, true, false)
		mmu.MapPage(1, 0x2000, true, false)
		mmu.MapPage(1, 0x3000, true, false)

		mmu.MapPage(1, 0x4000, true, false) // triggers eviction

		if mmu.FrameAllocator().AllocatedCount() != 3 {
			t.Errorf("should have 3 allocated, got %d", mmu.FrameAllocator().AllocatedCount())
		}
	})

	t.Run("LRU eviction", func(t *testing.T) {
		mmu := NewMMU(3, NewLRUPolicy())
		mmu.CreateAddressSpace(1)

		mmu.MapPage(1, 0x1000, true, false)
		mmu.MapPage(1, 0x2000, true, false)
		mmu.MapPage(1, 0x3000, true, false)

		mmu.Translate(1, 0x1000, false)
		mmu.Translate(1, 0x3000, false)

		mmu.MapPage(1, 0x4000, true, false) // evicts LRU

		if mmu.FrameAllocator().AllocatedCount() != 3 {
			t.Errorf("should have 3 allocated, got %d", mmu.FrameAllocator().AllocatedCount())
		}
	})

	t.Run("Clock eviction", func(t *testing.T) {
		mmu := NewMMU(3, NewClockPolicy())
		mmu.CreateAddressSpace(1)

		mmu.MapPage(1, 0x1000, true, false)
		mmu.MapPage(1, 0x2000, true, false)
		mmu.MapPage(1, 0x3000, true, false)

		mmu.MapPage(1, 0x4000, true, false) // triggers Clock eviction

		if mmu.FrameAllocator().AllocatedCount() != 3 {
			t.Errorf("should have 3 allocated, got %d", mmu.FrameAllocator().AllocatedCount())
		}
	})
}

func TestMMUProperties(t *testing.T) {
	t.Run("TLB accessor", func(t *testing.T) {
		mmu := NewMMU(16, nil)
		if mmu.TLB().Capacity() != 64 {
			t.Error("TLB should have capacity 64")
		}
	})

	t.Run("frame allocator accessor", func(t *testing.T) {
		mmu := NewMMU(16, nil)
		if mmu.FrameAllocator().TotalFrames() != 16 {
			t.Error("should have 16 frames")
		}
	})

	t.Run("active PID starts -1", func(t *testing.T) {
		mmu := NewMMU(16, nil)
		if mmu.ActivePID() != -1 {
			t.Errorf("ActivePID = %d, want -1", mmu.ActivePID())
		}
	})

	t.Run("active PID after context switch", func(t *testing.T) {
		mmu := NewMMU(16, nil)
		mmu.CreateAddressSpace(1)
		mmu.ContextSwitch(1)
		if mmu.ActivePID() != 1 {
			t.Errorf("ActivePID = %d, want 1", mmu.ActivePID())
		}
	})
}

func TestMMUWrite(t *testing.T) {
	t.Run("write sets dirty bit", func(t *testing.T) {
		mmu := NewMMU(16, nil)
		mmu.CreateAddressSpace(1)
		mmu.MapPage(1, 0x1000, true, false)

		mmu.Translate(1, 0x1000, true)

		vpn := 0x1000 >> PageOffsetBits
		_, pte, ok := mmu.TLB().Lookup(vpn)
		if ok && pte != nil {
			if !pte.Dirty {
				t.Error("dirty bit should be set after write")
			}
		}
	})
}
