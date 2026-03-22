package filesystem

import "testing"

func TestNewInodeDefaults(t *testing.T) {
	inode := NewInode(0, FileTypeRegular)
	if inode.InodeNumber != 0 {
		t.Errorf("expected inode number 0, got %d", inode.InodeNumber)
	}
	if inode.Type != FileTypeRegular {
		t.Errorf("expected REGULAR, got %d", inode.Type)
	}
	if inode.Size != 0 {
		t.Errorf("expected size 0, got %d", inode.Size)
	}
	if inode.Permissions != 0o755 {
		t.Errorf("expected 0755, got %o", inode.Permissions)
	}
	if inode.LinkCount != 1 {
		t.Errorf("expected link count 1, got %d", inode.LinkCount)
	}
	if inode.IndirectBlock != -1 {
		t.Errorf("expected indirect block -1, got %d", inode.IndirectBlock)
	}
}

func TestInodeDirectBlocksInitialized(t *testing.T) {
	inode := NewInode(5, FileTypeRegular)
	for i, b := range inode.DirectBlks {
		if b != -1 {
			t.Errorf("direct_blocks[%d] = %d, expected -1", i, b)
		}
	}
}

func TestInodeDirectBlocksIndependent(t *testing.T) {
	inode1 := NewInode(0, FileTypeRegular)
	inode2 := NewInode(1, FileTypeRegular)
	inode1.DirectBlks[0] = 42
	if inode2.DirectBlks[0] != -1 {
		t.Error("modifying inode1 should not affect inode2")
	}
}

func TestInodeDirectoryType(t *testing.T) {
	inode := NewInode(0, FileTypeDirectory)
	if inode.Type != FileTypeDirectory {
		t.Errorf("expected DIRECTORY, got %d", inode.Type)
	}
}

func TestFileTypeValues(t *testing.T) {
	if FileTypeRegular != 1 {
		t.Error("REGULAR should be 1")
	}
	if FileTypeDirectory != 2 {
		t.Error("DIRECTORY should be 2")
	}
	if FileTypeSymlink != 3 {
		t.Error("SYMLINK should be 3")
	}
	if FileTypePipe != 6 {
		t.Error("PIPE should be 6")
	}
	if FileTypeSocket != 7 {
		t.Error("SOCKET should be 7")
	}
}
