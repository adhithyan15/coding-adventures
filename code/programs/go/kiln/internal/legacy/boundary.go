package legacy

import (
	"encoding/json"
	"fmt"
	"strings"

	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/key"
)

// boundary is one entry of the old build tool's repository-source-input
// boundary file: a set of packages (applies_to) and the repository paths they
// consume from outside their own directory (inputs).  KILN03 3.3 item 2 treats
// it as an extra source of reads, never the only one.
type boundary struct {
	exact, descendant, excluded []string
	inputs                      []string
}

type boundaryFile struct {
	SchemaVersion int `json:"schema_version"`
	Boundaries    []struct {
		ID        string          `json:"id"`
		AppliesTo json.RawMessage `json:"applies_to"`
		Inputs    []struct {
			Path string `json:"path"`
		} `json:"inputs"`
	} `json:"boundaries"`
}

type appliesTo struct {
	ExactRoots      []string `json:"exact_roots"`
	DescendantRoots []string `json:"descendant_roots"`
	ExcludedRoots   []string `json:"excluded_roots"`
}

// parseBoundaries decodes the boundary file.  The file belongs to the old
// tool and carries bookkeeping fields Kiln has no use for, so unknown fields at
// the top level and on entries are tolerated; `applies_to` is decoded
// STRICTLY, because a new kind of root that Kiln silently ignored would drop a
// package's inputs.
func parseBoundaries(data []byte) ([]boundary, error) {
	var bf boundaryFile
	if err := json.Unmarshal(data, &bf); err != nil {
		return nil, fmt.Errorf("%s: %w", BoundaryPath, err)
	}
	if bf.SchemaVersion != 1 {
		return nil, fmt.Errorf("%s: schema_version %d, want 1", BoundaryPath, bf.SchemaVersion)
	}
	out := make([]boundary, 0, len(bf.Boundaries))
	for _, b := range bf.Boundaries {
		var at appliesTo
		if err := strictDecode(b.AppliesTo, &at); err != nil {
			return nil, fmt.Errorf("%s: boundary %q applies_to: %w", BoundaryPath, short(b.ID), err)
		}
		bd := boundary{exact: at.ExactRoots, descendant: at.DescendantRoots, excluded: at.ExcludedRoots}
		for _, r := range [][]string{at.ExactRoots, at.DescendantRoots, at.ExcludedRoots} {
			for _, p := range r {
				if err := key.ValidatePath(p); err != nil {
					return nil, fmt.Errorf("%s: boundary %q root: %w", BoundaryPath, short(b.ID), err)
				}
			}
		}
		for _, in := range b.Inputs {
			if err := key.ValidatePath(in.Path); err != nil {
				return nil, fmt.Errorf("%s: boundary %q input: %w", BoundaryPath, short(b.ID), err)
			}
			bd.inputs = append(bd.inputs, in.Path)
		}
		out = append(out, bd)
	}
	return out, nil
}

// under reports whether rel is root or lies beneath it.
func under(rel, root string) bool {
	return rel == root || strings.HasPrefix(rel, root+"/")
}

// appliesTo reports whether the boundary covers the package directory rel.
//
// Reading choices, all toward over-approximation: a descendant root includes
// the root itself, and an excluded root removes the root and everything under
// it (the one place a boundary can REMOVE reads; shared ancestor files are
// still picked up by the ancestor-configuration rule).
func (b boundary) appliesTo(rel string) bool {
	for _, x := range b.excluded {
		if under(rel, x) {
			return false
		}
	}
	for _, x := range b.exact {
		if rel == x {
			return true
		}
	}
	for _, x := range b.descendant {
		if under(rel, x) {
			return true
		}
	}
	return false
}
