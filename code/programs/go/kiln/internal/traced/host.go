package traced

import (
	"strings"

	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/legacy"
	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/source"
)

// Reads outside the repository (KILN04 section 5.3) and the /proc rules of
// section 5.1.
//
// A manifest never stores an out-of-repository path.  What an action may read
// out there is fixed by host-roots.json, and each kind of root is covered by
// something in the weak key:
//
//	image      Host.ImageID(); a file a setup step touched after boot is not
//	tool       the tool's pin and probe, when the tool is in the weak key
//	state      a lockfile the action read (stage 5; for now every read fails)
//	ephemeral  nothing, by design: randomness, the CPU count
//
// Anything else is `host-read:<path>`, with $HOME and TMPDIR redacted.

// fetchersEnabled switches on the `state` rule (stage 5).  Until fetchers
// exist nothing can have filled a state root legitimately, so every read of
// one is a host-read.  Tests flip it to exercise the branch.
var fetchersEnabled = false

// redact replaces the scrubbed $HOME and TMPDIR prefixes of a path, so a
// reason never prints the runner's directory names, and makes the result safe
// to print.
func (c *classifier) redact(p string) string {
	best := ""
	for _, v := range []struct{ name, val string }{{"$HOME", c.cfg.Home}, {"$TMPDIR", c.cfg.Tmpdir}} {
		if v.val == "" || !(p == v.val || strings.HasPrefix(p, v.val+"/")) {
			continue
		}
		if len(v.val) > len(best) {
			best = v.name + p[len(v.val):]
		}
	}
	if best != "" {
		p = best
	}
	return sanitize(p)
}

// covers says whether root is dir or an ancestor of it.
func covers(root, p string) bool { return p == root || strings.HasPrefix(p, root+"/") }

// rootOf returns the longest host root covering p.
func (c *classifier) rootOf(p string) *preparedRoot {
	var best *preparedRoot
	for i := range c.cfg.roots {
		r := &c.cfg.roots[i]
		if covers(r.path, p) && (best == nil || len(r.path) > len(best.path)) {
			best = r
		}
	}
	return best
}

// ephemeralMatch checks the ephemeral list.  `X/**` also covers X itself: a
// stat or listing of the directory is as ubiquitous as reading beneath it.
func (c *classifier) ephemeralMatch(p string) bool {
	for _, pat := range c.cfg.ephemeral {
		if source.Match(pat, p) {
			return true
		}
		if base, ok := strings.CutSuffix(pat, "/**"); ok && p == base {
			return true
		}
	}
	return false
}

func (c *classifier) absentOK(p string) bool {
	for _, a := range c.cfg.absentOK {
		if covers(a, p) {
			return true
		}
	}
	return false
}

// observeVirtual applies the /proc rules.  Paths here are lexical: /proc is
// never walked, because the post-run disk's /proc/self is the classifier's own.
//
//	/proc/self/status, /proc/self/stat   they show TracerPid: trace-visible
//	on the ephemeral list                allowed (cpuinfo, meminfo, /dev/fd/N)
//	anything else, numeric pids included incomplete: the pids are the tracer's
func (c *classifier) observeVirtual(p string) {
	switch {
	case p == "/proc/self/status" || p == "/proc/self/stat":
		c.reason("trace-visible:" + sanitize(p))
	case c.ephemeralMatch(p):
	default:
		c.reason(rProc)
	}
}

// observeHost applies the host-root rules to an out-of-repository path the
// action did not create.
func (c *classifier) observeHost(abs string, present bool) {
	if c.ephemeralMatch(abs) {
		return
	}
	if c.isTmpRoot(abs) || (c.cfg.Home != "" && abs == c.cfg.Home) {
		// The fresh $HOME and TMPDIR directories themselves.  $HOME is also
		// allowed when the checkout lies beneath it (hosted runners): it is the
		// sandbox's tmpfs mount point, so the directory node is the same in
		// every run.  Only the node: nothing under it (section 5.5).
		return
	}
	if r := c.rootOf(abs); r != nil {
		c.observeRoot(r, abs, present)
		return
	}
	if !present && c.absentOK(abs) {
		return
	}
	c.reason("host-read:" + c.redact(abs))
}

func (c *classifier) observeRoot(r *preparedRoot, abs string, present bool) {
	switch r.kind {
	case legacy.RootEphemeral:
	case legacy.RootImage:
		// An absent probe is covered by the image too, but a setup step may
		// have ADDED the name's neighbours: the ctime of the deepest directory
		// that exists is what says so.
		for p := abs; covers(r.path, p); p = parentOf(p) {
			info, err := c.cfg.FS.Lstat(p)
			switch {
			case err == nil && (info.Ctime.IsZero()):
				// Cannot show it is as old as the image.
				c.reason("host-read:" + c.redact(abs))
			case err == nil && info.Ctime.After(c.cfg.JobStart):
				c.reason("host-read:setup-modified:" + c.redact(abs))
			case err != nil && isNotExist(err) && !present && p != r.path:
				continue
			case err != nil:
				c.reason("host-read:" + c.redact(abs))
			}
			return
		}
		c.reason("host-read:" + c.redact(abs))
	case legacy.RootTool:
		if !c.cfg.Tools[r.tool] {
			c.reason("host-read:tool-not-keyed:" + sanitize(r.tool))
		}
	case legacy.RootState:
		if !present {
			return // state roots start empty (section 5.3)
		}
		if !(fetchersEnabled && c.cfg.Pinned[r.ecosystem]) {
			c.reason("host-read:" + c.redact(abs))
		}
	default:
		c.reason("host-read:" + c.redact(abs))
	}
}

// writeHost handles a write outside the repository.  Only fetcher processes
// may write under a state root (section 5.3), and stage 5 is where a fetcher
// is first recognised, so every writer is a non-fetcher for now.  Any other
// out-of-repository write is to the action's own scratch space or fails on the
// read-only filesystem; neither is an input.
func (c *classifier) writeHost(abs string) {
	if r := c.rootOf(abs); r != nil && r.kind == legacy.RootState {
		c.reason("state-write:" + c.redact(abs))
	}
}
