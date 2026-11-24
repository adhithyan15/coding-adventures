package traced

import (
	"debug/elf"
	"encoding/binary"
	"io/fs"
	"math/rand"
	"os"
	"sort"
	"strconv"
	"strings"
	"testing"

	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/trace"
)

// A DIFFERENTIAL FUZZER for the classifier.
//
// An in-memory filesystem MODEL executes random operations with Linux-like
// semantics, so it knows the TRUTH: which path each call really resolved to and
// which symlinks it really crossed.  It emits the trace events a tracer would
// see (true results, -y paths) and hands its FINAL state to the classifier as
// the post-run disk.
//
// What it models:
//
//   - two processes whose calls overlap in time: each operation has an
//     interval [start, end], its effect lands at a random point inside it, and
//     events are emitted in completion order, a long call carrying the line of
//     its "<unfinished ...>" marker as StartLine;
//   - one or two BUILD steps, each with its own trace, over one filesystem;
//   - randomised Root (also beneath HOME, as on hosted runners), HOME and TMPDIR;
//   - tracked files and links, pre-existing UNTRACKED leftovers (file,
//     directory, link), host files, and files whose contents change so a later
//     exec reads different bytes;
//   - mkdir, symlink, rename, RENAME_EXCHANGE, link, unlink, rmdir, open (read,
//     truncate+create, exclusive create, create, truncate only), stat, lstat,
//     chdir, exec, directory listings and dirfd-relative lookups.
//
// The WRITES-COMPLETE oracle (KILN04 section 5.5, PR 4): whenever the record
// says WritesComplete, EVERY in-repository path the model really wrote lies
// under Writes or TrackedWrites, WITH OR WITHOUT reasons (a read-side reason
// must not be able to hide a write).  Perturbations aim at each false-trigger:
// writes through /proc/self/cwd (virtual), chmod of the root and of names
// reached through links and "..", and, after the run, a truncated trace, a
// killed step, a process with no exit line and a wrong recipe, each of which
// must make WritesComplete false.
//
// The oracle (KILN04 sections 5.1, 5.2, 7): when the record carries NO reason,
// every path the model really looked at must be the action's own or be covered
// by an entry (pkg:, path: on a prefix, an untracked observation, list: for a
// listing), every in-repository absence must appear in AbsentObservations,
// and every write must lie under a write root.  A record with a reason is never
// turned into a manifest, so it is always safe; the fuzzer hunts for records
// WITHOUT reasons that miss something.

type mnode struct {
	kind   NodeKind
	kids   map[string]*mnode
	target string
	data   []byte
}

func mdir() *mnode { return &mnode{kind: KindDir, kids: map[string]*mnode{}} }

type frame struct {
	n    *mnode
	path string
}

type touch struct {
	path   string
	create bool
	own    bool
	absent bool
	list   bool
	write  bool
	wonly  bool // a write record only: coverage is judged by the lookup's own touch
}

type mproc struct{ cwdPath, cwdExpr string }

type model struct {
	root, home, tmp string
	top             *mnode
	procs           map[int]*mproc
	own             map[string]bool
	touches         []touch
	dirs            []string // candidate directory descriptors
	opStart         int      // index of the first touch of the current operation
	core            bool     // a process dumped core (L1)
	sigpipe         bool     // a child was killed by SIGPIPE (M5)
	noNetNS         bool     // the run has no fresh network namespace (W11)
	// tmpLinked: a symbolic link was made (or moved) inside the fresh Home or
	// Tmpdir.  KILN04 D40: until one is, a lookup there is own-or-absent.
	tmpLinked bool
	// staleCwd holds the cwd texts a rename left pointing at a directory that is
	// no longer there (the kernel resolves ".." physically, the trace keeps
	// text).  unsound says a path was resolved relative to one: nothing the
	// model says about such a lookup is trustworthy, so the classifier must have
	// refused (KILN04 D40).
	staleCwd map[string]bool
	unsound  bool
	// reshaped is the model's own reading of KILN04 D43's whole-action taint:
	// a rename or exchange SUCCEEDED and moved a directory.  It is read off
	// the model's node kinds, which are the truth, not off the classifier's
	// history of what the action created.
	reshaped bool
	// dotdotProbes are the failed relative stats the model issued, as the cwd
	// text and the path: at the end the model walks them over the final disk
	// to see which stopped at a missing component with a ".." after it.
	dotdotProbes []dotdotProbe
	// execs are the execs of the step being built, in the order their effects
	// landed (per pid that is the trace's order too); stepExecs holds them per
	// step (KILN04 D48 step 5, exechead_model_test.go).
	execs     []execRec
	stepExecs [][]execRec
}

type dotdotProbe struct{ cwd, p string }

func (m *model) mkpath(abs string, n *mnode) {
	parts := splitPath(abs)
	cur := m.top
	for i, p := range parts {
		if i == len(parts)-1 {
			cur.kids[p] = n
			return
		}
		next := cur.kids[p]
		if next == nil {
			next = mdir()
			cur.kids[p] = next
		}
		cur = next
	}
}

// byPath finds a node by an already-resolved path (no link is followed).
func (m *model) byPath(abs string) *mnode {
	cur := m.top
	for _, p := range splitPath(abs) {
		if cur.kind != KindDir {
			return nil
		}
		cur = cur.kids[p]
		if cur == nil {
			return nil
		}
	}
	return cur
}

type mres struct {
	node   *mnode
	parent *mnode
	name   string
	final  string
	stop   string
	errno  string
	links  []string
	isDot  bool
}

func (m *model) framesAt(path string) []frame {
	fr := []frame{{m.top, ""}}
	cur := m.top
	p := ""
	for _, c := range splitPath(path) {
		cur = cur.kids[c]
		if cur == nil {
			break
		}
		if cur.kind != KindDir {
			// The cwd text now names a file: the process's real cwd is a
			// directory that was removed or moved away, like a stale cwd.
			m.unsound = true
			break
		}
		p += "/" + c
		fr = append(fr, frame{cur, p})
	}
	return fr
}

func pathOrRoot(p string) string {
	if p == "" {
		return "/"
	}
	return p
}

// resolve is the model's namei, from the directory at path `from` for a
// relative p.
func (m *model) resolve(p, from string, follow bool) mres {
	var r mres
	if p == "" {
		r.errno = "ENOENT"
		return r
	}
	trailing := strings.HasSuffix(p, "/")
	if trailing {
		follow = true
	}
	if !strings.HasPrefix(p, "/") && m.staleCwd[from] {
		m.unsound = true
	}
	var stack []frame
	if strings.HasPrefix(p, "/") {
		stack = []frame{{m.top, ""}}
	} else {
		stack = m.framesAt(from)
	}
	queue := reverse(splitPath(p))
	hops := 0
	for len(queue) > 0 {
		name := queue[len(queue)-1]
		queue = queue[:len(queue)-1]
		last := len(queue) == 0
		top := stack[len(stack)-1]
		switch name {
		case ".":
			if last {
				r.isDot = true
				r.node, r.final = top.n, pathOrRoot(top.path)
			}
			continue
		case "..":
			if len(stack) > 1 {
				stack = stack[:len(stack)-1]
			}
			top = stack[len(stack)-1]
			if last {
				r.isDot = true
				r.node, r.final = top.n, pathOrRoot(top.path)
			}
			continue
		}
		child := top.n.kids[name]
		full := top.path + "/" + name
		if child == nil {
			r.parent, r.name, r.final = top.n, name, full
			r.errno = "ENOENT"
			if !last {
				r.stop = full
			}
			return r
		}
		switch child.kind {
		case KindLink:
			if last && !follow {
				r.node, r.parent, r.name, r.final = child, top.n, name, full
				return r
			}
			hops++
			if hops > maxHops {
				r.errno, r.stop = "ELOOP", full
				return r
			}
			r.links = append(r.links, full)
			if strings.HasPrefix(child.target, "/") {
				stack = []frame{{m.top, ""}}
			}
			queue = append(queue, reverse(splitPath(child.target))...)
			if strings.HasSuffix(child.target, "/") && last {
				trailing = true
			}
		case KindDir:
			stack = append(stack, frame{child, full})
			if last {
				r.node, r.parent, r.name, r.final = child, top.n, name, full
				return r
			}
		default:
			if !last || trailing {
				r.errno, r.stop = "ENOTDIR", full
				r.final = full
				return r
			}
			r.node, r.parent, r.name, r.final = child, top.n, name, full
			return r
		}
	}
	top := stack[len(stack)-1]
	r.node, r.final = top.n, pathOrRoot(top.path)
	if len(stack) > 1 {
		r.parent = stack[len(stack)-2].n
	}
	return r
}

func (m *model) owned(p string) bool {
	for q := p; q != ""; q = parentOf(q) {
		if m.own[q] {
			return true
		}
	}
	return false
}

// disown drops ownership of p and everything beneath it.
func (m *model) disown(p string) {
	for q := range m.own {
		if q == p || strings.HasPrefix(q, p+"/") {
			delete(m.own, q)
		}
	}
}

func (m *model) add(t touch) {
	t.own = m.owned(t.path)
	m.touches = append(m.touches, t)
}

// look records what a lookup really looked at.
func (m *model) look(r mres, create, write, absent bool) {
	for _, l := range r.links {
		m.add(touch{path: l})
	}
	switch {
	case r.stop != "":
		m.add(touch{path: r.stop, absent: r.errno == "ENOENT"})
	case r.final != "":
		m.add(touch{path: r.final, create: create, write: write, absent: absent || r.errno == "ENOENT"})
	}
}

func (m *model) readOnlyHost(p string) bool {
	for _, d := range []string{"/usr", "/etc", "/lib64", "/bin"} {
		if p == d || strings.HasPrefix(p, d+"/") {
			return true
		}
	}
	return false
}

// erofs says a write at p fails: p is in a read-only declared root, or outside
// everything the sandbox leaves writable.  The sandbox's new root is sealed
// read-only (KILN04 D43), so above the mounts (and in the directories the plan
// made only to reach them) nothing can be created, removed, renamed or changed.
func (m *model) erofs(p string) bool {
	return m.readOnlyHost(p) || !m.writable(p)
}

// modelScratch are the sandbox's fresh scratch tmpfs besides Home and Tmpdir.
var modelScratch = []string{"/tmp", "/var/tmp", "/run"}

// writable says p is the checkout, Home, Tmpdir or a scratch directory, or
// inside one: the mounts the step may write to.
func (m *model) writable(p string) bool {
	for _, d := range append([]string{m.root, m.home, m.tmp}, modelScratch...) {
		if p == d || strings.HasPrefix(p, d+"/") {
			return true
		}
	}
	return false
}

// sandboxHas is the model's reading of the D43 exists-set: what can be in the
// sealed root.  Everything else is absent in every run.
func (m *model) sandboxHas(p string) bool {
	if p == "/" || m.writable(p) {
		return true
	}
	for _, d := range []string{"/usr", "/etc", "/lib64", "/bin", "/dev", "/proc", "/sys"} {
		if p == d || strings.HasPrefix(p, d+"/") {
			return true
		}
	}
	for _, a := range append([]string{m.root, m.home, m.tmp, "/usr", "/etc", "/lib64", "/bin"}, modelScratch...) {
		if strings.HasPrefix(a, p+"/") {
			return true
		}
	}
	return false
}

func (m *model) underCwd(abs string) bool {
	for _, pr := range m.procs {
		if abs != "" && (pr.cwdPath == abs || strings.HasPrefix(pr.cwdPath, abs+"/")) {
			return true
		}
	}
	return false
}

func (m *model) event(pid int, e trace.Event, errno string) trace.Event {
	if errno != "" && errno != "ENOENT" {
		// The call failed for another reason: what it found at a name it never
		// got to judge is not an observation of absence.
		for i := m.opStart; i < len(m.touches); i++ {
			m.touches[i].absent = false
		}
	}
	e.Cwd = m.procs[pid].cwdExpr
	e.Pid = pid
	e.Result = trace.Result{OK: errno == "", Errno: errno}
	return e
}

func (m *model) doStat(pid int, p string, follow bool) []trace.Event {
	pr := m.procs[pid]
	r := m.resolve(p, pr.cwdPath, follow)
	m.look(r, false, false, false)
	if r.errno != "" && !strings.HasPrefix(p, "/") {
		m.dotdotProbes = append(m.dotdotProbes, dotdotProbe{pr.cwdExpr, p})
	}
	sys := "stat"
	if !follow {
		sys = "lstat"
	}
	return []trace.Event{m.event(pid, base(sys, trace.KindMetadata, p), r.errno)}
}

func (m *model) doDirStat(pid int, dir, p string) []trace.Event {
	d := m.byPath(dir)
	if d == nil || d.kind != KindDir || strings.HasPrefix(p, "/") {
		return m.doStat(pid, p, true)
	}
	r := m.resolve(p, dir, true)
	m.look(r, false, false, false)
	e := base("newfstatat", trace.KindMetadata, p)
	e.DirfdPath = dir
	e.Flags = "0"
	return []trace.Event{m.event(pid, e, r.errno)}
}

func (m *model) doChdir(pid int, p string) []trace.Event {
	pr := m.procs[pid]
	r := m.resolve(p, pr.cwdPath, true)
	m.look(r, false, false, false)
	errno := r.errno
	if errno == "" && (r.node == nil || r.node.kind != KindDir) {
		errno = "ENOTDIR"
	}
	e := m.event(pid, base("chdir", trace.KindDirectory, p), errno)
	if errno == "" {
		pr.cwdPath = r.final
		if pr.cwdPath == "/" {
			pr.cwdPath = ""
		}
		if strings.HasPrefix(p, "/") {
			pr.cwdExpr = p
		} else {
			pr.cwdExpr = strings.TrimRight(pr.cwdExpr, "/") + "/" + p
		}
	}
	return []trace.Event{e}
}

func (m *model) doMkdirLike(pid int, sys, p, target string) []trace.Event {
	pr := m.procs[pid]
	orig := p
	if sys == "mkdir" {
		p = stripSlashes(p)
	}
	r := m.resolve(p, pr.cwdPath, false)
	create := r.errno == "ENOENT" && r.stop == "" && !r.isDot
	m.look(r, create, false, false)
	errno := r.errno
	switch {
	case errno == "" && r.node != nil:
		errno = "EEXIST"
	case errno == "" && r.node == nil:
		errno = "EINVAL"
	}
	if sys == "symlink" && strings.HasSuffix(p, "/") && errno != "" {
		errno = "ENOENT"
	}
	if errno == "ENOENT" && create && !(sys == "symlink" && strings.HasSuffix(p, "/")) {
		if m.erofs(r.final) {
			errno = "EROFS"
		} else {
			errno = ""
			n := mdir()
			if sys == "symlink" {
				n = &mnode{kind: KindLink, target: target}
			}
			r.parent.kids[r.name] = n
			if sys == "symlink" && m.freshAbsent(r.final) {
				m.tmpLinked = true
			}
			m.add(touch{path: r.final, write: true, wonly: true})
			m.own[r.final] = true
		}
	}
	e := base(sys, trace.KindWrite, orig)
	e.Target = target
	return []trace.Event{m.event(pid, e, errno)}
}

func (m *model) doUnlink(pid int, sys, p string) []trace.Event {
	pr := m.procs[pid]
	orig := p
	if sys == "rmdir" {
		p = stripSlashes(p)
	}
	r := m.resolve(p, pr.cwdPath, false)
	m.look(r, false, false, false)
	errno := r.errno
	if errno == "" {
		switch {
		case r.isDot || r.node == nil || r.parent == nil:
			errno = "EINVAL"
		case m.erofs(r.final):
			errno = "EROFS"
		case sys == "unlink" && r.node.kind == KindDir:
			errno = "EISDIR"
		case sys == "rmdir" && r.node.kind != KindDir:
			errno = "ENOTDIR"
		case sys == "rmdir" && len(r.node.kids) > 0:
			errno = "ENOTEMPTY"
		case sys == "rmdir" && m.underCwd(r.final):
			errno = "EBUSY"
		}
	}
	if errno == "" {
		delete(r.parent.kids, r.name)
		m.add(touch{path: r.final, write: true, wonly: true})
		m.disown(r.final)
	}
	return []trace.Event{m.event(pid, base(sys, trace.KindWrite, orig), errno)}
}

func (m *model) doRenameLink(pid int, sys, a, b string, exchange bool) []trace.Event {
	pr := m.procs[pid]
	src := m.resolve(a, pr.cwdPath, false)
	dst := m.resolve(b, pr.cwdPath, false)
	m.look(src, false, false, false)
	m.look(dst, true, false, false)
	errno := ""
	switch {
	case src.errno != "":
		errno = src.errno
	case dst.errno != "" && dst.errno != "ENOENT":
		errno = dst.errno
	case dst.errno == "ENOENT" && dst.stop != "":
		errno = "ENOENT"
	case src.isDot || dst.isDot || src.node == nil || src.parent == nil:
		errno = "EINVAL"
	case m.erofs(src.final) || m.erofs(dst.final):
		errno = "EROFS"
	case m.mountOf(src.final) != m.mountOf(dst.final) && (m.mountOf(src.final) != "" || m.mountOf(dst.final) != ""):
		// The sandbox's HOME and TMPDIR are separate tmpfs mounts, and so is the
		// bound checkout: a rename or link across them is EXDEV (KILN04 D40
		// relies on it).  The model must not invent one that cannot happen.
		errno = "EXDEV"
	case exchange:
		if dst.node == nil {
			errno = "ENOENT"
		} else if m.underCwd(src.final) || m.underCwd(dst.final) ||
			strings.HasPrefix(dst.final, src.final+"/") || strings.HasPrefix(src.final, dst.final+"/") {
			errno = "EBUSY"
		}
	case sys == "link" && src.node.kind == KindDir:
		errno = "EPERM"
	case sys == "link" && dst.node != nil:
		errno = "EEXIST"
	case sys == "rename":
		switch {
		case m.underCwd(src.final) && !(m.freshAbsent(src.final) && m.freshAbsent(dst.final)):
			// A directory under a cwd is not renamed, except inside the fresh
			// tmpfs: there the model lets it happen so it can check that the
			// classifier refuses the stale cwd TEXT that results (KILN04 D40).
			errno = "EBUSY"
		case src.node.kind == KindDir && (dst.final == src.final || strings.HasPrefix(dst.final, src.final+"/")):
			errno = "EINVAL"
		case dst.node != nil && src.node == dst.node:
			errno = "NOOP"
		case dst.node != nil && src.node.kind == KindDir && dst.node.kind != KindDir:
			errno = "ENOTDIR"
		case dst.node != nil && src.node.kind != KindDir && dst.node.kind == KindDir:
			errno = "EISDIR"
		case dst.node != nil && dst.node.kind == KindDir && len(dst.node.kids) > 0:
			errno = "ENOTEMPTY"
		}
	}
	if errno == "" || errno == "NOOP" {
		if strings.HasSuffix(a, "/") || strings.HasSuffix(b, "/") {
			errno = "ENOTDIR"
		}
	}
	e := base(sys, trace.KindWrite, a)
	e.Path2 = b
	if exchange {
		e.Syscall, e.Flags = "renameat2", "RENAME_EXCHANGE"
	}
	if errno == "NOOP" {
		return []trace.Event{m.event(pid, e, "")}
	}
	if errno == "" {
		m.add(touch{path: src.final, write: true, wonly: true})
		m.add(touch{path: dst.final, write: true, wonly: true})
		// D43: what can leave a cwd TEXT stale is moving a DIRECTORY (either
		// end of an exchange).  The classifier cannot see kinds of objects it
		// has no history of, so it also taints on moving anything the action
		// did not create; that over-taint is safe and the model does not
		// demand it (its own-file bookkeeping is coarser than the
		// classifier's, so asking for it would fail on the model's own gaps).
		if src.node.kind == KindDir || (exchange && dst.node.kind == KindDir) {
			m.reshaped = true
		}
		if sys == "rename" && !exchange {
			for _, q := range m.procs {
				if q.cwdPath != "" && (q.cwdPath == src.final || strings.HasPrefix(q.cwdPath, src.final+"/")) {
					if m.staleCwd == nil {
						m.staleCwd = map[string]bool{}
					}
					m.staleCwd[q.cwdPath] = true
				}
			}
		}
		if (src.node.kind == KindLink && m.freshAbsent(dst.final)) || (exchange && dst.node.kind == KindLink && m.freshAbsent(src.final)) {
			m.tmpLinked = true
		}
		switch {
		case exchange:
			src.parent.kids[src.name], dst.parent.kids[dst.name] = dst.node, src.node
			m.disown(src.final)
			m.disown(dst.final)
		default:
			if sys == "rename" {
				delete(src.parent.kids, src.name)
				m.disown(src.final)
			}
			dst.parent.kids[dst.name] = src.node
			m.own[dst.final] = true
		}
	}
	return []trace.Event{m.event(pid, e, errno)}
}

var contents = [][]byte{nil, nil, nil, []byte("garbage")}

func (m *model) doOpen(pid int, p string, mode int, content int) []trace.Event {
	pr := m.procs[pid]
	flags := []string{"O_RDONLY", "O_WRONLY|O_CREAT|O_TRUNC", "O_WRONLY|O_CREAT|O_EXCL", "O_WRONLY|O_CREAT", "O_WRONLY|O_TRUNC",
		"O_RDONLY|O_CREAT|O_NOFOLLOW|O_PATH", "O_RDONLY|O_CREAT|O_NOFOLLOW"}[mode]
	// Modes 5 and 6 do not follow a final link; O_PATH (5) ignores O_CREAT and
	// opens a link itself.
	create := mode == 1 || mode == 2 || mode == 3 || mode == 6
	writes := mode != 0 && mode != 5
	r := m.resolve(p, pr.cwdPath, mode != 2 && mode < 5)
	m.look(r, create && r.stop == "", false, false)
	errno := r.errno
	final := r.final
	made := false
	switch {
	case errno == "" && r.node != nil && r.node.kind == KindLink && mode != 5:
		errno = "ELOOP"
	case errno == "" && r.node == nil:
		errno = "EINVAL"
	case errno == "" && mode == 2:
		errno = "EEXIST"
	case errno == "" && r.node.kind == KindDir && writes:
		errno = "EISDIR"
	case errno == "" && writes && m.erofs(final):
		errno = "EROFS"
	case errno == "ENOENT" && r.stop == "" && create && strings.HasSuffix(p, "/"):
		errno = "EISDIR"
	case errno == "ENOENT" && r.stop == "" && create && !r.isDot:
		if m.erofs(final) {
			errno = "EROFS"
		} else {
			errno = ""
			made = true
			r.parent.kids[r.name] = &mnode{kind: KindFile}
			m.own[final] = true
			r.node = r.parent.kids[r.name]
		}
	}
	if errno == "" && writes {
		m.add(touch{path: final, write: true, wonly: true})
		if (mode == 1 || mode == 4) && !made && r.node != nil && r.node.kind == KindFile {
			r.node.data = nil
		}
		if r.node != nil && r.node.kind == KindFile && content >= 0 {
			switch content % 4 {
			case 0:
				r.node.data = buildELFStatic()
			case 1:
				r.node.data = []byte("#!" + m.root + "/code/p/a/f1\n")
			case 2:
				r.node.data = []byte("#!" + m.root + "/code/p/b/f2\n")
			default:
				r.node.data = []byte("garbage")
			}
		}
	}
	e := base("openat", trace.KindOpen, p)
	e.Flags = flags
	if errno == "" {
		e.ReturnFDPath = final
	}
	return []trace.Event{m.event(pid, e, errno)}
}

// doChmod changes the mode of whatever p names (following links): a write to
// the final object, which must exist.  Aimed at the root and at names reached
// through links and "..".
func (m *model) doChmod(pid int, p string) []trace.Event {
	pr := m.procs[pid]
	r := m.resolve(p, pr.cwdPath, true)
	m.look(r, false, false, false)
	errno := r.errno
	switch {
	case errno == "" && (r.node == nil || r.node.kind == KindLink):
		errno = "EINVAL"
	case errno == "" && m.erofs(r.final):
		errno = "EROFS"
	}
	if errno == "" {
		m.add(touch{path: r.final, write: true, wonly: true})
	}
	return []trace.Event{m.event(pid, base("chmod", trace.KindWrite, p), errno)}
}

// noFollowSyscalls are the metadata writes that do not follow a final symlink,
// and the flag (if any) that says so.  The kernel still FOLLOWS a final name
// that ends in a slash, and demands a directory there (M1): resolve applies
// that rule already.
var noFollowSyscalls = []struct{ sys, flags string }{
	{"lchown", ""}, {"lsetxattr", ""}, {"lremovexattr", ""},
	{"utimensat", "AT_SYMLINK_NOFOLLOW"}, {"fchownat", "AT_SYMLINK_NOFOLLOW"}, {"fchmodat2", "AT_SYMLINK_NOFOLLOW"},
}

// doMetaNoFollow is a NOFOLLOW metadata write, half the time with a trailing
// slash added: it changes the link itself, or, with the slash, the directory
// the link points to.
func (m *model) doMetaNoFollow(pid int, which int, p string, slash bool) []trace.Event {
	pr := m.procs[pid]
	if slash && !strings.HasSuffix(p, "/") {
		p += "/"
	}
	r := m.resolve(p, pr.cwdPath, false)
	m.look(r, false, false, false)
	errno := r.errno
	switch {
	case errno == "" && r.node == nil:
		errno = "EINVAL"
	case errno == "" && m.erofs(r.final):
		errno = "EROFS"
	}
	if errno == "" {
		m.add(touch{path: r.final, write: true, wonly: true})
	}
	c := noFollowSyscalls[which%len(noFollowSyscalls)]
	e := base(c.sys, trace.KindWrite, p)
	e.Flags = c.flags
	return []trace.Event{m.event(pid, e, errno)}
}

// doFdWrite is a descriptor metadata write (fchmod and friends): the trace
// carries the descriptor's own -y path, which is already resolved.
func (m *model) doFdWrite(pid int, p string) []trace.Event {
	pr := m.procs[pid]
	r := m.resolve(p, pr.cwdPath, true)
	if r.errno != "" || r.node == nil || r.node.kind == KindLink || m.erofs(r.final) {
		return nil
	}
	m.look(r, false, false, false)
	m.add(touch{path: r.final, write: true, wonly: true})
	return []trace.Event{m.event(pid, base("fchmod", trace.KindWrite, r.final), "")}
}

// doBind binds a Unix socket at p: it creates the socket file (W4).
func (m *model) doBind(pid int, p string) []trace.Event {
	pr := m.procs[pid]
	p = stripSlashes(p)
	r := m.resolve(p, pr.cwdPath, false)
	create := r.errno == "ENOENT" && r.stop == "" && !r.isDot
	m.look(r, create, false, false)
	errno := r.errno
	switch {
	case errno == "" && r.node != nil:
		errno = "EADDRINUSE"
	case errno == "" && r.node == nil:
		errno = "EINVAL"
	case errno == "ENOENT" && create:
		if m.erofs(r.final) {
			errno = "EROFS"
		} else {
			errno = ""
			r.parent.kids[r.name] = &mnode{kind: KindFile}
			m.add(touch{path: r.final, write: true, wonly: true})
			m.own[r.final] = true
		}
	}
	e := base("bind", trace.KindNetwork, "")
	e.Addr = &trace.Addr{Family: "AF_UNIX", Address: p, Port: -1}
	e.SockPath = "socket:[1]"
	return []trace.Event{m.event(pid, e, errno)}
}

// doReopen re-opens a descriptor that is a pipe or socket (`> /dev/stderr`):
// no repository path, so no write.
func (m *model) doReopen(pid int, flags, ret string) []trace.Event {
	e := base("openat", trace.KindOpen, "/proc/self/fd/9")
	e.Flags, e.ReturnFDPath = flags, ret
	return []trace.Event{m.event(pid, e, "")}
}

// doVirtual runs f on the path p spelled through /proc/self/cwd: the kernel
// really resolves to p, but the trace prints the virtual name, which the
// classifier cannot place (W5).  A path that is not under the cwd is run as
// it is.
func (m *model) doVirtual(pid int, p string, f func(q string) []trace.Event) []trace.Event {
	pr := m.procs[pid]
	rel := p
	if strings.HasPrefix(p, "/") {
		var ok bool
		if rel, ok = strings.CutPrefix(p, pr.cwdPath+"/"); !ok {
			return f(p)
		}
	}
	evs := f(p)
	for i := range evs {
		if evs[i].Path == p {
			evs[i].Path = "/proc/self/cwd/" + rel
		}
	}
	return evs
}

func (m *model) doList(pid int, p string) []trace.Event {
	pr := m.procs[pid]
	r := m.resolve(p, pr.cwdPath, true)
	if r.errno != "" || r.node == nil || r.node.kind != KindDir {
		return nil
	}
	m.add(touch{path: r.final, list: true})
	e := base("getdents64", trace.KindListing, r.final)
	return []trace.Event{m.event(pid, e, "")}
}

// execChain follows the kernel's loading chain of an exec.
func (m *model) execChain(pid int, p string, depth int) string {
	pr := m.procs[pid]
	r := m.resolve(p, pr.cwdPath, true)
	m.look(r, false, false, false)
	if r.errno != "" {
		return r.errno
	}
	if r.node == nil || r.node.kind != KindFile {
		return "EACCES"
	}
	switch {
	case strings.HasPrefix(string(r.node.data), "#!"):
		if depth >= 5 {
			return "ELOOP"
		}
		interp, ok := shebangInterpreter(r.node.data)
		if !ok {
			return "ENOEXEC"
		}
		return m.execChain(pid, interp, depth+1)
	case len(r.node.data) >= 4 && string(r.node.data[:4]) == elf.ELFMAG:
		// The kernel also reads the PT_INTERP loader.  Only a loader inside the
		// checkout is judged (the host's is outside the oracle); no ELF of the
		// plain generator names one, the biased exec-head generator does.
		if ip, ok := elfInterpreter(r.node.data); ok && strings.HasPrefix(ip, m.root+"/") {
			m.look(m.resolve(ip, "/", true), false, false, false)
		}
		return ""
	}
	return "ENOEXEC"
}

func (m *model) doExec(pid int, p string) []trace.Event {
	// What the exec supervisor of KILN04 D48 would have pinned: the bytes of the
	// FIRST program as they are at this instant (the effect time).  Resolution is
	// read-only here; execChain does the touching.
	first := m.resolve(p, m.procs[pid].cwdPath, true)
	errno := m.execChain(pid, p, 0)
	rec := execRec{pid: pid, path: p, ok: errno == ""}
	if rec.ok && first.node != nil && first.node.kind == KindFile {
		rec.head = append([]byte(nil), first.node.data...)
		if len(rec.head) > headLimit {
			rec.head = rec.head[:headLimit]
		}
	}
	m.execs = append(m.execs, rec)
	e := base("execve", trace.KindProcess, p)
	e.Argv = []string{"x"}
	return []trace.Event{m.event(pid, e, errno)}
}

// execRec is one exec the model made: what a supervisor would have seen.
type execRec struct {
	pid  int
	path string
	ok   bool
	head []byte
}

// modelFS is the model's final state as the post-run disk.
type modelFS struct{ m *model }

func (f modelFS) Lstat(p string) (FileInfo, error) {
	n := f.m.byPath(p)
	if n == nil {
		return FileInfo{}, fs.ErrNotExist
	}
	return FileInfo{Kind: n.kind, Ctime: oldCtime}, nil
}

func (f modelFS) Readlink(p string) (string, error) {
	n := f.m.byPath(p)
	if n == nil || n.kind != KindLink {
		return "", fs.ErrNotExist
	}
	return n.target, nil
}

func (f modelFS) ReadHead(p string, n int) ([]byte, error) {
	x := f.m.byPath(p)
	if x == nil || x.kind != KindFile {
		return nil, fs.ErrNotExist
	}
	if len(x.data) > n {
		return x.data[:n], nil
	}
	return x.data, nil
}

var (
	rootChoices = []func(home string) string{
		func(string) string { return "/w/repo" },
		func(home string) string { return home + "/work/repo" },
	}
	homeChoices = []string{"/home/runner", "/hh"}
	tmpChoices  = []string{"/tmp/kiln-1", "/tmp/t2"}
)

// newModel seeds the repository, leftovers, image files and scratch
// directories; it returns the tracked tree of the initial state.
func newModel(root, home, tmp string) (*model, *memTree) {
	m := &model{root: root, home: home, tmp: tmp, top: mdir(), own: map[string]bool{},
		procs: map[int]*mproc{1: {root, root}, 2: {root, root}}}
	tree := newMemTree()
	elfStatic := buildELFStatic()
	tracked := func(rel string, n *mnode) {
		m.mkpath(root+"/"+rel, n)
		if n.kind == KindLink {
			tree.link(rel, n.target)
		} else {
			tree.add(rel, Node{Kind: KindFile}, string(n.data))
		}
	}
	tracked("code/p/a/f1", &mnode{kind: KindFile, data: elfStatic})
	tracked("code/p/a/lnk", &mnode{kind: KindLink, target: "../b"})
	tracked("code/p/b/f2", &mnode{kind: KindFile, data: []byte("#!" + root + "/code/p/a/f1\n")})
	tracked("code/p/b/g", &mnode{kind: KindFile, data: []byte("data")})
	tracked("code/scripts/s.sh", &mnode{kind: KindFile, data: []byte("#!" + root + "/code/p/b/f2\n")})
	tracked("go.work", &mnode{kind: KindFile, data: []byte("go 1.26\n")})
	// Pre-existing untracked leftovers of earlier actions.
	m.mkpath(root+"/code/p/a/left.txt", &mnode{kind: KindFile, data: elfStatic})
	m.mkpath(root+"/code/p/a/leftdir/f", &mnode{kind: KindFile})
	m.mkpath(root+"/code/p/a/leftlnk", &mnode{kind: KindLink, target: "../b"})
	m.mkpath(root+"/junk", &mnode{kind: KindFile})
	m.mkpath("/usr/bin/sh", &mnode{kind: KindFile, data: buildELF(loader, elf.ELFCLASS64, binary.LittleEndian)})
	m.mkpath(loader, &mnode{kind: KindFile, data: []byte("ld")})
	m.mkpath("/usr/lib/x", &mnode{kind: KindFile})
	m.mkpath("/etc/x", &mnode{kind: KindFile})
	m.mkpath(tmp+"/.keep", &mnode{kind: KindFile})
	delete(m.byPath(tmp).kids, ".keep")
	m.mkpath(home+"/.keep", &mnode{kind: KindFile})
	delete(m.byPath(home).kids, ".keep")
	for _, d := range modelScratch {
		m.mkpath(d+"/.keep", &mnode{kind: KindFile})
		delete(m.byPath(d).kids, ".keep")
	}
	m.dirs = []string{root, root + "/code", root + "/code/p/a", root + "/code/p/b", tmp, home, "/etc", "/run", "/var/tmp"}
	return m, tree
}

var modelNames = []string{"code", "p", "a", "b", "f1", "f2", "g", "lnk", "L", "M", "n", "x", "y", "..", ".", "scripts",
	"s.sh", "go.work", "new", "left.txt", "leftdir", "leftlnk", "junk", "d1", "d2",
	"package.json", "run", "var", "tmp", "home", "w", "opt", "systemd"}

// modelPath builds a path from bytes.
func (m *model) path(data []byte, i *int) string {
	next := func() byte {
		b := data[*i%len(data)]
		*i++
		return b
	}
	// Now and then name the checkout's own parent directory exactly, so the
	// model can rename, remove or chmod an ancestor of the checkout (KILN04 D39
	// covers looks at an ancestor, never writes to one; D40's premise).
	if b := next(); b%24 == 0 {
		return parentOf(m.root)
	}
	var prefix string
	switch next() % 13 {
	case 0, 1:
		prefix = m.root + "/"
	case 2:
		prefix = m.tmp + "/"
	case 3:
		prefix = parentOf(m.root) + "/"
	case 4:
		prefix = m.home + "/"
	case 5:
		prefix = "/usr/lib/"
	case 6:
		prefix = "/etc/"
	case 9:
		prefix = "/run/"
	case 10:
		prefix = "/var/tmp/"
	case 11, 12:
		prefix = "/" // the sealed root's top: nothing is there (D43)
	}
	n := int(next()%4) + 1
	parts := make([]string, n)
	for k := range parts {
		parts[k] = modelNames[int(next())%len(modelNames)]
	}
	p := prefix + strings.Join(parts, "/")
	if next()%8 == 0 {
		p += "/"
	}
	return p
}

type mop struct {
	pid                int
	start, effect, end int
	run                func() []trace.Event
	events             []trace.Event
}

// burstOp is one operation of a burst: an operation kind of runStep's switch
// and the name it acts on, relative to the burst's base directory.
type burstOp struct {
	kind   int
	suffix string
	// inRoot says suffix is a path under the checkout, not under the base.
	inRoot bool
}

// bursts are the build patterns that KILN04 section 5.5 relaxes.  A burst
// issues the operations of a pattern on ONE name, each with its own random
// process, duration, start offset and effect point, so the fuzzer sees the
// pattern in order, interleaved and overlapped.
//
//	0  mkdir out; create+truncate out/x; read it; stat it     (a kept output)
//	1  create+truncate t.o; read it; unlink it                (a temporary)
//	2  create exclusively t.o; stat it; unlink it
//	3  plain O_CREAT t.o; stat it; unlink it
//	4  create+truncate t.o with contents; exec it; unlink it
//	5  create+truncate t.o; stat it twice
//	6  stat of the base directory itself ($HOME, $TMPDIR, the checkout, ...)
//	7  stat a leftover link; unlink it; create a file in its place; stat again
//	8  the same for an untracked leftover file and a tracked link
//	9  mkdir o; symlink o/l; create through the link; stat the link
var bursts = [][]burstOp{
	{{0, "/o", false}, {8, "/o/x", false}, {7, "/o/x", false}, {12, "/o/x", false}},
	{{8, "/t.o", false}, {7, "/t.o", false}, {5, "/t.o", false}},
	{{9, "/t.o", false}, {12, "/t.o", false}, {5, "/t.o", false}},
	{{10, "/t.o", false}, {12, "/t.o", false}, {5, "/t.o", false}},
	{{8, "/t.o", false}, {15, "/t.o", false}, {5, "/t.o", false}},
	{{8, "/t.o", false}, {12, "/t.o", false}, {12, "/t.o", false}},
	{{12, "", false}},
	{{12, "/code/p/a/leftlnk", true}, {5, "/code/p/a/leftlnk", true}, {8, "/code/p/a/leftlnk", true}, {12, "/code/p/a/leftlnk", true}},
	{{12, "/code/p/a/left.txt", true}, {5, "/code/p/a/left.txt", true}, {10, "/code/p/a/left.txt", true}, {12, "/code/p/a/left.txt", true}},
	{{0, "/o", false}, {1, "/o/l", false}, {8, "/o/l", false}, {12, "/o/l", false}},
	{{100, "/t.o", false}, {12, "/t.o", false}, {5, "/t.o", false}},
	{{100, "/code/p/a/leftlnk", true}, {12, "/code/p/a/leftlnk", true}, {5, "/code/p/a/leftlnk", true}},
	{{101, "/code/p/a/leftlnk", true}, {12, "/code/p/a/leftlnk", true}, {5, "/code/p/a/leftlnk", true}},
	// 13: the stale cwd (KILN04 D40).  mkdir o; chdir o; rename o away; mkdir o
	// again; then a path relative to the cwd text, which now names a different
	// directory from the one the kernel's cwd is in.  A suffix that starts with
	// "~" is a literal relative path, not joined to the base.
	{{0, "/o", false}, {14, "/o", false}, {2, "/o", false}, {0, "/o", false}, {12, "~../u.o", false}, {12, "~../o/x", false}},
	// 14..17: absent-before-made and metadata writes by path (KILN04 D42).
	//   14  stat ENOENT; plain O_CREAT; stat; chmod            (the linker's output)
	//   15  stat ENOENT of a directory; mkdir it; mkdir inside it   (mkdir -p)
	//   16  create+truncate; chmod; stat; chmod                (a post-link chmod)
	//   17  dangling link; stat through it; unlink; create; chmod
	{{12, "/t.o", false}, {10, "/t.o", false}, {12, "/t.o", false}, {104, "/t.o", false}},
	{{12, "/o", false}, {0, "/o", false}, {0, "/o/x", false}},
	{{8, "/t.o", false}, {104, "/t.o", false}, {12, "/t.o", false}, {104, "/t.o", false}},
	{{1, "/t.o", false}, {12, "/t.o", false}, {5, "/t.o", false}, {10, "/t.o", false}, {104, "/t.o", false}},
	//   18  stat ENOENT of u.o; create t.o; rename t.o INTO u.o: the first entry
	//       of u.o is a rename, not a creation, so the rule must refuse
	{{12, "/u.o", false}, {8, "/t.o", false}, {2, "/t.o", false}, {12, "/u.o", false}},
	// 19, 20: the stale cwd against the sealed root (KILN04 D43).  chdir into o,
	// move o away (and maybe make it again), then probe names that the cwd TEXT
	// resolves into the sealed root or a scratch directory but the kernel, from
	// the moved directory, resolves elsewhere.
	{{0, "/o", false}, {14, "/o", false}, {2, "/o", false}, {12, "~../../../../../package.json", false}, {12, "~../../../tmp/x", false}, {12, "~../../../../run/x", false}},
	{{0, "/o", false}, {14, "/o", false}, {2, "/o", false}, {0, "/o", false}, {12, "~../../../../../package.json", false}, {12, "~../../x", false}},
	// 21: scratch absences: probe, create, remove, probe again.
	{{12, "/q", false}, {8, "/q", false}, {5, "/q", false}, {12, "/q", false}},
	// 22..27: the namespace-reshaping taint (KILN04 D43).  A "~/..." name is an
	// absolute path the sealed root cannot hold (/package.json, /opt/x): its
	// absence is a constant of the plan, UNLESS the action has moved something
	// that could leave a cwd text stale, in which case the classifier refuses
	// every such absence at the end, before or after the move.
	//   22  an own DIRECTORY is renamed, absences probed before and after
	{{12, "~/package.json", false}, {0, "/o", false}, {2, "/o", false}, {12, "~/package.json", false}, {12, "~/opt/x", false}},
	//   23  a directory the action did NOT create is renamed (the model demands
	//       the refusal: a directory moved)
	{{12, "~/package.json", false}, {2, "/code/p/a/leftdir", true}, {12, "~/package.json", false}, {12, "~/opt/x", false}},
	//   24  a file the action did not create is renamed (tracked or leftover).
	//       The classifier taints (it cannot tell the kind of an object it has
	//       no history of); the model does not demand it, since a regular file
	//       never leaves a cwd stale.  The burst checks nothing UNSAFE happens.
	{{12, "~/opt/x", false}, {2, "/code/p/b/g", true}, {12, "~/package.json", false}, {2, "/code/p/a/left.txt", true}},
	//   25  two own directories are EXCHANGED (renameat2 RENAME_EXCHANGE)
	{{0, "/o", false}, {0, "/u.o", false}, {12, "~/package.json", false}, {3, "/o", false}, {12, "~/var/x", false}},
	//   26  an own REGULAR file is renamed, or two are exchanged: this must
	//       not taint, so the absences stay constants (the model's sandboxHas
	//       oracle accepts them with no reason)
	{{12, "~/package.json", false}, {8, "/t.o", false}, {8, "/u.o", false}, {2, "/t.o", false}, {3, "/u.o", false}, {12, "~/opt/x", false}},
	//   27  failed relative probes through ".." from a cwd that was renamed
	//       away: "gone" is missing in the cwd TEXT and a ".." follows, so the
	//       text says ENOENT while the kernel starts from the moved directory
	{{0, "/o", false}, {14, "/o", false}, {12, "~gone/../x", false}, {2, "/o", false}, {0, "/o", false}, {12, "~gone/../x", false}, {12, "~../o/../x", false}, {12, "~../work/repo/nope", false}, {12, "~../../w/repo/nope", false}},
}

// perturbations are the operations a burst may be followed by, aimed at the
// burst's name (a bare kind, or a rename INTO the name when swap is set): the
// negative cases of the relaxations.  q is the burst's second name.
var perturbations = []struct {
	kind int
	swap bool
}{{2, false}, {2, true}, {3, false}, {4, false}, {4, true}, {1, false}, {0, false}, {5, false}, {9, false}, {10, false}, {14, false}, {100, false}, {101, false}, {102, false}, {103, false}, {104, false}, {105, false}, {106, false}, {109, false}}

func (m *model) burstBase(b byte) string {
	return []string{m.root + "/code/p/a", m.root + "/code/p/a/d1", m.root, m.tmp, m.home, m.tmp + "/d1",
		m.home + "/d1", "/usr/lib", parentOf(m.root), m.root + "/code/p/b", "/run", "/var/tmp", "/tmp"}[int(b)%13]
}

// runStep schedules, executes and serialises one BUILD step's operations.
func (m *model) runStep(data []byte, i *int, count int) *trace.Trace {
	var ops []*mop
	free := map[int]int{}
	next := func() int {
		b := int(data[*i%len(data)])
		*i++
		return b
	}
	// add schedules one operation of the given kind on p (and q, for the calls
	// that take a second name; "" reads one from the data).
	add := func(kind int, p, q string) {
		pid := 1 + next()%2
		dur := next() % 5
		off := next() % 3
		eff := next() % (dur + 1)
		start := free[pid] + off
		end := start + dur
		free[pid] = end + 1
		o := &mop{pid: pid, start: start, effect: start + eff, end: end}
		second := func() string {
			if q != "" {
				return q
			}
			return m.path(data, i)
		}
		switch kind {
		case 0:
			o.run = func() []trace.Event { return m.doMkdirLike(pid, "mkdir", p, "") }
		case 1:
			t := second()
			o.run = func() []trace.Event { return m.doMkdirLike(pid, "symlink", p, t) }
		case 2, 3, 4:
			q := second()
			o.run = func() []trace.Event {
				return m.doRenameLink(pid, map[int]string{2: "rename", 3: "rename", 4: "link"}[kind], p, q, kind == 3)
			}
		case 5:
			o.run = func() []trace.Event { return m.doUnlink(pid, "unlink", p) }
		case 6:
			o.run = func() []trace.Event { return m.doUnlink(pid, "rmdir", p) }
		case 7:
			o.run = func() []trace.Event { return m.doOpen(pid, p, 0, -1) }
		case 8:
			c := next()
			o.run = func() []trace.Event { return m.doOpen(pid, p, 1, c) }
		case 9:
			c := next()
			o.run = func() []trace.Event { return m.doOpen(pid, p, 2, c) }
		case 10:
			o.run = func() []trace.Event { return m.doOpen(pid, p, 3, -1) }
		case 100:
			o.run = func() []trace.Event { return m.doOpen(pid, p, 5, -1) }
		case 101:
			o.run = func() []trace.Event { return m.doOpen(pid, p, 6, -1) }
		case 102:
			c := next()
			o.run = func() []trace.Event {
				return m.doVirtual(pid, p, func(q string) []trace.Event { return m.doOpen(pid, q, 1, c) })
			}
		case 103:
			o.run = func() []trace.Event {
				return m.doVirtual(pid, p, func(q string) []trace.Event { return m.doMkdirLike(pid, "mkdir", q, "") })
			}
		case 104:
			o.run = func() []trace.Event { return m.doChmod(pid, p) }
		case 105:
			w, slash := next(), next()%2 == 0
			o.run = func() []trace.Event { return m.doMetaNoFollow(pid, int(w), p, slash) }
		case 106:
			o.run = func() []trace.Event { return m.doBind(pid, p) }
		case 107:
			// W6: a write under .git
			g := m.root + "/.git/" + modelNames[int(next())%len(modelNames)]
			o.run = func() []trace.Event { return m.doMkdirLike(pid, "mkdir", g, "") }
		case 108:
			// W6: a name no record can hold (invalid UTF-8)
			g := m.root + "/code/p/a/bad\xff" + modelNames[int(next())%len(modelNames)]
			o.run = func() []trace.Event { return m.doOpen(pid, g, 1, -1) }
		case 109:
			o.run = func() []trace.Event { return m.doFdWrite(pid, p) }
		case 110:
			// a child dumps core: the kernel writes ./core with no system call (L1)
			o.run = func() []trace.Event { m.core = true; return nil }
		case 112:
			// an httptest-style server: bind port 0, connect to its port (M4)
			port := 20000 + next()
			o.run = func() []trace.Event {
				b := base("bind", trace.KindNetwork, "")
				b.Addr = &trace.Addr{Family: "AF_INET", Address: "127.0.0.1", Port: 0}
				b.SockPath = "socket:[1]"
				c := base("connect", trace.KindNetwork, "")
				c.Addr = &trace.Addr{Family: "AF_INET", Address: "127.0.0.1", Port: port}
				c.SockPath = "socket:[1]"
				return []trace.Event{m.event(pid, b, ""), m.event(pid, c, "")}
			}
		case 114:
			// a loopback peer nobody bound, with no network-namespace
			// guarantee (StepNetNS false): a host daemon may answer (W11)
			o.run = func() []trace.Event {
				m.noNetNS = true
				c := base("connect", trace.KindNetwork, "")
				c.Addr = &trace.Addr{Family: "AF_INET", Address: "127.0.0.1", Port: 4226}
				c.SockPath = "socket:[1]"
				return []trace.Event{m.event(pid, c, "")}
			}
		case 113:
			// a child killed by SIGPIPE (yes | head): hides no write (M5)
			o.run = func() []trace.Event { m.sigpipe = true; return nil }
		case 111:
			f := []string{"O_WRONLY", "O_WRONLY|O_CREAT|O_TRUNC", "O_RDWR|O_APPEND"}[int(next())%3]
			ret := []string{"pipe:[77]", "socket:[5]", "anon_inode:[eventfd]"}[int(next())%3]
			o.run = func() []trace.Event { return m.doReopen(pid, f, ret) }
		case 11:
			c := next()
			o.run = func() []trace.Event { return m.doOpen(pid, p, 4, c) }
		case 12:
			o.run = func() []trace.Event { return m.doStat(pid, p, true) }
		case 13:
			o.run = func() []trace.Event { return m.doStat(pid, p, false) }
		case 14:
			o.run = func() []trace.Event { return m.doChdir(pid, p) }
		case 15:
			o.run = func() []trace.Event { return m.doExec(pid, p) }
		case 16:
			o.run = func() []trace.Event { return m.doList(pid, p) }
		default:
			d := m.dirs[next()%len(m.dirs)]
			rel := m.path(data, i)
			if strings.HasPrefix(rel, "/") {
				rel = strings.TrimLeft(rel, "/")
			}
			o.run = func() []trace.Event { return m.doDirStat(pid, d, rel) }
		}
		ops = append(ops, o)
	}
	for k := 0; k < count && *i < len(data); k++ {
		kind := next() % (33 + len(bursts))
		if kind >= 18+len(bursts) {
			add(100+kind-18-len(bursts), m.path(data, i), "")
			continue
		}
		if kind < 18 {
			add(kind, m.path(data, i), "")
			continue
		}
		b := bursts[(kind-18)%len(bursts)]
		base := m.burstBase(byte(next()))
		alt := base + "/u.o"
		full := func(op burstOp) string {
			if strings.HasPrefix(op.suffix, "~") {
				return op.suffix[1:]
			}
			if op.inRoot {
				return m.root + op.suffix
			}
			return base + op.suffix
		}
		for _, op := range b {
			add(op.kind, full(op), alt)
		}
		if next()%3 == 0 {
			pt := perturbations[next()%len(perturbations)]
			name := full(b[len(b)-1])
			if next()%3 == 0 {
				// Aim at a directory the name goes through instead.
				if q := parentOf(name); q != "" {
					name = q
				}
			}
			if pt.swap {
				add(pt.kind, alt, name)
			} else {
				add(pt.kind, name, alt)
			}
		}
	}
	// Effects land in effect-time order.
	order := make([]int, len(ops))
	for k := range order {
		order[k] = k
	}
	sort.SliceStable(order, func(a, b int) bool { return ops[order[a]].effect < ops[order[b]].effect })
	for _, k := range order {
		m.opStart = len(m.touches)
		ops[k].events = ops[k].run()
	}
	// Lines: a long call is split into an "<unfinished ...>" line and a
	// "resumed" line.
	type entry struct {
		tick  int
		end   bool
		op    int
		start bool
	}
	var es []entry
	for k, o := range ops {
		if len(o.events) == 0 {
			continue
		}
		if o.end > o.start {
			es = append(es, entry{o.start, false, k, true})
		}
		es = append(es, entry{o.end, true, k, false})
	}
	sort.SliceStable(es, func(a, b int) bool {
		if es[a].tick != es[b].tick {
			return es[a].tick < es[b].tick
		}
		return !es[a].end && es[b].end
	})
	startLine := map[int]int{}
	endLine := map[int]int{}
	for ln, e := range es {
		if e.end {
			endLine[e.op] = ln + 2
		} else {
			startLine[e.op] = ln + 2
		}
	}
	sh := execve("/usr/bin/sh", "sh", "-c", "step")
	sh.Pid, sh.Line, sh.StartLine, sh.Cwd = 1, 1, 1, m.root
	tr := &trace.Trace{
		RootPid: 1,
		Procs:   map[int]*trace.Process{1: {Pid: 1, Exit: &trace.Exit{Pid: 1}}, 2: {Pid: 2, Parent: 1, Exit: &trace.Exit{Pid: 2}}},
		Exits:   []trace.Exit{{Pid: 2}, {Pid: 1}},
	}
	if m.sigpipe {
		sp := trace.Exit{Pid: 2, Killed: true, Signal: "SIGPIPE"}
		tr.Procs[2].Exit = &sp
		tr.Exits = []trace.Exit{sp, {Pid: 1}}
	}
	if m.core {
		cd := trace.Exit{Pid: 2, Killed: true, Signal: "SIGSEGV", CoreDumped: true}
		tr.Procs[2].Exit = &cd
		tr.Exits = []trace.Exit{cd, {Pid: 1}}
	}
	tr.Events = append(tr.Events, sh)
	type placed struct {
		e    trace.Event
		line int
	}
	var ps []placed
	for k, o := range ops {
		for _, e := range o.events {
			e.Line = endLine[k]
			e.StartLine = e.Line
			if s, ok := startLine[k]; ok {
				e.StartLine = s
			}
			ps = append(ps, placed{e, e.Line})
		}
	}
	sort.SliceStable(ps, func(a, b int) bool { return ps[a].line < ps[b].line })
	for _, p := range ps {
		tr.Events = append(tr.Events, p.e)
	}
	m.stepExecs = append(m.stepExecs, m.execs)
	m.execs = nil
	return tr
}

// buildModel runs the whole action from data.
func buildModel(data []byte) (m *model, tree *memTree, steps []*trace.Trace) {
	b := data[0]
	home := homeChoices[int(b>>1)%2]
	root := rootChoices[int(b)%2](home)
	tmp := tmpChoices[int(b>>2)%2]
	m, tree = newModel(root, home, tmp)
	i := 1
	total := 0
	for i < len(data) && total < 48 {
		n := 6 + int(data[i%len(data)])%12
		i++
		steps = append(steps, m.runStep(data, &i, n))
		total += n
		if len(steps) == 2 {
			break
		}
	}
	return m, tree, steps
}

func FuzzDifferential(f *testing.F) {
	f.Add([]byte{0, 3, 1, 1, 0, 7, 0, 2, 1})
	f.Add([]byte{1, 0, 2, 3, 4, 0, 5, 7, 0, 1, 2, 3})
	f.Add([]byte{3, 5, 5, 5, 9, 2, 2, 2, 7, 6, 6, 6, 2, 1, 1, 1})
	f.Add([]byte("mkdir symlink rename link unlink open stat chdir exec"))
	f.Fuzz(runDifferential)
}

// TestDifferentialSeeded runs the differential oracle over pseudo-random
// inputs, so the ordinary test run (and a mutation check of the classifier)
// exercises it without the fuzzing engine.  The seeds are fixed: a failure
// reproduces.
func TestDifferentialSeeded(t *testing.T) {
	n := 4000
	if testing.Short() {
		n = 400
	}
	if v, err := strconv.Atoi(os.Getenv("KILN_DIFF_SEEDS")); err == nil && v > 0 {
		n = v // for mutation checks: a long deterministic run
	}
	for seed := 0; seed < n; seed++ {
		r := rand.New(rand.NewSource(int64(seed)))
		data := make([]byte, 40+r.Intn(260))
		for i := range data {
			data[i] = byte(r.Intn(256))
		}
		t.Run("", func(t *testing.T) {
			defer func() {
				if t.Failed() {
					t.Logf("seed %d", seed)
				}
			}()
			runDifferential(t, data)
		})
	}
}

func runDifferential(t *testing.T, data []byte) {
	{
		if len(data) < 4 {
			return
		}
		m, tree, steps := buildModel(data)
		runDifferentialOn(t, data, m, tree, steps, true, false)
	}
}

// runDifferentialOn judges one built action: the plain record, then (exec-head
// case) the same action with sidecars.  cuts allows the whole-trace
// perturbation of W10; biased selects the exec-head case's biased mode
// (exechead_biased_test.go).
func runDifferentialOn(t *testing.T, data []byte, m *model, tree *memTree, steps []*trace.Trace, cuts, biased bool) {
	{
		h := newHarness(t)
		h.cfg.Root, h.cfg.Home, h.cfg.Tmpdir = m.root, m.home, m.tmp
		h.cfg.ScratchDirs = modelScratch
		h.cfg.Packages = NewPackageIndex([]string{"code/p/a", "code/p/b"})
		h.cfg.Tree = tree
		h.cfg.FS = modelFS{m}
		recipe := make([]string, len(steps))
		for i := range recipe {
			recipe[i] = "step"
		}
		// The last byte picks a whole-trace perturbation (W10).  Each must make
		// WritesComplete false; the writes-complete oracle is then moot.
		cut := ""
		if cuts {
			cut = perturbTrace(data[len(data)-1], steps, recipe)
		}
		census := NewCensus() // observer only: the record is byte-identical with it
		rec, err := Classify(h.cfg, Input{Package: "code/p/a", Traces: steps, Recipe: recipe, StepNetNS: !m.noNetNS, Census: census})
		if err != nil {
			// The action moved or removed the checkout's own directories: the
			// post-run disk no longer shows a real Root, which is refused.
			return
		}
		if cut != "" {
			if rec.WritesComplete {
				t.Fatalf("%s must make WritesComplete false", cut)
			}
			return
		}
		judgeRecord(t, m, tree, rec, census)
		// KILN04 D48 step 5: the same action again, with the exec-head sidecar the
		// supervisor would have left (and with perturbed ones).
		checkExecHeads(t, m, tree, h, steps, recipe, data, census, biased)
	}
}

// judgeRecord holds the oracles that judge ONE record of the action against the
// model's truth: the writes-complete oracle and, when the record carries no
// reason, the read-set oracle.  It is applied to the plain record and, since
// KILN04 D48 step 5, to the record made WITH an honest exec-head sidecar, which
// is the only way an action that execs a file under the tmpfs ever reaches the
// read-set oracle (without a sidecar it is always refused as exec-unreadable).
func judgeRecord(t *testing.T, m *model, tree *memTree, rec *RawRecord, census *Census) {
	{
		if m.noNetNS && rec.WritesComplete {
			t.Fatal("an unbound loopback peer without a network namespace must make WritesComplete false")
		}
		if m.core && rec.WritesComplete {
			t.Fatal("a process that dumped core must make WritesComplete false")
		}
		// The writes-complete oracle holds with or without reasons.
		if rec.WritesComplete {
			checkWrites(t, m, rec)
		}
		// KILN04 D43: a FAILED relative call whose text walk stops at a missing
		// component with a ".." after it speaks for the text only if the cwd
		// text is where the kernel started.  When the action reshaped the
		// namespace, the classifier must refuse it as an unplaced lookup
		// (unstable-path) with the DOTDOT sub-code, which only the census
		// shows: another unstable-path (component-changed, ...) must not stand
		// in for it, or breaking the dotdot refusal would go unnoticed.
		if m.reshaped && m.hasDotdotStop() && len(censusKinds(census)["trace-incomplete:unstable-path:dotdot"]) == 0 {
			t.Fatalf("the action reshaped the namespace, but a failed probe through .. after a missing component was not refused as unstable-path:dotdot\nreasons %q census %v\nroot %s home %s tmp %s probes %q", rec.Reasons, censusKinds(census), m.root, m.home, m.tmp, m.dotdotProbes)
		}
		if len(rec.Reasons) > 0 {
			return // a record with a reason is never trusted: always safe
		}
		// KILN04 D43, the other half: once the model says the action reshaped
		// the namespace, an absent probe the sealed root would have allowed is
		// NOT a constant (a stale cwd may have made it a look at a real
		// directory), so a record that still has no reason lets it through.
		if m.reshaped {
			for _, tc := range m.touches {
				if sealedAbsentProbe(m, tc) {
					t.Fatalf("the action reshaped the namespace, but the absent probe of %s (sealed-root absence) left no reason\nroot %s home %s tmp %s", tc.path, m.root, m.home, m.tmp)
				}
			}
		}
		if m.unsound {
			t.Fatalf("a path was resolved from a stale cwd (a directory renamed under it), but the record has no reason\nroot %s home %s tmp %s", m.root, m.home, m.tmp)
		}
		check(t, m, tree, rec)
	}
}

// hasDotdotStop says one of the failed relative probes has, in its text, a
// component that NEVER existed during the whole action (absent on the final
// disk and never created, written or removed by any call) followed by a "..".
// Whatever order the classifier replays the calls in, and whether or not a
// renamed-away cwd directory was made again meanwhile, its walk goes plain at
// that component and meets the "..": the refusal must be the dotdot one.  (A
// component the action DID make and later moved is ambiguous, because the
// classifier judges names inside a fresh tmpfs from the trace's history at the
// time of the call: the unit test pins that case in a fixed order.)  A link on
// the way gives up: the classifier follows it, this lexical walk does not.
func (m *model) hasDotdotStop() bool {
	touched := map[string]bool{}
	for _, tc := range m.touches {
		if tc.create || tc.write || tc.wonly {
			touched[tc.path] = true
		}
	}
	for _, pr := range m.dotdotProbes {
		var path []string
		sawNever := false
		for _, name := range splitPath(pr.cwd + "/" + pr.p) {
			switch name {
			case ".":
				continue
			case "..":
				if sawNever {
					return true
				}
				if len(path) > 0 {
					path = path[:len(path)-1]
				}
				continue
			}
			path = append(path, name)
			full := "/" + strings.Join(path, "/")
			if n := m.byPath(full); n != nil {
				if n.kind == KindLink {
					break
				}
			} else if !touched[full] {
				sawNever = true
			}
		}
	}
	return false
}

// sealedAbsentProbe says tc is a look the classifier may only excuse because
// the sealed root cannot hold the path (D43): an ENOENT for a path outside the
// checkout and the sandbox's exists-set that the action did not make.
func sealedAbsentProbe(m *model, tc touch) bool {
	return tc.absent && !tc.own && !tc.create && !tc.write && !tc.wonly && !tc.list &&
		tc.path != "" && !strings.HasPrefix(tc.path, m.root+"/") && !m.sandboxHas(tc.path)
}

// perturbTrace applies the whole-trace perturbation the byte b selects (most
// bytes select none) and says which one it was.
func perturbTrace(b byte, steps []*trace.Trace, recipe []string) string {
	st := steps[int(b>>4)%len(steps)]
	switch b % 24 {
	case 0:
		st.Procs[1].Exit = nil
		return "a step with no exit line"
	case 1:
		st.Procs[1].Exit = &trace.Exit{Pid: 1, Killed: true, Signal: "SIGKILL"}
		return "a killed step"
	case 2:
		st.Procs[2].Exit = nil
		return "a process with no exit line"
	case 3:
		recipe[int(b>>4)%len(recipe)] = "another step"
		return "a recipe mismatch"
	}
	return ""
}

// checkWrites is the writes-complete invariant: every in-repository path the
// model really wrote is under a write root of Writes, or is a TrackedWrites
// path itself (the classifier's own coarsening).
func checkWrites(t *testing.T, m *model, rec *RawRecord) {
	t.Helper()
	for _, tc := range m.touches {
		if !tc.write && !tc.wonly {
			continue
		}
		rel, in := strings.CutPrefix(tc.path, m.root+"/")
		if !in || m.readOnlyHost(tc.path) {
			continue
		}
		if !underAny(rec.Writes, rel) && !underAny(rec.TrackedWrites, rel) {
			t.Fatalf("WritesComplete, but %s was written and is in no write set\nroot %s home %s tmp %s\nwrites %q tracked %q reasons %q",
				tc.path, m.root, m.home, m.tmp, rec.Writes, rec.TrackedWrites, rec.Reasons)
		}
	}
}

func inSet(set []string, v string) bool {
	for _, s := range set {
		if s == v {
			return true
		}
	}
	return false
}

func underAny(set []string, rel string) bool {
	for _, s := range set {
		if rel == s || strings.HasPrefix(rel, s+"/") {
			return true
		}
	}
	return false
}

func check(t *testing.T, m *model, tree *memTree, rec *RawRecord) {
	t.Helper()
	fail := func(tc touch, what string) {
		t.Fatalf("%s: looked at %s %+v\nroot %s home %s tmp %s\nentries %q absent %q untracked %q writes %q",
			what, tc.path, tc, m.root, m.home, m.tmp, rec.Entries, rec.AbsentObservations, rec.UntrackedObservations, rec.Writes)
	}
	for _, tc := range m.touches {
		if tc.path == "" || tc.path == m.root {
			continue
		}
		if tc.path == m.root+"/.git" && !tc.write && !tc.wonly {
			continue // a stat of ".git" itself is nothing, by design (section 5.2)
		}
		rel, in := strings.CutPrefix(tc.path, m.root+"/")
		// KILN04 D39/D40: a write-class call that changed a proper ancestor of
		// the checkout is host-read, so a record with no reason never has one.
		// (The first such write is the one that counts: a name the action made
		// itself is its own.)
		if (tc.write || tc.wonly) && !tc.own && !tc.create && !m.own[tc.path] && m.isCheckoutAncestor(tc.path) {
			fail(tc, "a write to an ancestor of the checkout, but the record has no reason")
		}
		if tc.wonly {
			if in && !underAny(rec.Writes, rel) && !m.readOnlyHost(tc.path) {
				fail(tc, "write not under a write root")
			}
			continue
		}
		if !in {
			if tc.own || tc.create || tc.path == m.tmp || tc.path == m.home || m.isScratchDir(tc.path) || m.readOnlyHost(tc.path) {
				continue
			}
			// KILN04 D43: an absent probe of a path the sealed root cannot hold.
			if (tc.absent || m.byPath(tc.path) == nil) && !m.sandboxHas(tc.path) {
				continue
			}
			// KILN04 D39: a present, non-listing look at a proper ancestor of
			// the checkout, Home or Tmpdir, and an absent probe strictly inside
			// the fresh Home or Tmpdir (never the checkout or its ancestors).
			if !tc.absent && !tc.list && m.properAncestor(tc.path) {
				continue
			}
			// (A stop the model recorded as present because the call's errno
			// was ENOTDIR from its other operand is still absent on the model's
			// disk: the fresh tmpfs holds only what the action made.)
			if (tc.absent || m.byPath(tc.path) == nil) && m.freshAbsent(tc.path) {
				continue
			}
			// KILN04 D40: with no symbolic link made inside the fresh tmpfs, any
			// look strictly inside it found the action's own file or nothing,
			// whatever the model's disk says: it is not an input.
			if !m.tmpLinked && m.freshAbsent(tc.path) {
				continue
			}
			fail(tc, "outside the repository, but the record has no reason")
		}
		if tc.write && !underAny(rec.Writes, rel) {
			fail(tc, "write not under a write root")
		}
		if tc.own {
			continue
		}
		if tc.list {
			if !inSet(rec.Entries, "list:"+rel) {
				fail(tc, "listing not covered")
			}
			continue
		}
		if !covered(rec, rel) {
			fail(tc, "path not covered")
		}
		if tc.absent && tree.Stat(rel).Kind == KindAbsent && !underAny(rec.AbsentObservations, rel) {
			// An absence is covered by an absent observation on a prefix.
			ok := false
			for _, a := range rec.AbsentObservations {
				ok = ok || rel == a || strings.HasPrefix(rel, a+"/")
			}
			if !ok {
				fail(tc, "absence not recorded")
			}
		}
	}
}

// properAncestor says p is a proper ancestor, "/" included, of the checkout,
// Home or Tmpdir: the model's reading of D39 rule (a).  (The model declares no
// other anchors; the host-root and ephemeral anchors are covered by
// ancestor_test.go.)
//
// The node must also be a directory on the model's disk (D39 hardening: a
// symlink or file there is judged by the ordinary rules).  The model issues no
// getxattr family calls, so the xattr exclusion has no model counterpart; it is
// covered by ancestor_test.go.
func (m *model) properAncestor(p string) bool {
	for _, a := range append([]string{m.root, m.home, m.tmp}, modelScratch...) {
		if p == "/" || strings.HasPrefix(a, p+"/") {
			n := m.byPath(p)
			return n != nil && n.kind == KindDir
		}
	}
	return false
}

// freshAbsent says p is strictly inside Home or Tmpdir and is neither inside
// the checkout nor an ancestor of it: the model's reading of D39 rule (b).
func (m *model) freshAbsent(p string) bool {
	if p == m.root || strings.HasPrefix(p, m.root+"/") || strings.HasPrefix(m.root, p+"/") {
		return false
	}
	for _, d := range append([]string{m.home, m.tmp}, modelScratch...) {
		if strings.HasPrefix(p, d+"/") && !m.ancestorOfMount(p) {
			return true
		}
	}
	return false
}

// ancestorOfMount says p is a directory the plan makes only to mount Home or
// Tmpdir beneath it: it exists in every run, so it is not fresh.
func (m *model) ancestorOfMount(p string) bool {
	for _, a := range []string{m.home, m.tmp} {
		if strings.HasPrefix(a, p+"/") {
			return true
		}
	}
	return false
}

// isScratchDir says p is one of the scratch mount points: a node of the plan.
func (m *model) isScratchDir(p string) bool {
	for _, d := range modelScratch {
		if p == d {
			return true
		}
	}
	return false
}

// mountOf names the sandbox mount p lives on, when it is one of the three the
// model distinguishes: "repo" (the bound checkout), "tmp" and "home" (the
// scratch tmpfs).  "" is everything else (the read-only host).
func (m *model) mountOf(p string) string {
	switch {
	case p == m.root || strings.HasPrefix(p, m.root+"/"):
		return "repo"
	case strings.HasPrefix(p, m.tmp+"/") || p == m.tmp:
		return "tmp"
	case strings.HasPrefix(p, m.home+"/") || p == m.home:
		return "home"
	}
	for _, d := range modelScratch {
		if p == d || strings.HasPrefix(p, d+"/") {
			return "scratch" + d
		}
	}
	return ""
}

// isCheckoutAncestor says p is a proper ancestor of the checkout other than
// "/" and Home itself (Home is the sandbox's mount point, never movable).
func (m *model) isCheckoutAncestor(p string) bool {
	return p != "/" && p != m.home && p != m.tmp && strings.HasPrefix(m.root, p+"/")
}

func covered(rec *RawRecord, rel string) bool {
	has := map[string]bool{}
	for _, e := range rec.Entries {
		has[e] = true
	}
	for _, u := range rec.UntrackedObservations {
		if u == rel {
			return true
		}
	}
	ix := NewPackageIndex([]string{"code/p/a", "code/p/b"})
	if q, ok := ix.Enclosing(rel); ok {
		return has["pkg:"+q]
	}
	for p := rel; ; {
		if has["path:"+p] {
			return true
		}
		i := strings.LastIndexByte(p, '/')
		if i < 0 {
			return false
		}
		p = p[:i]
	}
}
