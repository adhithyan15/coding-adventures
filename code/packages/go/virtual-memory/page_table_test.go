package virtualmemory

import "testing"

func TestPageTable(t *testing.T) {
	t.Run("empty table", func(t *testing.T) {
		pt := NewPageTable()
		if pt.MappedCount() != 0 {
			t.Error("empty table should have 0 mappings")
		}
		_, ok := pt.Lookup(0)
		if ok {
			t.Error("lookup on empty table should return false")
		}
	})

	t.Run("map and lookup", func(t *testing.T) {
		pt := NewPageTable()
		pt.MapPage(5, 42, true, false, true)

		pte, ok := pt.Lookup(5)
		if !ok {
			t.Fatal("lookup should succeed")
		}
		if pte.FrameNumber != 42 {
			t.Errorf("FrameNumber = %d, want 42", pte.FrameNumber)
		}
		if !pte.Present {
			t.Error("Present should be true")
		}
	})

	t.Run("lookup unmapped returns false", func(t *testing.T) {
		pt := NewPageTable()
		pt.MapPage(0, 0, true, false, true)
		_, ok := pt.Lookup(999)
		if ok {
			t.Error("unmapped VPN should return false")
		}
	})

	t.Run("map with permissions", func(t *testing.T) {
		pt := NewPageTable()
		pt.MapPage(10, 20, false, true, false)

		pte, _ := pt.Lookup(10)
		if pte.Writable {
			t.Error("should not be writable")
		}
		if !pte.Executable {
			t.Error("should be executable")
		}
		if pte.UserAccessible {
			t.Error("should not be user-accessible")
		}
	})

	t.Run("unmap returns PTE", func(t *testing.T) {
		pt := NewPageTable()
		pt.MapPage(3, 7, true, false, true)

		pte, ok := pt.UnmapPage(3)
		if !ok {
			t.Fatal("unmap should succeed")
		}
		if pte.FrameNumber != 7 {
			t.Errorf("FrameNumber = %d, want 7", pte.FrameNumber)
		}
		_, found := pt.Lookup(3)
		if found {
			t.Error("page should be unmapped")
		}
	})

	t.Run("unmap nonexistent", func(t *testing.T) {
		pt := NewPageTable()
		_, ok := pt.UnmapPage(42)
		if ok {
			t.Error("unmapping nonexistent should return false")
		}
	})

	t.Run("mapped count", func(t *testing.T) {
		pt := NewPageTable()
		pt.MapPage(0, 0, true, false, true)
		pt.MapPage(1, 1, true, false, true)
		if pt.MappedCount() != 2 {
			t.Errorf("MappedCount = %d, want 2", pt.MappedCount())
		}
		pt.UnmapPage(0)
		if pt.MappedCount() != 1 {
			t.Errorf("MappedCount = %d, want 1", pt.MappedCount())
		}
	})

	t.Run("entries returns all mappings", func(t *testing.T) {
		pt := NewPageTable()
		pt.MapPage(10, 100, true, false, true)
		pt.MapPage(20, 200, true, false, true)

		entries := pt.Entries()
		if len(entries) != 2 {
			t.Errorf("len(entries) = %d, want 2", len(entries))
		}
		if entries[10].FrameNumber != 100 || entries[20].FrameNumber != 200 {
			t.Error("entries have wrong frame numbers")
		}
	})

	t.Run("overwrite mapping", func(t *testing.T) {
		pt := NewPageTable()
		pt.MapPage(5, 10, true, false, true)
		pt.MapPage(5, 99, true, false, true)

		pte, _ := pt.Lookup(5)
		if pte.FrameNumber != 99 {
			t.Errorf("FrameNumber = %d, want 99", pte.FrameNumber)
		}
	})
}
