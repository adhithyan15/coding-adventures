package traced

import (
	"errors"
	"io/fs"
	"strings"
	"syscall"
)

// Resolution (KILN04 section 5.1).
//
// strace prints the path a program ASKED for.  The kernel looked at a
// different one when a symlink was in the way, so before anything is
// classified the path is walked component by component, the way the kernel
// does it:
//
//	cur = "/"                         resolved directory so far
//	for each component c of the path:
//	    "."   -> nothing
//	    ".."  -> the parent of the RESOLVED cur       (never lexical)
//	    else  -> look c up under cur:
//	               link  -> splice its target in front of the rest (<= 40 times)
//	               dir   -> cur = cur/c
//	               other -> cur = cur/c, the rest is plain text
//
// So `L/../x` where L is a link to /a/b means /a/x, beside the link's target,
// exactly as the kernel reads it.  Every link crossed is itself an
// observation: if it were retargeted, the same text would lead elsewhere.
//
// The walk reads the POST-RUN disk, which is only the truth about the call if
// nothing the walk went through changed in between.  Whether that holds is the
// stability rule (classify.go); the walk just reports what it crossed.
//
// Under $HOME and TMPDIR the disk is never asked: they are fresh sandbox
// tmpfs, gone after the run.  Only what the trace itself created is there.

// maxHops is the kernel's limit on symlinks followed in one lookup.
const maxHops = 40

// maxResolvedLen bounds a path while it is being resolved.
const maxResolvedLen = 4096

// walkBudget is the most component steps one action may spend walking paths
// (a variable so a test can lower it).  A hostile symlink farm can make each
// lookup cost 40 hops, so the total is bounded, not just each walk.
var walkBudget = 50_000_000

type node struct {
	kind   NodeKind
	target string
}

type walkResult struct {
	final string   // the resolved absolute path
	links []string // every symlink crossed, in order
	dirs  []string // every directory crossed
	// finals is the final component when it was FOLLOWED (a stat or open, not
	// an lstat or unlink) and exists: if it was a link at call time and is not
	// one now, the disk misleads, so the action must not have touched it.
	finals  []string
	virtual bool // the path lies in /proc or /dev/fd: not walked
	// reached says the final component was found by a lookup: the walk did not
	// go plain before it, and the path does not end in "/" or "/.".  Only then
	// does an ENOENT mean "this name is absent".
	reached bool
	// dotdot says a ".." came after the walk went plain (a missing component
	// or a file): where it leads cannot be known.  stop is that component.
	dotdot      bool
	stop        string
	stopPresent bool
	// stopFinal says the stop was the final name, and a name that is not
	// followed is the object itself: no link or directory behind it depends on
	// what the disk says.
	stopFinal bool
	// stopMid says the walk stopped BEFORE the final component.
	stopMid bool
}

func splitPath(p string) []string {
	var out []string
	for _, s := range strings.Split(p, "/") {
		if s != "" {
			out = append(out, s)
		}
	}
	return out
}

// parentOf is the parent of an absolute path; the root is "" and its own parent.
func parentOf(abs string) string {
	i := strings.LastIndexByte(abs, '/')
	if i <= 0 {
		return ""
	}
	return abs[:i]
}

func isVirtualRoot(abs string) bool { return abs == "/proc" || abs == "/dev/fd" }

func isVirtual(abs string) bool {
	return abs == "/proc" || strings.HasPrefix(abs, "/proc/") || abs == "/dev/fd" || strings.HasPrefix(abs, "/dev/fd/")
}

func isNotExist(err error) bool {
	return errors.Is(err, fs.ErrNotExist) || errors.Is(err, syscall.ENOTDIR)
}

// rel returns the repository-relative form of abs.
func (c *classifier) rel(abs string) (string, bool) {
	if abs == c.cfg.Root {
		return "", true
	}
	if strings.HasPrefix(abs, c.cfg.Root+"/") {
		return abs[len(c.cfg.Root)+1:], true
	}
	return "", false
}

// fresh says abs may lie in the fresh sandbox tmpfs.  The checkout is never
// fresh (hosted runners put it under $HOME), nor is an ancestor of it, nor
// anything under a declared host root (a `state` root such as
// $HOME/.cargo/registry persists).
func (c *classifier) fresh(abs string) bool {
	if covers(c.cfg.Root, abs) || strings.HasPrefix(c.cfg.Root, abs+"/") {
		return false
	}
	for i := range c.cfg.roots {
		if p := c.cfg.roots[i].path; covers(p, abs) || strings.HasPrefix(p, abs+"/") {
			return false
		}
	}
	return true
}

// underTmp says abs is strictly inside the fresh scrubbed HOME or TMPDIR.
func (c *classifier) underTmp(abs string) bool {
	for _, d := range []string{c.cfg.Home, c.cfg.Tmpdir} {
		if d != "" && strings.HasPrefix(abs, d+"/") && c.fresh(abs) {
			return true
		}
	}
	return false
}

func (c *classifier) isTmpRoot(abs string) bool {
	return abs != "" && (abs == c.cfg.Home || abs == c.cfg.Tmpdir) && c.fresh(abs)
}

// lookup says what abs is.  An unreadable path is a reason and reads as absent.
func (c *classifier) lookup(abs string) node {
	if c.isTmpRoot(abs) {
		return node{kind: KindDir}
	}
	if c.underTmp(abs) {
		// Fresh tmpfs: either the action made it (the trace knows) or it is
		// absent.  The post-run disk is never asked.
		if t, ok := c.links[abs]; ok {
			return node{kind: KindLink, target: t}
		}
		if c.dirs[abs] {
			return node{kind: KindDir}
		}
		if _, ok := c.created[abs]; ok {
			return node{kind: KindFile}
		}
		return node{}
	}
	if n, ok := c.cache[abs]; ok {
		return n
	}
	n := c.lookupFS(abs)
	c.cache[abs] = n
	return n
}

func (c *classifier) lookupFS(abs string) node {
	info, err := c.cfg.FS.Lstat(abs)
	switch {
	case err == nil:
		if info.Kind == KindLink {
			t, err := c.cfg.FS.Readlink(abs)
			if err != nil || t == "" {
				c.reason(rUnresolvable)
				return node{}
			}
			return node{kind: KindLink, target: t}
		}
		return node{kind: info.Kind}
	case isNotExist(err):
	default:
		c.reason(rUnresolvable)
		return node{}
	}
	if rel, ok := c.rel(abs); ok {
		t := c.cfg.Tree.Stat(rel)
		return node{kind: t.Kind, target: t.Target}
	}
	return node{}
}

// walk resolves the absolute path full.  follow says whether a symlink in the
// final position is followed (open, stat) or is itself the object (lstat,
// readlink, unlink, rename).  A trailing slash always follows.
func (c *classifier) walk(full string, follow bool) (walkResult, string) {
	var res walkResult
	if strings.HasSuffix(full, "/") {
		follow = true
	}
	queue := reverse(splitPath(full))
	cur := ""
	hops := 0
	plain := false // the rest is text: past a missing component, a file or /proc
	virtBase := ""
	slashTarget := false
	for len(queue) > 0 {
		c.steps++
		if c.steps > walkBudget || len(queue) > maxResolvedLen || len(cur) > maxResolvedLen {
			return res, rSize
		}
		name := queue[len(queue)-1]
		queue = queue[:len(queue)-1]
		switch name {
		case ".":
			continue
		case "..":
			if plain && !res.virtual {
				res.dotdot = true
				return res, ""
			}
			cur = parentOf(cur)
			if res.virtual && !strings.HasPrefix(cur+"/", virtBase+"/") {
				res.virtual, plain = false, false
			}
			continue
		}
		next := cur + "/" + name
		if isVirtualRoot(next) {
			cur, plain, res.virtual, virtBase = next, true, true, next
			continue
		}
		if plain {
			cur = next
			continue
		}
		last := len(queue) == 0
		n := c.lookup(next)
		switch n.kind {
		case KindLink:
			if last && !follow {
				cur = next
				res.reached = true
				continue
			}
			hops++
			if hops > maxHops {
				return res, rLoop
			}
			res.links = append(res.links, next)
			slashTarget = slashTarget || slashy(n.target)
			if strings.HasPrefix(n.target, "/") {
				cur = ""
			}
			queue = append(queue, reverse(splitPath(n.target))...)
		case KindDir:
			if last && follow {
				res.finals = append(res.finals, next)
			}
			if !last {
				// Only a directory the path goes THROUGH is a traversed
				// component; a final directory is the object, observed as such.
				res.dirs = append(res.dirs, next)
			}
			cur = next
			res.reached = last
		default: // a file, a device, or nothing: the walk ends here
			if last && follow && n.kind != KindAbsent {
				res.finals = append(res.finals, next)
			}
			cur, plain = next, true
			res.reached = last
			if !last || n.kind == KindAbsent {
				// A final FILE is the object itself; only a missing name or a
				// file in the middle of a path is a place the walk stopped.
				res.stop, res.stopPresent = next, n.kind != KindAbsent
				res.stopFinal = last && !follow
				res.stopMid = !last
			}
		}
	}
	res.final = cur
	if res.final == "" {
		res.final = "/"
	}
	// "x/" and "x/." name the directory x but ask the kernel more than a
	// lookup of the name, so the answer does not say the NAME is absent.
	if slashy(full) || slashTarget {
		res.reached = false
	}
	return res, ""
}

func slashy(p string) bool { return strings.HasSuffix(p, "/") || strings.HasSuffix(p, "/.") }

func reverse(s []string) []string {
	out := make([]string, len(s))
	for i, v := range s {
		out[len(s)-1-i] = v
	}
	return out
}

// target is a resolved call target.
type target struct {
	abs     string
	virtual bool
	reached bool
}

// join makes the absolute path expression of a path argument: absolute as
// given, else relative to the directory descriptor's -y path, else to the
// working directory expression.
func join(cwd, dirfd, p string) string {
	if strings.HasPrefix(p, "/") {
		return p
	}
	base := cwd
	if dirfd != "" {
		base = dirfd
	}
	return strings.TrimRight(base, "/") + "/" + p
}

// resolve walks full, records every link crossed as an observation and every
// link and directory crossed for the stability check, and returns the final
// path.  A ".." after a component the walk could not enter cannot be placed:
// on a failed call the component where the walk stopped is what the call
// looked at; on a successful one the call went somewhere unknown.
func (c *classifier) resolve(full string, follow bool) (target, bool) {
	w, bad := c.walk(full, follow)
	if bad != "" {
		c.reason(bad)
		return target{}, false
	}
	for _, l := range w.links {
		c.noteTraversal(l)
		c.observe(l, true, false)
	}
	for _, d := range w.dirs {
		c.noteTraversal(d)
	}
	// A successful open settles its final name: the descriptor's -y path is
	// the kernel's own, so when the walk ends at exactly that path the name was
	// not a link at the moment of the open.  Only the FINAL component: every
	// link and directory crossed was noted above and keeps the strict rule.
	settled := c.openRet != "" && w.final == c.openRet
	for _, f := range w.finals {
		switch {
		case settled:
		case c.relax && c.lookup(f).kind == KindFile:
			c.noteRel(c.relFinal, f)
		default:
			c.noteFinal(f)
		}
	}
	if w.stop != "" && !w.stopFinal {
		switch last := w.stop == w.final && !w.stopMid; {
		case last && settled:
		case last && c.relax && !w.stopPresent:
			c.noteRel(c.relAbs, w.stop)
		default:
			c.noteMissing(w.stop, w.stopPresent)
		}
	}
	if w.stopMid && c.callOK {
		// A successful call went THROUGH a component the post-run disk shows as
		// missing (or not a directory): the disk contradicts the call, so
		// something changed it, and nothing here can say what the call saw.
		c.reason(rUnstable)
	}
	if w.dotdot {
		if c.callOK {
			c.reason(rUnstable)
		} else {
			c.observe(w.stop, w.stopPresent, false)
		}
		return target{}, false
	}
	return target{abs: w.final, virtual: w.virtual, reached: w.reached}, true
}
