// Package cas implements generic Content-Addressable Storage (CAS).
//
// # What Is Content-Addressable Storage?
//
// Ordinary storage maps a *name* to content — you ask for "photo.jpg" and get
// that photo. CAS flips the relationship: you ask for the *hash of the content*,
// and you get that content back. The hash is both the address and the integrity
// check.
//
//	Traditional:  name  ──► content   (name can lie; content can change)
//	CAS:          hash  ──► content   (hash is derived from content, cannot lie)
//
// The defining property: if you know the hash, you know the content. If the
// stored bytes do not hash to the address you requested, the store is corrupt.
// CAS is self-authenticating — trust the hash, trust the data.
//
// # How Git Uses CAS
//
// Git's entire history is built on this principle. Every blob (file snapshot),
// tree (directory listing), commit, and tag is stored by the SHA-1 hash of its
// serialised bytes. Two identical files share one stored object. Renaming a file
// creates zero new storage. History is an immutable DAG of hashes pointing to
// hashes. This package provides that CAS layer — hashing and storage only.
//
// # Architecture
//
//	┌──────────────────────────────────────────────────────────┐
//	│  ContentAddressableStore                                  │
//	│                                                           │
//	│  Put(data)        → SHA-1 key, delegate to BlobStore     │
//	│  Get(key)         → fetch from BlobStore, verify hash    │
//	│  FindByPrefix(hex)→ prefix search via BlobStore          │
//	└─────────────────────┬────────────────────────────────────┘
//	                      │ BlobStore interface
//	         ┌────────────┴──────────────────────────────┐
//	         │                                           │
//	  LocalDiskStore                    (future: S3Store, MemStore, …)
//	  root/<xx>/<38-hex-chars>
//	  atomic rename writes
//
// # Example
//
//	dir, _ := os.MkdirTemp("", "cas-example-*")
//	defer os.RemoveAll(dir)
//
//	store, _ := NewLocalDiskStore(dir)
//	cas := NewContentAddressableStore(store)
//
//	key, _ := cas.Put([]byte("hello, world"))
//	data, _ := cas.Get(key)
//	fmt.Println(string(data)) // hello, world
package content_addressable_storage

import (
	"encoding/hex"
	"errors"
	"fmt"
	"os"
	"path/filepath"

	sha1pkg "github.com/adhithyan15/coding-adventures/code/packages/go/sha1"
)

// ─── Hex Utilities ────────────────────────────────────────────────────────────
//
// Keys are [20]byte arrays, but humans interact with them as 40-char lowercase
// hex strings (e.g., "a3f4b2c1d0e9f8a7b6c5d4e3f2a1b0c9d8e7f6a5").
//
// KeyToHex  — converts [20]byte → 40-char lowercase hex string.
// HexToKey  — parses a 40-char hex string → [20]byte; returns error on bad input.

// KeyToHex converts a 20-byte SHA-1 key to a 40-character lowercase hex string.
//
//	key := [20]byte{0xa3, 0xf4, ...}
//	fmt.Println(KeyToHex(key)) // "a3f4..."
func KeyToHex(key [20]byte) string {
	return hex.EncodeToString(key[:])
}

// HexToKey parses a 40-character hex string into a 20-byte key.
//
// Returns an error if the string is not exactly 40 valid hex characters.
func HexToKey(h string) ([20]byte, error) {
	var key [20]byte
	if len(h) != 40 {
		return key, fmt.Errorf("cas: expected 40 hex chars, got %d", len(h))
	}
	b, err := hex.DecodeString(h)
	if err != nil {
		return key, fmt.Errorf("cas: invalid hex string: %w", err)
	}
	copy(key[:], b)
	return key, nil
}

// decodeHexPrefix decodes an arbitrary-length hex string (1–40 chars, may be
// odd-length) into a byte prefix for use in prefix searches.
//
// Odd-length strings are right-padded with '0' before decoding, because a
// nibble prefix like "a3f" means "starts with 0xa3, 0xf0" — the trailing
// nibble is the high nibble of the next byte.
//
// Returns an error if the string is empty or contains non-hex characters.
func decodeHexPrefix(s string) ([]byte, error) {
	if s == "" {
		return nil, errors.New("cas: prefix cannot be empty")
	}
	// Validate all characters are valid hex digits before any decoding.
	for _, c := range s {
		if !((c >= '0' && c <= '9') || (c >= 'a' && c <= 'f') || (c >= 'A' && c <= 'F')) {
			return nil, fmt.Errorf("cas: invalid hex character: %q", c)
		}
	}
	// Pad to even length so hex.DecodeString accepts it.
	padded := s
	if len(s)%2 == 1 {
		padded = s + "0"
	}
	b, err := hex.DecodeString(padded)
	if err != nil {
		return nil, fmt.Errorf("cas: decode prefix: %w", err)
	}
	return b, nil
}

// ─── BlobStore Interface ──────────────────────────────────────────────────────
//
// BlobStore is the single abstraction that separates CAS logic from persistence.
// Any type that can store and retrieve byte blobs by a 20-byte key qualifies.
//
// Implementations are responsible only for raw byte storage; integrity checking
// (re-hashing on read) is the CAS layer's job. This separation of concerns
// means you can swap in a cloud or in-memory backend without touching the CAS
// logic, and conversely the CAS logic never needs to know about filesystems.

// BlobStore is a pluggable key-value store for raw byte blobs keyed by a
// 20-byte SHA-1 hash.
//
// Implement BlobStore to add a new storage backend. All methods use a value
// receiver convention (implemented by pointer on LocalDiskStore) so that
// implementations can be wrapped in interfaces without copying.
//
//   - Put is idempotent: storing the same key twice is not an error.
//   - Get returns an error wrapping ErrNotFound if the key is absent.
//   - KeysWithPrefix returns all stored keys whose first len(prefix) bytes
//     equal prefix; used for abbreviated-hash lookup.
type BlobStore interface {
	Put(key [20]byte, data []byte) error
	Get(key [20]byte) ([]byte, error)
	Exists(key [20]byte) (bool, error)
	KeysWithPrefix(prefix []byte) ([][20]byte, error)
}

// ErrNotFound is the sentinel error returned (wrapped) by BlobStore.Get when a
// key does not exist. Callers can use errors.Is(err, cas.ErrNotFound) to
// distinguish a missing object from a genuine I/O failure.
var ErrNotFound = errors.New("cas: key not found")

// ─── CasError ─────────────────────────────────────────────────────────────────
//
// CasError is a typed error that distinguishes the several failure modes that
// ContentAddressableStore can produce. Using a concrete type (rather than plain
// error strings) lets callers write switch/errors.As logic that does not depend
// on fragile string matching.
//
// Variants:
//
//	ErrKindStore         — the underlying BlobStore returned an error
//	ErrKindNotFound      — the requested key is not present in the store
//	ErrKindCorrupted     — stored bytes don't hash to the expected key
//	ErrKindAmbiguous     — a hex prefix matches two or more objects
//	ErrKindPrefixMissing — a hex prefix matches zero objects
//	ErrKindInvalidPrefix — the prefix string is not valid hexadecimal

// ErrKind classifies the error type returned by CAS operations.
type ErrKind int

const (
	ErrKindStore         ErrKind = iota // backend I/O failure
	ErrKindNotFound                     // key not present in store
	ErrKindCorrupted                    // stored bytes don't hash to key
	ErrKindAmbiguous                    // hex prefix matches 2+ objects
	ErrKindPrefixMissing                // hex prefix matches 0 objects
	ErrKindInvalidPrefix                // not valid hex or empty
)

// CasError carries a kind discriminator, an optional key, and the underlying
// cause (for ErrKindStore).
type CasError struct {
	Kind  ErrKind
	Key   [20]byte // meaningful for NotFound and Corrupted
	Str   string   // meaningful for Ambiguous, PrefixMissing, InvalidPrefix
	Cause error    // non-nil for ErrKindStore
}

func (e *CasError) Error() string {
	switch e.Kind {
	case ErrKindStore:
		return fmt.Sprintf("cas: store error: %v", e.Cause)
	case ErrKindNotFound:
		return fmt.Sprintf("cas: object not found: %s", KeyToHex(e.Key))
	case ErrKindCorrupted:
		return fmt.Sprintf("cas: object corrupted: %s", KeyToHex(e.Key))
	case ErrKindAmbiguous:
		return fmt.Sprintf("cas: ambiguous prefix: %s", e.Str)
	case ErrKindPrefixMissing:
		return fmt.Sprintf("cas: object not found for prefix: %s", e.Str)
	case ErrKindInvalidPrefix:
		return fmt.Sprintf("cas: invalid hex prefix: %q", e.Str)
	default:
		return "cas: unknown error"
	}
}

// Unwrap returns the underlying store error, enabling errors.Is / errors.As to
// inspect the root cause of an ErrKindStore error.
func (e *CasError) Unwrap() error {
	return e.Cause
}

// helper constructors keep the call sites compact.

func storeErr(cause error) *CasError   { return &CasError{Kind: ErrKindStore, Cause: cause} }
func notFoundErr(key [20]byte) *CasError { return &CasError{Kind: ErrKindNotFound, Key: key} }
func corruptedErr(key [20]byte) *CasError { return &CasError{Kind: ErrKindCorrupted, Key: key} }
func ambiguousErr(prefix string) *CasError {
	return &CasError{Kind: ErrKindAmbiguous, Str: prefix}
}
func prefixMissingErr(prefix string) *CasError {
	return &CasError{Kind: ErrKindPrefixMissing, Str: prefix}
}
func invalidPrefixErr(prefix string) *CasError {
	return &CasError{Kind: ErrKindInvalidPrefix, Str: prefix}
}

// ─── ContentAddressableStore ──────────────────────────────────────────────────
//
// ContentAddressableStore wraps a BlobStore and adds three things the store
// alone cannot provide:
//
//  1. Automatic keying  — callers pass content; SHA-1 is computed internally.
//  2. Integrity check   — on every Get, SHA-1(returned bytes) must equal the key.
//  3. Prefix resolution — converts abbreviated hex (like "a3f4b2") to a full key.
//
// The generic parameter S is constrained to BlobStore. Using a type parameter
// (rather than a plain interface field) means the compiler can inline method
// calls when S is a concrete type, and it preserves the full concrete error type
// of the backend without boxing.

// ContentAddressableStore wraps a BlobStore and adds automatic keying,
// integrity verification on read, and abbreviated-hex prefix resolution.
//
// S must implement BlobStore. Use NewContentAddressableStore to create an
// instance.
type ContentAddressableStore[S BlobStore] struct {
	store S
}

// NewContentAddressableStore creates a new CAS wrapping the given BlobStore.
func NewContentAddressableStore[S BlobStore](store S) *ContentAddressableStore[S] {
	return &ContentAddressableStore[S]{store: store}
}

// Put hashes data with SHA-1, stores it via the BlobStore, and returns the key.
//
// Idempotent: if the same content has already been stored, the existing key is
// returned and no write is performed (the BlobStore handles the short-circuit).
//
//	key1, _ := cas.Put([]byte("hello"))
//	key2, _ := cas.Put([]byte("hello")) // no-op — key2 == key1
func (c *ContentAddressableStore[S]) Put(data []byte) ([20]byte, error) {
	key := sha1pkg.Sum1(data)
	// Delegate directly to the store. BlobStore.Put is required to be idempotent,
	// so no pre-check for existence is needed here. Avoiding the exists→put
	// two-step eliminates a TOCTOU window.
	if err := c.store.Put(key, data); err != nil {
		return [20]byte{}, storeErr(err)
	}
	return key, nil
}

// Get retrieves the blob stored under key and verifies its integrity.
//
// The returned bytes are guaranteed to hash to key. If the store returns bytes
// that do not hash to key, a *CasError with ErrKindCorrupted is returned. If
// the key is not present, a *CasError with ErrKindNotFound is returned.
func (c *ContentAddressableStore[S]) Get(key [20]byte) ([]byte, error) {
	data, err := c.store.Get(key)
	if err != nil {
		if errors.Is(err, ErrNotFound) {
			return nil, notFoundErr(key)
		}
		return nil, storeErr(err)
	}
	// Integrity check: re-hash the returned bytes. If the stored file was
	// modified (disk corruption, manual editing, hardware fault), the hashes
	// won't match and we return Corrupted rather than silently handing back
	// wrong data. This is the core CAS guarantee.
	actual := sha1pkg.Sum1(data)
	if actual != key {
		return nil, corruptedErr(key)
	}
	return data, nil
}

// Exists reports whether a key is present in the store without fetching the
// blob. Does not perform an integrity check — use Get for that.
func (c *ContentAddressableStore[S]) Exists(key [20]byte) (bool, error) {
	ok, err := c.store.Exists(key)
	if err != nil {
		return false, storeErr(err)
	}
	return ok, nil
}

// FindByPrefix resolves an abbreviated hex string to a full 20-byte key.
//
// Accepts any non-empty hex string of 1–40 characters. Odd-length strings are
// treated as nibble prefixes ("a3f" matches any key starting with 0xa3, 0xf_).
//
//	key, err := cas.FindByPrefix("a3f4b2")
//
// Errors:
//   - *CasError{Kind: ErrKindInvalidPrefix} — empty string or non-hex chars.
//   - *CasError{Kind: ErrKindPrefixMissing} — no keys match.
//   - *CasError{Kind: ErrKindAmbiguous}     — two or more keys match.
func (c *ContentAddressableStore[S]) FindByPrefix(hexPrefix string) ([20]byte, error) {
	prefixBytes, err := decodeHexPrefix(hexPrefix)
	if err != nil {
		return [20]byte{}, invalidPrefixErr(hexPrefix)
	}

	matches, err := c.store.KeysWithPrefix(prefixBytes)
	if err != nil {
		return [20]byte{}, storeErr(err)
	}

	// Sort for deterministic behaviour in tests (filesystem readdir order is
	// undefined and varies by OS).
	sortKeys(matches)

	switch len(matches) {
	case 0:
		return [20]byte{}, prefixMissingErr(hexPrefix)
	case 1:
		return matches[0], nil
	default:
		return [20]byte{}, ambiguousErr(hexPrefix)
	}
}

// Inner returns the underlying BlobStore. Useful when you need backend-specific
// operations not exposed by ContentAddressableStore (e.g., listing all keys for
// garbage collection, or querying storage statistics).
func (c *ContentAddressableStore[S]) Inner() S {
	return c.store
}

// sortKeys sorts a slice of [20]byte keys in lexicographic order.
// We implement this without importing "sort" to keep the dependency list small
// and to make the comparison explicit for readers.
func sortKeys(keys [][20]byte) {
	n := len(keys)
	// Simple insertion sort — the expected slice size for prefix searches is
	// tiny (almost always 0 or 1 element, rarely more than a handful).
	for i := 1; i < n; i++ {
		k := keys[i]
		j := i - 1
		for j >= 0 && keyLess(k, keys[j]) {
			keys[j+1] = keys[j]
			j--
		}
		keys[j+1] = k
	}
}

// keyLess reports whether a < b in lexicographic byte order.
func keyLess(a, b [20]byte) bool {
	for i := 0; i < 20; i++ {
		if a[i] != b[i] {
			return a[i] < b[i]
		}
	}
	return false
}

// ─── LocalDiskStore ───────────────────────────────────────────────────────────
//
// LocalDiskStore is a filesystem-backed BlobStore using the Git 2/38 fanout
// layout: objects are stored at <root>/<xx>/<38-hex-chars> where xx is the
// first byte of the SHA-1 key encoded as two lowercase hex digits.
//
// Why 2/38?  A repository with 100 000 objects would put 100 000 files in a
// single directory without fanout. Most filesystems degrade at that scale.
// Splitting on the first byte creates up to 256 sub-directories (~00/ through
// ff/), keeping each to at most ~400 entries for a 100k-object repo. Git has
// used this layout since its initial commit in 2005.
//
// Atomic writes:
//   - Write data to a temp file with a PID+timestamp suffix (not a fixed .tmp
//     name, which could be pre-targeted by a symlink attack on shared systems).
//   - Call os.Rename(temp, final) — atomic on POSIX, best-effort on Windows.
//   - If Rename fails because the destination already exists (concurrent writer
//     stored the same object), treat that as a successful idempotent write.

// LocalDiskStore is a filesystem BlobStore using Git's 2/38 fanout layout.
//
// Create with NewLocalDiskStore. The zero value is not valid.
type LocalDiskStore struct {
	root string
}

// NewLocalDiskStore creates (or opens) a LocalDiskStore rooted at root.
// The directory is created if it does not exist.
func NewLocalDiskStore(root string) (*LocalDiskStore, error) {
	if err := os.MkdirAll(root, 0o755); err != nil {
		return nil, fmt.Errorf("cas: create root %s: %w", root, err)
	}
	return &LocalDiskStore{root: root}, nil
}

// objectPath computes the storage path for a given key using the 2/38 fanout.
//
//	key = [0xa3, 0xf4, 0xb2, …]
//	dir  = <root>/a3/
//	file = <root>/a3/f4b2…    (38 hex chars)
func (s *LocalDiskStore) objectPath(key [20]byte) string {
	h := KeyToHex(key)
	// h is always exactly 40 chars; split at position 2.
	return filepath.Join(s.root, h[:2], h[2:])
}

// Put stores data under key. Idempotent: if the file already exists, the write
// is skipped (same content guaranteed by the CAS layer above).
func (s *LocalDiskStore) Put(key [20]byte, data []byte) error {
	finalPath := s.objectPath(key)

	// Short-circuit: if the file already exists, the object is already stored.
	// Because the key is a SHA-1 hash of the content, the stored bytes are
	// guaranteed to be identical — no need to overwrite or re-verify here.
	if _, err := os.Stat(finalPath); err == nil {
		return nil // already exists
	}

	// Ensure the two-char fanout directory (e.g., "a3/") exists.
	dir := filepath.Dir(finalPath)
	if err := os.MkdirAll(dir, 0o755); err != nil {
		return fmt.Errorf("cas: mkdir %s: %w", dir, err)
	}

	// Create a temp file with an unpredictable name inside the fanout directory.
	//
	// We use os.CreateTemp which internally uses os.OpenFile with O_EXCL and an
	// OS-generated random suffix — this is both correct and race-safe even when
	// multiple goroutines call Put concurrently with the same key.
	//
	// Security note: a fixed path like "a3/f4b2....tmp" could be pre-targeted
	// by a local attacker who places a symlink there before our write, redirecting
	// the file creation to an arbitrary path. os.CreateTemp's random suffix makes
	// the name infeasible to predict.
	//
	// We include the object base name as a pattern prefix so temp files are
	// recognisable (e.g., during forensic inspection of a crashed store) while
	// still being uniquely named.
	baseName := filepath.Base(finalPath)
	f, err := os.CreateTemp(dir, baseName+".*"+".tmp")
	if err != nil {
		return fmt.Errorf("cas: create temp file: %w", err)
	}
	tmpPath := f.Name()

	_, writeErr := f.Write(data)
	closeErr := f.Close()
	if writeErr != nil {
		_ = os.Remove(tmpPath) // best-effort cleanup
		return fmt.Errorf("cas: write temp file: %w", writeErr)
	}
	if closeErr != nil {
		_ = os.Remove(tmpPath)
		return fmt.Errorf("cas: close temp file: %w", closeErr)
	}

	// Rename into place. On POSIX this is atomic. On Windows it may fail if
	// the destination exists (race with another writer) — we treat that as
	// success because the stored bytes must be identical (same hash, same data).
	if err := os.Rename(tmpPath, finalPath); err != nil {
		_ = os.Remove(tmpPath) // clean up orphan temp file
		// If the final file now exists, another writer stored the same object
		// concurrently. That is fine.
		if _, statErr := os.Stat(finalPath); statErr == nil {
			return nil
		}
		return fmt.Errorf("cas: rename into place: %w", err)
	}
	return nil
}

// Get retrieves the blob stored under key. Returns an error wrapping ErrNotFound
// if the key is not present.
func (s *LocalDiskStore) Get(key [20]byte) ([]byte, error) {
	path := s.objectPath(key)
	data, err := os.ReadFile(path)
	if err != nil {
		if os.IsNotExist(err) {
			return nil, fmt.Errorf("cas: get %s: %w", KeyToHex(key), ErrNotFound)
		}
		return nil, fmt.Errorf("cas: read %s: %w", path, err)
	}
	return data, nil
}

// Exists reports whether key is present without reading the blob.
func (s *LocalDiskStore) Exists(key [20]byte) (bool, error) {
	_, err := os.Stat(s.objectPath(key))
	if err == nil {
		return true, nil
	}
	if os.IsNotExist(err) {
		return false, nil
	}
	return false, fmt.Errorf("cas: stat %s: %w", s.objectPath(key), err)
}

// KeysWithPrefix returns all stored keys whose first len(prefix) bytes equal
// prefix. Used for abbreviated-hash lookup.
//
// Algorithm:
//  1. prefix[0] determines the single fanout bucket to scan (e.g., 0xa3 → "a3/").
//  2. Read the directory entries. Each valid entry is a 38-char hex filename.
//  3. Reconstruct the full 40-char hex, parse to [20]byte, filter by prefix.
func (s *LocalDiskStore) KeysWithPrefix(prefix []byte) ([][20]byte, error) {
	// An empty prefix would match everything — the CAS layer rejects this via
	// InvalidPrefix before calling us, but we guard defensively.
	if len(prefix) == 0 {
		return nil, nil
	}

	// The first byte of the prefix determines the fanout bucket. Encode it as
	// a two-hex-char directory name.
	firstByteHex := fmt.Sprintf("%02x", prefix[0])
	bucket := filepath.Join(s.root, firstByteHex)

	// If the bucket directory doesn't exist, no objects with this prefix are stored.
	entries, err := os.ReadDir(bucket)
	if err != nil {
		if os.IsNotExist(err) {
			return nil, nil
		}
		return nil, fmt.Errorf("cas: readdir %s: %w", bucket, err)
	}

	var keys [][20]byte

	for _, entry := range entries {
		name := entry.Name()

		// Valid object files are exactly 38 hex chars. Skip temp files
		// (*.tmp), hidden files, or any other artifacts.
		if len(name) != 38 {
			continue
		}

		// Reconstruct the full 40-char hex: bucket name (2) + filename (38).
		fullHex := firstByteHex + name
		key, err := HexToKey(fullHex)
		if err != nil {
			continue // not a valid hash — skip
		}

		// Filter: the key's first len(prefix) bytes must equal prefix.
		// We already know key[0] == prefix[0] (by bucket selection), so
		// this check is only meaningful when len(prefix) > 1.
		match := true
		for i := 0; i < len(prefix) && i < 20; i++ {
			if key[i] != prefix[i] {
				match = false
				break
			}
		}
		if match {
			keys = append(keys, key)
		}
	}

	return keys, nil
}

// Verify at compile time that *LocalDiskStore satisfies the BlobStore interface.
// This gives a clear error at the point of definition rather than at usage.
var _ BlobStore = (*LocalDiskStore)(nil)

// ─── MemStore (in-package test helper, also useful to callers) ────────────────
//
// MemStore is a simple in-memory BlobStore backed by a Go map. It is useful for
// tests that don't want to touch the filesystem, and for demonstrating that
// ContentAddressableStore works with any BlobStore implementation — not just
// LocalDiskStore. Exported so callers can use it in their own test suites.

// MemStore is a fully in-memory BlobStore backed by a map. Not safe for
// concurrent use by multiple goroutines without external synchronization.
//
// Useful in tests and as an example of a minimal BlobStore implementation.
type MemStore struct {
	blobs map[[20]byte][]byte
}

// NewMemStore creates an empty MemStore.
func NewMemStore() *MemStore {
	return &MemStore{blobs: make(map[[20]byte][]byte)}
}

// Put stores data under key. Idempotent.
func (m *MemStore) Put(key [20]byte, data []byte) error {
	if _, ok := m.blobs[key]; !ok {
		// Copy the slice so the caller cannot mutate our stored bytes.
		cp := make([]byte, len(data))
		copy(cp, data)
		m.blobs[key] = cp
	}
	return nil
}

// Get retrieves the blob stored under key. Returns ErrNotFound if absent.
func (m *MemStore) Get(key [20]byte) ([]byte, error) {
	data, ok := m.blobs[key]
	if !ok {
		return nil, fmt.Errorf("cas: get %s: %w", KeyToHex(key), ErrNotFound)
	}
	// Return a copy so callers cannot mutate our internal state.
	cp := make([]byte, len(data))
	copy(cp, data)
	return cp, nil
}

// Exists reports whether key is present.
func (m *MemStore) Exists(key [20]byte) (bool, error) {
	_, ok := m.blobs[key]
	return ok, nil
}

// KeysWithPrefix returns all stored keys whose first len(prefix) bytes equal prefix.
func (m *MemStore) KeysWithPrefix(prefix []byte) ([][20]byte, error) {
	if len(prefix) == 0 {
		return nil, nil
	}
	var out [][20]byte
	for k := range m.blobs {
		match := true
		for i := 0; i < len(prefix) && i < 20; i++ {
			if k[i] != prefix[i] {
				match = false
				break
			}
		}
		if match {
			out = append(out, k)
		}
	}
	return out, nil
}

// Verify at compile time that *MemStore satisfies the BlobStore interface.
var _ BlobStore = (*MemStore)(nil)
