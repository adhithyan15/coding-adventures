package record

// The build tool's plan, as the recorder reads it (KILN04 section 4.8, stage
// 4b-1, decision P6)
// =========================================================================
//
// `build-tool -emit-plan` writes a build plan (build-plan-v1) that splits the
// packages into prerequisite-closed shards. Kiln needs three facts from it, and
// nothing else:
//
//	packages[]   name -> rel_path   (a shard lists NAMES, Kiln keys by PATH)
//	shards[]     index, the packages assigned to the shard, and the packages it
//	             RUNS (assigned plus their prerequisites)
//
// The plan is the build tool's file, not a Kiln format, so it is decoded
// loosely (an extra field the build tool adds later is not an error) but what is
// read is checked hard:
//
//   - exactly N shards, indices exactly 0..N-1, so a plan for another shard
//     count cannot be mistaken for this one;
//   - every package a shard names is a package of the plan, and every assigned
//     package is also one the shard runs;
//   - every rel_path is a valid Kiln path and no two packages share a name or a
//     path.
//
// The partition digest is derived from this projection (recfmt.PartitionDigest),
// not taken from the file's bytes, so two machines whose plan files differ in an
// irrelevant byte still agree on the partition.

import (
	"bytes"
	"encoding/json"
	"fmt"
	"io"
	"os"
	"sort"

	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/recfmt"
)

// MaxPlanBytes bounds the plan file; the real repository's is a few MiB.
const MaxPlanBytes = 256 << 20

// planLimit is the cap as enforced; a test lowers it instead of writing 256 MiB.
var planLimit int64 = MaxPlanBytes

type planFile struct {
	Packages []struct {
		Name    string `json:"name"`
		RelPath string `json:"rel_path"`
	} `json:"packages"`
	Shards []struct {
		Index    int      `json:"index"`
		Assigned []string `json:"assigned_packages"`
		Names    []string `json:"package_names"`
	} `json:"shards"`
}

// Partition is the plan projected onto package paths.
type Partition struct {
	Shards []recfmt.ShardPlan
}

// ReadPartition decodes the plan at p, which must hold exactly count shards.
func ReadPartition(p string, count int) (*Partition, error) {
	st, err := os.Lstat(p)
	if err != nil {
		return nil, err
	}
	if !st.Mode().IsRegular() {
		return nil, fmt.Errorf("plan %q is not a regular file", printable(p))
	}
	f, err := os.Open(p)
	if err != nil {
		return nil, err
	}
	defer f.Close()
	data, err := io.ReadAll(io.LimitReader(f, planLimit+1))
	if err != nil {
		return nil, err
	}
	if int64(len(data)) > planLimit {
		return nil, fmt.Errorf("plan exceeds %d bytes", planLimit)
	}
	return parsePartition(data, count)
}

func parsePartition(data []byte, count int) (*Partition, error) {
	var pf planFile
	dec := json.NewDecoder(bytes.NewReader(data))
	if err := dec.Decode(&pf); err != nil {
		return nil, fmt.Errorf("plan: %w", err)
	}
	if _, err := dec.Token(); err != io.EOF {
		return nil, fmt.Errorf("plan: unexpected data after the JSON value")
	}
	paths := make(map[string]string, len(pf.Packages)) // name -> rel_path
	seenPath := map[string]bool{}
	for _, pk := range pf.Packages {
		if err := recfmt.CheckPath(pk.RelPath); err != nil {
			return nil, fmt.Errorf("plan: package %q: %v", printable(pk.Name), err)
		}
		if pk.Name == "" {
			return nil, fmt.Errorf("plan: a package has no name")
		}
		if _, dup := paths[pk.Name]; dup || seenPath[pk.RelPath] {
			return nil, fmt.Errorf("plan: package %q or its path %q is listed twice", printable(pk.Name), printable(pk.RelPath))
		}
		paths[pk.Name] = pk.RelPath
		seenPath[pk.RelPath] = true
	}
	if len(pf.Shards) != count {
		return nil, fmt.Errorf("plan: has %d shards, want exactly %d", len(pf.Shards), count)
	}
	out := &Partition{}
	seenIdx := map[int]bool{}
	for _, sh := range pf.Shards {
		if sh.Index < 0 || sh.Index >= count || seenIdx[sh.Index] {
			return nil, fmt.Errorf("plan: shard index %d is out of range or repeated", sh.Index)
		}
		seenIdx[sh.Index] = true
		runs, err := projectNames(paths, sh.Names)
		if err != nil {
			return nil, fmt.Errorf("plan: shard %d: %w", sh.Index, err)
		}
		assigned, err := projectNames(paths, sh.Assigned)
		if err != nil {
			return nil, fmt.Errorf("plan: shard %d: %w", sh.Index, err)
		}
		inRuns := make(map[string]bool, len(runs))
		for _, r := range runs {
			inRuns[r] = true
		}
		for _, a := range assigned {
			if !inRuns[a] {
				return nil, fmt.Errorf("plan: shard %d is assigned %q but does not run it", sh.Index, printable(a))
			}
		}
		out.Shards = append(out.Shards, recfmt.ShardPlan{Index: sh.Index, Packages: runs, Assigned: assigned})
	}
	sort.Slice(out.Shards, func(a, b int) bool { return out.Shards[a].Index < out.Shards[b].Index })
	return out, nil
}

// projectNames maps package names to sorted rel_paths, refusing an unknown name
// or a repeat.
func projectNames(paths map[string]string, names []string) ([]string, error) {
	out := make([]string, 0, len(names))
	seen := map[string]bool{}
	for _, n := range names {
		rel, ok := paths[n]
		if !ok {
			return nil, fmt.Errorf("package %q is not in the plan's package list", printable(n))
		}
		if seen[rel] {
			return nil, fmt.Errorf("package %q is listed twice", printable(n))
		}
		seen[rel] = true
		out = append(out, rel)
	}
	sort.Strings(out)
	return out, nil
}

// Digest is the binding's partition_sha256.
func (p *Partition) Digest() (string, error) { return recfmt.PartitionDigest(p.Shards) }

// Runs returns the sorted rel_paths shard index runs.
func (p *Partition) Runs(index int) ([]string, error) {
	for _, s := range p.Shards {
		if s.Index == index {
			return append([]string(nil), s.Packages...), nil
		}
	}
	return nil, fmt.Errorf("plan has no shard %d", index)
}
