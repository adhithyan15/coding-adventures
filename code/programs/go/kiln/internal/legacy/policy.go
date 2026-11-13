package legacy

import (
	"fmt"
	"io"
)

// Paths of the first committed policy files (KILN03 section 7); toolchains.json,
// env.json and host-roots.json (toolchains.go) are the other three.  They have
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

	tc    *Toolchains // toolchains.json: pins, probes, roots (KILN03 4)
	env   *EnvPolicy  // env.json: the environment allowlist (KILN03 5)
	roots *HostRoots  // host-roots.json: digest only for now (KILN04 5.3)
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

// loadPolicy reads pinned.json, never.json, toolchains.json, env.json and
// host-roots.json.  A missing or malformed file is
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
	if data, err = readSmall(src, ToolchainsPath, maxPolicyBytes); err != nil {
		return nil, err
	}
	if pol.tc, err = ParseToolchains(data); err != nil {
		return nil, fmt.Errorf("%s: %w", ToolchainsPath, err)
	}
	if data, err = readSmall(src, EnvPath, maxPolicyBytes); err != nil {
		return nil, err
	}
	if pol.env, err = ParseEnvPolicy(data); err != nil {
		return nil, fmt.Errorf("%s: %w", EnvPath, err)
	}
	if data, err = readSmall(src, HostRootsPath, maxPolicyBytes); err != nil {
		return nil, err
	}
	if pol.roots, err = ParseHostRoots(data, pol.tc); err != nil {
		return nil, fmt.Errorf("%s: %w", HostRootsPath, err)
	}
	return pol, nil
}
