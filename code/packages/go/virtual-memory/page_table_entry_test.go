package virtualmemory

import "testing"

func TestConstants(t *testing.T) {
	t.Run("PageSize is 4KB", func(t *testing.T) {
		if PageSize != 4096 {
			t.Errorf("PageSize = %d, want 4096", PageSize)
		}
	})

	t.Run("PageOffsetBits is 12", func(t *testing.T) {
		if PageOffsetBits != 12 {
			t.Errorf("PageOffsetBits = %d, want 12", PageOffsetBits)
		}
	})

	t.Run("VPNBits is 20", func(t *testing.T) {
		if VPNBits != 20 {
			t.Errorf("VPNBits = %d, want 20", VPNBits)
		}
	})

	t.Run("PageSize matches offset bits", func(t *testing.T) {
		if PageSize != (1 << PageOffsetBits) {
			t.Errorf("PageSize %d != 1<<%d = %d", PageSize, PageOffsetBits, 1<<PageOffsetBits)
		}
	})

	t.Run("total address bits is 32", func(t *testing.T) {
		if VPNBits+PageOffsetBits != 32 {
			t.Errorf("VPNBits+PageOffsetBits = %d, want 32", VPNBits+PageOffsetBits)
		}
	})
}

func TestPageTableEntry(t *testing.T) {
	t.Run("default values", func(t *testing.T) {
		pte := NewPageTableEntry()
		if pte.FrameNumber != 0 {
			t.Error("FrameNumber should be 0")
		}
		if pte.Present {
			t.Error("Present should be false")
		}
		if pte.Dirty {
			t.Error("Dirty should be false")
		}
		if pte.Accessed {
			t.Error("Accessed should be false")
		}
		if !pte.Writable {
			t.Error("Writable should be true")
		}
		if pte.Executable {
			t.Error("Executable should be false")
		}
		if !pte.UserAccessible {
			t.Error("UserAccessible should be true")
		}
	})

	t.Run("custom values", func(t *testing.T) {
		pte := PageTableEntry{
			FrameNumber:    42,
			Present:        true,
			Dirty:          true,
			Accessed:       true,
			Writable:       false,
			Executable:     true,
			UserAccessible: false,
		}
		if pte.FrameNumber != 42 {
			t.Errorf("FrameNumber = %d, want 42", pte.FrameNumber)
		}
		if !pte.Present || !pte.Dirty || !pte.Accessed {
			t.Error("flags not set correctly")
		}
		if pte.Writable || !pte.Executable || pte.UserAccessible {
			t.Error("permission flags not set correctly")
		}
	})

	t.Run("flags are mutable", func(t *testing.T) {
		pte := NewPageTableEntry()
		pte.Accessed = true
		pte.Dirty = true
		if !pte.Accessed || !pte.Dirty {
			t.Error("flags should be mutable")
		}
	})

	t.Run("copy creates independent instance", func(t *testing.T) {
		original := PageTableEntry{FrameNumber: 5, Present: true, Writable: true}
		cp := original.Copy()

		if cp.FrameNumber != 5 || !cp.Present || !cp.Writable {
			t.Error("copy should have same values")
		}

		cp.FrameNumber = 99
		cp.Writable = false
		if original.FrameNumber != 5 || !original.Writable {
			t.Error("modifying copy should not affect original")
		}
	})

	t.Run("copy preserves all flags", func(t *testing.T) {
		original := PageTableEntry{
			FrameNumber: 42, Present: true, Dirty: true,
			Accessed: true, Writable: false, Executable: true,
			UserAccessible: false,
		}
		cp := original.Copy()
		if cp.FrameNumber != 42 || !cp.Present || !cp.Dirty || !cp.Accessed ||
			cp.Writable || !cp.Executable || cp.UserAccessible {
			t.Error("copy did not preserve all flags")
		}
	})
}
