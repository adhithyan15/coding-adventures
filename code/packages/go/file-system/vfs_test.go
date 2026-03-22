package filesystem

import (
	"bytes"
	"fmt"
	"testing"
)

// =======================================================================
// Format tests
// =======================================================================

func TestFormatCreatesRootDirectory(t *testing.T) {
	vfs := NewDefaultVFS()
	if err := vfs.Format(); err != nil {
		t.Fatal(err)
	}
	root := vfs.Stat("/")
	if root == nil {
		t.Fatal("root inode should exist")
	}
	if root.Type != FileTypeDirectory {
		t.Error("root should be a directory")
	}
	if root.InodeNumber != 0 {
		t.Errorf("root should be inode 0, got %d", root.InodeNumber)
	}
}

func TestFormatRootHasDotEntries(t *testing.T) {
	vfs := NewDefaultVFS()
	vfs.Format()
	entries := vfs.ReadDir("/")
	names := entryNames(entries)
	if !contains(names, ".") || !contains(names, "..") {
		t.Error("root should have . and .. entries")
	}
}

func TestFormatDotPointsToRoot(t *testing.T) {
	vfs := NewDefaultVFS()
	vfs.Format()
	entries := vfs.ReadDir("/")
	for _, e := range entries {
		if e.Name == "." || e.Name == ".." {
			if e.InodeNumber != 0 {
				t.Errorf("%s should point to inode 0, got %d", e.Name, e.InodeNumber)
			}
		}
	}
}

func TestFormatSuperblockUpdated(t *testing.T) {
	vfs := NewDefaultVFS()
	vfs.Format()
	sb := vfs.GetSuperblock()
	if !sb.IsValid() {
		t.Error("superblock should be valid")
	}
	if sb.FreeInodes >= sb.TotalInodes {
		t.Error("at least one inode should be used")
	}
	if sb.FreeBlocks >= sb.TotalBlocks {
		t.Error("at least one block should be used")
	}
}

// =======================================================================
// MkDir tests
// =======================================================================

func TestMkDirCreatesDirectory(t *testing.T) {
	vfs := NewDefaultVFS()
	vfs.Format()
	if vfs.MkDir("/home", 0o755) != 0 {
		t.Fatal("mkdir should succeed")
	}
	inode := vfs.Stat("/home")
	if inode == nil {
		t.Fatal("/home should exist")
	}
	if inode.Type != FileTypeDirectory {
		t.Error("/home should be a directory")
	}
}

func TestMkDirHasDotEntries(t *testing.T) {
	vfs := NewDefaultVFS()
	vfs.Format()
	vfs.MkDir("/home", 0o755)
	entries := vfs.ReadDir("/home")
	names := entryNames(entries)
	if !contains(names, ".") || !contains(names, "..") {
		t.Error("new dir should have . and ..")
	}
}

func TestMkDirDotPointsCorrectly(t *testing.T) {
	vfs := NewDefaultVFS()
	vfs.Format()
	vfs.MkDir("/home", 0o755)
	homeInode := vfs.Stat("/home")
	entries := vfs.ReadDir("/home")
	for _, e := range entries {
		if e.Name == "." && e.InodeNumber != homeInode.InodeNumber {
			t.Error(". should point to the directory itself")
		}
		if e.Name == ".." && e.InodeNumber != 0 {
			t.Error(".. should point to parent (root)")
		}
	}
}

func TestMkDirNested(t *testing.T) {
	vfs := NewDefaultVFS()
	vfs.Format()
	if vfs.MkDir("/a", 0o755) != 0 {
		t.Error("mkdir /a failed")
	}
	if vfs.MkDir("/a/b", 0o755) != 0 {
		t.Error("mkdir /a/b failed")
	}
	if vfs.MkDir("/a/b/c", 0o755) != 0 {
		t.Error("mkdir /a/b/c failed")
	}
	if vfs.Stat("/a/b/c") == nil {
		t.Error("/a/b/c should exist")
	}
}

func TestMkDirAlreadyExists(t *testing.T) {
	vfs := NewDefaultVFS()
	vfs.Format()
	vfs.MkDir("/home", 0o755)
	if vfs.MkDir("/home", 0o755) != -1 {
		t.Error("duplicate mkdir should fail")
	}
}

func TestMkDirParentNotExists(t *testing.T) {
	vfs := NewDefaultVFS()
	vfs.Format()
	if vfs.MkDir("/nonexistent/child", 0o755) != -1 {
		t.Error("mkdir with missing parent should fail")
	}
}

func TestMkDirAppearsInParent(t *testing.T) {
	vfs := NewDefaultVFS()
	vfs.Format()
	vfs.MkDir("/home", 0o755)
	entries := vfs.ReadDir("/")
	names := entryNames(entries)
	if !contains(names, "home") {
		t.Error("home should appear in root readdir")
	}
}

// =======================================================================
// Open/Close/Read/Write tests
// =======================================================================

func TestOpenCreateWriteCloseReadRoundtrip(t *testing.T) {
	vfs := NewDefaultVFS()
	vfs.Format()

	fd := vfs.Open("/hello.txt", O_WRONLY|O_CREAT)
	if fd < 3 {
		t.Fatal("open should return fd >= 3")
	}
	written := vfs.Write(fd, []byte("Hello, World!"))
	if written != 13 {
		t.Errorf("expected 13 bytes written, got %d", written)
	}
	if vfs.Close(fd) != 0 {
		t.Error("close should succeed")
	}

	fd = vfs.Open("/hello.txt", O_RDONLY)
	if fd < 3 {
		t.Fatal("reopen should return fd >= 3")
	}
	data := vfs.Read(fd, 100)
	if string(data) != "Hello, World!" {
		t.Errorf("expected 'Hello, World!', got %q", string(data))
	}
	vfs.Close(fd)
}

func TestOpenNonexistentWithoutCreat(t *testing.T) {
	vfs := NewDefaultVFS()
	vfs.Format()
	if vfs.Open("/nonexistent.txt", O_RDONLY) != -1 {
		t.Error("should return -1 for nonexistent file without O_CREAT")
	}
}

func TestWriteAdvancesOffset(t *testing.T) {
	vfs := NewDefaultVFS()
	vfs.Format()
	fd := vfs.Open("/test.txt", O_RDWR|O_CREAT)
	vfs.Write(fd, []byte("aaa"))
	vfs.Write(fd, []byte("bbb"))
	vfs.Lseek(fd, 0, SeekSet)
	data := vfs.Read(fd, 10)
	if string(data) != "aaabbb" {
		t.Errorf("expected 'aaabbb', got %q", string(data))
	}
	vfs.Close(fd)
}

func TestReadAtEOFReturnsNil(t *testing.T) {
	vfs := NewDefaultVFS()
	vfs.Format()
	fd := vfs.Open("/test.txt", O_RDWR|O_CREAT)
	vfs.Write(fd, []byte("hi"))
	vfs.Lseek(fd, 0, SeekSet)
	vfs.Read(fd, 2)
	data := vfs.Read(fd, 10)
	if data != nil {
		t.Errorf("expected nil at EOF, got %q", string(data))
	}
	vfs.Close(fd)
}

func TestWriteReadOnlyReturnsError(t *testing.T) {
	vfs := NewDefaultVFS()
	vfs.Format()
	fd := vfs.Open("/test.txt", O_WRONLY|O_CREAT)
	vfs.Write(fd, []byte("data"))
	vfs.Close(fd)

	fd = vfs.Open("/test.txt", O_RDONLY)
	if vfs.Write(fd, []byte("more")) != -1 {
		t.Error("write to read-only fd should return -1")
	}
	vfs.Close(fd)
}

func TestReadWriteOnlyReturnsNil(t *testing.T) {
	vfs := NewDefaultVFS()
	vfs.Format()
	fd := vfs.Open("/test.txt", O_WRONLY|O_CREAT)
	data := vfs.Read(fd, 10)
	if data != nil {
		t.Error("read from write-only fd should return nil")
	}
	vfs.Close(fd)
}

func TestCloseInvalidFD(t *testing.T) {
	vfs := NewDefaultVFS()
	vfs.Format()
	if vfs.Close(99) != -1 {
		t.Error("close invalid fd should return -1")
	}
}

func TestWriteInvalidFD(t *testing.T) {
	vfs := NewDefaultVFS()
	vfs.Format()
	if vfs.Write(99, []byte("data")) != -1 {
		t.Error("write to invalid fd should return -1")
	}
}

func TestReadInvalidFD(t *testing.T) {
	vfs := NewDefaultVFS()
	vfs.Format()
	if vfs.Read(99, 10) != nil {
		t.Error("read from invalid fd should return nil")
	}
}

func TestOpenCreatInSubdirectory(t *testing.T) {
	vfs := NewDefaultVFS()
	vfs.Format()
	vfs.MkDir("/data", 0o755)
	fd := vfs.Open("/data/log.txt", O_WRONLY|O_CREAT)
	if fd < 3 {
		t.Fatal("open should succeed")
	}
	vfs.Write(fd, []byte("log entry"))
	vfs.Close(fd)

	fd = vfs.Open("/data/log.txt", O_RDONLY)
	data := vfs.Read(fd, 100)
	if string(data) != "log entry" {
		t.Errorf("expected 'log entry', got %q", string(data))
	}
	vfs.Close(fd)
}

// =======================================================================
// Lseek tests
// =======================================================================

func TestLseekSet(t *testing.T) {
	vfs := NewDefaultVFS()
	vfs.Format()
	fd := vfs.Open("/test.txt", O_RDWR|O_CREAT)
	vfs.Write(fd, []byte("Hello, World!"))
	pos := vfs.Lseek(fd, 7, SeekSet)
	if pos != 7 {
		t.Errorf("expected 7, got %d", pos)
	}
	data := vfs.Read(fd, 6)
	if string(data) != "World!" {
		t.Errorf("expected 'World!', got %q", string(data))
	}
	vfs.Close(fd)
}

func TestLseekCur(t *testing.T) {
	vfs := NewDefaultVFS()
	vfs.Format()
	fd := vfs.Open("/test.txt", O_RDWR|O_CREAT)
	vfs.Write(fd, []byte("ABCDEFGHIJ"))
	vfs.Lseek(fd, 0, SeekSet)
	vfs.Read(fd, 3) // offset = 3
	pos := vfs.Lseek(fd, 2, SeekCur)
	if pos != 5 {
		t.Errorf("expected 5, got %d", pos)
	}
	data := vfs.Read(fd, 5)
	if string(data) != "FGHIJ" {
		t.Errorf("expected 'FGHIJ', got %q", string(data))
	}
	vfs.Close(fd)
}

func TestLseekEnd(t *testing.T) {
	vfs := NewDefaultVFS()
	vfs.Format()
	fd := vfs.Open("/test.txt", O_RDWR|O_CREAT)
	vfs.Write(fd, []byte("Hello!"))
	pos := vfs.Lseek(fd, -3, SeekEnd)
	if pos != 3 {
		t.Errorf("expected 3, got %d", pos)
	}
	data := vfs.Read(fd, 3)
	if string(data) != "lo!" {
		t.Errorf("expected 'lo!', got %q", string(data))
	}
	vfs.Close(fd)
}

func TestLseekInvalidFD(t *testing.T) {
	vfs := NewDefaultVFS()
	vfs.Format()
	if vfs.Lseek(99, 0, SeekSet) != -1 {
		t.Error("lseek on invalid fd should return -1")
	}
}

func TestLseekBeforeBeginning(t *testing.T) {
	vfs := NewDefaultVFS()
	vfs.Format()
	fd := vfs.Open("/test.txt", O_RDWR|O_CREAT)
	if vfs.Lseek(fd, -1, SeekSet) != -1 {
		t.Error("seeking before 0 should return -1")
	}
	vfs.Close(fd)
}

func TestLseekInvalidWhence(t *testing.T) {
	vfs := NewDefaultVFS()
	vfs.Format()
	fd := vfs.Open("/test.txt", O_RDWR|O_CREAT)
	if vfs.Lseek(fd, 0, 99) != -1 {
		t.Error("invalid whence should return -1")
	}
	vfs.Close(fd)
}

// =======================================================================
// Stat tests
// =======================================================================

func TestStatRoot(t *testing.T) {
	vfs := NewDefaultVFS()
	vfs.Format()
	inode := vfs.Stat("/")
	if inode == nil || inode.Type != FileTypeDirectory {
		t.Error("root should be a directory")
	}
}

func TestStatFile(t *testing.T) {
	vfs := NewDefaultVFS()
	vfs.Format()
	fd := vfs.Open("/test.txt", O_WRONLY|O_CREAT)
	vfs.Write(fd, []byte("12345"))
	vfs.Close(fd)

	inode := vfs.Stat("/test.txt")
	if inode == nil {
		t.Fatal("file should exist")
	}
	if inode.Type != FileTypeRegular {
		t.Error("should be regular file")
	}
	if inode.Size != 5 {
		t.Errorf("expected size 5, got %d", inode.Size)
	}
}

func TestStatNonexistent(t *testing.T) {
	vfs := NewDefaultVFS()
	vfs.Format()
	if vfs.Stat("/nonexistent") != nil {
		t.Error("expected nil for nonexistent path")
	}
}

func TestStatEmptyPath(t *testing.T) {
	vfs := NewDefaultVFS()
	vfs.Format()
	if vfs.Stat("") != nil {
		t.Error("expected nil for empty path")
	}
}

func TestStatRelativePath(t *testing.T) {
	vfs := NewDefaultVFS()
	vfs.Format()
	if vfs.Stat("test.txt") != nil {
		t.Error("expected nil for relative path")
	}
}

// =======================================================================
// ReadDir tests
// =======================================================================

func TestReadDirRoot(t *testing.T) {
	vfs := NewDefaultVFS()
	vfs.Format()
	entries := vfs.ReadDir("/")
	names := entryNames(entries)
	if !contains(names, ".") || !contains(names, "..") {
		t.Error("root should have . and ..")
	}
}

func TestReadDirWithFiles(t *testing.T) {
	vfs := NewDefaultVFS()
	vfs.Format()
	vfs.Open("/a.txt", O_WRONLY|O_CREAT)
	vfs.Open("/b.txt", O_WRONLY|O_CREAT)
	entries := vfs.ReadDir("/")
	names := entryNames(entries)
	if !contains(names, "a.txt") || !contains(names, "b.txt") {
		t.Error("files should appear in readdir")
	}
}

func TestReadDirNonexistent(t *testing.T) {
	vfs := NewDefaultVFS()
	vfs.Format()
	if vfs.ReadDir("/nonexistent") != nil {
		t.Error("expected nil for nonexistent")
	}
}

func TestReadDirOnFile(t *testing.T) {
	vfs := NewDefaultVFS()
	vfs.Format()
	fd := vfs.Open("/test.txt", O_WRONLY|O_CREAT)
	vfs.Close(fd)
	if vfs.ReadDir("/test.txt") != nil {
		t.Error("readdir on regular file should return nil")
	}
}

// =======================================================================
// Unlink tests
// =======================================================================

func TestUnlinkRemovesFile(t *testing.T) {
	vfs := NewDefaultVFS()
	vfs.Format()
	fd := vfs.Open("/test.txt", O_WRONLY|O_CREAT)
	vfs.Write(fd, []byte("data"))
	vfs.Close(fd)

	if vfs.Unlink("/test.txt") != 0 {
		t.Error("unlink should succeed")
	}
	if vfs.Stat("/test.txt") != nil {
		t.Error("file should not exist after unlink")
	}
}

func TestUnlinkFreesBlocks(t *testing.T) {
	vfs := NewDefaultVFS()
	vfs.Format()
	freeBefore := vfs.GetSuperblock().FreeBlocks

	fd := vfs.Open("/test.txt", O_WRONLY|O_CREAT)
	vfs.Write(fd, bytes.Repeat([]byte("x"), 1024))
	vfs.Close(fd)
	freeAfterWrite := vfs.GetSuperblock().FreeBlocks

	vfs.Unlink("/test.txt")
	freeAfterUnlink := vfs.GetSuperblock().FreeBlocks

	if freeAfterWrite >= freeBefore {
		t.Error("writing should use blocks")
	}
	if freeAfterUnlink <= freeAfterWrite {
		t.Error("unlink should free blocks")
	}
}

func TestUnlinkFreesInode(t *testing.T) {
	vfs := NewDefaultVFS()
	vfs.Format()
	freeInodesBefore := vfs.GetSuperblock().FreeInodes

	fd := vfs.Open("/test.txt", O_WRONLY|O_CREAT)
	vfs.Close(fd)
	freeInodesAfterCreate := vfs.GetSuperblock().FreeInodes

	vfs.Unlink("/test.txt")
	freeInodesAfterUnlink := vfs.GetSuperblock().FreeInodes

	if freeInodesAfterCreate >= freeInodesBefore {
		t.Error("creating should use an inode")
	}
	if freeInodesAfterUnlink <= freeInodesAfterCreate {
		t.Error("unlink should free an inode")
	}
}

func TestUnlinkNonexistent(t *testing.T) {
	vfs := NewDefaultVFS()
	vfs.Format()
	if vfs.Unlink("/nonexistent") != -1 {
		t.Error("unlink nonexistent should return -1")
	}
}

func TestUnlinkDirectoryFails(t *testing.T) {
	vfs := NewDefaultVFS()
	vfs.Format()
	vfs.MkDir("/dir", 0o755)
	if vfs.Unlink("/dir") != -1 {
		t.Error("unlink directory should return -1")
	}
}

func TestUnlinkRootFails(t *testing.T) {
	vfs := NewDefaultVFS()
	vfs.Format()
	if vfs.Unlink("/") != -1 {
		t.Error("unlink root should return -1")
	}
}

// =======================================================================
// Append tests
// =======================================================================

func TestAppendWritesAtEnd(t *testing.T) {
	vfs := NewDefaultVFS()
	vfs.Format()

	fd := vfs.Open("/log.txt", O_WRONLY|O_CREAT)
	vfs.Write(fd, []byte("first "))
	vfs.Close(fd)

	fd = vfs.Open("/log.txt", O_WRONLY|O_APPEND)
	vfs.Write(fd, []byte("second"))
	vfs.Close(fd)

	fd = vfs.Open("/log.txt", O_RDONLY)
	data := vfs.Read(fd, 100)
	if string(data) != "first second" {
		t.Errorf("expected 'first second', got %q", string(data))
	}
	vfs.Close(fd)
}

// =======================================================================
// Truncate tests
// =======================================================================

func TestTruncClearsFile(t *testing.T) {
	vfs := NewDefaultVFS()
	vfs.Format()

	fd := vfs.Open("/test.txt", O_WRONLY|O_CREAT)
	vfs.Write(fd, []byte("old data"))
	vfs.Close(fd)

	fd = vfs.Open("/test.txt", O_WRONLY|O_TRUNC)
	vfs.Write(fd, []byte("new"))
	vfs.Close(fd)

	fd = vfs.Open("/test.txt", O_RDONLY)
	data := vfs.Read(fd, 100)
	if string(data) != "new" {
		t.Errorf("expected 'new', got %q", string(data))
	}
	vfs.Close(fd)
}

// =======================================================================
// Multi-block tests
// =======================================================================

func TestWriteReadSpanningBlocks(t *testing.T) {
	vfs := NewDefaultVFS()
	vfs.Format()

	data := bytes.Repeat([]byte("ABCDEFGH"), 128) // 1024 bytes
	fd := vfs.Open("/big.bin", O_RDWR|O_CREAT)
	written := vfs.Write(fd, data)
	if written != 1024 {
		t.Errorf("expected 1024 written, got %d", written)
	}
	vfs.Lseek(fd, 0, SeekSet)
	readBack := vfs.Read(fd, 1024)
	if !bytes.Equal(readBack, data) {
		t.Error("data read back does not match")
	}
	vfs.Close(fd)
}

func TestWriteAllDirectBlocks(t *testing.T) {
	vfs := NewDefaultVFS()
	vfs.Format()

	data := bytes.Repeat([]byte("X"), DirectBlocks*BlockSize)
	fd := vfs.Open("/full.bin", O_RDWR|O_CREAT)
	written := vfs.Write(fd, data)
	if written != DirectBlocks*BlockSize {
		t.Errorf("expected %d, got %d", DirectBlocks*BlockSize, written)
	}
	vfs.Lseek(fd, 0, SeekSet)
	readBack := vfs.Read(fd, DirectBlocks*BlockSize)
	if !bytes.Equal(readBack, data) {
		t.Error("data mismatch")
	}
	vfs.Close(fd)
}

func TestWriteUsesIndirectBlock(t *testing.T) {
	vfs := NewVFS(256, 32)
	vfs.Format()

	size := (DirectBlocks + 2) * BlockSize
	data := bytes.Repeat([]byte("Y"), size)
	fd := vfs.Open("/indirect.bin", O_RDWR|O_CREAT)
	written := vfs.Write(fd, data)
	if written != size {
		t.Errorf("expected %d, got %d", size, written)
	}

	inode := vfs.Stat("/indirect.bin")
	if inode.IndirectBlock == -1 {
		t.Error("should have an indirect block")
	}

	vfs.Lseek(fd, 0, SeekSet)
	readBack := vfs.Read(fd, size)
	if !bytes.Equal(readBack, data) {
		t.Error("data mismatch with indirect block")
	}
	vfs.Close(fd)
}

// =======================================================================
// Path resolution tests
// =======================================================================

func TestResolveRoot(t *testing.T) {
	vfs := NewDefaultVFS()
	vfs.Format()
	inode := vfs.ResolvePath("/")
	if inode == nil || inode.InodeNumber != 0 {
		t.Error("/ should resolve to inode 0")
	}
}

func TestResolveNestedPath(t *testing.T) {
	vfs := NewDefaultVFS()
	vfs.Format()
	vfs.MkDir("/a", 0o755)
	vfs.MkDir("/a/b", 0o755)
	vfs.MkDir("/a/b/c", 0o755)

	fd := vfs.Open("/a/b/c/file.txt", O_WRONLY|O_CREAT)
	vfs.Close(fd)

	inode := vfs.ResolvePath("/a/b/c/file.txt")
	if inode == nil || inode.Type != FileTypeRegular {
		t.Error("should resolve to a regular file")
	}
}

func TestResolveNonexistentPath(t *testing.T) {
	vfs := NewDefaultVFS()
	vfs.Format()
	if vfs.ResolvePath("/nonexistent/path") != nil {
		t.Error("expected nil")
	}
}

func TestResolveThroughFileFails(t *testing.T) {
	vfs := NewDefaultVFS()
	vfs.Format()
	fd := vfs.Open("/file.txt", O_WRONLY|O_CREAT)
	vfs.Close(fd)
	if vfs.ResolvePath("/file.txt/child") != nil {
		t.Error("traversing through file should fail")
	}
}

func TestResolveTrailingSlash(t *testing.T) {
	vfs := NewDefaultVFS()
	vfs.Format()
	vfs.MkDir("/home", 0o755)
	inode := vfs.ResolvePath("/home/")
	if inode == nil || inode.Type != FileTypeDirectory {
		t.Error("/home/ should resolve to directory")
	}
}

// =======================================================================
// Edge cases
// =======================================================================

func TestOpenCreatParentNotDirectory(t *testing.T) {
	vfs := NewDefaultVFS()
	vfs.Format()
	fd := vfs.Open("/file.txt", O_WRONLY|O_CREAT)
	vfs.Close(fd)
	if vfs.Open("/file.txt/child.txt", O_WRONLY|O_CREAT) != -1 {
		t.Error("should fail when parent is not a directory")
	}
}

func TestOpenCreatNoParent(t *testing.T) {
	vfs := NewDefaultVFS()
	vfs.Format()
	if vfs.Open("/nonexistent/file.txt", O_WRONLY|O_CREAT) != -1 {
		t.Error("should fail when parent doesn't exist")
	}
}

func TestMultipleFilesInSameDirectory(t *testing.T) {
	vfs := NewDefaultVFS()
	vfs.Format()
	for i := 0; i < 5; i++ {
		fd := vfs.Open(fmt.Sprintf("/file%d.txt", i), O_WRONLY|O_CREAT)
		vfs.Write(fd, []byte(fmt.Sprintf("content %d", i)))
		vfs.Close(fd)
	}

	for i := 0; i < 5; i++ {
		fd := vfs.Open(fmt.Sprintf("/file%d.txt", i), O_RDONLY)
		data := vfs.Read(fd, 100)
		expected := fmt.Sprintf("content %d", i)
		if string(data) != expected {
			t.Errorf("expected %q, got %q", expected, string(data))
		}
		vfs.Close(fd)
	}
}

func TestFullWorkflow(t *testing.T) {
	vfs := NewDefaultVFS()
	vfs.Format()

	vfs.MkDir("/home", 0o755)
	vfs.MkDir("/home/alice", 0o755)

	fd := vfs.Open("/home/alice/notes.txt", O_RDWR|O_CREAT)
	vfs.Write(fd, []byte("These are Alice's notes.\n"))
	vfs.Write(fd, []byte("Second line.\n"))
	vfs.Close(fd)

	entries := vfs.ReadDir("/home/alice")
	names := entryNames(entries)
	if !contains(names, "notes.txt") {
		t.Error("notes.txt should appear in readdir")
	}

	fd = vfs.Open("/home/alice/notes.txt", O_RDONLY)
	data := vfs.Read(fd, 1000)
	expected := "These are Alice's notes.\nSecond line.\n"
	if string(data) != expected {
		t.Errorf("expected %q, got %q", expected, string(data))
	}
	vfs.Close(fd)

	inode := vfs.Stat("/home/alice/notes.txt")
	if inode == nil || inode.Type != FileTypeRegular {
		t.Error("should be a regular file")
	}
	if inode.Size != len(expected) {
		t.Errorf("expected size %d, got %d", len(expected), inode.Size)
	}
}

func TestInodeExhaustion(t *testing.T) {
	vfs := NewVFS(64, 4)
	vfs.Format()
	// inode 0 = root, so 3 more files
	for i := 0; i < 3; i++ {
		fd := vfs.Open(fmt.Sprintf("/%c.txt", 'a'+i), O_WRONLY|O_CREAT)
		if fd < 3 {
			t.Fatalf("file %d should succeed", i)
		}
		vfs.Close(fd)
	}
	if vfs.Open("/d.txt", O_WRONLY|O_CREAT) != -1 {
		t.Error("should fail when inodes exhausted")
	}
}

func TestOpenExistingWithoutCreat(t *testing.T) {
	vfs := NewDefaultVFS()
	vfs.Format()
	fd := vfs.Open("/test.txt", O_WRONLY|O_CREAT)
	vfs.Write(fd, []byte("data"))
	vfs.Close(fd)

	fd = vfs.Open("/test.txt", O_RDONLY)
	data := vfs.Read(fd, 10)
	if string(data) != "data" {
		t.Errorf("expected 'data', got %q", string(data))
	}
	vfs.Close(fd)
}

// =======================================================================
// Helpers
// =======================================================================

func entryNames(entries []*DirectoryEntry) []string {
	names := make([]string, len(entries))
	for i, e := range entries {
		names[i] = e.Name
	}
	return names
}

func contains(slice []string, item string) bool {
	for _, s := range slice {
		if s == item {
			return true
		}
	}
	return false
}
