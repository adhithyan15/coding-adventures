// Package trace parses the output of
//
//	strace -f --quiet=attach -y -s 4096 -e signal=none -e trace=... -o <file>
//
// into an ordered list of path events plus the process tree, exactly as
// code/specs/KILN04-traced-read-sets.md §4.3-§5.1 describes (stage 1a).
//
// # What this package does, and what it does not
//
// It is PURE.  It never runs strace, never touches the filesystem and never
// resolves a path.  It reads text and answers two questions:
//
//   - what did every process of the traced action try to do, in the order the
//     tracer saw it (Trace.Events), and
//   - is that account COMPLETE (every process has an exit line, every split
//     call is resumed, every system call is one we know the meaning of)?
//
// Paths are reported "as printed": a relative path stays relative, and the
// event carries the thing it is relative to (Event.Cwd, the process's working
// directory expression at that moment, or Event.DirfdPath, the -y path of the
// directory descriptor).  Walking those paths component by component against
// the post-run tree, expanding symlinks, applying ".." to resolved directories
// and judging stability is stage 1b.  The boundary is deliberate: this package
// can be fuzzed and tested on canned text with no filesystem at all.
//
// # Fail closed
//
// Anything it cannot place with certainty is an *Error whose message is
// "trace-incomplete:<detail>".  A partial trace would yield a manifest that
// misses a read, and a manifest that misses a read produces false cache hits.
//
// # The working directory
//
// Each process has a working directory.  It starts as the initialCwd given to
// Parse; chdir(path) and fchdir(fd) change it; fork/vfork/clone copy it to the
// child, or SHARE it when the clone flags include CLONE_FS (a later chdir by
// either is then seen by both).  The tracked value is an UNRESOLVED
// expression: after chdir("../x") from /a/b it is "/a/b/../x", because only a
// filesystem walk (stage 1b) can say where ".." leads through a symlink.
// fchdir is the exception: the -y annotation is the kernel's own resolved
// path, so that is used verbatim.  The parser never relies on an AT_FDCWD
// annotation.
package trace

import (
	"io"
	"sort"
	"strings"
)

// Result is a call's outcome.
type Result struct {
	// OK is true when the call succeeded (strace printed a non-error value).
	OK bool
	// Errno is the errno name ("ENOENT", "EINPROGRESS") of a failed call.
	Errno string
	// Value is the number returned: a descriptor for open, the child pid for
	// fork and clone.  Zero when none was printed.
	Value int64
}

// Event is one observation or write attempt.  A failed path call is still an
// event (an ENOENT is an observation that the path was absent).
type Event struct {
	Line    int    // 1-based line of the trace where the call completed
	Pid     int    // process (thread) that made the call
	Syscall string // e.g. "openat"
	Kind    Kind
	// Cwd is the process's working directory expression when the call ran.
	Cwd string
	// Path is the primary path argument, as printed (unresolved).  For
	// listing and fchdir it is the directory descriptor's -y path; for
	// utimensat(fd, NULL, ...) it is that descriptor's path.  For rename,
	// link and linkat it is the SOURCE; for symlink it is the link being
	// created.
	Path string
	// Path2 is the destination of rename, renameat, renameat2, link, linkat.
	Path2 string
	// Target is the text stored in a symlink (symlink, symlinkat).  It is data,
	// not a path the call looked at.
	Target string
	// DirfdPath is the -y path of the directory descriptor Path is relative to.
	// Empty means AT_FDCWD (or a call without a dirfd): relative paths are
	// relative to Cwd.  Dirfd2Path is the same for Path2.
	DirfdPath  string
	Dirfd2Path string
	// ReturnFDPath is the -y annotation of the descriptor a successful open
	// returned: the kernel's own resolved path (or a special name such as
	// "pipe:[123]").  An open of /proc/self/fd/N is classified by this.
	ReturnFDPath string
	Result       Result
	// Flags is the raw text of the flags argument (open flags, the clone flag
	// set, socket domain/type, ...), left for stage 1b to interpret.
	Flags string
	// Addr is the destination of a network call; nil when there is none.
	Addr *Addr
}

// Exit is a process's exit line.
type Exit struct {
	Pid        int
	Code       int    // exit status, when not Killed
	Killed     bool   // "killed by SIG..."
	Signal     string // e.g. "SIGKILL"
	CoreDumped bool
}

// Process is a node of the process tree.
type Process struct {
	Pid        int
	Parent     int    // 0 for the root
	ShareFS    bool   // created with CLONE_FS: shares its parent's cwd
	CloneFlags string // raw clone flags, "" for fork/vfork/root
	// LastExec is the path of the last successful execve/execveat, as printed.
	LastExec string
	Exit     *Exit
	Children []int // in spawn order
}

// Trace is the parsed result.
type Trace struct {
	RootPid int
	Events  []Event
	Procs   map[int]*Process
	Order   []int  // pids in the order they first appeared
	Exits   []Exit // exit lines in trace order
}

// cwdCell is shared between processes that share a filesystem context.
type cwdCell struct{ path string }

type proc struct {
	*Process
	cwd    *cwdCell
	exited bool
}

type spawn struct {
	parent  int
	shareFS bool
	flags   string
}

type model struct {
	lim       Limits
	root      int
	initCwd   string
	trace     *Trace
	procs     map[int]*proc
	spawns    map[int]spawn
	spawnList []int
	visiting  map[int]bool
}

// Parse reads a strace trace from r.  initialCwd is the working directory of
// the first process (absolute).  Zero fields of limits take DefaultLimits.
// Every failure is an *Error whose message begins "trace-incomplete:".
func Parse(r io.Reader, initialCwd string, limits Limits) (*Trace, error) {
	lim := limits.withDefaults()
	if !strings.HasPrefix(initialCwd, "/") || strings.IndexByte(initialCwd, 0) >= 0 || len(initialCwd) > 4096 {
		return nil, incomplete(0, reasonInitialCwd)
	}
	recs, root, e := readRecords(r, lim)
	if e != nil {
		return nil, e
	}
	if root == 0 {
		return nil, incomplete(0, reasonEmpty)
	}
	m := &model{
		lim: lim, root: root, initCwd: initialCwd,
		trace:    &Trace{RootPid: root, Procs: map[int]*Process{}},
		procs:    map[int]*proc{},
		spawns:   map[int]spawn{},
		visiting: map[int]bool{},
	}
	if e := m.buildSpawns(recs); e != nil {
		return nil, e
	}
	for i := range recs {
		if e := m.apply(&recs[i]); e != nil {
			return nil, e
		}
	}
	if e := m.finish(); e != nil {
		return nil, e
	}
	return m.trace, nil
}

// buildSpawns scans every completed fork/vfork/clone for the child pid it
// returned.  This is a separate pass because strace prints the PARENT's return
// line after the child may already have started running, so a child's first
// lines can precede the line that says whose child it is.
func (m *model) buildSpawns(recs []record) *Error {
	for i := range recs {
		c := recs[i].call
		if c == nil {
			continue
		}
		var share bool
		var flags string
		switch c.name {
		case "fork", "vfork":
		case "clone":
			var r string
			flags, share, r = analyzeClone(c.args)
			if r != "" {
				return incomplete(recs[i].line, r)
			}
		default:
			continue
		}
		if !c.ret.ok() {
			continue
		}
		if !c.ret.hasVal || c.ret.value <= 0 || c.ret.value > 1<<30 {
			return incomplete(recs[i].line, reasonResult)
		}
		child := int(c.ret.value)
		if _, dup := m.spawns[child]; dup || child == m.root || child == recs[i].pid {
			return incomplete(recs[i].line, reasonPidReuse)
		}
		m.spawns[child] = spawn{parent: recs[i].pid, shareFS: share, flags: flags}
		m.spawnList = append(m.spawnList, child)
	}
	return nil
}

// analyzeClone reads the flags= argument of a clone call.  A clone that could
// escape the tracer (CLONE_UNTRACED) or the sandbox's namespaces (CLONE_NEW*)
// is refused.  Unknown tokens (a hex leftover, say) are refused too: a flag we
// cannot name might mean anything.
func analyzeClone(args []string) (flags string, shareFS bool, reason string) {
	for _, a := range args {
		v, ok := strings.CutPrefix(a, "flags=")
		if !ok {
			continue
		}
		for _, tok := range strings.Split(v, "|") {
			tok = strings.TrimSpace(tok)
			switch {
			case tok == "CLONE_UNTRACED", strings.HasPrefix(tok, "CLONE_NEW"):
				return "", false, reasonCloneFlags
			case tok == "CLONE_FS":
				shareFS = true
			}
			if !(strings.HasPrefix(tok, "CLONE_") || strings.HasPrefix(tok, "SIG")) || !isErrnoName(tok) {
				return "", false, reasonCloneFlags
			}
		}
		return v, shareFS, ""
	}
	return "", false, reasonCloneFlags
}

// get returns the state of pid, creating it from its spawn record (or as the
// root) the first time it is seen.
func (m *model) get(pid, line int) (*proc, *Error) {
	if p := m.procs[pid]; p != nil {
		return p, nil
	}
	if m.visiting[pid] {
		return nil, incomplete(line, reasonProcessCycle)
	}
	p := &proc{Process: &Process{Pid: pid}}
	if pid == m.root {
		p.cwd = &cwdCell{path: m.initCwd}
	} else {
		sp, ok := m.spawns[pid]
		if !ok {
			return nil, incomplete(line, reasonOrphanPid)
		}
		m.visiting[pid] = true
		parent, e := m.get(sp.parent, line)
		delete(m.visiting, pid)
		if e != nil {
			return nil, e
		}
		p.Parent, p.ShareFS, p.CloneFlags = sp.parent, sp.shareFS, sp.flags
		if sp.shareFS {
			p.cwd = parent.cwd
		} else {
			p.cwd = &cwdCell{path: parent.cwd.path}
		}
	}
	m.procs[pid] = p
	m.trace.Procs[pid] = p.Process
	m.trace.Order = append(m.trace.Order, pid)
	return p, nil
}

func (m *model) apply(rec *record) *Error {
	p, e := m.get(rec.pid, rec.line)
	if e != nil {
		return e
	}
	if rec.exit != nil {
		if p.exited {
			return incomplete(rec.line, reasonDuplicateExit)
		}
		p.exited = true
		ex := *rec.exit
		m.trace.Exits = append(m.trace.Exits, ex)
		p.Exit = &m.trace.Exits[len(m.trace.Exits)-1]
		return nil
	}
	if p.exited {
		return incomplete(rec.line, reasonEventAfterExit)
	}
	evs, e := m.call(p, rec)
	if e != nil {
		return e
	}
	if len(m.trace.Events)+len(evs) > m.lim.MaxEvents {
		return incomplete(rec.line, reasonSize)
	}
	m.trace.Events = append(m.trace.Events, evs...)
	return nil
}

// finish checks completeness: every pid that exists (seen, or returned by a
// clone) has an exit line.  A spawned child with no lines at all is either
// untraced or lost; both are fatal.
func (m *model) finish() *Error {
	for _, child := range m.spawnList {
		if _, e := m.get(child, 0); e != nil {
			return e
		}
		par := m.procs[m.spawns[child].parent]
		par.Children = append(par.Children, child)
	}
	pids := make([]int, 0, len(m.procs))
	for pid := range m.procs {
		pids = append(pids, pid)
	}
	sort.Ints(pids)
	for _, pid := range pids {
		if !m.procs[pid].exited {
			return incomplete(0, reasonMissingExit)
		}
	}
	return nil
}

func toResult(r result) Result {
	return Result{OK: r.ok(), Errno: r.errno, Value: r.value}
}

// dirfdPath resolves a directory-descriptor argument to its -y path.  AT_FDCWD
// gives "" (relative paths use the tracked cwd).
func dirfdPath(raw string) (string, *fdRef, string) {
	f, r := parseFD(raw)
	if r != "" {
		return "", nil, r
	}
	if f.cwd {
		return "", &f, ""
	}
	if !f.hasAnn {
		return "", nil, reasonFDPath
	}
	s, r := fdPathText(f.ann, true)
	return s, &f, r
}

func joinCwd(cwd, p string) (string, bool) {
	n := p
	if !strings.HasPrefix(p, "/") {
		n = strings.TrimRight(cwd, "/") + "/" + p
	}
	return n, len(n) <= 4096
}

// call turns one completed record into events and applies its effect on the
// process model.
func (m *model) call(p *proc, rec *record) ([]Event, *Error) {
	c := rec.call
	ent := lookup(c.name)
	fail := func(r string) ([]Event, *Error) { return nil, incomplete(rec.line, r) }
	if len(c.args) < ent.min || len(c.args) > ent.max {
		return fail(reasonArity)
	}
	if c.ret.unknown {
		return fail(reasonResult)
	}
	ev := Event{Line: rec.line, Pid: rec.pid, Syscall: c.name, Kind: ent.kind, Cwd: p.cwd.path, Result: toResult(c.ret)}
	arg := func(i int) string { return c.args[i-1] }

	var r string
	switch ent.role {
	case roleListing, roleFchdir:
		f, r := parseFD(arg(1))
		if r != "" {
			return fail(r)
		}
		if f.cwd || !f.hasAnn {
			return fail(reasonFDPath)
		}
		ev.Path, r = fdPathText(f.ann, true)
		if r != "" {
			return fail(r)
		}
		if ent.role == roleFchdir && c.ret.ok() {
			p.cwd.path = ev.Path
		}
	case roleFork:
		// nothing to record beyond the result (the child pid)
	case roleClone:
		ev.Flags, _, r = analyzeClone(c.args)
		if r != "" {
			return fail(r)
		}
	case roleSocket:
		ev.Flags = strings.Join(c.args, ", ")
		if strings.HasPrefix(arg(1), "AF_") && isErrnoName(arg(1)) {
			ev.Addr = &Addr{Family: arg(1), Port: -1}
		}
	case roleListen:
		ev.Flags = arg(2)
	case roleSockaddr:
		ev.Addr, r = parseSockaddr(arg(ent.addr))
		if r != "" {
			return fail(r)
		}
	case roleMsg:
		socks, r := findSockaddrs(arg(ent.addr))
		if r != "" {
			return fail(r)
		}
		seen := map[string]bool{}
		var evs []Event
		for _, s := range socks {
			if seen[s] {
				continue
			}
			seen[s] = true
			e2 := ev
			if e2.Addr, r = parseSockaddr(s); r != "" {
				return fail(r)
			}
			evs = append(evs, e2)
		}
		if len(evs) == 0 {
			evs = append(evs, ev)
		}
		return evs, nil
	default: // rolePath, roleCreat, roleChdir, roleExec, roleSymlink
		if r := m.fillPath(&ev, ent, c.args); r != "" {
			return fail(r)
		}
		if ent.role == roleCreat {
			ev.Flags = "O_CREAT|O_WRONLY|O_TRUNC"
		}
		if ent.role == roleChdir && c.ret.ok() {
			n, ok := joinCwd(p.cwd.path, ev.Path)
			if !ok {
				return fail(reasonCwdTooLong)
			}
			p.cwd.path = n
		}
		if ent.role == roleExec && c.ret.ok() {
			p.LastExec = ev.Path
			if ev.Path == "" {
				p.LastExec = ev.DirfdPath
			}
		}
		if ent.kind == KindOpen && c.ret.ok() {
			if !c.ret.hasAnn {
				return fail(reasonFDPath)
			}
			ev.ReturnFDPath, r = fdPathText(c.ret.ann, false)
			if r != "" {
				return fail(r)
			}
		}
	}
	return []Event{ev}, nil
}

// fillPath extracts dirfd, path, dirfd2, path2, flags and symlink target
// according to the table entry.
func (m *model) fillPath(ev *Event, ent entry, args []string) string {
	var r string
	if ent.dirfd > 0 {
		d, f, r2 := dirfdPath(args[ent.dirfd-1])
		if r2 != "" {
			return r2
		}
		ev.DirfdPath = d
		if ent.pth > 0 && ent.nullPath && args[ent.pth-1] == "NULL" {
			// utimensat(fd, NULL, ...) acts on the descriptor itself.
			if f.cwd {
				return reasonDirfd
			}
			ev.Path, ev.DirfdPath = d, ""
			goto rest
		}
	}
	if ent.pth > 0 {
		if ev.Path, r = pathArg(args[ent.pth-1]); r != "" {
			return r
		}
	}
rest:
	if ent.dirfd2 > 0 {
		if ev.Dirfd2Path, _, r = dirfdPath(args[ent.dirfd2-1]); r != "" {
			return r
		}
	}
	if ent.pth2 > 0 {
		if ev.Path2, r = pathArg(args[ent.pth2-1]); r != "" {
			return r
		}
	}
	if ent.target > 0 {
		if ev.Target, r = pathArg(args[ent.target-1]); r != "" {
			return r
		}
	}
	if ent.flags > 0 {
		ev.Flags = args[ent.flags-1]
	}
	return ""
}
