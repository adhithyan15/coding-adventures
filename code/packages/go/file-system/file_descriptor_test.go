package filesystem

import "testing"

func TestOpenFileDefaults(t *testing.T) {
	of := &OpenFile{InodeNumber: 5, Offset: 0, Flags: O_RDONLY, RefCount: 1}
	if of.InodeNumber != 5 {
		t.Errorf("expected 5, got %d", of.InodeNumber)
	}
	if of.Offset != 0 {
		t.Error("expected offset 0")
	}
	if of.Flags != O_RDONLY {
		t.Error("expected RDONLY")
	}
	if of.RefCount != 1 {
		t.Error("expected ref count 1")
	}
}

func TestOpenFileTableOpenStartsAt3(t *testing.T) {
	table := NewOpenFileTable()
	fd := table.Open(5, O_RDONLY)
	if fd != 3 {
		t.Errorf("expected fd 3, got %d", fd)
	}
}

func TestOpenFileTableOpenSequential(t *testing.T) {
	table := NewOpenFileTable()
	if table.Open(5, O_RDONLY) != 3 {
		t.Error("expected 3")
	}
	if table.Open(6, O_RDONLY) != 4 {
		t.Error("expected 4")
	}
	if table.Open(7, O_RDONLY) != 5 {
		t.Error("expected 5")
	}
}

func TestOpenFileTableGet(t *testing.T) {
	table := NewOpenFileTable()
	fd := table.Open(5, O_RDWR)
	entry := table.Get(fd)
	if entry == nil {
		t.Fatal("expected non-nil")
	}
	if entry.InodeNumber != 5 || entry.Flags != O_RDWR {
		t.Error("unexpected entry values")
	}
}

func TestOpenFileTableGetInvalid(t *testing.T) {
	table := NewOpenFileTable()
	if table.Get(99) != nil {
		t.Error("expected nil for invalid fd")
	}
}

func TestOpenFileTableClose(t *testing.T) {
	table := NewOpenFileTable()
	fd := table.Open(5, O_RDONLY)
	if !table.Close(fd) {
		t.Error("expected true")
	}
	if table.Get(fd) != nil {
		t.Error("expected nil after close")
	}
}

func TestOpenFileTableCloseInvalid(t *testing.T) {
	table := NewOpenFileTable()
	if table.Close(99) {
		t.Error("expected false for invalid fd")
	}
}

func TestOpenFileTableDup(t *testing.T) {
	table := NewOpenFileTable()
	fd1 := table.Open(5, O_RDONLY)
	fd2 := table.Dup(fd1)
	if fd2 < 0 {
		t.Fatal("expected valid fd")
	}
	if fd2 == fd1 {
		t.Error("dup should return different fd")
	}
	if table.Get(fd1) != table.Get(fd2) {
		t.Error("both fds should point to same entry")
	}
}

func TestOpenFileTableDupRefCount(t *testing.T) {
	table := NewOpenFileTable()
	fd1 := table.Open(5, O_RDONLY)
	table.Dup(fd1)
	if table.Get(fd1).RefCount != 2 {
		t.Error("expected ref count 2")
	}
}

func TestOpenFileTableDupInvalid(t *testing.T) {
	table := NewOpenFileTable()
	if table.Dup(99) != -1 {
		t.Error("expected -1 for invalid fd")
	}
}

func TestOpenFileTableDupSharedOffset(t *testing.T) {
	table := NewOpenFileTable()
	fd1 := table.Open(5, O_RDWR)
	fd2 := table.Dup(fd1)
	table.Get(fd1).Offset = 42
	if table.Get(fd2).Offset != 42 {
		t.Error("dup'd fds should share offset")
	}
}

func TestOpenFileTableDup2(t *testing.T) {
	table := NewOpenFileTable()
	fd1 := table.Open(5, O_RDONLY)
	result := table.Dup2(fd1, 1)
	if result != 1 {
		t.Errorf("expected 1, got %d", result)
	}
	if table.Get(1) != table.Get(fd1) {
		t.Error("fd 1 should point to same entry as fd1")
	}
}

func TestOpenFileTableDup2ClosesExisting(t *testing.T) {
	table := NewOpenFileTable()
	fd1 := table.Open(5, O_RDONLY)
	fd2 := table.Open(6, O_RDONLY)
	table.Dup2(fd1, fd2)
	entry := table.Get(fd2)
	if entry == nil || entry.InodeNumber != 5 {
		t.Error("fd2 should now point to inode 5")
	}
}

func TestOpenFileTableDup2Invalid(t *testing.T) {
	table := NewOpenFileTable()
	if table.Dup2(99, 1) != -1 {
		t.Error("expected -1 for invalid old_fd")
	}
}

func TestOpenFileTableCloseWithRefCount(t *testing.T) {
	table := NewOpenFileTable()
	fd1 := table.Open(5, O_RDONLY)
	fd2 := table.Dup(fd1)
	table.Close(fd1)
	entry := table.Get(fd2)
	if entry == nil || entry.RefCount != 1 {
		t.Error("fd2 should still work with ref count 1")
	}
}

func TestFileDescriptorTableAddGet(t *testing.T) {
	fdt := NewFileDescriptorTable()
	fdt.Add(3, 100)
	if fdt.GetGlobal(3) != 100 {
		t.Error("expected 100")
	}
}

func TestFileDescriptorTableGetNonexistent(t *testing.T) {
	fdt := NewFileDescriptorTable()
	if fdt.GetGlobal(99) != -1 {
		t.Error("expected -1 for nonexistent")
	}
}

func TestFileDescriptorTableRemove(t *testing.T) {
	fdt := NewFileDescriptorTable()
	fdt.Add(3, 100)
	if fdt.Remove(3) != 100 {
		t.Error("expected 100")
	}
	if fdt.GetGlobal(3) != -1 {
		t.Error("expected -1 after remove")
	}
}

func TestFileDescriptorTableRemoveNonexistent(t *testing.T) {
	fdt := NewFileDescriptorTable()
	if fdt.Remove(99) != -1 {
		t.Error("expected -1")
	}
}

func TestFileDescriptorTableClone(t *testing.T) {
	fdt := NewFileDescriptorTable()
	fdt.Add(3, 100)
	fdt.Add(4, 200)

	cloned := fdt.Clone()
	if cloned.GetGlobal(3) != 100 || cloned.GetGlobal(4) != 200 {
		t.Error("clone should have same mappings")
	}

	cloned.Remove(3)
	if cloned.GetGlobal(3) != -1 {
		t.Error("removing from clone should work")
	}
	if fdt.GetGlobal(3) != 100 {
		t.Error("original should be unaffected")
	}
}
