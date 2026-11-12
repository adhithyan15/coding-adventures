package legacy

import (
	"fmt"
	"io"
)

// Paths of the committed policy files (KILN03 section 7).  They have
// different owners and are deliberately separate files.
const (
	PinnedPath = "code/kiln/legacy/pinned.json"
	NeverPath  = "code/kiln/legacy/never.json"
	// BoundaryPath is the old build tool's declaration of repository inputs
	// that packages consume from outside their own directory.
	BoundaryPath = "code/specs/fixtures/build-tool-v1/repository-source-input-boundary.json"
	// BuildToolPath is the old build tool's source: the executor rewrites and
	// splits BUILD lines, so its source is part of what runs (KILN03 3.4).
	BuildToolPath = "code/programs/go/build-tool"

	maxPolicyBytes = 1 << 20
)

// policy is the pair of committed lists a plan consults.
type policy struct {
	pinned map[string]bool // languages whose toolchain pinning has landed
	never  map[string]bool // packages that are always uncacheable
}

type pinnedFile struct {
	Languages []string `json:"languages"`
}

type neverFile struct {
	Packages []string `json:"packages"`
}

// readSmall reads a whole tracked file through the Source, capped.
func readSmall(src Source, path string, limit int64) ([]byte, error) {
	rc, err := src.Open(path)
	if err != nil {
		return nil, fmt.Errorf("%s: %w", path, err)
	}
	defer rc.Close()
	data, err := io.ReadAll(io.LimitReader(rc, limit+1))
	if err != nil {
		return nil, fmt.Errorf("%s: %w", path, err)
	}
	if int64(len(data)) > limit {
		return nil, fmt.Errorf("%s exceeds the %d byte limit", path, limit)
	}
	return data, nil
}

// loadPolicy reads pinned.json and never.json.  A missing or malformed file is
// an error: treating "no file" as "nothing pinned, nothing banned" would let a
// deleted never.json silently re-admit a network-dependent package.  Every
// never-listed name must be a package of the graph, so a typo cannot silently
// disarm the list.
func loadPolicy(src Source, names map[string]bool) (*policy, error) {
	pol := &policy{pinned: map[string]bool{}, never: map[string]bool{}}
	data, err := readSmall(src, PinnedPath, maxPolicyBytes)
	if err != nil {
		return nil, err
	}
	var pf pinnedFile
	if err := strictDecode(data, &pf); err != nil {
		return nil, fmt.Errorf("%s: %w", PinnedPath, err)
	}
	if err := checkSorted(PinnedPath+" languages", pf.Languages); err != nil {
		return nil, err
	}
	for _, l := range pf.Languages {
		pol.pinned[l] = true
	}
	data, err = readSmall(src, NeverPath, maxPolicyBytes)
	if err != nil {
		return nil, err
	}
	var nf neverFile
	if err := strictDecode(data, &nf); err != nil {
		return nil, fmt.Errorf("%s: %w", NeverPath, err)
	}
	if err := checkNameList(NeverPath+" packages", nf.Packages, names); err != nil {
		return nil, err
	}
	for _, n := range nf.Packages {
		pol.never[n] = true
	}
	return pol, nil
}
