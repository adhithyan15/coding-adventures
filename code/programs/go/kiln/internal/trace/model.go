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
	// InitPid is the sandbox's PID-1 child under Options.HelperRoot, else 0.
	InitPid int
}

// StepExit is the exit status of the step: the init child's under HelperRoot,
// otherwise the root's.
func (t *Trace) StepExit() *Exit {
	pid := t.RootPid
	if t.InitPid != 0 {
		pid = t.InitPid
	}
	if p := t.Procs[pid]; p != nil {
		return p.Exit
	}
	return nil
}

// cwdCell is the filesystem context (the working directory) that processes
// created with CLONE_FS share.  refs counts the LIVE processes using it.
type cwdCell struct {
	path string
	refs int
}

type proc struct {
	*Process
	cwd      *cwdCell
	exited   bool
	startCwd string // cwd sampled at the "<unfinished ...>" line of a split call
	hasStart bool
}

type spawn struct {
	parent  int
	shareFS bool
	flags   string
	line    int // line of the parent's completed fork/clone
}

// maxProcessDepth bounds the parent chain walked when a process is created.
const maxProcessDepth = 4096

// Options configures ParseOptions.
type Options struct {
	Limits Limits
	// HelperRoot says the first (root) process is `kiln sandbox-exec`, the
	// sandbox helper of KILN04 section 4.3, and the trace has this shape:
	//
	//	helper (root) --spawns exactly one--> init child (PID 1 of the new PID
	//	namespace) --setup, then execve--> the step (sh -c <line>)
	//
	// All of the root's events are discarded.  It may spawn once (a second
	// spawn is helper-spawn); after that it may make only ignored-class calls
	// such as wait4 (anything else is helper-activity).  The init child's events
	// before its first SUCCESSFUL execve are setup and are discarded too, and
	// there unknown or refused names (unshare, mount, pivot_root, seccomp) are
	// tolerated; a spawn by it before that exec is helper-spawn.  The exec event
	// is kept, the child's working directory becomes the initialCwd given to
	// Parse (setup chdirs are discarded), and refusals apply normally from then
	// on.  Without a successful exec the trace is helper-no-exec.  The step's
	// exit status is the init child's (Trace.StepExit).
	HelperRoot bool
}

type model struct {
	lim       Limits
	opts      Options
	root      int
	initCwd   string
	trace     *Trace
	procs     map[int]*proc
	spawns    map[int]spawn
	spawnList []int
	initPid   int  // helper mode: the root's one child
	initExec  bool // helper mode: the init child has exec'd the step
	rootSpawn bool // helper mode: the root has spawned its child
}

// Parse reads a strace trace from r.  initialCwd is the working directory of
// the first process (absolute).  Zero fields of limits take DefaultLimits.
// Every failure is an *Error whose message begins "trace-incomplete:".
func Parse(r io.Reader, initialCwd string, limits Limits) (*Trace, error) {
	return ParseOptions(r, initialCwd, Options{Limits: limits})
}

// ParseOptions is Parse with options.
func ParseOptions(r io.Reader, initialCwd string, opts Options) (*Trace, error) {
	lim := opts.Limits.withDefaults()
	if !strings.HasPrefix(initialCwd, "/") || strings.IndexByte(initialCwd, 0) >= 0 || len(initialCwd) > 4096 {
		return nil, incomplete(0, reasonInitialCwd)
	}
	recs, root, e := readRecords(r, lim, opts.HelperRoot)
	if e != nil {
		return nil, e
	}
	if root == 0 {
		return nil, incomplete(0, reasonEmpty)
	}
	m := &model{
		lim: lim, opts: opts, root: root, initCwd: initialCwd,
		trace:  &Trace{RootPid: root, Procs: map[int]*Process{}},
		procs:  map[int]*proc{},
		spawns: map[int]spawn{},
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
	m.trace.InitPid = m.initPid
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
				if m.opts.HelperRoot && recs[i].pid == m.root {
					// The helper itself may clone into new namespaces; that is
					// its job.  Its child is never treated as sharing anything.
					flags, share = "", false
				} else {
					return incomplete(recs[i].line, r)
				}
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
		if m.opts.HelperRoot && recs[i].pid == m.root {
			if m.initPid != 0 {
				return incomplete(recs[i].line, reasonHelperSpawn)
			}
			m.initPid = child
		}
		m.spawns[child] = spawn{parent: recs[i].pid, shareFS: share, flags: flags, line: recs[i].line}
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

// get returns the state of pid, creating it the first time it is seen.
//
// A child is normally created the moment its parent's fork/vfork/clone
// SUCCEEDS (see apply), which snapshots the parent's working directory right
// then; a later chdir by the parent must not leak into the child.  The only
// time get itself creates a child is when the child's lines were printed
// BEFORE the parent's return line (the parent is still blocked inside the
// call, so its cwd is the clone-time cwd).  That is only possible while the
// spawn record is still ahead of the current line; otherwise the child should
// already exist and the pid is an orphan.
//
// The parent chain is walked iteratively with a depth cap, so a crafted chain
// of a million pids cannot exhaust the stack.
func (m *model) get(pid, line int) (*proc, *Error) {
	if p := m.procs[pid]; p != nil {
		return p, nil
	}
	var chain []int
	seen := map[int]bool{}
	cur := pid
	for m.procs[cur] == nil && cur != m.root {
		sp, ok := m.spawns[cur]
		if !ok || sp.line < line {
			return nil, incomplete(line, reasonOrphanPid)
		}
		if seen[cur] || len(chain) >= maxProcessDepth {
			return nil, incomplete(line, reasonProcessCycle)
		}
		seen[cur] = true
		chain = append(chain, cur)
		cur = sp.parent
	}
	if m.procs[cur] == nil { // the root
		m.add(&proc{Process: &Process{Pid: cur}, cwd: &cwdCell{path: m.initCwd, refs: 1}})
	}
	for k := len(chain) - 1; k >= 0; k-- {
		child := chain[k]
		sp := m.spawns[child]
		parent := m.procs[sp.parent]
		p := &proc{Process: &Process{Pid: child, Parent: sp.parent, ShareFS: sp.shareFS, CloneFlags: sp.flags}}
		if sp.shareFS {
			p.cwd = parent.cwd
			p.cwd.refs++
		} else {
			p.cwd = &cwdCell{path: parent.cwd.path, refs: 1}
		}
		m.add(p)
	}
	return m.procs[pid], nil
}

func (m *model) add(p *proc) {
	m.procs[p.Pid] = p
	m.trace.Procs[p.Pid] = p.Process
	m.trace.Order = append(m.trace.Order, p.Pid)
}

// setup reports whether pid is, right now, running sandbox setup rather than
// the step: the helper root always, the init child until it has exec'd.
func (m *model) setup(pid int) bool {
	return m.opts.HelperRoot && (pid == m.root || (pid == m.initPid && !m.initExec))
}

func isSpawn(name string) bool { return name == "fork" || name == "vfork" || name == "clone" }

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
		p.cwd.refs--
		m.trace.Exits = append(m.trace.Exits, *rec.exit)
		p.Exit = rec.exit // its own allocation: never aliases the slice above
		return nil
	}
	if p.exited {
		return incomplete(rec.line, reasonEventAfterExit)
	}
	if m.opts.HelperRoot && rec.pid == m.root {
		return m.applyHelperRoot(rec)
	}
	if rec.start {
		p.startCwd, p.hasStart = p.cwd.path, true
		return nil
	}
	c := rec.call
	setup := m.setup(rec.pid)
	if m.opts.HelperRoot {
		// Classification was deferred in pass 1.
		if e := classifyName(c.name, rec.line); e != nil {
			if setup {
				return nil // sandbox setup: unknown or refused is tolerated
			}
			return e
		}
	}
	evs, e := m.call(p, rec)
	if e != nil {
		return e
	}
	if isSpawn(c.name) && c.ret.ok() {
		if setup {
			return incomplete(rec.line, reasonHelperSpawn)
		}
		// Create the child NOW, copying (or sharing) the parent's cwd as it
		// is at this moment.
		if _, e := m.get(int(c.ret.value), rec.line-1); e != nil {
			return e
		}
	}
	if setup {
		if !(isExecName(c.name) && c.ret.ok()) {
			evs = nil // setup: not part of the step
		} else {
			// The step starts where the executor says, whatever setup did.
			ev := &evs[0]
			if ev.Path != "" && !strings.HasPrefix(ev.Path, "/") && ev.DirfdPath == "" && ev.Cwd != m.initCwd {
				return incomplete(rec.line, reasonHelperCwd)
			}
			m.initExec = true
			p.cwd.path = m.initCwd
			ev.Cwd = m.initCwd
		}
	}
	if len(m.trace.Events)+len(evs) > m.lim.MaxEvents {
		return incomplete(rec.line, reasonSize)
	}
	m.trace.Events = append(m.trace.Events, evs...)
	return nil
}

// applyHelperRoot handles a record of the sandbox helper.  Nothing it does
// becomes an event.  Its one spawn creates the init child; after that only
// ignored-class calls (which pass 1 has already dropped) are allowed.
func (m *model) applyHelperRoot(rec *record) *Error {
	if rec.start {
		return nil
	}
	c := rec.call
	if m.rootSpawn {
		return incomplete(rec.line, reasonHelperActivity)
	}
	if isSpawn(c.name) && c.ret.ok() {
		m.rootSpawn = true
		if _, e := m.get(int(c.ret.value), rec.line-1); e != nil {
			return e
		}
	}
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
	if m.opts.HelperRoot {
		if m.initPid == 0 {
			return incomplete(0, reasonHelperNoInit)
		}
		if !m.initExec {
			return incomplete(0, reasonHelperNoExec)
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
	// A split call ran in the directory that was current when it STARTED.
	cwdAtCall := p.cwd.path
	if rec.split && p.hasStart {
		cwdAtCall = p.startCwd
	}
	p.hasStart = false
	ev := Event{Line: rec.line, Pid: rec.pid, Syscall: c.name, Kind: ent.kind, Cwd: cwdAtCall, Result: toResult(c.ret)}
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
			if p.cwd.refs > 1 {
				return fail(reasonCwdRace)
			}
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
		// The domain must be a symbolic AF_* name; a number or pointer means
		// strace could not name the address family, so we cannot classify it.
		if !strings.HasPrefix(arg(1), "AF_") || !isErrnoName(arg(1)) {
			return fail(reasonSockaddr)
		}
		ev.Addr = &Addr{Family: arg(1), Port: -1}
	case roleListen:
		ev.Flags = arg(2)
	case roleSockaddr:
		ev.Addr, r = parseSockaddr(arg(ent.addr), c.name == "sendto")
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
			if e2.Addr, r = parseSockaddr(s, false); r != "" {
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
			if p.cwd.refs > 1 {
				// Another live process shares this cwd (CLONE_FS): its
				// relative paths would silently change under our feet.
				return fail(reasonCwdRace)
			}
			n, ok := joinCwd(cwdAtCall, ev.Path)
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
	if ev.Path == "" && ev.DirfdPath != "" && ent.flags > 0 && hasFlag(args[ent.flags-1], "AT_EMPTY_PATH") {
		// fexecve, fstat-by-statx and friends: the descriptor itself is the
		// object, so its -y path is the path.
		ev.Path, ev.DirfdPath = ev.DirfdPath, ""
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

// hasFlag reports whether the "A|B|C" flag text names flag.
func hasFlag(text, flag string) bool {
	for _, f := range strings.Split(text, "|") {
		if strings.TrimSpace(f) == flag {
			return true
		}
	}
	return false
}
