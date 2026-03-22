package filesystem

import "testing"

func TestSuperblockDefaults(t *testing.T) {
	sb := NewSuperblock(MaxBlocks, MaxInodes)
	if sb.Magic != SuperblockMagic {
		t.Errorf("expected magic %x, got %x", SuperblockMagic, sb.Magic)
	}
	if sb.BlockSize != BlockSize {
		t.Errorf("expected block size %d, got %d", BlockSize, sb.BlockSize)
	}
	if sb.TotalBlocks != MaxBlocks {
		t.Errorf("expected total blocks %d, got %d", MaxBlocks, sb.TotalBlocks)
	}
	if sb.TotalInodes != MaxInodes {
		t.Errorf("expected total inodes %d, got %d", MaxInodes, sb.TotalInodes)
	}
	if sb.FreeBlocks != MaxBlocks {
		t.Errorf("expected free blocks %d, got %d", MaxBlocks, sb.FreeBlocks)
	}
	if sb.FreeInodes != MaxInodes {
		t.Errorf("expected free inodes %d, got %d", MaxInodes, sb.FreeInodes)
	}
}

func TestSuperblockIsValid(t *testing.T) {
	sb := NewSuperblock(MaxBlocks, MaxInodes)
	if !sb.IsValid() {
		t.Error("expected superblock to be valid")
	}
	sb.Magic = 0xDEADBEEF
	if sb.IsValid() {
		t.Error("expected superblock with wrong magic to be invalid")
	}
}

func TestSuperblockCustomValues(t *testing.T) {
	sb := NewSuperblock(1024, 256)
	if sb.TotalBlocks != 1024 {
		t.Errorf("expected 1024, got %d", sb.TotalBlocks)
	}
	if sb.TotalInodes != 256 {
		t.Errorf("expected 256, got %d", sb.TotalInodes)
	}
}
