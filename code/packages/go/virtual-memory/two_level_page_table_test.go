package virtualmemory

import "testing"

func TestSplitAddress(t *testing.T) {
	tests := []struct {
		name                     string
		addr                     int
		wantL1, wantL2, wantOff  int
	}{
		{"zero", 0x00000000, 0, 0, 0},
		{"simple", 0x00012ABC, 0, 0x12, 0xABC},
		{"high L1", 0x00400000, 1, 0, 0},
		{"max", 0xFFFFFFFF, 0x3FF, 0x3FF, 0xFFF},
		{"offset only", 0x00000ABC, 0, 0, 0xABC},
		{"L2 boundary", 0x00001000, 0, 1, 0},
		{"L1=2", 0x00800000, 2, 0, 0},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			l1, l2, off := splitAddress(tt.addr)
			if l1 != tt.wantL1 || l2 != tt.wantL2 || off != tt.wantOff {
				t.Errorf("splitAddress(0x%X) = (%d, %d, %d), want (%d, %d, %d)",
					tt.addr, l1, l2, off, tt.wantL1, tt.wantL2, tt.wantOff)
			}
		})
	}
}

func TestTwoLevelPageTable(t *testing.T) {
	t.Run("empty translate", func(t *testing.T) {
		pt := NewTwoLevelPageTable()
		if pt.Translate(0x1000) != nil {
			t.Error("empty table should return nil")
		}
	})

	t.Run("map and translate", func(t *testing.T) {
		pt := NewTwoLevelPageTable()
		pt.Map(0x00001000, 42, true, false, true)

		result := pt.Translate(0x00001ABC)
		if result == nil {
			t.Fatal("translate should succeed")
		}
		expected := (42 << 12) | 0xABC
		if result.PhysicalAddr != expected {
			t.Errorf("PhysicalAddr = 0x%X, want 0x%X", result.PhysicalAddr, expected)
		}
		if result.PTE.FrameNumber != 42 {
			t.Errorf("FrameNumber = %d, want 42", result.PTE.FrameNumber)
		}
	})

	t.Run("unmapped L1 region", func(t *testing.T) {
		pt := NewTwoLevelPageTable()
		pt.Map(0x00001000, 1, true, false, true)
		if pt.Translate(0x00400000) != nil {
			t.Error("unmapped L1 region should return nil")
		}
	})

	t.Run("unmapped page in mapped region", func(t *testing.T) {
		pt := NewTwoLevelPageTable()
		pt.Map(0x00001000, 1, true, false, true)
		if pt.Translate(0x00002000) != nil {
			t.Error("unmapped page should return nil")
		}
	})

	t.Run("multiple pages same region", func(t *testing.T) {
		pt := NewTwoLevelPageTable()
		pt.Map(0x00001000, 10, true, false, true)
		pt.Map(0x00002000, 20, true, false, true)

		r1 := pt.Translate(0x00001000)
		r2 := pt.Translate(0x00002000)
		if r1 == nil || r2 == nil {
			t.Fatal("both should translate")
		}
		if r1.PTE.FrameNumber != 10 || r2.PTE.FrameNumber != 20 {
			t.Error("wrong frame numbers")
		}
	})

	t.Run("different L1 regions", func(t *testing.T) {
		pt := NewTwoLevelPageTable()
		pt.Map(0x00001000, 10, true, false, true)
		pt.Map(0x00401000, 20, true, false, true)

		r1 := pt.Translate(0x00001000)
		r2 := pt.Translate(0x00401000)
		if r1 == nil || r2 == nil {
			t.Fatal("both should translate")
		}
		if r1.PTE.FrameNumber != 10 || r2.PTE.FrameNumber != 20 {
			t.Error("wrong frame numbers")
		}
	})

	t.Run("unmap", func(t *testing.T) {
		pt := NewTwoLevelPageTable()
		pt.Map(0x00001000, 5, true, false, true)

		pte, ok := pt.Unmap(0x00001000)
		if !ok {
			t.Fatal("unmap should succeed")
		}
		if pte.FrameNumber != 5 {
			t.Error("wrong frame")
		}
		if pt.Translate(0x00001000) != nil {
			t.Error("should be unmapped")
		}
	})

	t.Run("unmap unmapped", func(t *testing.T) {
		pt := NewTwoLevelPageTable()
		_, ok := pt.Unmap(0x00001000)
		if ok {
			t.Error("unmap of unmapped should return false")
		}
	})

	t.Run("permissions preserved", func(t *testing.T) {
		pt := NewTwoLevelPageTable()
		pt.Map(0x00005000, 1, false, true, false)

		result := pt.Translate(0x00005000)
		if result == nil {
			t.Fatal("should translate")
		}
		if result.PTE.Writable || !result.PTE.Executable || result.PTE.UserAccessible {
			t.Error("permissions not preserved")
		}
	})

	t.Run("lookup VPN", func(t *testing.T) {
		pt := NewTwoLevelPageTable()
		pt.Map(0x00005000, 42, true, false, true)

		pte, ok := pt.LookupVPN(5)
		if !ok || pte.FrameNumber != 42 {
			t.Error("LookupVPN failed")
		}
	})

	t.Run("lookup VPN unmapped", func(t *testing.T) {
		pt := NewTwoLevelPageTable()
		_, ok := pt.LookupVPN(999)
		if ok {
			t.Error("unmapped VPN should return false")
		}
	})

	t.Run("map VPN", func(t *testing.T) {
		pt := NewTwoLevelPageTable()
		pt.MapVPN(5, 42, true, false, true)

		result := pt.Translate(0x00005000)
		if result == nil || result.PTE.FrameNumber != 42 {
			t.Error("MapVPN should create valid mapping")
		}
	})

	t.Run("offset preserved", func(t *testing.T) {
		pt := NewTwoLevelPageTable()
		pt.Map(0x00003000, 7, true, false, true)

		result := pt.Translate(0x00003123)
		if result == nil {
			t.Fatal("should translate")
		}
		if result.PhysicalAddr != 0x7123 {
			t.Errorf("PhysicalAddr = 0x%X, want 0x7123", result.PhysicalAddr)
		}
	})

	t.Run("directory access", func(t *testing.T) {
		pt := NewTwoLevelPageTable()
		dir := pt.Directory()
		for _, entry := range dir {
			if entry != nil {
				t.Error("empty directory should have nil entries")
			}
		}
		pt.Map(0x00001000, 1, true, false, true)
		dir = pt.Directory()
		if dir[0] == nil {
			t.Error("directory[0] should not be nil after mapping")
		}
	})
}
