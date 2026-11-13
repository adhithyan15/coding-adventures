package key

import (
	"encoding/hex"
	"fmt"
	"strings"
)

// The traced-key vector kinds (KILN02 section 6, KILN04 section 11).  They sit
// beside the `valid` and `invalid` arrays of the vector file:
//
//	digests          a state, missing-package, listing or T(q) digest
//	strong_consumed  the `consumed` section StrongConsumed builds
//	refused_entries  manifest entries that must be refused
//
// A weak key and a strong key are ordinary `valid` actions (semantics version
// 2); `strong_consumed` pins the step that BUILDS the strong key's consumed
// list from a weak key and manifest entries, which no plain action can show.

// Digest vector kinds.
const (
	DigestAbsent     = "state-absent"
	DigestDirectory  = "state-directory"
	DigestFile       = "state-file"
	DigestLink       = "state-link"
	DigestMissingPkg = "missing-package"
	DigestListing    = "listing"
	DigestNotDir     = "listing-not-directory"
	DigestTracedPkg  = "traced-package"
)

// ListingJSON is the JSON form of a ListingEntry.
type ListingJSON struct {
	Name string `json:"name"`
	Kind uint8  `json:"kind"`
}

// DigestVector is one digest with the inputs that make it.  Which inputs are
// read depends on Kind; the others must be left out.
type DigestVector struct {
	Name        string `json:"name"`
	Description string `json:"description"`
	Kind        string `json:"kind"`
	// state-file
	Exec       uint8  `json:"exec,omitempty"`
	ContentHex string `json:"content_hex,omitempty"`
	// state-link
	Target string `json:"target,omitempty"`
	// listing
	Entries []ListingJSON `json:"entries,omitempty"`
	// traced-package
	OwnDigestHex string   `json:"own_digest_hex,omitempty"`
	Nested       []string `json:"nested,omitempty"`
	DigestHex    string   `json:"digest_hex"`
}

// EntryDigestJSON is the JSON form of an EntryDigest.
type EntryDigestJSON struct {
	Name      string `json:"name"`
	DigestHex string `json:"digest_hex"`
}

// StrongConsumedVector pins StrongConsumed: from a weak key and entries to the
// consumed section, in the order the key encodes it.
type StrongConsumedVector struct {
	Name        string            `json:"name"`
	Description string            `json:"description"`
	WeakKeyHex  string            `json:"weak_key_hex"`
	Entries     []EntryDigestJSON `json:"entries"`
	Consumed    []ConsumedJSON    `json:"consumed"`
}

// RefusedEntryVector is a manifest entry list that must be refused.
type RefusedEntryVector struct {
	Name        string            `json:"name"`
	Description string            `json:"description"`
	WeakKeyHex  string            `json:"weak_key_hex"`
	Entries     []EntryDigestJSON `json:"entries"`
	Error       string            `json:"error"`
}

// Compute returns the digest the vector's inputs produce.
func (v DigestVector) Compute() ([32]byte, error) {
	var zero [32]byte
	// An input that its kind does not read would be silently ignored, which
	// could hide a typo in the file.  Refuse it instead.
	stray := func(ok bool, what string) error {
		if !ok {
			return fmt.Errorf("%s does not take %s", v.Kind, what)
		}
		return nil
	}
	if err := stray(v.Kind == DigestFile || v.Exec == 0 && v.ContentHex == "", "exec or content_hex"); err != nil {
		return zero, err
	}
	if err := stray(v.Kind == DigestLink || v.Target == "", "target"); err != nil {
		return zero, err
	}
	if err := stray(v.Kind == DigestListing || len(v.Entries) == 0, "entries"); err != nil {
		return zero, err
	}
	if err := stray(v.Kind == DigestTracedPkg || v.OwnDigestHex == "" && len(v.Nested) == 0, "own_digest_hex or nested"); err != nil {
		return zero, err
	}
	switch v.Kind {
	case DigestAbsent:
		return AbsentDigest(), nil
	case DigestDirectory:
		return DirectoryDigest(), nil
	case DigestFile:
		if v.Exec > 1 {
			return zero, fmt.Errorf("exec must be 0 or 1")
		}
		c, err := hex32(v.ContentHex, "content_hex")
		if err != nil {
			return zero, err
		}
		return FileDigest(v.Exec == 1, c), nil
	case DigestLink:
		return LinkDigest(v.Target)
	case DigestMissingPkg:
		return MissingPackageDigest(), nil
	case DigestNotDir:
		return NotDirectoryListingDigest(), nil
	case DigestListing:
		es := make([]ListingEntry, len(v.Entries))
		for i, e := range v.Entries {
			es[i] = ListingEntry{Name: e.Name, Kind: e.Kind}
		}
		return ListingDigest(es)
	case DigestTracedPkg:
		own, err := hex32(v.OwnDigestHex, "own_digest_hex")
		if err != nil {
			return zero, err
		}
		return TracedPackageDigest(own, v.Nested)
	}
	return zero, fmt.Errorf("unknown digest kind %q", v.Kind)
}

func hex32(s, what string) ([32]byte, error) {
	var out [32]byte
	raw, err := hex.DecodeString(s)
	if err != nil {
		return out, fmt.Errorf("%s: %w", what, err)
	}
	if len(raw) != 32 {
		return out, fmt.Errorf("%s must be exactly 32 bytes, got %d", what, len(raw))
	}
	copy(out[:], raw)
	return out, nil
}

func entryDigests(in []EntryDigestJSON) ([]EntryDigest, error) {
	out := make([]EntryDigest, 0, len(in))
	for _, e := range in {
		d, err := hex32(e.DigestHex, "digest_hex")
		if err != nil {
			return nil, err
		}
		out = append(out, EntryDigest{Name: e.Name, Digest: d})
	}
	return out, nil
}

// verifyTraced checks the three traced vector kinds and returns one message
// per failure.
func (vf VectorFile) verifyTraced() []string {
	var fails []string
	if len(vf.Digests) == 0 || len(vf.StrongConsumed) == 0 || len(vf.RefusedEntries) == 0 {
		fails = append(fails, "vector file must contain digests, strong_consumed and refused_entries vectors")
	}
	for _, v := range vf.Digests {
		got, err := v.Compute()
		switch {
		case err != nil:
			fails = append(fails, fmt.Sprintf("digest %s: unexpected error: %v", v.Name, err))
		case hex.EncodeToString(got[:]) != v.DigestHex:
			fails = append(fails, fmt.Sprintf("digest %s: mismatch\n  got  %x\n  want %s", v.Name, got, v.DigestHex))
		}
	}
	for _, v := range vf.StrongConsumed {
		w, err := hex32(v.WeakKeyHex, "weak_key_hex")
		var es []EntryDigest
		if err == nil {
			es, err = entryDigests(v.Entries)
		}
		var got []Consumed
		if err == nil {
			got, err = StrongConsumed(w, es)
		}
		if err != nil {
			fails = append(fails, fmt.Sprintf("strong_consumed %s: %v", v.Name, err))
			continue
		}
		if len(got) != len(v.Consumed) {
			fails = append(fails, fmt.Sprintf("strong_consumed %s: %d entries, want %d", v.Name, len(got), len(v.Consumed)))
			continue
		}
		for i, c := range got {
			want := v.Consumed[i]
			if c.Target != want.Target || c.Phase != want.Phase || hex.EncodeToString(c.Key) != want.KeyHex {
				fails = append(fails, fmt.Sprintf("strong_consumed %s: entry %d is (%s, %s, %x), want (%s, %s, %s)",
					v.Name, i, c.Target, c.Phase, c.Key, want.Target, want.Phase, want.KeyHex))
			}
		}
	}
	for _, v := range vf.RefusedEntries {
		if v.Error == "" {
			fails = append(fails, fmt.Sprintf("refused_entries %s: empty \"error\" field", v.Name))
			continue
		}
		w, err := hex32(v.WeakKeyHex, "weak_key_hex")
		var es []EntryDigest
		if err == nil {
			es, err = entryDigests(v.Entries)
		}
		if err != nil {
			fails = append(fails, fmt.Sprintf("refused_entries %s: %v", v.Name, err))
			continue
		}
		_, err = StrongConsumed(w, es)
		switch {
		case err == nil:
			fails = append(fails, fmt.Sprintf("refused_entries %s: accepted but must be refused", v.Name))
		case !strings.Contains(err.Error(), v.Error):
			fails = append(fails, fmt.Sprintf("refused_entries %s: error %q does not contain %q", v.Name, err, v.Error))
		}
	}
	return fails
}
