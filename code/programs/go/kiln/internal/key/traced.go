package key

// This file is the ENCODING half of the traced legacy key (KILN04 section 3
// and section 11, semantics version 2).  Everything here is a pure function
// of its arguments: no tree, no policy, no environment.  The code that decides
// WHICH values to feed these functions lives in internal/legacy.
//
// # What a traced key is made of
//
// A traced action has two keys.  The WEAK key W covers what a system-call
// trace cannot see.  The STRONG key repeats W's sections and replaces
// `consumed` with
//
//	weak:      -> W (32 raw bytes)
//	pkg:<q>    -> T(q)                  a package, plus its nested package names
//	path:<p>   -> a state digest        absent | directory | file | link
//	list:<d>   -> a listing digest      the sorted names and kinds in a directory
//
// one entry per entry of the manifest the recorder produced, each paired with
// its digest AS IT IS NOW.  If any one of those digests moved, the strong key
// moves; if none did, it does not.  That is the whole soundness argument.
//
// # Why the four state digests start with a tag byte
//
// "Absent" and "an empty directory" must never share a digest, nor a file
// whose bytes happen to look like a link target.  Each digest is SHA-256 of a
// ONE-BYTE TAG followed by a payload whose shape the tag fixes:
//
//	00                                  absent     (nothing at the path)
//	01                                  directory
//	02 || u8(exec) || SHA-256(content)  regular file
//	03 || bytes(target)                 symbolic link
//
// The tag is a raw byte, not a framed string: the payload shapes differ in
// length, and the tag is the first byte, so no payload can be re-read as
// another tag's payload.
//
// # Domain separation
//
// T(q) and the missing-package digest begin with their own framed names
// ("kiln-traced-pkg", "kiln-missing-package"), so a package digest can never
// equal a state digest or a listing digest, whatever the content.

import (
	"bytes"
	"crypto/sha256"
	"fmt"
	"sort"
	"strings"
)

// Entry-name prefixes of a manifest entry (KILN04 section 11) and the one
// consumed target that belongs to the strong key alone.
const (
	EntryPackage = "pkg:"
	EntryPath    = "path:"
	EntryList    = "list:"
	// WeakTarget is the strong key's link to its weak key.  It is not a
	// manifest entry: a manifest may not name it.
	WeakTarget = "weak:"
)

// Tag bytes of the path state digests.
const (
	stateAbsent    byte = 0x00
	stateDirectory byte = 0x01
	stateFile      byte = 0x02
	stateLink      byte = 0x03
)

// Kinds of a listing entry (KILN04 section 11).
const (
	ListFile uint8 = 1
	ListLink uint8 = 2
	ListDir  uint8 = 3
)

// AbsentDigest is the state digest of a path with nothing tracked at it.
func AbsentDigest() [32]byte { return sha256.Sum256([]byte{stateAbsent}) }

// DirectoryDigest is the state digest of a directory.  It deliberately says
// nothing about the directory's contents: contents are a `list:` entry's job.
func DirectoryDigest() [32]byte { return sha256.Sum256([]byte{stateDirectory}) }

// FileDigest is the state digest of a regular file: its executable bit (as the
// Source reports it, never the disk) and the SHA-256 of its content.
func FileDigest(exec bool, content [32]byte) [32]byte {
	b := []byte{stateFile, 0}
	if exec {
		b[1] = 1
	}
	return sha256.Sum256(append(b, content[:]...))
}

// LinkDigest is the state digest of a symbolic link: its target text, framed.
// The target must be valid UTF-8, like every string in a key.
func LinkDigest(target string) ([32]byte, error) {
	if err := checkUTF8("link target", target); err != nil {
		return [32]byte{}, err
	}
	return sha256.Sum256(putBytes([]byte{stateLink}, []byte(target))), nil
}

// MissingPackageDigest is what a `pkg:` entry digests to when the package no
// longer exists.  A fixed value, so "gone" is a state like any other.
func MissingPackageDigest() [32]byte {
	return sha256.Sum256(putStr(nil, "kiln-missing-package"))
}

// ListingEntry is one child of a listed directory.
type ListingEntry struct {
	Name string
	Kind uint8 // ListFile, ListLink or ListDir
}

// ListingDigest is SHA-256 of list([str(name) || u8(kind)]).  The names are
// the directory's immediate children, not paths, so a name is non-empty, valid
// UTF-8 and holds no '/'.  As everywhere in KILN02, the order is the caller's
// duty and a violation is an error: strictly ascending by name bytes, so an
// equal neighbour (a duplicate) is refused as well as a descending pair.
func ListingDigest(es []ListingEntry) ([32]byte, error) {
	b := putU64(nil, uint64(len(es)))
	for i, e := range es {
		if e.Name == "" || strings.ContainsRune(e.Name, '/') {
			return [32]byte{}, fmt.Errorf("listing: %q is not a child name", e.Name)
		}
		if err := checkUTF8("listing name", e.Name); err != nil {
			return [32]byte{}, err
		}
		switch e.Kind {
		case ListFile, ListLink, ListDir:
		default:
			return [32]byte{}, fmt.Errorf("listing: unknown kind %d for %q", e.Kind, e.Name)
		}
		if i > 0 {
			if err := checkOrder("listing", []byte(es[i-1].Name), []byte(e.Name)); err != nil {
				return [32]byte{}, err
			}
		}
		b = putU8(putStr(b, e.Name), e.Kind)
	}
	return sha256.Sum256(b), nil
}

// TracedPackageDigest is T(q) (KILN04 section 5.2):
//
//	SHA-256( str("kiln-traced-pkg") || bytes(D(q)) || list(str(rel_path)...) )
//
// own is D(q), the package's own digest (KILN03 section 3.1), which already
// covers every tracked name in q except those inside nested packages.  nested
// is the rel_path of each package nested DIRECTLY in q, strictly ascending.
// Together they make T(q) move when a tracked file appears where an action
// found nothing (D(q) changes) and when a new package appears there instead
// (the nested list changes).
func TracedPackageDigest(own [32]byte, nested []string) ([32]byte, error) {
	b := putBytes(putStr(nil, "kiln-traced-pkg"), own[:])
	b = putU64(b, uint64(len(nested)))
	for i, n := range nested {
		if err := checkPath(n); err != nil {
			return [32]byte{}, fmt.Errorf("nested package: %w", err)
		}
		if i > 0 {
			if err := checkOrder("nested packages", []byte(nested[i-1]), []byte(n)); err != nil {
				return [32]byte{}, err
			}
		}
		b = putStr(b, n)
	}
	return sha256.Sum256(b), nil
}

// CheckEntryName applies the manifest allowlist (KILN04 section 11): a name
// starts with `pkg:`, `path:` or `list:`, and the rest is a KILN02 path, except
// that `list:.` names the repository root.  Anything else, `weak:` included, is
// refused, so a manifest can never forge or shadow the strong key's own entry.
func CheckEntryName(name string) error {
	switch {
	case strings.HasPrefix(name, EntryPackage):
		return checkEntryPath(name, name[len(EntryPackage):])
	case strings.HasPrefix(name, EntryPath):
		return checkEntryPath(name, name[len(EntryPath):])
	case strings.HasPrefix(name, EntryList):
		if rest := name[len(EntryList):]; rest != "." {
			return checkEntryPath(name, rest)
		}
		return nil
	}
	return fmt.Errorf("manifest entry %q: prefix is not pkg:, path: or list:", clip(name))
}

func checkEntryPath(name, rest string) error {
	if err := checkPath(rest); err != nil {
		return fmt.Errorf("manifest entry %q: %w", clip(name), err)
	}
	return nil
}

// clip keeps an error message short and printable.
func clip(s string) string {
	if len(s) > 80 {
		s = s[:80] + "..."
	}
	return strings.ToValidUTF8(s, "?")
}

// EntryDigest pairs a manifest entry's name with its digest as it is now.
type EntryDigest struct {
	Name   string
	Digest [32]byte
}

// StrongConsumed builds the strong key's `consumed` section (KILN04 section
// 3.2): `weak:` -> the weak key, plus one entry per manifest entry, every phase
// the nominal `build`.  Each name passes CheckEntryName; the section is sorted
// here because a manifest keeps its own order, and two entries with one name are
// an error (the manifest would be ambiguous about which digest it meant).
func StrongConsumed(weak [32]byte, entries []EntryDigest) ([]Consumed, error) {
	out := make([]Consumed, 0, len(entries)+1)
	out = append(out, Consumed{Target: WeakTarget, Phase: PhaseBuild, Key: append([]byte(nil), weak[:]...)})
	for _, e := range entries {
		if err := CheckEntryName(e.Name); err != nil {
			return nil, err
		}
		d := e.Digest
		out = append(out, Consumed{Target: e.Name, Phase: PhaseBuild, Key: d[:]})
	}
	sort.SliceStable(out, func(a, b int) bool { return out[a].Target < out[b].Target })
	for i := 1; i < len(out); i++ {
		if out[i-1].Target == out[i].Target {
			return nil, fmt.Errorf("manifest entry %q appears twice", clip(out[i].Target))
		}
	}
	return out, nil
}

// SortedConsumed returns es sorted by (target, phase) bytewise: a helper for
// callers composing a `consumed` section from unordered parts.
func SortedConsumed(es []Consumed) []Consumed {
	out := append([]Consumed(nil), es...)
	sort.SliceStable(out, func(a, b int) bool {
		if c := bytes.Compare([]byte(out[a].Target), []byte(out[b].Target)); c != 0 {
			return c < 0
		}
		return out[a].Phase < out[b].Phase
	})
	return out
}
