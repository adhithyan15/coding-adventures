package traced

import (
	"debug/elf"
	"encoding/binary"
	"io/fs"
	"strings"
	"testing"

	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/trace"
)

// A DIFFERENTIAL FUZZER for the classifier.
//
// An in-memory filesystem MODEL executes random operations (mkdir, symlink,
// rename, link, unlink, rmdir, open, stat, lstat, chdir, exec) with Linux-like
// semantics, so it knows the TRUTH: which path each call really resolved to and
// which symlinks it really crossed.  It emits the trace events a tracer would
// see (with the true results and -y paths) and hands its FINAL state to the
// classifier as the post-run disk.
//
// The oracle (KILN04 section 5.1/5.2): for every path the model really looked
// at, one of these must hold, or the classifier is unsound:
//
//	(i)   the path is the action's own (it or an ancestor was created),
//	(ii)  the record covers it: the package's pkg: entry, a path: entry on a
//	      prefix of it, or an untracked observation, or
//	(iii) the record carries a reason.
//
// A record with a reason is never turned into a manifest, so (iii) is always
// safe; the fuzzer is looking for records WITHOUT reasons that miss a read.

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
}

type model struct {
	root    *mnode
	cwdPath string
	cwdExpr string
	own     map[string]bool
	touches []touch
	events  []trace.Event
}

func (m *model) mkpath(abs string, n *mnode) {
	parts := splitPath(abs)
	cur := m.root
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
	cur := m.root
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
	node   *mnode // the final object, nil when absent
	parent *mnode // the directory holding the final name (when the walk got there)
	name   string
	final  string // resolved path of the final name
	stop   string // where the walk stopped on an error mid-path
	errno  string
	links  []string
	isDot  bool // the path ended in "." or ".."
}

func (m *model) frames() []frame {
	fr := []frame{{m.root, ""}}
	cur := m.root
	path := ""
	for _, p := range splitPath(m.cwdPath) {
		cur = cur.kids[p]
		path += "/" + p
		fr = append(fr, frame{cur, path})
	}
	return fr
}

// resolve is the model's namei.
func (m *model) resolve(p string, follow bool) mres {
	var r mres
	if p == "" {
		r.errno = "ENOENT"
		return r
	}
	trailing := strings.HasSuffix(p, "/")
	if trailing {
		follow = true
	}
	var stack []frame
	if strings.HasPrefix(p, "/") {
		stack = []frame{{m.root, ""}}
	} else {
		stack = m.frames()
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
			if last {
				r.errno = "ENOENT"
				return r
			}
			r.stop, r.errno = full, "ENOENT"
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
				stack = []frame{{m.root, ""}}
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
	// Ran out of components after following links to a directory (e.g. "L"
	// where L -> dir): the last frame is the answer.
	top := stack[len(stack)-1]
	r.node, r.final = top.n, pathOrRoot(top.path)
	if len(stack) > 1 {
		r.parent = stack[len(stack)-2].n
	}
	return r
}

func pathOrRoot(p string) string {
	if p == "" {
		return "/"
	}
	return p
}

func (m *model) owned(p string, self bool) bool {
	for q := p; q != ""; q = parentOf(q) {
		if (self || q != p) && m.own[q] {
			return true
		}
	}
	return false
}

func (m *model) touchPath(p string, create bool) {
	m.touches = append(m.touches, touch{path: p, create: create, own: m.owned(p, true)})
}

// look records what a lookup really looked at: the links it crossed and the
// final (or stopping) path.
func (m *model) look(r mres, create bool) {
	for _, l := range r.links {
		m.touchPath(l, false)
	}
	switch {
	case r.stop != "":
		m.touchPath(r.stop, false)
	case r.final != "":
		m.touchPath(r.final, create)
	}
}

func (m *model) emit(e trace.Event, errno string) {
	e.Cwd = m.cwdExpr
	e.Result = trace.Result{OK: errno == "", Errno: errno}
	m.events = append(m.events, e)
}

func (m *model) join(p string) string { return p }

func (m *model) doStat(p string, follow bool) {
	r := m.resolve(p, follow)
	m.look(r, false)
	sys := "stat"
	if !follow {
		sys = "lstat"
	}
	e := base(sys, trace.KindMetadata, p)
	m.emit(e, r.errno)
}

func (m *model) doChdir(p string) {
	r := m.resolve(p, true)
	m.look(r, false)
	errno := r.errno
	if errno == "" && (r.node == nil || r.node.kind != KindDir) {
		errno = "ENOTDIR"
	}
	m.emit(base("chdir", trace.KindDirectory, p), errno)
	if errno == "" {
		m.cwdPath = r.final
		if m.cwdPath == "/" {
			m.cwdPath = ""
		}
		if strings.HasPrefix(p, "/") {
			m.cwdExpr = p
		} else {
			m.cwdExpr = strings.TrimRight(m.cwdExpr, "/") + "/" + p
		}
	}
}

func (m *model) underCwd(abs string) bool {
	return abs != "" && (m.cwdPath == abs || strings.HasPrefix(m.cwdPath, abs+"/"))
}

func (m *model) doMkdirLike(sys, p, target string) {
	orig := p
	if sys == "mkdir" {
		p = stripSlashes(p) // mkdir("x/") is mkdir("x"), and never follows a link
	}
	r := m.resolve(p, false)
	create := r.errno == "ENOENT" && r.stop == "" && !r.isDot
	m.look(r, create)
	errno := r.errno
	switch {
	case errno == "" && r.node != nil:
		errno = "EEXIST"
	case errno == "" && r.node == nil:
		errno = "EINVAL"
	}
	if sys == "symlink" && strings.HasSuffix(p, "/") && errno != "" {
		errno = "ENOENT" // a trailing slash asks for a directory
	}
	if errno == "ENOENT" && create && !(sys == "symlink" && strings.HasSuffix(p, "/")) {
		errno = ""
		n := mdir()
		if sys == "symlink" {
			n = &mnode{kind: KindLink, target: target}
		}
		r.parent.kids[r.name] = n
		m.own[r.final] = true
	}
	e := base(sys, trace.KindWrite, orig)
	e.Target = target
	m.emit(e, errno)
}

func (m *model) doUnlink(sys, p string) {
	orig := p
	if sys == "rmdir" {
		p = stripSlashes(p)
	}
	r := m.resolve(p, false)
	m.look(r, false)
	errno := r.errno
	if errno == "" {
		switch {
		case r.isDot || r.node == nil || r.parent == nil:
			errno = "EINVAL"
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
	}
	m.emit(base(sys, trace.KindWrite, orig), errno)
}

func (m *model) doRenameLink(sys, a, b string) {
	src := m.resolve(a, false)
	dst := m.resolve(b, false)
	m.look(src, false)
	m.look(dst, true)
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
	case sys == "link" && src.node.kind == KindDir:
		errno = "EPERM"
	case sys == "link" && dst.node != nil:
		errno = "EEXIST"
	case sys == "rename":
		switch {
		case m.underCwd(src.final):
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
			errno = "ENOTDIR" // a trailing slash names a directory
		}
	}
	e := base(sys, trace.KindWrite, a)
	e.Path2 = b
	if errno == "NOOP" {
		// A rename of a name onto another name of the same file does nothing.
		m.emit(e, "")
		return
	}
	if errno == "" {
		if sys == "rename" {
			delete(src.parent.kids, src.name)
		}
		dst.parent.kids[dst.name] = src.node
		m.own[dst.final] = true
	}
	m.emit(e, errno)
}

func (m *model) doOpen(p string, mode int) {
	flags := []string{"O_RDONLY", "O_WRONLY|O_CREAT|O_TRUNC", "O_WRONLY|O_CREAT|O_EXCL", "O_WRONLY|O_CREAT"}[mode]
	create := mode != 0
	r := m.resolve(p, mode != 2)
	m.look(r, create && r.stop == "")
	errno := r.errno
	final := r.final
	var made bool
	switch {
	case errno == "" && r.node != nil && r.node.kind == KindLink:
		errno = "ELOOP"
	case errno == "" && r.node == nil:
		errno = "EINVAL"
	case errno == "" && mode == 2:
		errno = "EEXIST"
	case errno == "" && r.node.kind == KindDir && mode != 0:
		errno = "EISDIR"
	case errno == "ENOENT" && r.stop == "" && create && strings.HasSuffix(p, "/"):
		errno = "EISDIR"
	case errno == "ENOENT" && r.stop == "" && create && !r.isDot:
		errno = ""
		made = true
		r.parent.kids[r.name] = &mnode{kind: KindFile}
		m.own[final] = true
	}
	if errno == "" && !made && mode == 1 && r.node != nil && r.node.kind == KindFile {
		r.node.data = nil
	}
	e := base("openat", trace.KindOpen, p)
	e.Flags = flags
	if errno == "" {
		e.ReturnFDPath = final
	}
	m.emit(e, errno)
}

// chain follows the kernel's loading chain of an exec and returns the files it
// read (for the oracle), or an errno.
func (m *model) execChain(p string, depth int) ([]string, string) {
	r := m.resolve(p, true)
	m.look(r, false)
	if r.errno != "" {
		return nil, r.errno
	}
	if r.node == nil || r.node.kind != KindFile {
		return nil, "EACCES"
	}
	switch {
	case strings.HasPrefix(string(r.node.data), "#!"):
		if depth >= 5 {
			return nil, "ELOOP"
		}
		interp, ok := shebangInterpreter(r.node.data)
		if !ok {
			return nil, "ENOEXEC"
		}
		_, errno := m.execChain(interp, depth+1)
		return nil, errno
	case len(r.node.data) >= 4 && string(r.node.data[:4]) == elf.ELFMAG:
		return nil, ""
	}
	return nil, "ENOEXEC"
}

func (m *model) doExec(p string) {
	_, errno := m.execChain(p, 0)
	e := base("execve", trace.KindProcess, p)
	e.Argv = []string{"x"}
	m.emit(e, errno)
}

// modelFS is the model's final state as the post-run disk.
type modelFS struct{ m *model }

func (f modelFS) Lstat(p string) (FileInfo, error) {
	n := f.m.byPath(p)
	if n == nil {
		return FileInfo{}, errNotExist
	}
	return FileInfo{Kind: n.kind, Ctime: oldCtime}, nil
}

func (f modelFS) Readlink(p string) (string, error) {
	n := f.m.byPath(p)
	if n == nil || n.kind != KindLink {
		return "", errNotExist
	}
	return n.target, nil
}

func (f modelFS) ReadHead(p string, n int) ([]byte, error) {
	x := f.m.byPath(p)
	if x == nil || x.kind != KindFile {
		return nil, errNotExist
	}
	if len(x.data) > n {
		return x.data[:n], nil
	}
	return x.data, nil
}

// newModel seeds the repository, the image files and the scratch directories,
// and returns the tracked tree of the initial state.
func newModel() (*model, *memTree) {
	m := &model{root: mdir(), cwdExpr: testRoot, cwdPath: testRoot, own: map[string]bool{}}
	tree := newMemTree()
	elfStatic := buildELFStatic()
	add := func(rel string, n *mnode) {
		m.mkpath(testRoot+"/"+rel, n)
		switch n.kind {
		case KindLink:
			tree.link(rel, n.target)
		default:
			tree.add(rel, Node{Kind: KindFile}, string(n.data))
		}
	}
	add("code/p/a/f1", &mnode{kind: KindFile, data: elfStatic})
	add("code/p/a/lnk", &mnode{kind: KindLink, target: "../b"})
	add("code/p/b/f2", &mnode{kind: KindFile, data: []byte("#!/w/repo/code/p/a/f1\n")})
	add("code/p/b/g", &mnode{kind: KindFile, data: []byte("data")})
	add("code/scripts/s.sh", &mnode{kind: KindFile, data: []byte("#!/w/repo/code/p/b/f2\n")})
	add("go.work", &mnode{kind: KindFile, data: []byte("go 1.26\n")})
	m.mkpath("/usr/bin/sh", &mnode{kind: KindFile, data: buildELF(loader, elf.ELFCLASS64, binary.LittleEndian)})
	m.mkpath(loader, &mnode{kind: KindFile, data: []byte("ld")})
	m.mkpath("/usr/lib/x", &mnode{kind: KindFile})
	m.mkpath("/etc/x", &mnode{kind: KindFile})
	m.mkpath("/tmp/kiln-1/seed", mdir())
	delete(m.byPath("/tmp/kiln-1").kids, "seed")
	m.mkpath("/home/runner/seed", mdir())
	delete(m.byPath("/home/runner").kids, "seed")
	return m, tree
}

var modelNames = []string{"code", "p", "a", "b", "f1", "f2", "g", "lnk", "L", "M", "n", "x", "y", "..", ".", "w", "repo", "scripts", "s.sh", "go.work", "new"}

var errNotExist = fs.ErrNotExist

// modelPath builds a path from bytes.
func modelPath(data []byte, i *int) string {
	next := func() byte {
		b := data[*i%len(data)]
		*i++
		return b
	}
	var prefix string
	switch next() % 6 {
	case 0:
		prefix = "/w/repo/"
	case 1:
		prefix = "/tmp/kiln-1/"
	case 2:
		prefix = "/w/"
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

// runModel executes ops from data and returns the model and the initial
// tracked tree.
func runModel(data []byte) (*model, *memTree) {
	m, tree := newModel()
	i := 0
	for ops := 0; i < len(data) && ops < 48; ops++ {
		op := data[i] % 12
		i++
		p := modelPath(data, &i)
		switch op {
		case 0:
			m.doMkdirLike("mkdir", p, "")
		case 1:
			m.doMkdirLike("symlink", p, modelPath(data, &i))
		case 2:
			m.doRenameLink("rename", p, modelPath(data, &i))
		case 3:
			m.doRenameLink("link", p, modelPath(data, &i))
		case 4:
			m.doUnlink("unlink", p)
		case 5:
			m.doUnlink("rmdir", p)
		case 6:
			m.doOpen(p, int(data[i%len(data)])%4)
			i++
		case 7:
			m.doStat(p, true)
		case 8:
			m.doStat(p, false)
		case 9:
			m.doChdir(p)
		case 10:
			m.doExec(p)
		case 11:
			m.doOpen(p, 0)
		}
	}
	return m, tree
}

func FuzzDifferential(f *testing.F) {
	f.Add([]byte{0, 3, 1, 1, 0, 7, 0, 2, 1})
	f.Add([]byte{1, 0, 2, 3, 4, 0, 5, 7, 0, 1, 2, 3})
	f.Add([]byte{3, 5, 5, 5, 9, 2, 2, 2, 7, 6, 6, 6, 2, 1, 1, 1})
	f.Add([]byte("mkdir symlink rename link unlink open stat chdir exec"))
	f.Fuzz(func(t *testing.T, data []byte) {
		if len(data) == 0 {
			return
		}
		m, tree := runModel(data)
		h := newHarness(t)
		h.cfg.Packages = NewPackageIndex([]string{"code/p/a", "code/p/b"})
		h.cfg.Tree = tree
		h.cfg.FS = modelFS{m}
		rec, err := Classify(h.cfg, Input{Package: "code/p/a", Traces: []*trace.Trace{step(m.events...)}, Recipe: []string{"step"}})
		if err != nil {
			// The action moved or removed the checkout's own directories: the
			// post-run disk no longer shows a real Root, which is refused.
			return
		}
		if len(rec.Reasons) > 0 {
			return // a record with a reason is never trusted: always safe
		}
		for _, tc := range m.touches {
			if tc.own || tc.path == "" {
				continue
			}
			rel, in := strings.CutPrefix(tc.path, testRoot+"/")
			if tc.path == testRoot {
				continue
			}
			if !in {
				if tc.create || tc.path == "/tmp/kiln-1" || tc.path == "/home/runner" {
					continue
				}
				t.Fatalf("looked at %s outside the repository, but the record has no reason\nevents: %+v", tc.path, m.events)
			}
			if !covered(rec, rel) {
				t.Fatalf("looked at %s (create=%v) but the record covers nothing for it\nentries %q untracked %q\nevents: %+v",
					tc.path, tc.create, rec.Entries, rec.UntrackedObservations, m.events)
			}
		}
	})
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
