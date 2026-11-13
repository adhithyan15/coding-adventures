package key

import (
	"bytes"
	"encoding/hex"
	"encoding/json"
	"errors"
	"fmt"
	"io"
	"os"
	"strings"
)

// MaxVectorFileBytes caps how much of a vector file is read.  The real file is
// a few tens of kilobytes; the cap only stops a mistaken or hostile path
// (a huge file, a device that never ends) from exhausting memory.
const MaxVectorFileBytes = 16 << 20

// ReadVectorFile reads a vector file with the production size cap.
func ReadVectorFile(path string) ([]byte, error) {
	return readVectorFile(path, MaxVectorFileBytes)
}

// readVectorFile is ReadVectorFile with the limit injectable so tests need not
// build a 16 MiB file.  Two guards run before any parsing:
//
//   - the opened file must be a regular file: a directory, FIFO or device
//     would either fail oddly or block forever, so they are refused.  The
//     open itself is non-blocking where the OS supports it (openNoBlock),
//     because opening a FIFO for reading otherwise waits for a writer;
//   - at most limit bytes are read.  We ask for limit+1 so that "exactly
//     limit" is accepted while "one byte more" is detectable and refused,
//     rather than silently truncated into a confusing parse error.
func readVectorFile(path string, limit int64) ([]byte, error) {
	f, err := openNoBlock(path)
	if err != nil {
		return nil, err
	}
	defer f.Close()
	if err := requireRegular(f); err != nil {
		return nil, err
	}
	return readLimited(f, limit)
}

// requireRegular refuses anything but a regular file.  The type is checked on
// the descriptor we already hold: a separate os.Stat of the path would leave a
// window in which the path is swapped for a FIFO or device before the open (a
// TOCTOU race); f.Stat describes exactly what we are about to read.
func requireRegular(f *os.File) error {
	info, err := f.Stat()
	if err != nil {
		return err
	}
	if !info.Mode().IsRegular() {
		return fmt.Errorf("%s: not a regular file", f.Name())
	}
	return nil
}

// readLimited reads r fully but refuses more than limit bytes.
func readLimited(r io.Reader, limit int64) ([]byte, error) {
	data, err := io.ReadAll(io.LimitReader(r, limit+1))
	if err != nil {
		return nil, err
	}
	if int64(len(data)) > limit {
		return nil, fmt.Errorf("input larger than %d bytes", limit)
	}
	return data, nil
}

// This file reads the golden-vector file (code/specs/fixtures/kiln/
// key-vectors-v1.json).  The JSON form of an action is deliberately plain:
// digests, keys, the recipe and tool identities are lowercase hex strings so
// that implementations in any language can read it.  JSON strings cannot
// carry invalid UTF-8, so a target may instead be given as "target_hex" to
// express bytes that are not text.

// ActionJSON is the JSON shape of an Action.
type ActionJSON struct {
	SemanticsVersion uint64         `json:"semantics_version"`
	OS               string         `json:"os"`
	Arch             string         `json:"arch"`
	HostImageID      *string        `json:"host_image_id"`
	Target           string         `json:"target"`
	TargetHex        string         `json:"target_hex,omitempty"`
	Phase            string         `json:"phase"`
	RecipeHex        string         `json:"recipe_hex"`
	Inputs           []FileJSON     `json:"inputs"`
	Reads            []FileJSON     `json:"reads"`
	Consumed         []ConsumedJSON `json:"consumed"`
	Tools            []ToolJSON     `json:"tools"`
	Env              []EnvJSON      `json:"env"`
}

// FileJSON is the JSON shape of a FileEntry.
type FileJSON struct {
	Path      string `json:"path"`
	Kind      uint8  `json:"kind"`
	Exec      uint8  `json:"exec"`
	DigestHex string `json:"digest_hex"`
}

// ConsumedJSON is the JSON shape of a Consumed entry.
type ConsumedJSON struct {
	Target string `json:"target"`
	Phase  string `json:"phase"`
	KeyHex string `json:"key_hex"`
}

// ToolJSON is the JSON shape of a Tool.
type ToolJSON struct {
	Name        string `json:"name"`
	Version     string `json:"version"`
	Kind        uint8  `json:"kind"`
	IdentityHex string `json:"identity_hex"`
}

// EnvJSON is the JSON shape of an EnvVar.
type EnvJSON struct {
	Name   string `json:"name"`
	Value  string `json:"value"`
	Origin uint8  `json:"origin"`
}

// ValidVector is one action with its expected preimage and key.
type ValidVector struct {
	Name        string     `json:"name"`
	Description string     `json:"description"`
	Action      ActionJSON `json:"action"`
	PreimageHex string     `json:"preimage_hex"`
	KeyHex      string     `json:"key_hex"`
}

// InvalidVector is an action that must be refused, with a substring of the
// expected error message.
type InvalidVector struct {
	Name        string     `json:"name"`
	Description string     `json:"description"`
	Action      ActionJSON `json:"action"`
	Error       string     `json:"error"`
}

// VectorFile is the whole golden-vector document.
type VectorFile struct {
	SchemaVersion uint64          `json:"schema_version"`
	Valid         []ValidVector   `json:"valid"`
	Invalid       []InvalidVector `json:"invalid"`
	// The traced-key kinds (KILN04 section 11; see vectors_traced.go).
	Digests        []DigestVector         `json:"digests"`
	StrongConsumed []StrongConsumedVector `json:"strong_consumed"`
	RefusedEntries []RefusedEntryVector   `json:"refused_entries"`
}

// ToAction converts the JSON form to an Action.  A bad hex string is an error.
func (j ActionJSON) ToAction() (Action, error) {
	a := Action{
		SemanticsVersion: j.SemanticsVersion,
		OS:               j.OS,
		Arch:             j.Arch,
		HostImageID:      j.HostImageID,
		Target:           j.Target,
		Phase:            j.Phase,
	}
	var err error
	// Two spellings of one field would make the vector ambiguous (which one
	// wins?), so supplying both is refused rather than resolved silently.
	if j.TargetHex != "" && j.Target != "" {
		return a, errors.New("set target or target_hex, not both")
	}
	if j.TargetHex != "" {
		raw, err := hex.DecodeString(j.TargetHex)
		if err != nil {
			return a, fmt.Errorf("target_hex: %w", err)
		}
		a.Target = string(raw)
	}
	if a.Recipe, err = hex.DecodeString(j.RecipeHex); err != nil {
		return a, fmt.Errorf("recipe_hex: %w", err)
	}
	if a.Inputs, err = filesFromJSON(j.Inputs); err != nil {
		return a, err
	}
	if a.Reads, err = filesFromJSON(j.Reads); err != nil {
		return a, err
	}
	for _, c := range j.Consumed {
		k, err := hex.DecodeString(c.KeyHex)
		if err != nil {
			return a, fmt.Errorf("key_hex: %w", err)
		}
		a.Consumed = append(a.Consumed, Consumed{Target: c.Target, Phase: c.Phase, Key: k})
	}
	for _, t := range j.Tools {
		id, err := hex.DecodeString(t.IdentityHex)
		if err != nil {
			return a, fmt.Errorf("identity_hex: %w", err)
		}
		a.Tools = append(a.Tools, Tool{Name: t.Name, Version: t.Version, Kind: t.Kind, Identity: id})
	}
	for _, e := range j.Env {
		a.Env = append(a.Env, EnvVar{Name: e.Name, Value: e.Value, Origin: e.Origin})
	}
	return a, nil
}

func filesFromJSON(in []FileJSON) ([]FileEntry, error) {
	var out []FileEntry
	for _, f := range in {
		d, err := hex.DecodeString(f.DigestHex)
		if err != nil {
			return nil, fmt.Errorf("digest_hex: %w", err)
		}
		out = append(out, FileEntry{Path: f.Path, Kind: f.Kind, Exec: f.Exec, Digest: d})
	}
	return out, nil
}

// ParseVectors decodes a vector document.  Unknown fields are rejected so a
// typo in the file cannot silently weaken a vector.
func ParseVectors(data []byte) (VectorFile, error) {
	var vf VectorFile
	dec := json.NewDecoder(bytes.NewReader(data))
	dec.DisallowUnknownFields()
	if err := dec.Decode(&vf); err != nil {
		return vf, fmt.Errorf("parse vectors: %w", err)
	}
	// A second JSON value after the document would be ignored by Decode, so a
	// spliced-in or concatenated file could pass with unchecked content.  The
	// next Decode must hit a clean end of input.
	if err := dec.Decode(&struct{}{}); err != io.EOF {
		return vf, errors.New("parse vectors: trailing data after vector document")
	}
	if vf.SchemaVersion != SchemaVersion {
		return vf, fmt.Errorf("vector file schema_version %d, want %d", vf.SchemaVersion, SchemaVersion)
	}
	return vf, nil
}

// Verify checks every valid vector (preimage and key must match exactly) and
// every invalid vector (must be refused, with the expected message).  It
// returns one message per failure; an empty slice means success.
func (vf VectorFile) Verify() []string {
	var fails []string
	if len(vf.Valid) == 0 || len(vf.Invalid) == 0 {
		fails = append(fails, "vector file must contain valid and invalid vectors")
	}
	for _, v := range vf.Valid {
		a, err := v.Action.ToAction()
		if err != nil {
			fails = append(fails, fmt.Sprintf("valid %s: %v", v.Name, err))
			continue
		}
		pre, err := Preimage(a)
		if err != nil {
			fails = append(fails, fmt.Sprintf("valid %s: unexpected error: %v", v.Name, err))
			continue
		}
		if got := hex.EncodeToString(pre); got != v.PreimageHex {
			fails = append(fails, fmt.Sprintf("valid %s: preimage mismatch\n  got  %s\n  want %s", v.Name, got, v.PreimageHex))
		}
		// Preimage just succeeded, so hash its result directly instead of
		// calling Key and discarding an error that cannot happen.
		if got := Hex(KeyOfPreimage(pre)); got != v.KeyHex {
			fails = append(fails, fmt.Sprintf("valid %s: key mismatch: got %s want %s", v.Name, got, v.KeyHex))
		}
	}
	for _, v := range vf.Invalid {
		// strings.Contains(msg, "") is always true, so an empty expectation
		// would make the vector pass vacuously for any refusal.  Require one.
		if v.Error == "" {
			fails = append(fails, fmt.Sprintf("invalid %s: empty \"error\" field", v.Name))
			continue
		}
		a, err := v.Action.ToAction()
		if err != nil {
			fails = append(fails, fmt.Sprintf("invalid %s: %v", v.Name, err))
			continue
		}
		_, err = Preimage(a)
		switch {
		case err == nil:
			fails = append(fails, fmt.Sprintf("invalid %s: accepted but must be refused", v.Name))
		case !strings.Contains(err.Error(), v.Error):
			fails = append(fails, fmt.Sprintf("invalid %s: error %q does not contain %q", v.Name, err, v.Error))
		}
	}
	return append(fails, vf.verifyTraced()...)
}
