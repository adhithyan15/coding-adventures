package filesystem

import "testing"

func TestInodeTableAllocate(t *testing.T) {
	table := NewInodeTable(4)
	inode := table.Allocate(FileTypeRegular)
	if inode == nil {
		t.Fatal("expected non-nil inode")
	}
	if inode.InodeNumber != 0 {
		t.Errorf("expected 0, got %d", inode.InodeNumber)
	}
	if inode.Type != FileTypeRegular {
		t.Errorf("expected REGULAR, got %d", inode.Type)
	}
}

func TestInodeTableAllocateSequential(t *testing.T) {
	table := NewInodeTable(4)
	i0 := table.Allocate(FileTypeDirectory)
	i1 := table.Allocate(FileTypeRegular)
	i2 := table.Allocate(FileTypeRegular)
	if i0.InodeNumber != 0 || i1.InodeNumber != 1 || i2.InodeNumber != 2 {
		t.Error("expected sequential allocation")
	}
}

func TestInodeTableAllocateExhaustion(t *testing.T) {
	table := NewInodeTable(2)
	table.Allocate(FileTypeRegular)
	table.Allocate(FileTypeRegular)
	if table.Allocate(FileTypeRegular) != nil {
		t.Error("expected nil when exhausted")
	}
}

func TestInodeTableGet(t *testing.T) {
	table := NewInodeTable(4)
	allocated := table.Allocate(FileTypeRegular)
	retrieved, err := table.Get(allocated.InodeNumber)
	if err != nil {
		t.Fatal(err)
	}
	if retrieved != allocated {
		t.Error("expected same inode")
	}
}

func TestInodeTableGetFreeSlot(t *testing.T) {
	table := NewInodeTable(4)
	inode, err := table.Get(0)
	if err != nil {
		t.Fatal(err)
	}
	if inode != nil {
		t.Error("expected nil for free slot")
	}
}

func TestInodeTableFree(t *testing.T) {
	table := NewInodeTable(4)
	inode := table.Allocate(FileTypeRegular)
	table.Free(inode.InodeNumber)
	retrieved, _ := table.Get(inode.InodeNumber)
	if retrieved != nil {
		t.Error("expected nil after free")
	}
}

func TestInodeTableFreeReuse(t *testing.T) {
	table := NewInodeTable(4)
	table.Allocate(FileTypeRegular) // 0
	table.Allocate(FileTypeRegular) // 1
	table.Free(0)
	reused := table.Allocate(FileTypeRegular)
	if reused == nil || reused.InodeNumber != 0 {
		t.Error("expected reuse of inode 0")
	}
}

func TestInodeTableFreeCount(t *testing.T) {
	table := NewInodeTable(4)
	if table.FreeCount() != 4 {
		t.Error("expected 4")
	}
	table.Allocate(FileTypeRegular)
	if table.FreeCount() != 3 {
		t.Error("expected 3")
	}
}

func TestInodeTableOutOfRange(t *testing.T) {
	table := NewInodeTable(4)
	err := table.Free(4)
	if err == nil {
		t.Error("expected error for out-of-range free")
	}
	err = table.Free(-1)
	if err == nil {
		t.Error("expected error for negative free")
	}
	_, err = table.Get(4)
	if err == nil {
		t.Error("expected error for out-of-range get")
	}
	_, err = table.Get(-1)
	if err == nil {
		t.Error("expected error for negative get")
	}
}

func TestInodeTableMaxInodeCount(t *testing.T) {
	table := NewInodeTable(42)
	if table.MaxInodeCount() != 42 {
		t.Errorf("expected 42, got %d", table.MaxInodeCount())
	}
}
