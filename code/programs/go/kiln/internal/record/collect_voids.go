package record

// The two void tokens `collect` adds that the checks after the build do not
// (KILN04 4.8; merge rules M3 and M4)
// =======================================================================
//
//	survivor         RUN-void (M3). A process of the sandbox was still alive
//	                 when `collect` looked (collect_survivor.go). It may write
//	                 at any moment, so no write set of the run can be believed.
//	state-preloaded  SHARD-void (M4). A state root held something when the first
//	                 step of the shard started (`traces/.state-preloaded`, left
//	                 by RunStep at sequence number 0, P12), so a cache the
//	                 recording restores may have fed an action's reads without
//	                 the trace showing where they came from.
//
// The header token is the only thing the shard carries; what the operator needs
// to find out WHY goes into the command line's problems, bounded and printable.

import (
	"errors"
	"fmt"
	"os"
	"path/filepath"
)

// StatePreloadedMarker is the file RunStep leaves in traces/ when, at sequence
// number 0, a state root was not empty. It holds the path of the first state
// root that was not.
const StatePreloadedMarker = ".state-preloaded"

// The tokens, as the merge knows them.
const (
	voidSurvivor       = "survivor"
	voidStatePreloaded = "state-preloaded"
)

// markStatePreloaded is RunStep's seq-0 check: the first state root (in the
// configuration's order) that is not absent or an empty real directory gets named
// in traces/.state-preloaded, written O_EXCL (so a second writer, which cannot
// exist under the serial lock, would not overwrite). An existing marker is left
// as it is. Every kind of failure of stateRootEmpty counts, not only a
// non-empty directory: a root that cannot be looked at cannot be shown empty.
func markStatePreloaded(dir string, stateRoots []string) error {
	for _, r := range stateRoots {
		if stateRootEmpty(r, os.Lstat, os.ReadDir) == nil {
			continue
		}
		err := writeExcl(filepath.Join(dir, "traces", StatePreloadedMarker), []byte(printable(r)))
		if err != nil && !errors.Is(err, os.ErrExist) {
			return fmt.Errorf("traces/%s: %w", StatePreloadedMarker, err)
		}
		return nil
	}
	return nil
}

// noteVoids adds the survivor and state-preloaded tokens, and the reasons, to
// the post-build result. survivors is what the scan found (empty: none).
func (c *collector) noteVoids(post *postResult, survivors []string) {
	if len(survivors) > 0 {
		post.void = uniqSorted(append(post.void, voidSurvivor))
		for _, s := range survivors {
			post.note("survivor: %s", s)
		}
	}
	// ANY entry of that name voids the shard, whatever it is, and so does a
	// traces/ that cannot be asked: "no marker" must be a fact.
	marker := filepath.Join(c.dir, "traces", StatePreloadedMarker)
	_, err := os.Lstat(marker)
	switch {
	case err == nil:
		why := "a state root was not empty when the first step started"
		if data, rerr := readBoundedFile(marker, 4096); rerr == nil && len(data) > 0 {
			why += ": " + string(data)
		}
		post.note("state-preloaded: %s", why)
	case errors.Is(err, os.ErrNotExist):
		return
	default:
		post.note("state-preloaded: traces/%s cannot be examined: %v", StatePreloadedMarker, err)
	}
	post.void = uniqSorted(append(post.void, voidStatePreloaded))
}
