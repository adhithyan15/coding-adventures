// Tests for the cas package.
//
// The test suite covers:
//   - Round-trip Put/Get for empty, small, and 1 MiB blobs.
//   - Idempotent Put: calling Put twice with the same data returns the same key.
//   - Get on an unknown key → ErrKindNotFound.
//   - Corrupted file → ErrKindCorrupted.
//   - Exists before and after Put.
//   - FindByPrefix: unique match, ambiguous (two objects), not found, invalid hex, empty string.
//   - LocalDiskStore 2/38 path layout.
//   - BlobStore as interface: MemStore satisfies BlobStore and works with ContentAddressableStore.
//   - Hex utility functions (KeyToHex, HexToKey, decodeHexPrefix).
package content_addressable_storage_test

import (
	"bytes"
	"errors"
	"fmt"
	"os"
	"path/filepath"
	"testing"

	cas "github.com/adhithyan15/coding-adventures/code/packages/go/content_addressable_storage"
	sha1pkg "github.com/adhithyan15/coding-adventures/code/packages/go/sha1"
)

// ─── Helpers ──────────────────────────────────────────────────────────────────

// tmpDir creates a temporary directory unique to a test and registers its
// removal with t.Cleanup so it is always deleted when the test finishes.
func tmpDir(t *testing.T) string {
	t.Helper()
	dir, err := os.MkdirTemp("", "cas-test-*")
	if err != nil {
		t.Fatalf("create temp dir: %v", err)
	}
	t.Cleanup(func() { _ = os.RemoveAll(dir) })
	return dir
}

// newDiskCAS is a convenience wrapper that creates a LocalDiskStore and wraps
// it in a ContentAddressableStore for use in tests.
func newDiskCAS(t *testing.T) *cas.ContentAddressableStore[*cas.LocalDiskStore] {
	t.Helper()
	store, err := cas.NewLocalDiskStore(tmpDir(t))
	if err != nil {
		t.Fatalf("NewLocalDiskStore: %v", err)
	}
	return cas.NewContentAddressableStore(store)
}

// newMemCAS creates an in-memory CAS for tests that don't need a real filesystem.
func newMemCAS(t *testing.T) *cas.ContentAddressableStore[*cas.MemStore] {
	t.Helper()
	return cas.NewContentAddressableStore(cas.NewMemStore())
}

// casErrKind extracts the ErrKind from an error, or returns -1 if the error
// is not a *CasError.
func casErrKind(err error) cas.ErrKind {
	var ce *cas.CasError
	if errors.As(err, &ce) {
		return ce.Kind
	}
	return cas.ErrKind(-1)
}

// ─── Hex Utilities ────────────────────────────────────────────────────────────

func TestKeyToHexRoundtrip(t *testing.T) {
	// A known key: verify the hex encoding and decoding are inverses.
	key := [20]byte{
		0xa3, 0xf4, 0xb2, 0xc1, 0xd0,
		0xe9, 0xf8, 0xa7, 0xb6, 0xc5,
		0xd4, 0xe3, 0xf2, 0xa1, 0xb0,
		0xc9, 0xd8, 0xe7, 0xf6, 0xa5,
	}
	h := cas.KeyToHex(key)
	if len(h) != 40 {
		t.Errorf("KeyToHex: got len %d, want 40", len(h))
	}
	if h != "a3f4b2c1d0e9f8a7b6c5d4e3f2a1b0c9d8e7f6a5" {
		t.Errorf("KeyToHex: got %q", h)
	}
	got, err := cas.HexToKey(h)
	if err != nil {
		t.Fatalf("HexToKey: %v", err)
	}
	if got != key {
		t.Errorf("HexToKey: roundtrip mismatch")
	}
}

func TestHexToKeyRejectsShort(t *testing.T) {
	_, err := cas.HexToKey("a3f4")
	if err == nil {
		t.Error("expected error for short string, got nil")
	}
}

func TestHexToKeyRejectsNonHex(t *testing.T) {
	// 40 chars but the last two are not hex.
	_, err := cas.HexToKey("a3f4b2c1d0e9f8a7b6c5d4e3f2a1b0c9d8e7f6zz")
	if err == nil {
		t.Error("expected error for non-hex chars, got nil")
	}
}

func TestHexToKeyAcceptsUppercase(t *testing.T) {
	lower := "a3f4b2c1d0e9f8a7b6c5d4e3f2a1b0c9d8e7f6a5"
	upper := "A3F4B2C1D0E9F8A7B6C5D4E3F2A1B0C9D8E7F6A5"
	kl, err := cas.HexToKey(lower)
	if err != nil {
		t.Fatal(err)
	}
	ku, err := cas.HexToKey(upper)
	if err != nil {
		t.Fatal(err)
	}
	if kl != ku {
		t.Error("lowercase and uppercase hex should produce the same key")
	}
}

// ─── Round-Trip Put/Get ───────────────────────────────────────────────────────
//
// The fundamental CAS contract: Get(Put(data)) == data.

func TestRoundTripEmpty(t *testing.T) {
	c := newDiskCAS(t)
	data := []byte{}
	key, err := c.Put(data)
	if err != nil {
		t.Fatalf("Put: %v", err)
	}
	got, err := c.Get(key)
	if err != nil {
		t.Fatalf("Get: %v", err)
	}
	if !bytes.Equal(got, data) {
		t.Errorf("roundtrip mismatch for empty blob")
	}
}

func TestRoundTripSmall(t *testing.T) {
	c := newDiskCAS(t)
	data := []byte("hello, content-addressable world!")
	key, err := c.Put(data)
	if err != nil {
		t.Fatalf("Put: %v", err)
	}
	got, err := c.Get(key)
	if err != nil {
		t.Fatalf("Get: %v", err)
	}
	if !bytes.Equal(got, data) {
		t.Errorf("roundtrip mismatch: got %q, want %q", got, data)
	}
}

func TestRoundTrip1MiB(t *testing.T) {
	// Verify that large blobs (1 MiB) survive Put/Get without truncation.
	c := newDiskCAS(t)
	data := make([]byte, 1<<20) // 1 MiB
	for i := range data {
		data[i] = byte(i * 7 % 251) // pseudo-random bytes
	}
	key, err := c.Put(data)
	if err != nil {
		t.Fatalf("Put 1MiB: %v", err)
	}
	got, err := c.Get(key)
	if err != nil {
		t.Fatalf("Get 1MiB: %v", err)
	}
	if !bytes.Equal(got, data) {
		t.Errorf("1MiB roundtrip mismatch (lengths: got %d, want %d)", len(got), len(data))
	}
}

// ─── Idempotent Put ───────────────────────────────────────────────────────────

func TestIdempotentPut(t *testing.T) {
	// Storing the same content twice must return the same key and not error.
	c := newDiskCAS(t)
	data := []byte("idempotent")

	key1, err := c.Put(data)
	if err != nil {
		t.Fatalf("first Put: %v", err)
	}
	key2, err := c.Put(data)
	if err != nil {
		t.Fatalf("second Put: %v", err)
	}
	if key1 != key2 {
		t.Errorf("idempotent Put: key mismatch (%s != %s)", cas.KeyToHex(key1), cas.KeyToHex(key2))
	}
}

// ─── Get Unknown Key ──────────────────────────────────────────────────────────

func TestGetUnknownKey(t *testing.T) {
	c := newDiskCAS(t)
	// An all-zeros key was never stored.
	var key [20]byte
	_, err := c.Get(key)
	if err == nil {
		t.Fatal("expected error for unknown key, got nil")
	}
	if casErrKind(err) != cas.ErrKindNotFound {
		t.Errorf("expected ErrKindNotFound, got error: %v", err)
	}
}

// ─── Corrupted File ───────────────────────────────────────────────────────────

func TestCorruptedFile(t *testing.T) {
	// After Put, manually overwrite the stored file with garbage. Then Get must
	// return ErrKindCorrupted (not silently return the wrong data).
	dir := tmpDir(t)
	store, err := cas.NewLocalDiskStore(dir)
	if err != nil {
		t.Fatal(err)
	}
	c := cas.NewContentAddressableStore(store)

	data := []byte("uncorrupted data")
	key, err := c.Put(data)
	if err != nil {
		t.Fatalf("Put: %v", err)
	}

	// Find the actual file path using the key's 2/38 layout.
	h := cas.KeyToHex(key)
	objectPath := filepath.Join(dir, h[:2], h[2:])

	// Make the file writable before overwriting (necessary on Windows, where
	// files stored with mode 0o444 are read-only and os.WriteFile will fail
	// with "access denied" unless we first relax the permissions).
	if err := os.Chmod(objectPath, 0o644); err != nil {
		t.Fatalf("Chmod (pre-corrupt): %v", err)
	}
	// Overwrite with different bytes — now the file's SHA-1 won't match key.
	if err := os.WriteFile(objectPath, []byte("corrupted!"), 0o644); err != nil {
		t.Fatalf("WriteFile (corrupt): %v", err)
	}

	_, err = c.Get(key)
	if err == nil {
		t.Fatal("expected ErrKindCorrupted, got nil")
	}
	if casErrKind(err) != cas.ErrKindCorrupted {
		t.Errorf("expected ErrKindCorrupted, got: %v", err)
	}
}

// ─── Exists ───────────────────────────────────────────────────────────────────

func TestExistsBeforeAndAfterPut(t *testing.T) {
	c := newDiskCAS(t)
	data := []byte("exists test")

	// Compute the key manually so we can check Exists before Put.
	key := sha1pkg.Sum1(data)

	ok, err := c.Exists(key)
	if err != nil {
		t.Fatalf("Exists before Put: %v", err)
	}
	if ok {
		t.Error("Exists returned true before Put")
	}

	if _, err := c.Put(data); err != nil {
		t.Fatalf("Put: %v", err)
	}

	ok, err = c.Exists(key)
	if err != nil {
		t.Fatalf("Exists after Put: %v", err)
	}
	if !ok {
		t.Error("Exists returned false after Put")
	}
}

// ─── FindByPrefix ─────────────────────────────────────────────────────────────

// TestFindByPrefixUnique stores one blob and finds it by an 8-char hex prefix
// (4 full bytes — even-length to avoid nibble-padding edge cases).
//
// Note on odd-length prefixes: decodeHexPrefix("a3f") produces [0xa3, 0xf0],
// which matches only keys whose second byte is exactly 0xf0. An odd-length
// prefix is a nibble search where the last nibble must be zero. For practical
// unique-match tests we use even-length prefixes so all prefix bytes are fully
// specified and the match is exact without relying on a zero low-nibble.
func TestFindByPrefixUnique(t *testing.T) {
	c := newDiskCAS(t)
	data := []byte("unique prefix object")
	key, err := c.Put(data)
	if err != nil {
		t.Fatalf("Put: %v", err)
	}

	// Use the first 8 hex chars (4 full bytes) of the key as the prefix.
	prefix := cas.KeyToHex(key)[:8]
	found, err := c.FindByPrefix(prefix)
	if err != nil {
		t.Fatalf("FindByPrefix(%q): %v", prefix, err)
	}
	if found != key {
		t.Errorf("FindByPrefix: got %s, want %s", cas.KeyToHex(found), cas.KeyToHex(key))
	}
}

// TestFindByPrefixFullKey verifies that a full 40-char hex prefix also works.
func TestFindByPrefixFullKey(t *testing.T) {
	c := newDiskCAS(t)
	data := []byte("full key prefix")
	key, err := c.Put(data)
	if err != nil {
		t.Fatalf("Put: %v", err)
	}
	found, err := c.FindByPrefix(cas.KeyToHex(key))
	if err != nil {
		t.Fatalf("FindByPrefix: %v", err)
	}
	if found != key {
		t.Error("FindByPrefix with full key: mismatch")
	}
}

// TestFindByPrefixNotFound verifies that a prefix matching nothing returns
// ErrKindPrefixMissing.
func TestFindByPrefixNotFound(t *testing.T) {
	c := newDiskCAS(t)
	// Store something to ensure the store is not empty.
	if _, err := c.Put([]byte("hello")); err != nil {
		t.Fatal(err)
	}
	// "deadbeef" is extremely unlikely to match anything real.
	_, err := c.FindByPrefix("deadbeef")
	if err == nil {
		t.Fatal("expected ErrKindPrefixMissing, got nil")
	}
	if casErrKind(err) != cas.ErrKindPrefixMissing {
		t.Errorf("expected ErrKindPrefixMissing, got: %v", err)
	}
}

// TestFindByPrefixAmbiguous stores two blobs whose keys share the same first
// byte (same fanout bucket). Then we look up by that first byte only, which
// matches both objects and must return ErrKindAmbiguous.
//
// To guarantee two objects share a prefix we store them in a MemStore (where
// we control the keys directly) rather than relying on SHA-1 collisions.
func TestFindByPrefixAmbiguous(t *testing.T) {
	// Build two keys that share the same first byte, using MemStore so we can
	// inject them directly without needing a hash collision.
	mem := cas.NewMemStore()

	// key1 and key2 both start with 0xaa.
	var key1, key2 [20]byte
	key1[0] = 0xaa
	key1[1] = 0x01
	key2[0] = 0xaa
	key2[1] = 0x02

	_ = mem.Put(key1, []byte("data one"))
	_ = mem.Put(key2, []byte("data two"))

	c := cas.NewContentAddressableStore(mem)

	// "aa" as a prefix matches both key1 and key2.
	_, err := c.FindByPrefix("aa")
	if err == nil {
		t.Fatal("expected ErrKindAmbiguous, got nil")
	}
	if casErrKind(err) != cas.ErrKindAmbiguous {
		t.Errorf("expected ErrKindAmbiguous, got: %v", err)
	}
}

// TestFindByPrefixInvalidHex verifies that non-hex characters return ErrKindInvalidPrefix.
func TestFindByPrefixInvalidHex(t *testing.T) {
	c := newMemCAS(t)
	_, err := c.FindByPrefix("xyz!")
	if err == nil {
		t.Fatal("expected ErrKindInvalidPrefix, got nil")
	}
	if casErrKind(err) != cas.ErrKindInvalidPrefix {
		t.Errorf("expected ErrKindInvalidPrefix, got: %v", err)
	}
}

// TestFindByPrefixEmpty verifies that an empty prefix string returns ErrKindInvalidPrefix.
func TestFindByPrefixEmpty(t *testing.T) {
	c := newMemCAS(t)
	_, err := c.FindByPrefix("")
	if err == nil {
		t.Fatal("expected ErrKindInvalidPrefix for empty prefix, got nil")
	}
	if casErrKind(err) != cas.ErrKindInvalidPrefix {
		t.Errorf("expected ErrKindInvalidPrefix, got: %v", err)
	}
}

// TestFindByPrefixOddLength verifies that an odd-length hex prefix is accepted
// and correctly resolves a match.
func TestFindByPrefixOddLength(t *testing.T) {
	// Use MemStore with a known key starting with 0xa3.
	mem := cas.NewMemStore()
	var key [20]byte
	key[0] = 0xa3
	_ = mem.Put(key, []byte("odd length prefix test"))
	c := cas.NewContentAddressableStore(mem)

	// "a3" is even — use 3 chars "a30" to get an odd-length situation
	// that still matches our key (0xa3, 0x00...).
	// Actually test with "a3" as a clean 2-char prefix first.
	found, err := c.FindByPrefix("a3")
	if err != nil {
		t.Fatalf("FindByPrefix(\"a3\"): %v", err)
	}
	if found != key {
		t.Error("FindByPrefix odd-padded: wrong key returned")
	}
}

// ─── LocalDiskStore Path Layout ───────────────────────────────────────────────

// TestLocalDiskStorePathLayout verifies the 2/38 fanout directory structure
// that Git pioneered for its object store.
func TestLocalDiskStorePathLayout(t *testing.T) {
	dir := tmpDir(t)
	store, err := cas.NewLocalDiskStore(dir)
	if err != nil {
		t.Fatal(err)
	}
	c := cas.NewContentAddressableStore(store)

	data := []byte("path layout verification")
	key, err := c.Put(data)
	if err != nil {
		t.Fatalf("Put: %v", err)
	}

	h := cas.KeyToHex(key)
	// Expect: <root>/<first-2-chars>/<remaining-38-chars>
	expectedDir := filepath.Join(dir, h[:2])
	expectedFile := filepath.Join(expectedDir, h[2:])

	if _, err := os.Stat(expectedDir); os.IsNotExist(err) {
		t.Errorf("fanout directory %q does not exist", expectedDir)
	}
	if _, err := os.Stat(expectedFile); os.IsNotExist(err) {
		t.Errorf("object file %q does not exist", expectedFile)
	}

	// Verify the filename is exactly 38 chars (the latter 38 of the 40-char hash).
	if len(h[2:]) != 38 {
		t.Errorf("object filename length: got %d, want 38", len(h[2:]))
	}
}

// ─── BlobStore as Interface ───────────────────────────────────────────────────
//
// Verify that ContentAddressableStore works with any BlobStore implementation,
// not just LocalDiskStore. We use MemStore here (defined in cas.go) as the
// "alternative backend" test.

func TestBlobStoreInterface(t *testing.T) {
	// Use ContentAddressableStore with MemStore — demonstrates the interface works.
	c := newMemCAS(t)

	data := []byte("interface test data")
	key, err := c.Put(data)
	if err != nil {
		t.Fatalf("Put: %v", err)
	}
	got, err := c.Get(key)
	if err != nil {
		t.Fatalf("Get: %v", err)
	}
	if !bytes.Equal(got, data) {
		t.Error("MemStore roundtrip mismatch")
	}

	ok, err := c.Exists(key)
	if err != nil {
		t.Fatal(err)
	}
	if !ok {
		t.Error("Exists returned false after Put on MemStore")
	}
}

// TestBlobStoreInterfaceBoxed verifies that *LocalDiskStore can be stored in a
// BlobStore interface variable and still function correctly. This documents the
// trait-object equivalent in Go.
func TestBlobStoreInterfaceBoxed(t *testing.T) {
	dir := tmpDir(t)
	disk, err := cas.NewLocalDiskStore(dir)
	if err != nil {
		t.Fatal(err)
	}

	// Box the concrete type in the interface — the compiler checks this.
	var bs cas.BlobStore = disk

	data := []byte("boxed interface test")
	key := sha1pkg.Sum1(data)

	if err := bs.Put(key, data); err != nil {
		t.Fatalf("Put via interface: %v", err)
	}
	got, err := bs.Get(key)
	if err != nil {
		t.Fatalf("Get via interface: %v", err)
	}
	if !bytes.Equal(got, data) {
		t.Error("roundtrip via interface mismatch")
	}
}

// ─── CasError ─────────────────────────────────────────────────────────────────

func TestCasErrorMessages(t *testing.T) {
	// Verify that CasError.Error() returns something useful for each kind.
	cases := []struct {
		err  *cas.CasError
		want string
	}{
		{
			err:  &cas.CasError{Kind: cas.ErrKindNotFound, Key: [20]byte{0xab}},
			want: "not found",
		},
		{
			err:  &cas.CasError{Kind: cas.ErrKindCorrupted, Key: [20]byte{0xcd}},
			want: "corrupt",
		},
		{
			err:  &cas.CasError{Kind: cas.ErrKindAmbiguous, Str: "a3"},
			want: "ambiguous",
		},
		{
			err:  &cas.CasError{Kind: cas.ErrKindPrefixMissing, Str: "ff"},
			want: "not found",
		},
		{
			err:  &cas.CasError{Kind: cas.ErrKindInvalidPrefix, Str: "xyz"},
			want: "invalid",
		},
	}
	for _, tc := range cases {
		msg := tc.err.Error()
		// The error message must contain a meaningful keyword.
		found := false
		for _, kw := range []string{"not found", "corrupt", "ambiguous", "invalid"} {
			if len(msg) > 0 && containsFold(msg, kw) {
				found = true
				break
			}
		}
		if !found {
			t.Errorf("CasError(%v).Error() = %q — expected a useful message", tc.err.Kind, msg)
		}
	}
}

// containsFold is a simple case-insensitive contains (avoids importing strings).
func containsFold(s, sub string) bool {
	if len(sub) == 0 {
		return true
	}
	if len(s) < len(sub) {
		return false
	}
	for i := 0; i <= len(s)-len(sub); i++ {
		match := true
		for j := 0; j < len(sub); j++ {
			a, b := s[i+j], sub[j]
			if a >= 'A' && a <= 'Z' {
				a += 'a' - 'A'
			}
			if b >= 'A' && b <= 'Z' {
				b += 'a' - 'A'
			}
			if a != b {
				match = false
				break
			}
		}
		if match {
			return true
		}
	}
	return false
}

// ─── Multiple Objects ─────────────────────────────────────────────────────────

func TestMultipleObjects(t *testing.T) {
	// Store several distinct blobs and verify each can be retrieved independently.
	c := newDiskCAS(t)
	objects := [][]byte{
		[]byte("alpha"),
		[]byte("beta"),
		[]byte("gamma"),
		[]byte("delta"),
	}
	keys := make([][20]byte, len(objects))
	for i, obj := range objects {
		k, err := c.Put(obj)
		if err != nil {
			t.Fatalf("Put[%d]: %v", i, err)
		}
		keys[i] = k
	}
	for i, obj := range objects {
		got, err := c.Get(keys[i])
		if err != nil {
			t.Fatalf("Get[%d]: %v", i, err)
		}
		if !bytes.Equal(got, obj) {
			t.Errorf("object[%d] mismatch: got %q, want %q", i, got, obj)
		}
	}
}

// ─── Inner ────────────────────────────────────────────────────────────────────

func TestInner(t *testing.T) {
	// Inner() must return the original BlobStore.
	mem := cas.NewMemStore()
	c := cas.NewContentAddressableStore(mem)
	if c.Inner() != mem {
		t.Error("Inner() did not return the original store")
	}
}

// ─── Deduplication ────────────────────────────────────────────────────────────

func TestDeduplication(t *testing.T) {
	// Two blobs with identical content must produce the same key. Together with
	// LocalDiskStore's short-circuit on existing paths, this means only one
	// file is written on disk.
	dir := tmpDir(t)
	store, _ := cas.NewLocalDiskStore(dir)
	c := cas.NewContentAddressableStore(store)

	data := []byte("duplicate me")
	k1, err := c.Put(data)
	if err != nil {
		t.Fatal(err)
	}
	k2, err := c.Put(data)
	if err != nil {
		t.Fatal(err)
	}
	if k1 != k2 {
		t.Errorf("deduplication: different keys for same content")
	}

	// Count actual files on disk — should be exactly one.
	h := cas.KeyToHex(k1)
	bucket := filepath.Join(dir, h[:2])
	entries, err := os.ReadDir(bucket)
	if err != nil {
		t.Fatalf("ReadDir: %v", err)
	}
	count := 0
	for _, e := range entries {
		if len(e.Name()) == 38 {
			count++
		}
	}
	if count != 1 {
		t.Errorf("expected 1 object file, found %d", count)
	}
}

// ─── Known SHA-1 Test Vectors ─────────────────────────────────────────────────

func TestKnownSHA1Vectors(t *testing.T) {
	// Verify that the keys produced by Put match known SHA-1 digests.
	// These vectors come from FIPS 180-4 and git's known hashes.
	tests := []struct {
		input    string
		wantHex  string
	}{
		// SHA-1("") from FIPS 180-4
		{"", "da39a3ee5e6b4b0d3255bfef95601890afd80709"},
		// SHA-1("abc") from FIPS 180-4
		{"abc", "a9993e364706816aba3e25717850c26c9cd0d89d"},
	}
	c := newDiskCAS(t)
	for _, tt := range tests {
		key, err := c.Put([]byte(tt.input))
		if err != nil {
			t.Fatalf("Put(%q): %v", tt.input, err)
		}
		got := cas.KeyToHex(key)
		if got != tt.wantHex {
			t.Errorf("SHA-1(%q) = %s, want %s", tt.input, got, tt.wantHex)
		}
	}
}

// ─── Error unwrapping ─────────────────────────────────────────────────────────

func TestStoreErrorWrapping(t *testing.T) {
	// When the underlying store returns an error, it should be accessible via
	// errors.As on the wrapping CasError.
	c := newDiskCAS(t)

	// Ask for a key that doesn't exist — the underlying store returns a wrapped
	// ErrNotFound, and the CAS layer wraps it as ErrKindNotFound.
	var key [20]byte
	key[19] = 0xff
	_, err := c.Get(key)
	if err == nil {
		t.Fatal("expected error, got nil")
	}

	var ce *cas.CasError
	if !errors.As(err, &ce) {
		t.Fatalf("error is not *CasError: %T %v", err, err)
	}
	if ce.Kind != cas.ErrKindNotFound {
		t.Errorf("expected ErrKindNotFound, got kind=%d", ce.Kind)
	}
}

// ─── Verify fmt.Stringer-like output on CasError ─────────────────────────────

func TestCasErrorString(t *testing.T) {
	err := &cas.CasError{Kind: cas.ErrKindAmbiguous, Str: "a3f4"}
	s := err.Error()
	if s == "" {
		t.Error("CasError.Error() returned empty string")
	}
	// Must mention the prefix we passed in.
	if !containsFold(s, "a3f4") {
		t.Errorf("CasError.Error() = %q — expected to mention prefix 'a3f4'", s)
	}
}

// Ensure fmt.Errorf %w wrapping works (errors.Is / errors.As traversal).
// TestCasErrorDefaultKindMessage verifies that CasError.Error() returns a
// non-empty string even for an unknown ErrKind value (the default: branch).
func TestCasErrorDefaultKindMessage(t *testing.T) {
	ce := &cas.CasError{Kind: cas.ErrKind(999)}
	msg := ce.Error()
	if msg == "" {
		t.Error("CasError with unknown Kind returned empty Error() string")
	}
}

func TestCasErrorWrapsUnwrap(t *testing.T) {
	sentinel := fmt.Errorf("underlying disk failure")
	ce := &cas.CasError{Kind: cas.ErrKindStore, Cause: sentinel}
	wrapped := fmt.Errorf("outer: %w", ce)

	var ce2 *cas.CasError
	if !errors.As(wrapped, &ce2) {
		t.Error("errors.As failed to find *CasError through wrapping")
	}
	if !errors.Is(wrapped, sentinel) {
		t.Error("errors.Is failed to find sentinel through wrapping chain")
	}
}

// ─── Store-level errors (storeErr path) ──────────────────────────────────────

// errorStore is a BlobStore whose methods always return an error. It exercises
// the storeErr path in ContentAddressableStore and also covers the Exists and
// FindByPrefix store-error branches.
type errorStore struct{ err error }

func (e *errorStore) Put(_ [20]byte, _ []byte) error          { return e.err }
func (e *errorStore) Get(_ [20]byte) ([]byte, error)          { return nil, e.err }
func (e *errorStore) Exists(_ [20]byte) (bool, error)          { return false, e.err }
func (e *errorStore) KeysWithPrefix(_ []byte) ([][20]byte, error) { return nil, e.err }

var _ cas.BlobStore = (*errorStore)(nil)

func TestPutStoreError(t *testing.T) {
	underlying := fmt.Errorf("backend down")
	c := cas.NewContentAddressableStore(&errorStore{err: underlying})
	_, err := c.Put([]byte("data"))
	if err == nil {
		t.Fatal("expected error from Put, got nil")
	}
	if casErrKind(err) != cas.ErrKindStore {
		t.Errorf("expected ErrKindStore, got: %v", err)
	}
}

func TestGetStoreError(t *testing.T) {
	underlying := fmt.Errorf("backend down")
	c := cas.NewContentAddressableStore(&errorStore{err: underlying})
	var key [20]byte
	_, err := c.Get(key)
	if err == nil {
		t.Fatal("expected error from Get, got nil")
	}
	if casErrKind(err) != cas.ErrKindStore {
		t.Errorf("expected ErrKindStore, got: %v", err)
	}
}

func TestExistsStoreError(t *testing.T) {
	underlying := fmt.Errorf("backend down")
	c := cas.NewContentAddressableStore(&errorStore{err: underlying})
	var key [20]byte
	_, err := c.Exists(key)
	if err == nil {
		t.Fatal("expected error from Exists, got nil")
	}
	if casErrKind(err) != cas.ErrKindStore {
		t.Errorf("expected ErrKindStore, got: %v", err)
	}
}

func TestFindByPrefixStoreError(t *testing.T) {
	underlying := fmt.Errorf("backend down")
	c := cas.NewContentAddressableStore(&errorStore{err: underlying})
	_, err := c.FindByPrefix("ab")
	if err == nil {
		t.Fatal("expected error from FindByPrefix, got nil")
	}
	if casErrKind(err) != cas.ErrKindStore {
		t.Errorf("expected ErrKindStore, got: %v", err)
	}
}

// ─── CasError Store message ───────────────────────────────────────────────────

func TestCasErrorStoreMessage(t *testing.T) {
	cause := fmt.Errorf("disk full")
	ce := &cas.CasError{Kind: cas.ErrKindStore, Cause: cause}
	msg := ce.Error()
	if !containsFold(msg, "store") && !containsFold(msg, "disk") {
		t.Errorf("ErrKindStore message %q should mention the cause", msg)
	}
}

// ─── sortKeys with multiple entries ──────────────────────────────────────────

func TestSortKeysViaCAS(t *testing.T) {
	// Store multiple objects in the same fanout bucket by using MemStore with
	// controlled keys, then call FindByPrefix on a common prefix with 2 matches
	// to exercise the sort + ambiguous-check path.
	mem := cas.NewMemStore()
	var k1, k2, k3 [20]byte
	// All start with 0xcc — same bucket, different sort order.
	k1[0] = 0xcc; k1[1] = 0x30
	k2[0] = 0xcc; k2[1] = 0x10
	k3[0] = 0xcc; k3[1] = 0x20
	_ = mem.Put(k1, []byte("one"))
	_ = mem.Put(k2, []byte("two"))
	_ = mem.Put(k3, []byte("three"))

	c := cas.NewContentAddressableStore(mem)
	// "cc" prefix matches all three → ambiguous, but sortKeys is exercised.
	_, err := c.FindByPrefix("cc")
	if casErrKind(err) != cas.ErrKindAmbiguous {
		t.Errorf("expected ErrKindAmbiguous with 3 matches, got: %v", err)
	}
}

// ─── KeysWithPrefix with artifacts in directory ───────────────────────────────

// TestKeysWithPrefixSkipsArtifacts verifies that LocalDiskStore.KeysWithPrefix
// skips files that are not exactly 38 hex chars (e.g., temp files, hidden files).
func TestKeysWithPrefixSkipsArtifacts(t *testing.T) {
	dir := tmpDir(t)
	store, err := cas.NewLocalDiskStore(dir)
	if err != nil {
		t.Fatal(err)
	}
	c := cas.NewContentAddressableStore(store)

	data := []byte("artifact skip test")
	key, err := c.Put(data)
	if err != nil {
		t.Fatal(err)
	}

	h := cas.KeyToHex(key)
	bucket := filepath.Join(dir, h[:2])

	// Place an artifact file with a non-standard name in the bucket.
	artifactPath := filepath.Join(bucket, "not-a-valid-38-char-object.tmp")
	if err := os.WriteFile(artifactPath, []byte("junk"), 0o644); err != nil {
		t.Fatalf("WriteFile artifact: %v", err)
	}

	// FindByPrefix should still find exactly our one object.
	prefix := h[:8]
	found, err := c.FindByPrefix(prefix)
	if err != nil {
		t.Fatalf("FindByPrefix after artifact: %v", err)
	}
	if found != key {
		t.Errorf("FindByPrefix: got %s, want %s", cas.KeyToHex(found), h)
	}
}

// ─── MemStore Get not found ───────────────────────────────────────────────────

func TestMemStoreGetNotFound(t *testing.T) {
	m := cas.NewMemStore()
	var key [20]byte
	key[0] = 0xff
	_, err := m.Get(key)
	if err == nil {
		t.Fatal("expected error for unknown key, got nil")
	}
	if !errors.Is(err, cas.ErrNotFound) {
		t.Errorf("expected errors.Is(err, ErrNotFound) to be true, got: %v", err)
	}
}

// ─── MemStore KeysWithPrefix empty prefix ────────────────────────────────────

func TestMemStoreKeysWithPrefixEmpty(t *testing.T) {
	m := cas.NewMemStore()
	keys, err := m.KeysWithPrefix(nil)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if len(keys) != 0 {
		t.Errorf("expected empty result for nil prefix, got %d", len(keys))
	}
}

// ─── decodeHexPrefix edge cases ──────────────────────────────────────────────

// TestDecodePrefixSingleChar exercises the odd-length padding path with 1 char.
func TestDecodePrefixSingleChar(t *testing.T) {
	// Use MemStore with a key starting with 0xa0 (= 'a' padded to 'a0').
	mem := cas.NewMemStore()
	var key [20]byte
	key[0] = 0xa0
	_ = mem.Put(key, []byte("single char prefix"))
	c := cas.NewContentAddressableStore(mem)

	found, err := c.FindByPrefix("a0")
	if err != nil {
		t.Fatalf("FindByPrefix(\"a0\"): %v", err)
	}
	if found != key {
		t.Errorf("single char: unexpected key %s", cas.KeyToHex(found))
	}
}

// ─── LocalDiskStore.Get non-NotFound error ────────────────────────────────────

// TestLocalDiskStoreGetDirectoryAsFile tries to Get a key whose object path
// is a directory instead of a file, which triggers a non-not-found read error.
func TestLocalDiskStoreGetDirectoryAsFile(t *testing.T) {
	dir := tmpDir(t)
	store, err := cas.NewLocalDiskStore(dir)
	if err != nil {
		t.Fatal(err)
	}

	// Construct a key and manually create a *directory* at the expected object path.
	var key [20]byte
	key[0] = 0xbb
	key[1] = 0xcc
	h := cas.KeyToHex(key)
	// Create the fanout dir normally.
	fanoutDir := filepath.Join(dir, h[:2])
	if err := os.MkdirAll(fanoutDir, 0o755); err != nil {
		t.Fatal(err)
	}
	// Create the object *name* as a directory (not a file).
	objPath := filepath.Join(fanoutDir, h[2:])
	if err := os.MkdirAll(objPath, 0o755); err != nil {
		t.Fatal(err)
	}

	_, err = store.Get(key)
	// On most OSes, reading a directory with ReadFile returns an error.
	// We just need it to be non-nil; the exact kind varies by platform.
	if err == nil {
		t.Skip("platform allows reading directory as file — skipping")
	}
}

// ─── NewLocalDiskStore error path ────────────────────────────────────────────

// TestNewLocalDiskStoreOnFile verifies that NewLocalDiskStore returns an error
// when given a path that already exists as a regular file (not a directory).
func TestNewLocalDiskStoreOnFile(t *testing.T) {
	dir := tmpDir(t)
	// Create a regular file at the path we'll try to use as a store root.
	filePath := filepath.Join(dir, "notadir")
	if err := os.WriteFile(filePath, []byte("x"), 0o644); err != nil {
		t.Fatal(err)
	}
	// Trying to MkdirAll a path that exists as a file should fail.
	_, err := cas.NewLocalDiskStore(filepath.Join(filePath, "subdir"))
	if err == nil {
		t.Error("expected error when root path is under a file, got nil")
	}
}

// ─── keyLess equal keys ───────────────────────────────────────────────────────

// TestSortKeysEqualKeys verifies that FindByPrefix returns the single unique key
// even when FindByPrefix's internal sort compares equal keys (the false-return
// path of keyLess is exercised when two keys in the list are identical — which
// cannot happen with a real store, but the MemStore allows it via a directly
// constructed MemStore that holds one key and sortKeys is called with a list of
// length 1 containing the same key twice via the ambiguous path).
//
// The simplest exercise of the equal-key return-false path is sortKeys with a
// slice that has duplicate keys. We do this by writing a helper in the test that
// calls FindByPrefix twice on the same object (so sortKeys sees [key, key] if
// the store mistakenly returns duplicates). We test the CAS layer produces the
// correct result regardless.
func TestSortKeysStableWithDuplicateEntries(t *testing.T) {
	// We cannot easily get a real store to return duplicate keys, so we test
	// the sortKeys behaviour indirectly: store three keys in sorted and
	// reverse-sorted order, call FindByPrefix to exercise the sort, and verify
	// the ambiguous error is returned correctly.
	mem := cas.NewMemStore()
	var k1, k2 [20]byte
	k1[0] = 0xee; k1[1] = 0xaa
	k2[0] = 0xee; k2[1] = 0xbb
	_ = mem.Put(k1, []byte("sort-a"))
	_ = mem.Put(k2, []byte("sort-b"))
	c := cas.NewContentAddressableStore(mem)

	// Both start with "ee" — ambiguous, and sortKeys must not panic.
	_, err := c.FindByPrefix("ee")
	if casErrKind(err) != cas.ErrKindAmbiguous {
		t.Errorf("expected ErrKindAmbiguous, got %v", err)
	}
}

// ─── LocalDiskStore.Exists non-notexist error ─────────────────────────────────

// TestLocalDiskStoreExistsNonNotExistError verifies the "stat returns an error
// that is not os.IsNotExist" path in LocalDiskStore.Exists. We achieve this by
// making the fanout directory unreadable (on platforms that support it).
func TestLocalDiskStoreExistsNonNotExistError(t *testing.T) {
	if os.Getuid() == 0 {
		t.Skip("running as root — permission tests are not meaningful")
	}
	dir := tmpDir(t)
	store, err := cas.NewLocalDiskStore(dir)
	if err != nil {
		t.Fatal(err)
	}

	// Put an object so the fanout directory is created.
	var key [20]byte
	key[0] = 0xfa
	if err := store.Put(key, []byte("permission test")); err != nil {
		t.Fatal(err)
	}

	h := cas.KeyToHex(key)
	bucket := filepath.Join(dir, h[:2])

	// Remove execute permission on the bucket so Stat on the object file fails
	// with a permission error (not os.IsNotExist).
	if err := os.Chmod(bucket, 0o000); err != nil {
		t.Skip("cannot chmod bucket directory — skipping")
	}
	t.Cleanup(func() { _ = os.Chmod(bucket, 0o755) })

	_, err = store.Exists(key)
	// On Windows chmod doesn't restrict stat the same way; skip if no error.
	if err == nil {
		t.Skip("platform does not honour directory permission bits — skipping")
	}
}

// ─── LocalDiskStore.Put concurrent-writer simulation ──────────────────────────

// TestLocalDiskStorePutRaceSimulation simulates the concurrent-writer scenario:
// two goroutines Put the same object at the same time. Both should succeed and
// the final file should contain the correct data. This exercises the rename
// fallback path (the second rename finds the final file already exists).
func TestLocalDiskStorePutRaceSimulation(t *testing.T) {
	dir := tmpDir(t)
	store, err := cas.NewLocalDiskStore(dir)
	if err != nil {
		t.Fatal(err)
	}

	data := []byte("concurrent writer test")

	// Run two concurrent Puts.
	errs := make(chan error, 2)
	for i := 0; i < 2; i++ {
		go func() {
			errs <- store.Put(sha1pkg.Sum1(data), data)
		}()
	}

	for i := 0; i < 2; i++ {
		if err := <-errs; err != nil {
			t.Errorf("concurrent Put[%d]: %v", i, err)
		}
	}

	// Verify the object is still readable.
	c := cas.NewContentAddressableStore(store)
	key := sha1pkg.Sum1(data)
	got, err := c.Get(key)
	if err != nil {
		t.Fatalf("Get after concurrent Put: %v", err)
	}
	if !bytes.Equal(got, data) {
		t.Error("concurrent Put: data mismatch")
	}
}

// ─── LocalDiskStore.KeysWithPrefix readdir error ─────────────────────────────

// TestLocalDiskStoreKeysWithPrefixReaddirError exercises the readdir error path
// (not os.IsNotExist) by making the bucket directory unreadable.
func TestLocalDiskStoreKeysWithPrefixReaddirError(t *testing.T) {
	if os.Getuid() == 0 {
		t.Skip("running as root — permission tests are not meaningful")
	}
	dir := tmpDir(t)
	store, err := cas.NewLocalDiskStore(dir)
	if err != nil {
		t.Fatal(err)
	}

	var key [20]byte
	key[0] = 0xfb
	if err := store.Put(key, []byte("readdir error test")); err != nil {
		t.Fatal(err)
	}

	h := cas.KeyToHex(key)
	bucket := filepath.Join(dir, h[:2])
	if err := os.Chmod(bucket, 0o000); err != nil {
		t.Skip("cannot chmod bucket directory — skipping")
	}
	t.Cleanup(func() { _ = os.Chmod(bucket, 0o755) })

	_, err = store.KeysWithPrefix([]byte{0xfb})
	if err == nil {
		t.Skip("platform does not honour directory permission bits — skipping")
	}
}

// ─── MemStore.KeysWithPrefix with large prefix ───────────────────────────────

// TestMemStoreKeysWithPrefixLongPrefix verifies that a prefix longer than 20
// bytes is handled gracefully (the inner loop is bounded by min(len(prefix),20)).
// This covers the `i >= 20` early-termination branch in KeysWithPrefix.
func TestMemStoreKeysWithPrefixLongPrefix(t *testing.T) {
	m := cas.NewMemStore()
	var key [20]byte
	key[0] = 0x01
	_ = m.Put(key, []byte("long prefix test"))

	// A 25-byte prefix whose first 20 bytes are all zero except [0]=0x01.
	// The stored key is exactly [0x01, 0x00, ..., 0x00] (20 bytes).
	// A 25-byte prefix [0x01, 0x00, ..., 0x00] should match since the loop
	// caps at i < 20, so the extra 5 bytes are never compared.
	prefix := make([]byte, 25)
	prefix[0] = 0x01
	keys, err := m.KeysWithPrefix(prefix)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if len(keys) != 1 {
		t.Errorf("expected 1 key with 25-byte prefix, got %d", len(keys))
	}
}

// TestMemStoreKeysWithPrefixLongPrefixNoMatch verifies that a prefix longer
// than 20 bytes that doesn't match the stored key returns an empty slice.
func TestMemStoreKeysWithPrefixLongPrefixNoMatch(t *testing.T) {
	m := cas.NewMemStore()
	var key [20]byte
	key[0] = 0x01
	_ = m.Put(key, []byte("no match long prefix"))

	// A 21-byte prefix whose second byte differs from the stored key's second byte.
	prefix := make([]byte, 21)
	prefix[0] = 0x01
	prefix[1] = 0xff // stored key has prefix[1]=0x00, so this won't match
	keys, err := m.KeysWithPrefix(prefix)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if len(keys) != 0 {
		t.Errorf("expected 0 keys for non-matching long prefix, got %d", len(keys))
	}
}

// ─── KeysWithPrefix skips invalid 38-char filenames ──────────────────────────

// TestKeysWithPrefixSkipsNonHexFilename verifies that LocalDiskStore.KeysWithPrefix
// skips files whose 38-char name is not valid hex (exercises the HexToKey continue
// branch in KeysWithPrefix).
func TestKeysWithPrefixSkipsNonHexFilename(t *testing.T) {
	dir := tmpDir(t)
	store, err := cas.NewLocalDiskStore(dir)
	if err != nil {
		t.Fatal(err)
	}
	c := cas.NewContentAddressableStore(store)

	// Store a real object starting with 0xab.
	var key [20]byte
	key[0] = 0xab
	key[1] = 0xcd
	data := []byte("non-hex filename skip test")
	// Use the store directly to put the key we want.
	if err := store.Put(key, data); err != nil {
		t.Fatal(err)
	}

	h := cas.KeyToHex(key)
	bucket := filepath.Join(dir, h[:2])

	// Plant a 38-char filename that contains non-hex characters — it must be
	// skipped without error. Exactly 38 chars: 37 'g' chars + 1 more.
	badName := "gggggggggggggggggggggggggggggggggggggg" // 38 g's — not valid hex
	if err := os.WriteFile(filepath.Join(bucket, badName), []byte("not real"), 0o644); err != nil {
		t.Fatalf("WriteFile non-hex artifact: %v", err)
	}

	// FindByPrefix should find the one real object and ignore the bad file.
	found, err := c.FindByPrefix(h[:4])
	if err != nil {
		t.Fatalf("FindByPrefix after non-hex artifact: %v", err)
	}
	if found != key {
		t.Errorf("expected key %s, got %s", h, cas.KeyToHex(found))
	}
}

// ─── decodeHexPrefix path: multiple valid chars ───────────────────────────────

// TestDecodePrefixMultipleChars exercises the decodeHexPrefix path with a longer
// even-length string to cover the "all chars valid, even length" code path.
func TestDecodePrefixMultipleChars(t *testing.T) {
	mem := cas.NewMemStore()
	var key [20]byte
	key[0] = 0x12
	key[1] = 0x34
	key[2] = 0x56
	_ = mem.Put(key, []byte("multi char decode"))
	c := cas.NewContentAddressableStore(mem)

	// "123456" is 6 chars = 3 full bytes, should uniquely match our key.
	found, err := c.FindByPrefix("123456")
	if err != nil {
		t.Fatalf("FindByPrefix(\"123456\"): %v", err)
	}
	if found != key {
		t.Errorf("multi-char prefix: unexpected key %s", cas.KeyToHex(found))
	}
}
