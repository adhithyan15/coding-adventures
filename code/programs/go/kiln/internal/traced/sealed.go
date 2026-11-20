package traced

import (
	"path"
	"sort"
	"strings"
)

// The sealed root (KILN04 D43).
//
// A traced step runs in a NEW ROOT that internal/sandbox/plan.go builds from an
// empty tmpfs and then seals read-only before the pivot.  Nothing is in it that
// the plan did not mount, so most of the host's tree simply does not exist for
// the step:
//
//	/                       sealed read-only tmpfs: nobody can create /package.json
//	  home/runner/work/x/x  the checkout (a writable bind), under directories
//	                        the plan made only to mount it (ancestor-chain nodes)
//	  usr, etc, ...         the declared host roots, read-only binds
//	  $HOME, $TMPDIR        fresh writable tmpfs
//	  /tmp /var/tmp /run    fresh writable tmpfs (sandbox.ScratchDirs)
//	  /dev /proc            the minimal /dev and a fresh /proc;  /sys is allowed too
//
// The classifier runs AFTER the step, on the host, and its FS answers about the
// HOST's disk.  For a path the plan does not contain that answer is about a
// place the step could not see: the host's /run/systemd/resolve/stub-resolv.conf
// is not in the sandbox, and neither is the host's /package.json or its /var/run
// symlink.  So a probe of such a path is a constant of the plan, exactly as an
// absent probe under the fresh Home is a constant of the tmpfs (D39 rule 1b).
//
// The rule, in one sentence: inside the sandbox a path EXISTS only if it is
//
//	under the checkout;
//	under a declared host root or a HostBind;
//	Home, Tmpdir or a scratch directory, or under one of them;
//	a PROPER ANCESTOR of any of those (a directory the plan creates to mount
//	  them: the classifier's `ancestors` set, D39);
//	under /dev, /proc or /sys.
//
// Anything else is SANDBOX-ABSENT: lookup says so without asking the disk, and
// an ABSENT probe of it is not an input.  A probe that SUCCEEDED at such a path
// contradicts the model (the plan has nothing there), so it keeps the reasons
// it always had: host-read for a look, and for a write, host-read as well
// (writeHost).  The exists-set is a SUPERSET of what the plan mounts (it also
// admits every declared root whether or not the host has it, and /sys, which
// the plan does not mount), so a mistake in it can only keep today's behaviour,
// never invent an absence.
//
// What this rule must survive, and why it does (spec D43 has the full list):
//
//   - a stale cwd after a directory rename: the trace keeps the cwd as TEXT and
//     the kernel resolves ".." physically, so text that names a sandbox-absent
//     path may physically reach an existing directory.  Every probe allowed
//     here is remembered, and if the action ever renamed (or exchanged, or
//     could not place) anything but a regular file it created itself, ALL of
//     them are refused at the end (reshaped, checkStability).
//   - a symlink the action made: the walk follows it, so the path the probe
//     really names is judged, not the link's own text.
//   - ".." from a scratch directory: the scratch directory is a mount root, and
//     its parent is "/" in the sandbox as it is in the walk.
//   - a path under a writable scratch directory: absent only while the action
//     has not made it, which the trace knows (underTmp, which now covers the scratch directories, follows the same
//     created / links / dirs model as the fresh Home and Tmpdir).

// sealedOn says the action ran in the sealed new root (Config.ScratchDirs).
func (c *classifier) sealedOn() bool { return len(c.cfg.ScratchDirs) > 0 }

// underScratch says abs is strictly inside a scratch directory.
func (c *classifier) underScratch(abs string) bool {
	for _, d := range c.cfg.ScratchDirs {
		if strings.HasPrefix(abs, d+"/") {
			return true
		}
	}
	return false
}

// sandboxHas says abs may exist inside the sandbox (see the rule above).  It is
// deliberately generous: true means "judge it as before".
func (c *classifier) sandboxHas(abs string) bool {
	if abs == "/" || c.cfg.ancestors[abs] || covers(c.cfg.Root, abs) {
		return true
	}
	for i := range c.cfg.roots {
		if covers(c.cfg.roots[i].path, abs) {
			return true
		}
	}
	for _, p := range c.cfg.HostBinds {
		if covers(p, abs) {
			return true
		}
	}
	// A declared root that is a symlink on the HOST (merged /usr: /bin ->
	// usr/bin) sends the walk, which runs on the host's disk, to its target and
	// through the target's ancestors.  Judging those as before keeps the rule's
	// promise that a gap in the exists-set never changes a verdict: without
	// this, /bin/sh walked to /usr/bin/sh, which a policy without /usr calls
	// sandbox-absent, and an exec the kernel performed turned from a host-read
	// into an unstable path.
	for _, t := range c.declaredRootLinks() {
		if covers(t, abs) || strings.HasPrefix(t, abs+"/") {
			return true
		}
	}
	for _, d := range []string{c.cfg.Home, c.cfg.Tmpdir} {
		if d != "" && covers(d, abs) {
			return true
		}
	}
	for _, d := range c.cfg.ScratchDirs {
		if covers(d, abs) {
			return true
		}
	}
	// The ephemeral entries are /dev, /proc and /sys names in practice; any
	// other literal prefix a policy lists is admitted for the same reason.
	for _, p := range c.cfg.ephemeral {
		if covers(literalPrefix(p), abs) {
			return true
		}
	}
	for _, d := range []string{"/dev", "/proc", "/sys"} {
		if covers(d, abs) {
			return true
		}
	}
	return false
}

// declaredRootLinks resolves, once, every declared root that is a symlink on
// the host to the absolute path its chain ends at (at most 8 hops; an
// unreadable or looping chain adds nothing, and the walk itself reports it).
// It asks the FS directly and adds no reason: it only widens sandboxHas, which
// is the direction that keeps the pre-D43 verdict.
func (c *classifier) declaredRootLinks() []string {
	if c.rootLinksDone {
		return c.rootLinks
	}
	c.rootLinksDone = true
	for i := range c.cfg.roots {
		p := c.cfg.roots[i].path
		linked := false
		for hop := 0; hop < 8; hop++ {
			info, err := c.cfg.FS.Lstat(p)
			if err != nil || info.Kind != KindLink {
				break
			}
			t, err := c.cfg.FS.Readlink(p)
			if err != nil || t == "" {
				linked = false
				break
			}
			if !strings.HasPrefix(t, "/") {
				t = path.Join(path.Dir(p), t)
			}
			p, linked = path.Clean(t), true
		}
		if linked && p != "/" {
			c.rootLinks = append(c.rootLinks, p)
		}
	}
	return c.rootLinks
}

// sandboxAbsent says the sealed root cannot contain abs: it is outside the
// exists-set, so it is absent in every run of this plan.
func (c *classifier) sandboxAbsent(abs string) bool {
	return c.sealedOn() && !c.sandboxHas(abs)
}

// SandboxHas reports whether the sealed-root rule (D43) lets abs exist inside
// the sandbox for this configuration.  The recorder's tests use it to pin that
// everything the sandbox plan mounts is in the set.
func SandboxHas(cfg Config, abs string) (bool, error) {
	p, err := prepare(cfg)
	if err != nil {
		return false, err
	}
	return newClassifier(p).sandboxHas(abs), nil
}

// noteSealedProbe remembers an absent probe that D43 allowed, for the
// whole-action reshaped check.
func (c *classifier) noteSealedProbe(abs string) {
	if c.sealedProbes == nil {
		c.sealedProbes = map[string]bool{}
	}
	c.sealedProbes[abs] = true
}

// noteMove records a SUCCESSFUL rename or exchange for the sealed-root rule
// (D43), anywhere in the sandbox.  Only moving a regular file the action
// created itself cannot change where a ".." leads.  Moving a directory, or an
// object the trace has no history of, can leave a process's cwd TEXT stale.
func (c *classifier) noteMove(src, dst string, exchange bool) {
	ownFile := func(p string) bool {
		_, made := c.created[p]
		return made && !c.dirs[p]
	}
	if !ownFile(src) || (exchange && !ownFile(dst)) {
		c.reshaped = true
	}
}

// settleReshaped runs after every event.  If the action reshaped the namespace
// (reshaped), a cwd TEXT may be stale, so two things that were judged by the
// text alone are refused:
//
//   - every absent probe D43 allowed becomes the host-read it was before;
//   - a failed call that stopped at a missing component of its text path with a
//     ".." after it (the old stat of "../work/x/x/nope" from a cwd that was
//     renamed away) is an unplaced lookup, which was found while writing D43:
//     rule 1b of D39 let it through with no reason when the stop lay in the
//     fresh tmpfs, though the kernel started from the renamed directory.
func (c *classifier) settleReshaped() {
	if !c.reshaped {
		return
	}
	if c.dotdotAt != "" {
		c.reasonSub(rUnstable, "dotdot", c.dotdotAt)
	}
	paths := make([]string, 0, len(c.sealedProbes))
	for p := range c.sealedProbes {
		paths = append(paths, p)
	}
	sort.Strings(paths)
	c.opKind = "lookup" // census only
	for _, p := range paths {
		c.reason("host-read:" + c.redact(p))
	}
}
