package record

// pre/classify.json (KILN04 section 4.8, decision P13)
// ====================================================
//
// `traced.Config` needs three policy inputs besides the tracked tree and the
// disk: the host roots (what an action may read outside the checkout and how
// such a read is judged), the pinned ecosystems, and, per action, which tools
// the weak key pins. They are properties of the COMMIT, not of the machine, and
// until P13 they were read from `legacy.Traced`, which needs Git and the whole
// policy. A step that classifies cannot build a `legacy.Traced`, so `prepare`
// writes the parsed values once, inside the sealed `pre/`:
//
//	{"schema":"kiln-record-classify/1",
//	 "host_roots":{"roots":[{"path":"/usr","kind":"image"}, ...],
//	               "ephemeral":["/dev/null", ...], "absent-ok":[...]},
//	 "pinned":["go","rust"]}
//
// The per-action tool names are in the action files (`tools`, schema
// `kiln-record-action/2`), because each action has its own.
//
// The decoder is strict like every file of `pre/`: no unknown key, no trailing
// data, no null list, every list strictly sorted. `prepare` has already
// validated host-roots.json against toolchains.json (legacy.ParseHostRoots);
// the decoder here checks only the shape, because the file is sealed and cannot
// have changed since.

import (
	"bytes"
	"encoding/json"
	"fmt"
	"io"
	"sort"

	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/legacy"
)

// ClassifySchema is the schema of pre/classify.json.
const ClassifySchema = "kiln-record-classify/1"

// ClassifyFile is pre/classify.json.
type ClassifyFile struct {
	Schema    string           `json:"schema"`
	HostRoots legacy.HostRoots `json:"host_roots"`
	Pinned    []string         `json:"pinned"`
}

func sortedUniqueStrings(what string, l []string) error {
	if l == nil {
		return fmt.Errorf("classify: %s is null", what)
	}
	for i := 1; i < len(l); i++ {
		if l[i-1] >= l[i] {
			return fmt.Errorf("classify: %s is not strictly sorted at %q", what, printable(l[i]))
		}
	}
	return nil
}

func (c *ClassifyFile) validate() error {
	if c.Schema != ClassifySchema {
		return fmt.Errorf("classify: schema %q", printable(c.Schema))
	}
	h := c.HostRoots
	if h.Roots == nil {
		return fmt.Errorf("classify: host_roots.roots is null")
	}
	for i, r := range h.Roots {
		if r.Path == "" || (i > 0 && h.Roots[i-1].Path >= r.Path) {
			return fmt.Errorf("classify: host_roots.roots is not strictly sorted by path at %q", printable(r.Path))
		}
		if r.Fetchers != nil {
			if err := sortedUniqueStrings("a fetchers list", r.Fetchers); err != nil {
				return err
			}
		}
	}
	for _, l := range []struct {
		what string
		xs   []string
	}{{"host_roots.ephemeral", h.Ephemeral}, {"host_roots.absent-ok", h.AbsentOK}, {"pinned", c.Pinned}} {
		if err := sortedUniqueStrings(l.what, l.xs); err != nil {
			return err
		}
	}
	return nil
}

// EncodeClassify validates and renders pre/classify.json for the parsed host
// roots and the pinned ecosystems (compact, no HTML escaping, no trailing
// newline, nil lists as []).
func EncodeClassify(h *legacy.HostRoots, pinned []string) ([]byte, error) {
	if h == nil {
		return nil, fmt.Errorf("classify: no host roots")
	}
	c := ClassifyFile{Schema: ClassifySchema, HostRoots: *h, Pinned: append([]string{}, pinned...)}
	sort.Strings(c.Pinned)
	if c.HostRoots.Roots == nil {
		c.HostRoots.Roots = []legacy.HostRoot{}
	}
	if c.HostRoots.Ephemeral == nil {
		c.HostRoots.Ephemeral = []string{}
	}
	if c.HostRoots.AbsentOK == nil {
		c.HostRoots.AbsentOK = []string{}
	}
	if err := c.validate(); err != nil {
		return nil, err
	}
	return marshalJSON(c)
}

// DecodeClassify reads pre/classify.json strictly.
func DecodeClassify(data []byte) (ClassifyFile, error) {
	var c ClassifyFile
	dec := json.NewDecoder(bytes.NewReader(data))
	dec.DisallowUnknownFields()
	if err := dec.Decode(&c); err != nil {
		return ClassifyFile{}, fmt.Errorf("classify: %w", err)
	}
	if _, err := dec.Token(); err != io.EOF {
		return ClassifyFile{}, fmt.Errorf("classify: unexpected data after the JSON value")
	}
	return c, c.validate()
}
