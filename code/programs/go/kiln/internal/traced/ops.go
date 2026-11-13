package traced

import (
	"strings"

	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/trace"
)

// This file turns the path-taking calls into observations and writes
// (KILN04 section 4.4's table, section 5.2's rules).

// flagSet splits "A|B|C" into its members.
func flagSet(text string) map[string]bool {
	out := map[string]bool{}
	for _, f := range strings.Split(text, "|") {
		if f = strings.TrimSpace(f); f != "" {
			out[f] = true
		}
	}
	return out
}

// openFlags is the part of an open's flags that matters here.
type openFlags struct {
	wronly, rdwr, creat, excl, trunc, nofollow, tmpfile bool
}

// openat2Field returns the text of a field ("flags", "resolve") of the struct
// openat2 takes, "" when absent.
func openat2Field(text, name string) string {
	i := strings.Index(text, name+"=")
	if !strings.HasPrefix(text, "{") || i < 0 {
		return ""
	}
	v := text[i+len(name)+1:]
	if j := strings.IndexAny(v, ",}"); j >= 0 {
		v = v[:j]
	}
	return strings.TrimSpace(v)
}

// parseOpenFlags reads open/openat/creat flags, and the "flags=" field of the
// struct openat2 takes.  Numeric leftovers are ignored: strace names every bit
// it knows, and an unknown one cannot be a creation or a write we recognise.
func parseOpenFlags(text string) openFlags {
	if strings.HasPrefix(text, "{") {
		text = openat2Field(text, "flags")
	}
	s := flagSet(text)
	return openFlags{
		wronly: s["O_WRONLY"], rdwr: s["O_RDWR"], creat: s["O_CREAT"], excl: s["O_EXCL"],
		trunc: s["O_TRUNC"], nofollow: s["O_NOFOLLOW"], tmpfile: s["O_TMPFILE"],
	}
}

// writes reports whether the open can change the file: a write access mode,
// or the flags that create or empty it (section 4.4).
func (f openFlags) writes() bool {
	return !f.tmpfile && (f.wronly || f.rdwr || f.creat || f.trunc)
}

// metaFollow says whether a metadata call follows a symlink in the last
// component.
func metaFollow(ev *trace.Event) bool {
	switch ev.Syscall {
	case "lstat", "readlink", "readlinkat", "lgetxattr", "llistxattr":
		return false
	}
	f := flagSet(ev.Flags)
	return !(f["AT_SYMLINK_NOFOLLOW"] || f["IN_DONT_FOLLOW"])
}

// writeFollow is metaFollow for the calls of the Write kind that change an
// existing object.
func writeFollow(ev *trace.Event) bool {
	switch ev.Syscall {
	case "lchown", "lsetxattr", "lremovexattr":
		return false
	}
	return !flagSet(ev.Flags)["AT_SYMLINK_NOFOLLOW"]
}

// open handles open, openat, openat2 and creat.
//
// The kernel's answer is the descriptor's -y annotation: the resolved path of
// what was really opened.  The walk still runs, because it finds the symlinks
// on the way (each one is an observation) and checks itself against the
// kernel: if the post-run disk walks to a different place than the kernel
// went, something changed under the action and the path is unstable.
//
// A successful open is the action's own file, NOT an observation, when its
// first touch provably started from empty (section 5.2):
//
//	creat(...)                  empties the file
//	O_CREAT|O_EXCL              fails unless the file is new
//	O_TRUNC                     empties the file
//	O_CREAT after an ENOENT     the action itself saw nothing there
//
// Every other open observes whatever was there, including a plain
// O_WRONLY|O_CREAT, which cannot tell a new file from an old one.
func (c *classifier) open(ev *trace.Event) {
	f := parseOpenFlags(ev.Flags)
	if ev.Syscall == "openat2" {
		// RESOLVE_NO_SYMLINKS, RESOLVE_BENEATH, RESOLVE_IN_ROOT, ... change
		// what a path means in ways the walk does not model.
		if v := openat2Field(ev.Flags, "resolve"); v != "" && v != "0" {
			c.reason(rResolveFlags)
		}
	}
	c.noFinals = ev.Result.OK && strings.HasPrefix(ev.ReturnFDPath, "/")
	tg, ok := c.targetOf(ev, !f.nofollow && !(f.creat && f.excl))
	c.noFinals = false
	if !ok {
		return
	}
	if !ev.Result.OK {
		c.observeResult(tg, ev.Result)
		return
	}
	if tg.virtual {
		c.openVirtual(ev, tg, f)
		return
	}
	abs := tg.abs
	ret := ev.ReturnFDPath
	if !strings.HasPrefix(ret, "/") {
		// The descriptor is a pipe, a socket or an anonymous inode, so the
		// kernel gives no path to prefer.  A FIFO or socket FILE opened by name
		// is still a path the action looked at: classify the walked one.
		c.openResolved(abs, f, ev.Syscall == "creat")
		return
	}
	if ret != abs {
		_, a := c.rel(abs)
		_, b := c.rel(ret)
		if a || b {
			c.reason(rUnstable)
		}
		abs = ret
	}
	c.openResolved(abs, f, ev.Syscall == "creat")
}

// parentTrusted says an ENOENT on a child of abs's parent can be believed: the
// parent is in the repository (the tracked tree and the stability rule vouch
// for it) or is the action's own.  Elsewhere a missing name may be a missing
// parent.
func (c *classifier) parentTrusted(abs string) bool {
	p := parentOf(abs)
	if _, ok := c.rel(p); ok {
		return true
	}
	return c.isCreated(p)
}

// openResolved applies the created-from-empty rule to a successfully opened
// path and records the write, if any.
func (c *classifier) openResolved(abs string, f openFlags, creat bool) {
	wasAbsent := c.absent[abs] && c.parentTrusted(abs)
	clean := !f.tmpfile && (creat || (f.creat && f.excl) || f.trunc || (f.creat && wasAbsent))
	if !clean {
		c.observe(abs, true, false)
	} else {
		if f.creat || creat {
			c.creating(abs)
		}
		c.created[abs] = true
		delete(c.absent, abs)
	}
	if f.creat || creat {
		// An O_CREAT open may have made the name: a namespace change.  Only
		// with O_EXCL does the result prove it did.
		c.nsChange(abs, f.creat && f.excl)
	}
	if f.writes() || creat {
		c.writePath(abs)
	}
}

// openVirtual handles a successful open under /proc or /dev/fd.  An open of
// /proc/self/fd/N or /dev/fd/N is classified by the -y path of the descriptor
// it returned (section 5.1): that is the file the action really read.
func (c *classifier) openVirtual(ev *trace.Event, tg target, f openFlags) {
	if isFDLink(tg.abs) {
		if strings.HasPrefix(ev.ReturnFDPath, "/") {
			c.openResolved(ev.ReturnFDPath, f, false)
		}
		return
	}
	c.observe(tg.abs, true, false)
}

// isFDLink matches /proc/self/fd/N and /dev/fd/N.
func isFDLink(abs string) bool {
	rest, ok := strings.CutPrefix(abs, "/proc/self/fd/")
	if !ok {
		rest, ok = strings.CutPrefix(abs, "/dev/fd/")
	}
	if !ok || rest == "" {
		return false
	}
	for _, r := range rest {
		if r < '0' || r > '9' {
			return false
		}
	}
	return true
}

// write handles the Write kind.
//
// A successful change to a path the action did not make also OBSERVES it: the
// call only worked because something was there, and what that was is an input
// (section 4.4).  A call whose path lies in /proc or /dev/fd is refused: the
// path says nothing about the object.
func (c *classifier) write(ev *trace.Event) {
	switch ev.Syscall {
	case "rename", "renameat", "renameat2":
		c.rename(ev)
		return
	case "link", "linkat":
		c.link(ev)
		return
	}
	follow := writeFollow(ev)
	switch ev.Syscall {
	case "mkdir", "mkdirat", "mknod", "mknodat", "symlink", "symlinkat", "unlink", "unlinkat", "rmdir":
		follow = false
	}
	if !follow && ev.Result.OK {
		// A call that succeeded on a name it does not follow treated a trailing
		// slash as "this must be a directory", not as "follow the link".
		e2 := *ev
		e2.Path = stripSlashes(ev.Path)
		ev = &e2
	}
	tg, ok := c.targetOf(ev, follow)
	if !ok {
		return
	}
	if tg.virtual {
		c.reason(rProc)
		return
	}
	if !ev.Result.OK {
		// A failed write is an observation of the path, never a write:
		// EEXIST says it was there, ENOENT that it was not.
		c.observeResult(tg, ev.Result)
		return
	}
	switch ev.Syscall {
	case "mkdir", "mkdirat":
		c.writePath(tg.abs)
		c.markCreated(tg.abs, true)
		c.dirs[tg.abs] = true
	case "mknod", "mknodat":
		c.writePath(tg.abs)
		c.markCreated(tg.abs, false)
	case "symlink", "symlinkat":
		c.writePath(tg.abs)
		c.markCreated(tg.abs, true)
		c.links[tg.abs] = ev.Target
	case "unlink", "unlinkat", "rmdir":
		c.observe(tg.abs, true, false)
		c.writePath(tg.abs)
		c.markRemoved(tg.abs)
	default:
		// chmod, chown, truncate, utimes, setxattr, fchmod, ...: the object is
		// changed in place, so it existed and its first-touch state is not ours.
		c.observe(tg.abs, true, false)
		c.writePath(tg.abs)
	}
}

// rename handles rename, renameat and renameat2.  The source is observed (its
// content is now where the destination is) and written (it is gone); the
// destination is written, and is the action's own from now on.
func (c *classifier) rename(ev *trace.Event) {
	srcPath, dstPath := ev.Path, ev.Path2
	if ev.Result.OK {
		srcPath, dstPath = stripSlashes(srcPath), stripSlashes(dstPath)
	}
	src, ok1 := c.resolve(join(ev.Cwd, ev.DirfdPath, srcPath), false)
	dst, ok2 := c.resolve(join(ev.Cwd, ev.Dirfd2Path, dstPath), false)
	if !ok1 || !ok2 {
		// One name could not be placed (see resolve); a failed call still
		// looked at the other.
		if !ev.Result.OK {
			if ok1 {
				c.observe(src.abs, presence(ev.Result), false)
			}
			if ok2 {
				c.observe(dst.abs, presence(ev.Result), false)
			}
		}
		return
	}
	if src.virtual || dst.virtual {
		c.reason(rProc)
		return
	}
	if !ev.Result.OK {
		// An ENOENT could be either name or either parent: observe both, as
		// ambiguous, and remember nothing as absent.
		c.observe(src.abs, presence(ev.Result), false)
		c.observe(dst.abs, presence(ev.Result), false)
		return
	}
	c.observe(src.abs, true, false)
	c.writePath(src.abs)
	c.writePath(dst.abs)
	srcLink, srcIsLink := c.links[src.abs]
	if flagSet(ev.Flags)["RENAME_EXCHANGE"] {
		// Both names existed before and still do: neither is created.
		c.observe(dst.abs, true, false)
		dstLink, dstIsLink := c.links[dst.abs]
		c.nsChange(src.abs, false)
		c.nsChange(dst.abs, false)
		delete(c.links, src.abs)
		delete(c.links, dst.abs)
		if dstIsLink {
			c.links[src.abs] = dstLink
		}
		if srcIsLink {
			c.links[dst.abs] = srcLink
		}
		return
	}
	srcDir := c.dirs[src.abs]
	c.markRemoved(src.abs)
	c.markCreated(dst.abs, false)
	if srcIsLink {
		c.links[dst.abs] = srcLink
	}
	if srcDir {
		c.dirs[dst.abs] = true
	}
}

// link handles link and linkat.  Like rename, the source is both an
// observation and a write (section 4.4); the destination is created.  Both
// names become aliases of one file, which the trace cannot follow, so neither
// may later be relied on as a stable component.
func (c *classifier) link(ev *trace.Event) {
	followSrc := flagSet(ev.Flags)["AT_SYMLINK_FOLLOW"]
	srcPath, dstPath := ev.Path, ev.Path2
	if ev.Result.OK {
		dstPath = stripSlashes(dstPath)
		if !followSrc {
			srcPath = stripSlashes(srcPath)
		}
	}
	src, ok1 := c.resolve(join(ev.Cwd, ev.DirfdPath, srcPath), followSrc)
	dst, ok2 := c.resolve(join(ev.Cwd, ev.Dirfd2Path, dstPath), false)
	if !ok1 || !ok2 {
		// One name could not be placed (see resolve); a failed call still
		// looked at the other.
		if !ev.Result.OK {
			if ok1 {
				c.observe(src.abs, presence(ev.Result), false)
			}
			if ok2 {
				c.observe(dst.abs, presence(ev.Result), false)
			}
		}
		return
	}
	if src.virtual || dst.virtual {
		c.reason(rProc)
		return
	}
	if !ev.Result.OK {
		c.observe(src.abs, presence(ev.Result), false)
		c.observe(dst.abs, presence(ev.Result), false)
		return
	}
	c.hard[src.abs], c.hard[dst.abs] = true, true
	c.observe(src.abs, true, false)
	c.writePath(src.abs)
	c.writePath(dst.abs)
	srcLink, srcIsLink := c.links[src.abs]
	c.markCreated(dst.abs, false)
	if srcIsLink {
		c.links[dst.abs] = srcLink
	}
}

// stripSlashes drops trailing slashes (keeping a lone "/").
func stripSlashes(p string) string {
	if q := strings.TrimRight(p, "/"); q != "" {
		return q
	}
	return p
}
