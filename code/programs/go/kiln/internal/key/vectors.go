package key

import (
	"encoding/hex"
	"encoding/json"
	"fmt"
	"strings"
)

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
	dec := json.NewDecoder(strings.NewReader(string(data)))
	dec.DisallowUnknownFields()
	if err := dec.Decode(&vf); err != nil {
		return vf, fmt.Errorf("parse vectors: %w", err)
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
		k, _ := Key(a)
		if got := Hex(k); got != v.KeyHex {
			fails = append(fails, fmt.Sprintf("valid %s: key mismatch: got %s want %s", v.Name, got, v.KeyHex))
		}
	}
	for _, v := range vf.Invalid {
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
	return fails
}
