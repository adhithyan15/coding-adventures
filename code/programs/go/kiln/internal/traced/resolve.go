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
// Lookups use, in this order:
//
//  1. the symlinks THIS action created, as the trace has them at this moment
//     (the post-run disk may have forgotten a link the action removed);
//  2. the post-run disk (Lstat, Readlink);
//  3. the tracked tree, for a path the disk no longer has.
//
// A component the disk does not have simply ends the walk: the kernel would
// have stopped there too (ENOENT), so the rest is kept as written.

// maxHops is the kernel's limit on symlinks followed in one lookup.
const maxHops = 40

type node struct {
	kind   NodeKind
	target string
}

// comp is a component a walk went through.
type comp struct {
	path  string
	exact bool // the action's own trace said what it was, at that moment
}

type walkResult struct {
	final   string // the resolved absolute path
	links   []comp // every symlink crossed, in order
	dirs    []comp // every directory crossed
	virtual bool   // the path lies in /proc or /dev/fd: not walked
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

// lookup says what abs is.  exact reports that the answer comes from the
// action's own trace, which knows the state AT THIS MOMENT, rather than from
// the post-run disk, which knows the state at the end.  An unreadable path is a
// reason and reads as absent.
func (c *classifier) lookup(abs string) (n node, exact bool) {
	if t, ok := c.links[abs]; ok {
		return node{kind: KindLink, target: t}, true
	}
	if c.dirs[abs] {
		return node{kind: KindDir}, true
	}
	if c.gone[abs] {
		return node{}, true
	}
	if n, ok := c.cache[abs]; ok {
		return n, false
	}
	n = c.lookupFS(abs)
	c.cache[abs] = n
	return n, false
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
	for len(queue) > 0 {
		name := queue[len(queue)-1]
		queue = queue[:len(queue)-1]
		switch name {
		case ".":
			continue
		case "..":
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
		n, exact := c.lookup(next)
		switch n.kind {
		case KindLink:
			if last && !follow {
				cur = next
				continue
			}
			hops++
			if hops > maxHops {
				return res, rLoop
			}
			res.links = append(res.links, comp{next, exact})
			if strings.HasPrefix(n.target, "/") {
				cur = ""
			}
			queue = append(queue, reverse(splitPath(n.target))...)
		case KindDir:
			res.dirs = append(res.dirs, comp{next, exact})
			cur = next
		default: // a file, a device, or nothing: the walk ends here
			cur, plain = next, true
		}
	}
	res.final = cur
	if res.final == "" {
		res.final = "/"
	}
	return res, ""
}

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
// path.
func (c *classifier) resolve(full string, follow bool) (target, bool) {
	w, bad := c.walk(full, follow)
	if bad != "" {
		c.reason(bad)
		return target{}, false
	}
	for _, l := range w.links {
		c.noteTraversal(l.path, l.exact)
		c.observe(l.path, true, false)
	}
	for _, d := range w.dirs {
		c.noteTraversal(d.path, d.exact)
	}
	return target{abs: w.final, virtual: w.virtual}, true
}
