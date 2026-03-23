package virtualmemory

import "testing"

func TestTLB(t *testing.T) {
	t.Run("empty lookup misses", func(t *testing.T) {
		tlb := NewTLB(4)
		_, _, ok := tlb.Lookup(5)
		if ok {
			t.Error("empty TLB should miss")
		}
		if tlb.Misses != 1 {
			t.Error("should increment misses")
		}
	})

	t.Run("insert and hit", func(t *testing.T) {
		tlb := NewTLB(4)
		pte := &PageTableEntry{FrameNumber: 42, Present: true}
		tlb.Insert(5, 42, pte)

		frame, retPTE, ok := tlb.Lookup(5)
		if !ok {
			t.Fatal("should hit")
		}
		if frame != 42 {
			t.Errorf("frame = %d, want 42", frame)
		}
		if retPTE.FrameNumber != 42 {
			t.Error("wrong PTE")
		}
		if tlb.Hits != 1 {
			t.Error("should increment hits")
		}
	})

	t.Run("different VPN misses", func(t *testing.T) {
		tlb := NewTLB(4)
		pte := &PageTableEntry{FrameNumber: 42, Present: true}
		tlb.Insert(5, 42, pte)

		_, _, ok := tlb.Lookup(10)
		if ok {
			t.Error("wrong VPN should miss")
		}
		if tlb.Misses != 1 {
			t.Error("should increment misses")
		}
	})

	t.Run("multiple entries", func(t *testing.T) {
		tlb := NewTLB(4)
		for i := 1; i <= 3; i++ {
			pte := &PageTableEntry{FrameNumber: i * 10, Present: true}
			tlb.Insert(i, i*10, pte)
		}
		for i := 1; i <= 3; i++ {
			frame, _, ok := tlb.Lookup(i)
			if !ok || frame != i*10 {
				t.Errorf("VPN %d: got frame %d, want %d", i, frame, i*10)
			}
		}
	})

	t.Run("update existing", func(t *testing.T) {
		tlb := NewTLB(4)
		pte1 := &PageTableEntry{FrameNumber: 10, Present: true}
		pte2 := &PageTableEntry{FrameNumber: 99, Present: true}
		tlb.Insert(5, 10, pte1)
		tlb.Insert(5, 99, pte2)

		frame, _, ok := tlb.Lookup(5)
		if !ok || frame != 99 {
			t.Error("should return updated entry")
		}
	})

	t.Run("eviction when full", func(t *testing.T) {
		tlb := NewTLB(3)
		for i := 1; i <= 3; i++ {
			pte := &PageTableEntry{FrameNumber: i * 10, Present: true}
			tlb.Insert(i, i*10, pte)
		}
		pte4 := &PageTableEntry{FrameNumber: 40, Present: true}
		tlb.Insert(4, 40, pte4)

		_, _, ok := tlb.Lookup(1)
		if ok {
			t.Error("VPN 1 should be evicted (LRU)")
		}
		_, _, ok = tlb.Lookup(4)
		if !ok {
			t.Error("VPN 4 should be present")
		}
	})

	t.Run("access prevents eviction", func(t *testing.T) {
		tlb := NewTLB(3)
		for i := 1; i <= 3; i++ {
			pte := &PageTableEntry{FrameNumber: i * 10, Present: true}
			tlb.Insert(i, i*10, pte)
		}
		tlb.Lookup(1) // make VPN 1 most recently used

		pte4 := &PageTableEntry{FrameNumber: 40, Present: true}
		tlb.Insert(4, 40, pte4) // should evict VPN 2

		_, _, ok := tlb.Lookup(1)
		if !ok {
			t.Error("VPN 1 should be protected by access")
		}
		_, _, ok = tlb.Lookup(2)
		if ok {
			t.Error("VPN 2 should be evicted")
		}
	})

	t.Run("flush clears all", func(t *testing.T) {
		tlb := NewTLB(4)
		pte := &PageTableEntry{FrameNumber: 10, Present: true}
		tlb.Insert(1, 10, pte)
		tlb.Insert(2, 20, pte)

		tlb.Flush()

		if tlb.Size() != 0 {
			t.Error("flush should clear all entries")
		}
		_, _, ok := tlb.Lookup(1)
		if ok {
			t.Error("flushed entries should miss")
		}
	})

	t.Run("invalidate single", func(t *testing.T) {
		tlb := NewTLB(4)
		pte := &PageTableEntry{FrameNumber: 10, Present: true}
		tlb.Insert(1, 10, pte)
		tlb.Insert(2, 20, pte)

		tlb.Invalidate(1)

		_, _, ok := tlb.Lookup(1)
		if ok {
			t.Error("invalidated entry should miss")
		}
		_, _, ok = tlb.Lookup(2)
		if !ok {
			t.Error("other entry should remain")
		}
	})

	t.Run("invalidate nonexistent is safe", func(t *testing.T) {
		tlb := NewTLB(4)
		tlb.Invalidate(999) // should not panic
	})

	t.Run("hit rate no lookups", func(t *testing.T) {
		tlb := NewTLB(4)
		if tlb.HitRate() != 0.0 {
			t.Error("hit rate should be 0 with no lookups")
		}
	})

	t.Run("hit rate 100%", func(t *testing.T) {
		tlb := NewTLB(4)
		pte := &PageTableEntry{FrameNumber: 10, Present: true}
		tlb.Insert(1, 10, pte)
		tlb.Lookup(1)
		tlb.Lookup(1)
		if tlb.HitRate() != 1.0 {
			t.Errorf("hit rate = %f, want 1.0", tlb.HitRate())
		}
	})

	t.Run("hit rate 50%", func(t *testing.T) {
		tlb := NewTLB(4)
		pte := &PageTableEntry{FrameNumber: 10, Present: true}
		tlb.Insert(1, 10, pte)
		tlb.Lookup(1)  // hit
		tlb.Lookup(99) // miss
		if tlb.HitRate() != 0.5 {
			t.Errorf("hit rate = %f, want 0.5", tlb.HitRate())
		}
	})

	t.Run("size and capacity", func(t *testing.T) {
		tlb := NewTLB(128)
		if tlb.Capacity() != 128 {
			t.Errorf("Capacity = %d, want 128", tlb.Capacity())
		}
		if tlb.Size() != 0 {
			t.Error("empty TLB should have size 0")
		}
		pte := &PageTableEntry{FrameNumber: 10, Present: true}
		tlb.Insert(1, 10, pte)
		if tlb.Size() != 1 {
			t.Errorf("Size = %d, want 1", tlb.Size())
		}
	})

	t.Run("counters persist across flush", func(t *testing.T) {
		tlb := NewTLB(4)
		pte := &PageTableEntry{FrameNumber: 10, Present: true}
		tlb.Insert(1, 10, pte)
		tlb.Lookup(1)  // hit
		tlb.Lookup(99) // miss

		tlb.Flush()

		if tlb.Hits != 1 || tlb.Misses != 1 {
			t.Error("counters should persist across flush")
		}
	})
}
