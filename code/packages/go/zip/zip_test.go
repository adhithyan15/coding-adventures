package zip_test

import (
	"fmt"
	"testing"

	zip "github.com/adhithyan15/coding-adventures/code/packages/go/zip"
)

// =============================================================================
// CRC-32
// =============================================================================

func TestCRC32KnownValue(t *testing.T) {
	if got := zip.CRC32([]byte("hello world"), 0); got != 0x0D4A1185 {
		t.Fatalf("CRC32('hello world') = %08X, want 0D4A1185", got)
	}
	if got := zip.CRC32([]byte("123456789"), 0); got != 0xCBF43926 {
		t.Fatalf("CRC32('123456789') = %08X, want CBF43926", got)
	}
}

func TestCRC32Empty(t *testing.T) {
	if got := zip.CRC32(nil, 0); got != 0 {
		t.Fatalf("CRC32('') = %08X, want 00000000", got)
	}
}

func TestCRC32Incremental(t *testing.T) {
	full := zip.CRC32([]byte("hello world"), 0)
	c1 := zip.CRC32([]byte("hello "), 0)
	c2 := zip.CRC32([]byte("world"), c1)
	if c2 != full {
		t.Fatalf("incremental CRC32 = %08X, want %08X", c2, full)
	}
}

// =============================================================================
// TC-1: Round-trip single file (Stored)
// =============================================================================

func TestZipStoredRoundtrip(t *testing.T) {
	data := []byte("hello, world")
	zw := zip.NewZipWriter()
	zw.AddFile("hello.txt", data, false)
	archive := zw.Finish()

	files, err := zip.Unzip(archive)
	if err != nil {
		t.Fatal(err)
	}
	if string(files["hello.txt"]) != string(data) {
		t.Fatalf("round-trip mismatch")
	}
}

// =============================================================================
// TC-2: Round-trip single file (DEFLATE)
// =============================================================================

func TestZipDeflateRoundtrip(t *testing.T) {
	base := []byte("the quick brown fox jumps over the lazy dog ")
	text := make([]byte, 0, len(base)*10)
	for i := 0; i < 10; i++ {
		text = append(text, base...)
	}
	entries := []struct {
		Name string
		Data []byte
	}{{"text.txt", text}}
	archive := zip.Zip(entries)
	files, err := zip.Unzip(archive)
	if err != nil {
		t.Fatal(err)
	}
	if string(files["text.txt"]) != string(text) {
		t.Fatal("DEFLATE round-trip mismatch")
	}
}

// =============================================================================
// TC-3: Multiple files
// =============================================================================

func TestZipMultipleFiles(t *testing.T) {
	allBytes := make([]byte, 256)
	for i := range allBytes {
		allBytes[i] = byte(i)
	}
	type pair struct{ name string; data []byte }
	pairs := []pair{
		{"a.txt", []byte("file A content")},
		{"b.txt", []byte("file B content")},
		{"c.bin", allBytes},
	}
	entries := make([]struct {
		Name string
		Data []byte
	}, len(pairs))
	for i, p := range pairs {
		entries[i] = struct {
			Name string
			Data []byte
		}{p.name, p.data}
	}
	archive := zip.Zip(entries)
	files, err := zip.Unzip(archive)
	if err != nil {
		t.Fatal(err)
	}
	if len(files) != 3 {
		t.Fatalf("expected 3 files, got %d", len(files))
	}
	for _, p := range pairs {
		if string(files[p.name]) != string(p.data) {
			t.Fatalf("mismatch for %s", p.name)
		}
	}
}

// =============================================================================
// TC-4: Directory entry
// =============================================================================

func TestZipDirectoryEntry(t *testing.T) {
	zw := zip.NewZipWriter()
	zw.AddDirectory("mydir/")
	zw.AddFile("mydir/file.txt", []byte("contents"), true)
	archive := zw.Finish()

	zr, err := zip.NewZipReader(archive)
	if err != nil {
		t.Fatal(err)
	}
	var foundDir, foundFile bool
	for _, e := range zr.Entries() {
		if e.Name == "mydir/" && e.IsDirectory {
			foundDir = true
		}
		if e.Name == "mydir/file.txt" {
			foundFile = true
		}
	}
	if !foundDir {
		t.Fatal("directory entry missing")
	}
	if !foundFile {
		t.Fatal("file inside dir missing")
	}
}

// =============================================================================
// TC-5: CRC-32 mismatch detected
// =============================================================================

func TestZipCRCMismatchDetected(t *testing.T) {
	zw := zip.NewZipWriter()
	zw.AddFile("f.txt", []byte("test data"), false)
	archive := zw.Finish()

	corrupted := make([]byte, len(archive))
	copy(corrupted, archive)
	// Offset 35 = 30-byte fixed header + 5-byte name "f.txt"
	corrupted[35] ^= 0xFF

	_, err := zip.Unzip(corrupted)
	if err == nil {
		t.Fatal("expected CRC error, got nil")
	}
}

// =============================================================================
// TC-6: Random access (read single entry)
// =============================================================================

func TestZipRandomAccess(t *testing.T) {
	entries := make([]struct {
		Name string
		Data []byte
	}, 10)
	for i := range entries {
		entries[i].Name = fmt.Sprintf("f%d.txt", i)
		entries[i].Data = []byte(fmt.Sprintf("content %d", i))
	}
	archive := zip.Zip(entries)

	zr, err := zip.NewZipReader(archive)
	if err != nil {
		t.Fatal(err)
	}
	var entry5 *zip.ZipEntry
	for _, e := range zr.Entries() {
		if e.Name == "f5.txt" {
			e2 := e
			entry5 = &e2
			break
		}
	}
	if entry5 == nil {
		t.Fatal("f5.txt not found")
	}
	data5, err := zr.Read(*entry5)
	if err != nil {
		t.Fatal(err)
	}
	if string(data5) != "content 5" {
		t.Fatalf("expected 'content 5', got %q", data5)
	}
}

// =============================================================================
// TC-7: Incompressible data stored
// =============================================================================

func TestZipIncompressibleStored(t *testing.T) {
	seed := uint32(42)
	data := make([]byte, 1024)
	for i := range data {
		seed = seed*1664525 + 1013904223
		data[i] = byte(seed >> 24)
	}
	zw := zip.NewZipWriter()
	zw.AddFile("random.bin", data, true)
	archive := zw.Finish()

	zr, err := zip.NewZipReader(archive)
	if err != nil {
		t.Fatal(err)
	}
	entry := zr.Entries()[0]
	if entry.Method != 0 {
		t.Fatalf("expected Stored (0), got %d", entry.Method)
	}
	got, err := zr.Read(entry)
	if err != nil {
		t.Fatal(err)
	}
	if string(got) != string(data) {
		t.Fatal("data mismatch")
	}
}

// =============================================================================
// TC-8: Empty file
// =============================================================================

func TestZipEmptyFile(t *testing.T) {
	zw := zip.NewZipWriter()
	zw.AddFile("empty.txt", nil, true)
	archive := zw.Finish()
	files, err := zip.Unzip(archive)
	if err != nil {
		t.Fatal(err)
	}
	if len(files["empty.txt"]) != 0 {
		t.Fatal("expected empty file")
	}
}

// =============================================================================
// TC-9: Large file with compression
// =============================================================================

func TestZipLargeFileCompressed(t *testing.T) {
	base := []byte("abcdefghij")
	data := make([]byte, 0, len(base)*10000)
	for i := 0; i < 10000; i++ {
		data = append(data, base...)
	}
	zw := zip.NewZipWriter()
	zw.AddFile("big.bin", data, true)
	archive := zw.Finish()
	files, err := zip.Unzip(archive)
	if err != nil {
		t.Fatal(err)
	}
	if string(files["big.bin"]) != string(data) {
		t.Fatal("data mismatch")
	}
	if len(archive) >= len(data) {
		t.Fatalf("expected compression: archive=%d data=%d", len(archive), len(data))
	}
}

// =============================================================================
// TC-10: Unicode filename
// =============================================================================

func TestZipUnicodeFilename(t *testing.T) {
	zw := zip.NewZipWriter()
	zw.AddFile("日本語/résumé.txt", []byte("content"), true)
	archive := zw.Finish()
	files, err := zip.Unzip(archive)
	if err != nil {
		t.Fatal(err)
	}
	if string(files["日本語/résumé.txt"]) != "content" {
		t.Fatal("unicode filename mismatch")
	}
}

// =============================================================================
// TC-11: Nested paths
// =============================================================================

func TestZipNestedPaths(t *testing.T) {
	type pair struct{ name string; data []byte }
	pairs := []pair{
		{"root.txt", []byte("root")},
		{"dir/file.txt", []byte("nested")},
		{"dir/sub/deep.txt", []byte("deep")},
	}
	entries := make([]struct {
		Name string
		Data []byte
	}, len(pairs))
	for i, p := range pairs {
		entries[i] = struct {
			Name string
			Data []byte
		}{p.name, p.data}
	}
	archive := zip.Zip(entries)
	files, err := zip.Unzip(archive)
	if err != nil {
		t.Fatal(err)
	}
	for _, p := range pairs {
		if string(files[p.name]) != string(p.data) {
			t.Fatalf("mismatch for %s", p.name)
		}
	}
}

// =============================================================================
// TC-12: Empty archive
// =============================================================================

func TestZipEmptyArchive(t *testing.T) {
	zw := zip.NewZipWriter()
	archive := zw.Finish()
	files, err := zip.Unzip(archive)
	if err != nil {
		t.Fatal(err)
	}
	if len(files) != 0 {
		t.Fatalf("expected 0 files, got %d", len(files))
	}
}

// =============================================================================
// ReadByName
// =============================================================================

func TestZipReadByName(t *testing.T) {
	zw := zip.NewZipWriter()
	zw.AddFile("alpha.txt", []byte("AAA"), true)
	zw.AddFile("beta.txt", []byte("BBB"), true)
	archive := zw.Finish()

	zr, err := zip.NewZipReader(archive)
	if err != nil {
		t.Fatal(err)
	}
	data, err := zr.ReadByName("beta.txt")
	if err != nil {
		t.Fatal(err)
	}
	if string(data) != "BBB" {
		t.Fatalf("got %q, want BBB", data)
	}
	_, err = zr.ReadByName("nope.txt")
	if err == nil {
		t.Fatal("expected error for missing entry")
	}
}

// =============================================================================
// DOSDatetime / DOSEpoch
// =============================================================================

func TestDOSDatetimeEpoch(t *testing.T) {
	dt := zip.DOSDatetime(1980, 1, 1, 0, 0, 0)
	if dt>>16 != 33 {
		t.Fatalf("date field = %d, want 33", dt>>16)
	}
	if dt&0xFFFF != 0 {
		t.Fatalf("time field = %d, want 0", dt&0xFFFF)
	}
}

func TestDOSEpochConstant(t *testing.T) {
	if zip.DOSEpoch != zip.DOSDatetime(1980, 1, 1, 0, 0, 0) {
		t.Fatal("DOSEpoch != DOSDatetime(1980,1,1,0,0,0)")
	}
}

// =============================================================================
// Error paths
// =============================================================================

func TestZipReaderNoEOCD(t *testing.T) {
	_, err := zip.NewZipReader(make([]byte, 100))
	if err == nil {
		t.Fatal("expected error for data with no EOCD")
	}
}

func TestZipReaderTooShort(t *testing.T) {
	_, err := zip.NewZipReader([]byte{1, 2, 3})
	if err == nil {
		t.Fatal("expected error for too-short data")
	}
}

func TestZipReadDirectory(t *testing.T) {
	zw := zip.NewZipWriter()
	zw.AddDirectory("emptydir/")
	archive := zw.Finish()
	zr, err := zip.NewZipReader(archive)
	if err != nil {
		t.Fatal(err)
	}
	entries := zr.Entries()
	data, err := zr.Read(entries[0])
	if err != nil {
		t.Fatal(err)
	}
	if data != nil {
		t.Fatal("expected nil for directory")
	}
}
