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

// absentOK says an ABSENT probe of p is allowed by the absent-ok list.
//
// An exact probe of an entry is judged by the traced syscall result (the caller
// has already established it was absent).  A probe of a CHILD of an entry (say
// /sys/fs/selinux/enforce) is allowed only if the entry itself is absent on the
// classifier's disk: Lstat of it gives ENOENT or ENOTDIR.  Any other answer
// (present, EACCES, ...) means the entry is a real directory whose contents are
// undeclared input, so the probe is a host-read (KILN04 D27, section 5.3).
// An entry inside the fresh $HOME or TMPDIR is absent by construction (the
// sandbox's tmpfs), and the disk is never asked beneath them (section 5.1).
func (c *classifier) absentOK(p string) bool {
	for _, a := range c.cfg.absentOK {
		if p == a {
			return true
		}
		if !covers(a, p) {
			continue
		}
		// This shortcut relies on the fresh-tmpfs model: $HOME and TMPDIR start
		// empty in the sandbox, so an entry beneath them is absent unless the
		// action itself created it (isCreated handles that) or the host
		// pre-populated it (the state-preloaded void catches a real
		// pre-existing entry before any step runs).
		if c.underTmp(a) {
			return true
		}
		if _, err := c.cfg.FS.Lstat(a); err != nil && isNotExist(err) {
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
	case procMagicBeyond(p):
		// A magic link with more path after it: the name says nothing of the
		// object (D28), whatever the ephemeral list matches.
		c.reasonAt(rProc, p)
	case c.ephemeralMatch(p):
	default:
		c.reasonAt(rProc, p)
	}
}

// procParts splits a /proc or /dev/fd name into the components after
// /proc/<pid|self|thread-self>.  /dev/fd is a link to /proc/self/fd, so
// /dev/fd/N/x is /proc/self/fd/N/x.  ok is false for any other name.
func procParts(p string) (rest []string, ok bool) {
	parts := splitPath(p)
	if len(parts) >= 2 && parts[0] == "dev" && parts[1] == "fd" {
		return append([]string{"fd"}, parts[2:]...), true
	}
	if len(parts) < 3 || parts[0] != "proc" {
		return nil, false
	}
	return parts[2:], true
}

// procMagicBeyond says p passes THROUGH a /proc magic link: cwd, root, exe,
// fd/N, map_files/X or ns/X is followed by a further component.  Such a path
// names whatever the link points at, which the lexical text does not show
// (D28).  A magic link as the FINAL component (/proc/self/exe) is a plain
// name here and is judged by the ephemeral list.  The scan covers every
// depth, so /proc/self/task/5/cwd/x and /proc/self/task/root/x both count.
func procMagicBeyond(p string) bool {
	rest, ok := procParts(p)
	if !ok {
		return false
	}
	for i, n := range rest {
		switch n {
		case "cwd", "root", "exe":
			if i < len(rest)-1 {
				return true
			}
		case "fd", "map_files", "ns":
			if i < len(rest)-2 {
				return true
			}
		}
	}
	return false
}

// procMagicFinal says p's last component IS a magic link: cwd, root, exe,
// fd/N, map_files/X or ns/X.  Opening one gives a descriptor on the link's
// target, so the open needs strace's -y path to say what that is (D28).
func procMagicFinal(p string) bool {
	rest, ok := procParts(p)
	if !ok || len(rest) == 0 {
		return false
	}
	switch rest[len(rest)-1] {
	case "cwd", "root", "exe":
		return true
	}
	if len(rest) >= 2 {
		switch rest[len(rest)-2] {
		case "fd", "map_files", "ns":
			return true
		}
	}
	return false
}

// observeHost applies the host-root rules to an out-of-repository path the
// action did not create.
//
// Two rules keep the sandbox's scaffolding from being an input (KILN04 D39).
// A present, non-listing look at a PROPER ANCESTOR of an anchor (checkout,
// declared root, Home, Tmpdir, the literal prefix of an ephemeral or absent-ok
// entry) learns only that node's type and metadata.  And an absent probe
// strictly inside the fresh Home or Tmpdir is a constant: the tmpfs starts
// empty and the trace says when the action made a name.
func (c *classifier) observeHost(abs string, present, listing bool) {
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
	if !present && c.underTmp(abs) {
		return
	}
	if r := c.rootOf(abs); r != nil {
		c.observeRoot(r, abs, present)
		return
	}
	// A directory that is an ancestor of an anchor is a real directory by
	// construction.  A stat, access, readlink, chdir or O_DIRECTORY open shows
	// its type and metadata only; a listing shows sibling names, which are host
	// state, so it falls through.  A child of an ancestor (/home/go.work) is
	// not an ancestor and is judged by the rules below.  An ancestor inside a
	// declared root (nested anchors) was handled by the root's own rule above.
	if present && !listing && c.cfg.ancestors[abs] {
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
