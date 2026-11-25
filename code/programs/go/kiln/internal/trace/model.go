// Package trace parses the output of
//
//	strace -f --quiet=attach -y -s 4096 -e signal=all -e trace=... -o <file>
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
	"strconv"
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
	Line int // 1-based line of the trace where the call completed
	// StartLine is the line where the call STARTED: the "<unfinished ...>" line
	// of a split call, else Line.  A lookup that ran while another process
	// completed a rename is ordered by its start, not its completion.
	StartLine int
	Pid       int    // process (thread) that made the call
	Syscall   string // e.g. "openat"
	Kind      Kind
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
	// SockPath is the -y name of the socket a network call used
	// ("socket:[123]"), or of the descriptor socket() returned; "" when the
	// annotation was absent.
	SockPath string
	// Argv is the argument vector of an execve/execveat call, decoded exactly.
	// It is nil when it could not be decoded exactly (a truncated string, a
	// trailing "..." that means strace printed only the first elements, a
	// pointer): the recipe comparison (stage 1b) then fails closed.
	Argv []string
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
	// ExitLines is parallel to Exits: ExitLines[i] is the 1-based trace line of
	// Exits[i].  (It is a separate slice, not a field of Exit, because many
	// callers compare Exit values.)
	ExitLines []int
	// ThreadOrphans (D44a) are the orphan pids (a pid whose only record is its
	// exit, D44) that are provably threads: every dying spawn the D35 rule
	// dropped was a clone with CLONE_THREAD.  Nil otherwise.  A thread cannot
	// outlive its thread group, so traced.DaemonLeft leaves these exits out of
	// its survivor test; the group's other members are still tested.
	ThreadOrphans map[int]bool
	// InitExitLine is, under Options.HelperRoot, the 1-based start line of the
	// call that ended the step's thread group, made after the step's exec by
	// the init process or a CLONE_THREAD thread of it:
	//
	//   - the earliest exit_group that did not return (one per thread; a
	//     failed call, EPERM from a seccomp filter the step installed, is no
	//     candidate, and a split call's candidacy is withdrawn when its
	//     resumed half shows a result), else
	//   - the start line of the LAST exit (one thread's exit, syscall 60) that
	//     did not return: a single-threaded init may end with a raw exit, and
	//     every pthread ends with one, so only the last counts, else
	//   - 0.
	//
	// 0 therefore means the init group made no exit call at all and died by a
	// signal (or its exit was never printed): the step is already failed and
	// incomplete.  traced.DaemonLeft then counts SIGKILL deaths only.
	//
	// strace prints exit_group at PTRACE_EVENT_EXIT, i.e. when the step's
	// thread group starts to die.  The kernel then kills every other process
	// of the PID namespace; those deaths are the "daemon left behind" and their
	// "+++ killed by SIGKILL +++" lines may come before OR after the init's own
	// "+++ exited +++" line, but (by design) after this call line.  So this
	// line, not init's exit line, is the boundary that tells a daemon from a
	// child the step reaped itself.
	InitExitLine int
	// RestartedSpawns counts the fork, vfork and clone calls that ended
	// `= ? ERESTARTNOINTR` and were accepted as having created nothing (D50,
	// restartedSpawn).  A census counter only: nothing reads it for a verdict.
	RestartedSpawns int
	// BareOpens counts the successful read-only open and openat calls with no
	// descriptor annotation that were accepted (D50, bareReadOnlyOpen).  A
	// census counter only.
	BareOpens int
	// InitPid is the sandbox's PID-1 child under Options.HelperRoot, else 0.
	InitPid int
	// RootExecArgv and InitExecArgv are, under Options.HelperRoot, the argument
	// vectors of the two execs of the Kiln binary that setup makes and the
	// parser otherwise discards: the root's one exec (strace starting `kiln
	// sandbox-exec ... --config-sha256 H`) and the init child's first exec of
	// the binary (`kiln sandbox-init ... --sha256 H`).  The recorder compares
	// the digests in them with the sealed sandbox configuration's (KILN04 4.8,
	// G7).  Each is nil when that exec is absent or its argv could not be
	// decoded exactly (Event.Argv).
	RootExecArgv []string
	InitExecArgv []string
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

// StepEvent returns the exec event that started the step: the first
// SUCCESSFUL execve/execveat of the process whose exit status is the step's
// (the init child under HelperRoot, else the root).  Failed execs before it are
// ENOENT probes of the shell's PATH search and are not the step.  It returns
// nil when there is none.
func (t *Trace) StepEvent() *Event {
	pid := t.RootPid
	if t.InitPid != 0 {
		pid = t.InitPid
	}
	for i := range t.Events {
		e := &t.Events[i]
		if e.Pid == pid && e.Kind == KindProcess && isExecName(e.Syscall) && e.Result.OK {
			return e
		}
	}
	return nil
}

// cwdCell is the filesystem context (the working directory) that processes
// created with CLONE_FS share.  refs counts the LIVE processes using it.
//
// lastChange and lastDep are what D46 needs to tell whether two calls on this
// cell were ordered or ran at once; see cwdOrdered.  Both are line numbers of a
// COMPLETION (0 = none yet), and both live on the cell, not on the process,
// because the cell is the thing that is shared.
type cwdCell struct {
	path       string
	refs       int
	lastChange int // the line where the last successful chdir/fchdir completed
	lastDep    int // the line where the last cwd-dependent call completed
}

type proc struct {
	*Process
	cwd       *cwdCell
	exited    bool
	startCwd  string // cwd sampled at the "<unfinished ...>" line of a split call
	startLine int    // the line of that "<unfinished ...>" marker
	hasStart  bool
	grp       helperGroup
}

// helperGroup says which part of the sandbox helper a process belongs to.
type helperGroup int

const (
	grpNone helperGroup = iota
	grpRoot             // the helper root and its CLONE_THREAD threads
	grpInit             // the init child and the threads it made during setup
)

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
	// sandbox helper of KILN04 section 4.3.  The helper is the Go Kiln binary
	// (multi-threaded) and the tree has this shape:
	//
	//	helper (root, plus its CLONE_THREAD threads)
	//	  --spawns exactly one non-thread child-->  init child (PID 1 of the new
	//	  PID namespace, plus its threads)
	//	    --setup, re-exec of HelperExe, then the first exec of any OTHER path-->
	//	      the step (sh -c <line>)
	//
	// Root group: no event of the root or its threads reaches Events.  If it
	// execs successfully, the first exec must be of HelperExe (else
	// helper-exe) and a later one is helper-activity.  It may make exactly one
	// non-thread spawn (a second is helper-spawn); after that only ignored-class
	// calls are allowed (else helper-activity).  Before the spawn its calls are
	// setup and are discarded.
	//
	// Init group: the setup window lasts until the init child's first successful
	// exec of a path other than HelperExe; THAT exec is the step.  Inside the
	// window, events are discarded and unknown or refused names are tolerated;
	// execs of HelperExe are allowed; CLONE_THREAD threads belong to the init
	// group; a non-thread spawn is helper-spawn; but a FAILED exec of any other
	// path is kept as an observation (an ENOENT probe).  At the step's exec the
	// tracked cwd must equal the initCwd given to Parse, whatever the path form
	// (else helper-cwd).  A window thread still making calls after the step's
	// exec is helper-activity.  Without a step exec the trace is helper-no-exec.
	// The step's exit status is the init child's (Trace.StepExit).
	HelperRoot bool
	// HelperExe is the resolved path of the Kiln binary; required with
	// HelperRoot.  An exec of "/proc/self/exe" inside the init window is
	// treated as the same binary.
	HelperExe string
}

type model struct {
	lim        Limits
	opts       Options
	root       int
	initCwd    string
	trace      *Trace
	procs      map[int]*proc
	spawns     map[int]spawn
	spawnList  []int
	initPid    int          // helper mode: the root's one non-thread child
	stepExec   bool         // helper mode: the init child has exec'd the step
	rootSpawn  bool         // helper mode: the root group has spawned the init child
	rootExeced bool         // helper mode: the root group has had its one exec
	initExeced bool         // helper mode: the init child's first successful exec of the helper binary was kept
	rootMemo   map[int]bool // inRootGroup answers, so each pid is walked once
	groupMemo  map[int]int  // threadGroupOf answers (D33), likewise
	initMemo   map[int]bool // inInitThreads answers
	// exitCand: pid -> start line of its exit_group that has not returned, for
	// init-group threads after the step's exec.  rawExits: start lines of
	// exit calls (one thread) that have not returned, same threads.  See
	// Trace.InitExitLine.
	exitCand  map[int]int
	rawExits  map[int]bool
	walkSteps int // thread-chain steps taken (a test watches this stays linear)
	// orphans are the pids whose only record is their exit and that the dropped
	// dying spawns can account for (D44); nil when there are too many.
	orphans map[int]bool
	// threadOrphans: orphans is non-empty and no dropped spawn was a process
	// (D44a); copied to Trace.ThreadOrphans.
	threadOrphans bool
}

// Parse reads a strace trace from r.  initialCwd is the working directory of
// the first process (absolute).  Zero fields of limits take DefaultLimits.
// Every failure is an *Error whose message begins "trace-incomplete:".
func Parse(r io.Reader, initialCwd string, limits Limits) (*Trace, error) {
	return ParseOptions(r, initialCwd, Options{Limits: limits})
}

// ParseOptions is Parse with options.
//
// The grammar path reads the lines (D24 PR 6: it is the production default).
// The hand-written scanner is kept only as an ORACLE on probation: when the
// unexported crossCheck hook is set (by this package's tests, or by
// KILN_TRACE_DIFF=1) it reads them too and the two answers are compared, but the
// grammar path's answer is the one returned.  There is deliberately no switch
// that makes the old scanner the answer: it is the lenient reader, so that
// would be a soundness downgrade.  See crosscheck.go.
func ParseOptions(r io.Reader, initialCwd string, opts Options) (*Trace, error) {
	if crossCheck != nil {
		return crossCheck(r, initialCwd, opts)
	}
	return parseWith(r, initialCwd, opts, true)
}

// parseWith is ParseOptions with the reader of lines chosen: the old scanner
// (grammar false) or the grammar path (true).
func parseWith(r io.Reader, initialCwd string, opts Options, grammar bool) (*Trace, error) {
	lim := opts.Limits.withDefaults()
	if opts.HelperRoot && (!strings.HasPrefix(opts.HelperExe, "/") || strings.IndexByte(opts.HelperExe, 0) >= 0) {
		return nil, incomplete(0, reasonHelperConfig)
	}
	if !strings.HasPrefix(initialCwd, "/") || strings.IndexByte(initialCwd, 0) >= 0 || len(initialCwd) > 4096 {
		return nil, incomplete(0, reasonInitialCwd)
	}
	recs, root, e := readRecords(r, lim, opts.HelperRoot, grammar)
	if e != nil {
		return nil, e
	}
	if root == 0 {
		return nil, incomplete(0, reasonEmpty)
	}
	m := newModel(lim, opts, root, initialCwd)
	if e := m.buildSpawns(recs); e != nil {
		return nil, e
	}
	m.findOrphans(recs)
	for i := range recs {
		if e := m.apply(&recs[i]); e != nil {
			return nil, e
		}
	}
	if e := m.finish(); e != nil {
		return nil, e
	}
	m.trace.InitPid = m.initPid
	m.trace.InitExitLine = m.exitMarker()
	return m.trace, nil
}

func newModel(lim Limits, opts Options, root int, initialCwd string) *model {
	return &model{
		lim: lim, opts: opts, root: root, initCwd: initialCwd,
		trace:    &Trace{RootPid: root, Procs: map[int]*Process{}},
		procs:    map[int]*proc{},
		spawns:   map[int]spawn{},
		rootMemo: map[int]bool{},
		initMemo: map[int]bool{},
		exitCand: map[int]int{},
		rawExits: map[int]bool{},
	}
}

// buildSpawns scans every completed fork/vfork/clone for the child pid it
// returned.  This is a separate pass because strace prints the PARENT's return
// line after the child may already have started running, so a child's first
// lines can precede the line that says whose child it is.
func (m *model) buildSpawns(recs []record) *Error {
	type badClone struct {
		parent, line int
		reason       string
	}
	var bad, untranslated []badClone
	for i := range recs {
		c := recs[i].call
		if c == nil {
			continue
		}
		var share bool
		var flags string
		switch c.name {
		case "fork", "vfork":
		case "clone", "clone3":
			var r string
			if c.name == "clone3" {
				flags, share, r = analyzeClone3(c.args)
			} else {
				flags, share, r = analyzeClone(c.args)
			}
			if r != "" && c.name == "clone" && filterAnswered("clone", c.ret) && cloneOnlyForbidden(c.args) {
				// The filter's own EPERM for a clone with CLONE_UNTRACED or
				// CLONE_NEW* (section 4.3 item 5): nothing was spawned.
				r = ""
			}
			if r != "" {
				// Judged below: the helper's own group may clone into new
				// namespaces; nobody else may.
				bad = append(bad, badClone{recs[i].pid, recs[i].line, r})
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
		if c.ret.hasHost && (c.ret.host <= 0 || c.ret.host > 1<<30) {
			return incomplete(recs[i].line, reasonResult)
		}
		child := c.ret.pid()
		if m.opts.HelperRoot && !c.ret.hasHost {
			// Judged below: inside the sandbox's PID namespace a returned pid is
			// namespace-local, and without strace's translation it cannot be
			// matched to the lines that follow.
			untranslated = append(untranslated, badClone{recs[i].pid, recs[i].line, reasonPidnsUntranslated})
		}
		if _, dup := m.spawns[child]; dup || child == m.root || child == recs[i].pid {
			return incomplete(recs[i].line, reasonPidReuse)
		}
		m.spawns[child] = spawn{parent: recs[i].pid, shareFS: share, flags: flags, line: recs[i].line}
		m.spawnList = append(m.spawnList, child)
	}
	for _, b := range bad {
		if !(m.opts.HelperRoot && m.inRootGroup(b.parent)) {
			return incomplete(b.line, b.reason)
		}
	}
	// Only the helper root, in strace's own PID namespace, may return a pid
	// without the translation annotation.
	for _, u := range untranslated {
		if !m.inRootGroup(u.parent) {
			return incomplete(u.line, u.reason)
		}
	}
	if m.opts.HelperRoot {
		for _, child := range m.spawnList {
			sp := m.spawns[child]
			if hasFlag(sp.flags, "CLONE_THREAD") || !m.inRootGroup(sp.parent) {
				continue
			}
			if m.initPid != 0 {
				return incomplete(sp.line, reasonHelperSpawn)
			}
			m.initPid = child
		}
	}
	return nil
}

// inRootGroup reports whether pid is the helper root or a CLONE_THREAD thread
// (transitively) of it.  It uses only the spawn records, so it is valid before
// pass 2.
//
// Every answer is memoised for every pid on the walked chain, so a chain of
// thousands of threads is walked once in total, not once per spawn that asks
// about its end.  (Without the memo, a 4095-deep thread chain that forks a
// million times would cost billions of lookups.)  A chain deeper than
// maxProcessDepth is "not in the group", which fails closed.
func (m *model) inRootGroup(pid int) bool {
	var path []int
	res := false
	for cur, n := pid, 0; ; n++ {
		if v, ok := m.rootMemo[cur]; ok {
			res = v
			break
		}
		m.walkSteps++
		path = append(path, cur)
		if cur == m.root {
			res = true
			break
		}
		sp, ok := m.spawns[cur]
		if !ok || !hasFlag(sp.flags, "CLONE_THREAD") || n >= maxProcessDepth {
			break
		}
		cur = sp.parent
	}
	for _, p := range path {
		m.rootMemo[p] = res
	}
	return res
}

// exitMarker settles Trace.InitExitLine: the earliest surviving exit_group
// candidate, else the last surviving raw exit, else 0.
func (m *model) exitMarker() int {
	best := 0
	for _, line := range m.exitCand {
		if best == 0 || line < best {
			best = line
		}
	}
	if best != 0 {
		return best
	}
	for line := range m.rawExits {
		if line > best {
			best = line
		}
	}
	return best
}

// inInitThreads reports whether pid is the init child or a CLONE_THREAD thread
// (transitively) of it: the init's thread group, whose exit_group ends the
// step.  Like inRootGroup it uses only the spawn records, memoises every pid
// on the walked chain (so the walk is linear in total) and treats a chain
// deeper than maxProcessDepth as "no", which fails closed.
func (m *model) inInitThreads(pid int) bool {
	if m.initPid == 0 {
		return false
	}
	var path []int
	res := false
	for cur, n := pid, 0; ; n++ {
		if v, ok := m.initMemo[cur]; ok {
			res = v
			break
		}
		m.walkSteps++
		path = append(path, cur)
		if cur == m.initPid {
			res = true
			break
		}
		sp, ok := m.spawns[cur]
		if !ok || !hasFlag(sp.flags, "CLONE_THREAD") || n >= maxProcessDepth {
			break
		}
		cur = sp.parent
	}
	for _, p := range path {
		m.initMemo[p] = res
	}
	return res
}

// analyzeClone3 reads clone3's struct argument, which strace 6.x prints as
//
//	clone3({flags=A|B, child_tid=0x.., ..., tls=0x..} => {parent_tid=[N]}, 88) = N
//
// (the "=> {...}" part is the output annotation, absent when the call failed).
// It returns the flags the way analyzeClone does.  Anything it cannot decode
// exactly (not a struct, no leading flags=, a truncated struct, a second
// argument other than the size, trailing text that is not the annotation) is
// clone-flags: a clone3 whose flags are unknown might be anything.
func analyzeClone3(args []string) (flags string, shareFS bool, reason string) {
	if len(args) != 2 || !strings.HasPrefix(args[0], "{") {
		return "", false, reasonCloneFlags
	}
	st := args[0]
	depth, end := 0, -1
	for i := 0; i < len(st) && end < 0; i++ {
		switch st[i] {
		case '{', '[', '(':
			depth++
		case '}', ']', ')':
			depth--
			if depth == 0 {
				end = i
			}
		}
	}
	if end < 0 {
		return "", false, reasonCloneFlags
	}
	if tail := strings.TrimSpace(st[end+1:]); tail != "" && !strings.HasPrefix(tail, "=>") {
		return "", false, reasonCloneFlags
	}
	body := st[1:end]
	if strings.Contains(body, "...") || !strings.HasPrefix(body, "flags=") {
		return "", false, reasonCloneFlags
	}
	text := body[len("flags="):]
	if k := strings.IndexByte(text, ','); k >= 0 {
		text = text[:k]
	}
	if text == "" || strings.ContainsAny(text, " {}[]()") {
		return "", false, reasonCloneFlags
	}
	return analyzeClone([]string{"flags=" + text})
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
				reason = reasonCloneFlags
			case tok == "CLONE_FS":
				shareFS = true
			}
			if !(strings.HasPrefix(tok, "CLONE_") || strings.HasPrefix(tok, "SIG")) || !isErrnoName(tok) {
				reason = reasonCloneFlags
			}
		}
		return v, shareFS, reason
	}
	return "", false, reasonCloneFlags
}

// knownCloneToken is the closed set of names strace prints in a clone's flags=
// argument: the Linux CLONE_* flags and the signal names an exit signal can be.
// A token outside it (CLONE_BOGUS) is one we cannot vouch for, so a clone that
// carries it stays clone-flags even when it failed with the filter's EPERM.
var knownCloneToken = func() map[string]bool {
	m := map[string]bool{}
	for _, n := range []string{
		"CLONE_VM", "CLONE_FS", "CLONE_FILES", "CLONE_SIGHAND", "CLONE_PIDFD", "CLONE_PTRACE",
		"CLONE_VFORK", "CLONE_PARENT", "CLONE_THREAD", "CLONE_NEWNS", "CLONE_SYSVSEM",
		"CLONE_SETTLS", "CLONE_PARENT_SETTID", "CLONE_CHILD_CLEARTID", "CLONE_DETACHED",
		"CLONE_UNTRACED", "CLONE_CHILD_SETTID", "CLONE_NEWCGROUP", "CLONE_NEWUTS",
		"CLONE_NEWIPC", "CLONE_NEWUSER", "CLONE_NEWPID", "CLONE_NEWNET", "CLONE_IO",
		"CLONE_CLEAR_SIGHAND", "CLONE_INTO_CGROUP", "CLONE_NEWTIME",
		"SIGHUP", "SIGINT", "SIGQUIT", "SIGILL", "SIGTRAP", "SIGABRT", "SIGBUS", "SIGFPE",
		"SIGKILL", "SIGUSR1", "SIGSEGV", "SIGUSR2", "SIGPIPE", "SIGALRM", "SIGTERM",
		"SIGSTKFLT", "SIGCHLD", "SIGCONT", "SIGSTOP", "SIGTSTP", "SIGTTIN", "SIGTTOU",
		"SIGURG", "SIGXCPU", "SIGXFSZ", "SIGVTALRM", "SIGPROF", "SIGWINCH", "SIGIO",
		"SIGPWR", "SIGSYS",
	} {
		m[n] = true
	}
	return m
}()

// cloneOnlyForbidden reports whether the only thing wrong with a clone's flags
// is a forbidden one (CLONE_UNTRACED or CLONE_NEW*): every token is in
// knownCloneToken.  Only then is a clone that failed with the filter's EPERM tolerated; a
// flag we cannot name stays clone-flags whatever the result.
func cloneOnlyForbidden(args []string) bool {
	forbidden := false
	for _, a := range args {
		v, ok := strings.CutPrefix(a, "flags=")
		if !ok {
			continue
		}
		for _, tok := range strings.Split(v, "|") {
			tok = strings.TrimSpace(tok)
			if !knownCloneToken[tok] {
				return false
			}
			if tok == "CLONE_UNTRACED" || strings.HasPrefix(tok, "CLONE_NEW") {
				forbidden = true
			}
		}
		return forbidden
	}
	return false
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
		root := &proc{Process: &Process{Pid: cur}, cwd: &cwdCell{path: m.initCwd, refs: 1}}
		if m.opts.HelperRoot {
			root.grp = grpRoot
		}
		m.add(root)
	}
	for k := len(chain) - 1; k >= 0; k-- {
		child := chain[k]
		sp := m.spawns[child]
		parent := m.procs[sp.parent]
		p := &proc{Process: &Process{Pid: child, Parent: sp.parent, ShareFS: sp.shareFS, CloneFlags: sp.flags}}
		p.grp = m.groupFor(child, sp, parent)
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

// groupFor says which part of the sandbox helper a new child belongs to.  A
// CLONE_THREAD thread of a root-group process is a root-group thread.  The
// root's one non-thread child is the init child.  A thread of an init-group
// process belongs to the init group while setup lasts: a thread of the init
// child created after the step's exec is part of the step itself.
func (m *model) groupFor(child int, sp spawn, parent *proc) helperGroup {
	if !m.opts.HelperRoot {
		return grpNone
	}
	thread := hasFlag(sp.flags, "CLONE_THREAD")
	switch parent.grp {
	case grpRoot:
		if thread {
			return grpRoot
		}
		if child == m.initPid {
			return grpInit
		}
	case grpInit:
		if thread && (parent.Pid != m.initPid || !m.stepExec) {
			return grpInit
		}
	}
	return grpNone
}

func (m *model) add(p *proc) {
	m.procs[p.Pid] = p
	m.trace.Procs[p.Pid] = p.Process
	m.trace.Order = append(m.trace.Order, p.Pid)
}

func isSpawn(name string) bool {
	return name == "fork" || name == "vfork" || name == "clone" || name == "clone3"
}

func (m *model) isHelperExe(path string) bool {
	return path == m.opts.HelperExe || path == "/proc/self/exe"
}

// threadGroupOf walks pid's spawns while they carry CLONE_THREAD and returns
// the group leader (the first pid that is not a thread, or has no spawn
// record).  Every answer is memoised for every pid on the walked chain and each
// step is counted in walkSteps, like inRootGroup and inInitThreads, so a
// hostile deep chain times many ghosts costs one walk, not 4096 lookups each.
//
// A chain deeper than maxProcessDepth stops at the pid reached and answers
// -cur-1 (negative, so never a leader).  Two pids whose walks stop at the same
// ancestor therefore compare EQUAL, and so do pids that share a memoised chain.
// That is still sound for D33: a negative answer is only given along links that
// are all CLONE_THREAD, so equal answers really are one thread group.  The
// bound can only make two threads of one group compare UNEQUAL (a refusal),
// never two groups equal.
func (m *model) threadGroupOf(pid int) int {
	if m.groupMemo == nil {
		m.groupMemo = map[int]int{}
	}
	var path []int
	res := 0
	for cur, n := pid, 0; ; n++ {
		if v, ok := m.groupMemo[cur]; ok {
			res = v
			break
		}
		m.walkSteps++
		path = append(path, cur)
		sp, ok := m.spawns[cur]
		if !ok || !hasFlag(sp.flags, "CLONE_THREAD") {
			res = cur
			break
		}
		if n >= maxProcessDepth {
			res = -cur - 1
			break
		}
		cur = sp.parent
	}
	for _, p := range path {
		m.groupMemo[p] = res
	}
	return res
}

// findOrphans (D44) finds the pids that appear only as an exit: the children of
// a spawn whose return line died with its parent, so no spawn record names
// them.  They are accepted only when the dropped dying spawns (record
// .spawnDropped) are at least as many as the orphans; with more orphans than
// spawns some orphan is unexplained and ALL of them keep refusing as orphan-pid.
//
// "Only record" means no call, start marker or exit_group marker: the lines
// that leave no record are ignored-class calls, signals and filter-answered
// refusals, none of which touches a path.  A pid with two records (even two
// exits) is not an orphan.  Deferred (model.go get): matching each orphan to a
// particular dropped spawn.  The orphan's exit stays in Trace.Exits with no Procs
// entry, which DaemonLeft counts as a possible survivor (fail closed), unless
// every dropped spawn was a CLONE_THREAD clone (D44a): then the orphans are
// listed in Trace.ThreadOrphans and DaemonLeft skips them.
func (m *model) findOrphans(recs []record) {
	dropped, processes := 0, 0
	count := map[int]int{}
	for i := range recs {
		switch recs[i].spawnDropped {
		case dropThread:
			dropped++
		case dropProcess:
			dropped++
			processes++
		}
		count[recs[i].pid]++
	}
	var orphans []int
	for i := range recs {
		pid := recs[i].pid
		if recs[i].exit == nil || count[pid] != 1 || pid == m.root {
			continue
		}
		if _, spawned := m.spawns[pid]; spawned {
			continue
		}
		orphans = append(orphans, pid)
	}
	if len(orphans) == 0 || len(orphans) > dropped {
		return
	}
	m.orphans = map[int]bool{}
	for _, pid := range orphans {
		m.orphans[pid] = true
	}
	// D44a: with no fork, vfork or non-thread clone among the dropped spawns,
	// every orphan is a CLONE_THREAD thread.  (No matching of orphan to spawn is
	// needed: the claim is about all of them.)  A thread cannot outlive its
	// thread group, and a surviving group is caught by its own members' exits,
	// which have Procs entries.  One process-kind spawn anywhere keeps the
	// orphans as possible survivors.
	if processes == 0 {
		m.trace.ThreadOrphans = map[int]bool{}
		for _, pid := range orphans {
			m.trace.ThreadOrphans[pid] = true
		}
	}
}

func (m *model) apply(rec *record) *Error {
	if rec.exit != nil && m.orphans[rec.pid] {
		m.trace.Exits = append(m.trace.Exits, *rec.exit)
		m.trace.ExitLines = append(m.trace.ExitLines, rec.line)
		return nil
	}
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
		m.trace.ExitLines = append(m.trace.ExitLines, rec.line)
		p.Exit = rec.exit // its own allocation: never aliases the slice above
		return nil
	}
	if p.exited {
		return incomplete(rec.line, reasonEventAfterExit)
	}
	// D33: a phantom execve line was accepted in pass 1 against this exec; it
	// is sound only if its thread was torn down by this very exec, so it must
	// be in the same thread group.  Otherwise refuse at the ghost's line.
	if len(rec.ghosts) > 0 {
		owner := m.threadGroupOf(rec.pid)
		for _, g := range rec.ghosts {
			if m.threadGroupOf(g.pid) != owner {
				return incomplete(g.line, reasonUnresumed)
			}
		}
	}
	if rec.exitCall && rec.retract {
		// A failed exit_group or exit whose start line was a candidate:
		// withdraw exactly that candidate.
		if rec.rawExit {
			delete(m.rawExits, rec.line)
		} else if m.exitCand[p.Pid] == rec.line {
			delete(m.exitCand, p.Pid)
		}
		return nil
	}
	if rec.exitCall {
		if m.stepExec && m.inInitThreads(p.Pid) {
			if rec.rawExit {
				m.rawExits[rec.line] = true
			} else if _, ok := m.exitCand[p.Pid]; !ok {
				m.exitCand[p.Pid] = rec.line
			}
		}
		return nil
	}
	if p.grp == grpRoot {
		return m.applyHelperRoot(p, rec)
	}
	inWindow := p.grp == grpInit && !m.stepExec
	if p.grp == grpInit && m.stepExec && p.Pid != m.initPid {
		// A thread made during setup still running after the step's exec.
		return incomplete(rec.line, reasonHelperActivity)
	}
	if rec.start {
		p.startCwd, p.startLine, p.hasStart = p.cwd.path, rec.line, true
		return nil
	}
	c := rec.call
	if m.opts.HelperRoot {
		// A successful unshare(CLONE_FS) in the init's window gives the calling
		// thread a private working directory: stage 3b's helper does it so its
		// setup chdirs are not a cwd-race with the Go runtime's threads.
		if inWindow && c.name == "unshare" && c.ret.ok() && len(c.args) > 0 && unshareHasFS(c.args[0]) {
			// The copy reads the cwd, so it is a cwd-dependent call too (D46).
			// Pass 1 emits no start MARKER for unshare (a refused-class name),
			// but a split call's record carries its start line (startLine), so
			// its interval is [startLine, line] like any other.  The helper's
			// own unshare is split whenever another Go thread's stop lands
			// between its two halves, which under load is common: refusing it
			// made the recorder's own probes fail at random.  A split record
			// with startLine == 0 is a defensive refusal: both readers set
			// it on every split record, so it only happens if a future
			// producer forgets, and then it is refused when another sharer
			// exists, because its interval cannot be bounded.
			start := rec.line
			if rec.split {
				if rec.startLine == 0 && p.cwd.refs > 1 {
					return withCall(incomplete(rec.line, reasonCwdRace), c.name)
				}
				if rec.startLine > 0 {
					start = rec.startLine
				}
			}
			if !p.cwd.cwdOrdered(start, rec.line, true, false) {
				return withCall(incomplete(rec.line, reasonCwdRace), c.name)
			}
			p.cwd.refs--
			p.cwd = &cwdCell{path: p.cwd.path, refs: 1}
			return nil
		}
		// Classification was deferred in pass 1.  A refused name the sandbox
		// filter answered (clone3 = -1 ENOSYS, mount = -1 EPERM) is the
		// filter working, in the init window and in the step alike: no
		// event, no error.  (finishCall already drops these; the check is
		// kept here because this is the path a recording takes.)
		if tolerated, e := classifyCall(c.name, c.ret, rec.line); tolerated {
			return nil
		} else if e != nil {
			if inWindow {
				if c.name == "clone3" && c.ret.ok() {
					// A clone3 of the init's window is tolerated like clone
					// (the Go runtime makes its threads with it), and tracked
					// like clone: threads only, never a second process.
					child := c.ret.pid()
					if !hasFlag(m.spawns[child].flags, "CLONE_THREAD") {
						return incomplete(rec.line, reasonHelperSpawn)
					}
					if _, e := m.get(child, rec.line-1); e != nil {
						return e
					}
				}
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
		if inWindow && !hasFlag(evs[0].Flags, "CLONE_THREAD") {
			return incomplete(rec.line, reasonHelperSpawn)
		}
		// Create the child NOW, copying (or sharing) the parent's cwd as it
		// is at this moment.
		if _, e := m.get(c.ret.pid(), rec.line-1); e != nil {
			return e
		}
	}
	if inWindow {
		if evs, e = m.windowFilter(p, rec, evs); e != nil {
			return e
		}
	}
	if len(m.trace.Events)+len(evs) > m.lim.MaxEvents {
		return incomplete(rec.line, reasonSize)
	}
	m.trace.Events = append(m.trace.Events, evs...)
	return nil
}

// windowFilter decides what survives of a call the init group made during
// setup.  Execs are the interesting ones: a re-exec of the Kiln binary is part
// of setup; the first successful exec of anything else is the step; a failed
// exec of anything else is a real observation (the step's shell probing PATH
// looks exactly like that) and is kept.
func (m *model) windowFilter(p *proc, rec *record, evs []Event) ([]Event, *Error) {
	c := rec.call
	if !isExecName(c.name) {
		return nil, nil
	}
	ev := evs[0]
	if m.isHelperExe(ev.Path) {
		// The init child's own re-exec of the binary carries the digest of the
		// configuration it will read; keep its argv (the event itself is setup,
		// and is still dropped).
		if c.ret.ok() && p.Pid == m.initPid && !m.initExeced {
			m.initExeced = true
			m.trace.InitExecArgv = ev.Argv
		}
		return nil, nil
	}
	if !c.ret.ok() {
		return evs, nil
	}
	if p.Pid != m.initPid {
		return nil, incomplete(rec.line, reasonHelperActivity)
	}
	// The step starts where the executor says.  Whatever setup did to the
	// working directory, it must have left it there, for every path form.
	if ev.Cwd != m.initCwd {
		return nil, incomplete(rec.line, reasonHelperCwd)
	}
	m.stepExec = true
	return evs, nil
}

// withCall names the closed-table call a refusal was raised for (Error.Call).
func withCall(e *Error, name string) *Error {
	if SyscallClass(name) != "" {
		e.Call = name
	}
	return e
}

// applyHelperRoot handles a record of the sandbox helper root or one of its
// threads.  Nothing it does becomes an event.
func (m *model) applyHelperRoot(p *proc, rec *record) *Error {
	if rec.start {
		return nil
	}
	c := rec.call
	if m.rootSpawn {
		// After the init child exists the root may only wait (ignored calls,
		// which pass 1 has already dropped).
		return incomplete(rec.line, reasonHelperActivity)
	}
	switch {
	case isSpawn(c.name) && c.ret.ok():
		child := c.ret.pid()
		if !hasFlag(m.spawns[child].flags, "CLONE_THREAD") {
			m.rootSpawn = true
		}
		if _, e := m.get(child, rec.line-1); e != nil {
			return e
		}
	case isExecName(c.name) && c.ret.ok():
		evs, e := m.call(p, rec)
		if e != nil {
			return e
		}
		if m.rootExeced {
			return incomplete(rec.line, reasonHelperActivity)
		}
		m.rootExeced = true
		if evs[0].Path != m.opts.HelperExe {
			return incomplete(rec.line, reasonHelperExe)
		}
		m.trace.RootExecArgv = evs[0].Argv
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
		if !m.stepExec {
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

// cwdDependsOn reports whether a handled call's result depends on the working
// directory (KILN04 D46): some path it resolves is relative and no directory
// descriptor anchors it.  A path anchored by a real dirfd carries that
// descriptor's absolute -y name (DirfdPath, Dirfd2Path) and an absolute path
// ignores the cwd, so neither depends on it.  AT_FDCWD leaves the dirfd path
// empty, which is exactly the case that does depend on it.  Target is a
// symlink's CONTENT, which no call resolves, so it never counts.
//
// A dirfd slot that is AT_FDCWD (empty dirfd path) with an EMPTY path is
// dependent too: statx(AT_FDCWD, "", AT_EMPTY_PATH), newfstatat, fchownat,
// linkat and execveat then act on the cwd directory itself.  Only an entry
// that HAS the dirfd slot counts, so a bare empty Path2 does not.
func cwdDependsOn(ev *Event, ent entry) bool {
	rel := func(path, dirfdPath string) bool {
		return path != "" && path[0] != '/' && dirfdPath == ""
	}
	empty := func(path, dirfdPath string, slot int) bool {
		return slot > 0 && path == "" && dirfdPath == ""
	}
	return rel(ev.Path, ev.DirfdPath) || rel(ev.Path2, ev.Dirfd2Path) ||
		empty(ev.Path, ev.DirfdPath, ent.dirfd) || empty(ev.Path2, ev.Dirfd2Path, ent.dirfd2)
}

// cwdRelative reports whether a socket address names a filesystem object by a
// relative path: an AF_UNIX pathname address that is non-empty, not abstract
// and does not start with '/'.  bind, connect and sendto resolve it against the
// cwd (D46).  A nil address (sendto without one) depends on nothing.
func (a *Addr) cwdRelative() bool {
	return a != nil && a.Family == "AF_UNIX" && !a.Abstract && a.Address != "" && a.Address[0] != '/'
}

// cwdOrdered is the whole of D46.  It is called when a call on cell c
// COMPLETES at line `line`, having started at line `start` (the same line for
// an uninterrupted call), and says whether the model may trust the order in
// which the lines show it and the other calls that share the cell.
//
//	dep: the call's own result depends on the cwd (cwdDependsOn, or a fork
//	     that copies the cwd).
//	mod: the call is a SUCCESSFUL chdir or fchdir.
//
// Two calls A and B on one cell "overlap" when their line intervals
// [startA, lineA] and [startB, lineB] intersect.  The lines are the order in
// which strace handled the stops, and a call's effect happens between its own
// two stops, so disjoint intervals are a true order and intersecting ones are
// not.  Take the call that completes LATER; the earlier one has already been
// seen, and the two intervals intersect exactly when the later one STARTED
// before the earlier one COMPLETED.  So at each completion there are two
// questions, both about completions already seen:
//
//	a call that is dep or mod: did a chdir complete after I started?
//	                           (lastChange > start)
//	a call that is mod:        did a cwd-dependent call complete after I
//	                           started? (lastDep > start)
//
// Either yes is a race.  A call that is neither (an absolute open, an
// ignored-class call) is never ambiguous: it asks nothing and records nothing.
// When both answers are no, every other call is wholly before or wholly after
// this one, so applying the change at this completion line is the true order.
func (c *cwdCell) cwdOrdered(start, line int, dep, mod bool) bool {
	if (dep || mod) && c.lastChange > start {
		return false
	}
	if mod && c.lastDep > start {
		return false
	}
	if dep {
		c.lastDep = line
	}
	if mod {
		c.lastChange = line
	}
	return true
}

// call turns one completed record into events and applies its effect on the
// process model.
func (m *model) call(p *proc, rec *record) ([]Event, *Error) {
	c := rec.call
	ent := lookup(c.name)
	fail := func(r string) ([]Event, *Error) {
		e := incomplete(rec.line, r)
		if SyscallClass(c.name) != "" {
			e.Call = c.name // a closed-table name, census only (Error.Call)
		}
		return nil, e
	}
	if len(c.args) < ent.min || len(c.args) > ent.max {
		return fail(reasonArity)
	}
	if shortMknod(c) {
		return fail(reasonArity) // a device node always prints its dev (D44)
	}
	if c.ret.unknown {
		if !restartedSpawnCheck(c, ent) {
			return fail(reasonResult)
		}
		// D50 rule 2b.  The call created nothing and will be restarted by the
		// kernel as a fresh call, so there is no event and no spawn record.
		// A split call's saved start (cwd, line) belongs to this call alone and
		// is spent.
		p.hasStart = false
		m.trace.RestartedSpawns++
		return nil, nil
	}
	// A split call ran in the directory that was current when it STARTED.
	cwdAtCall, startLine := p.cwd.path, rec.line
	if rec.split && p.hasStart {
		cwdAtCall, startLine = p.startCwd, p.startLine
	}
	p.hasStart = false
	ev := Event{Line: rec.line, StartLine: startLine, Pid: rec.pid, Syscall: c.name, Kind: ent.kind, Cwd: cwdAtCall, Result: toResult(c.ret)}
	arg := func(i int) string { return c.args[i-1] }

	if ent.kind == KindNetwork {
		// The socket's own -y name (socket:[inode]) lets a later stage tell one
		// socket from another and read its type from the socket() call.
		if ent.role == roleSocket {
			if c.ret.hasAnn {
				ev.SockPath, _ = fdPathText(c.ret.ann, false)
			}
		} else if f, r := parseFD(arg(1)); r == "" && f.hasAnn {
			ev.SockPath, _ = fdPathText(f.ann, false)
		}
	}

	var r string
	switch ent.role {
	case roleListing, roleFchdir, roleFdPath:
		f, r := parseFD(arg(1))
		if r != "" {
			return fail(r)
		}
		if f.cwd || !f.hasAnn {
			return fail(reasonFDPath)
		}
		// D41: fchmod, fchown, fsetxattr and fremovexattr on a pipe, socket or
		// anonymous inode change an object that lives in no directory, so no file
		// readable by path changes.  No event, whatever the result.  A bare number
		// or any other non-absolute annotation still falls through to fd-path.
		if ent.role == roleFdPath && AnonymousFD(string(f.ann)) {
			return nil, nil
		}
		ev.Path, r = fdPathText(f.ann, true)
		if r != "" {
			return fail(r)
		}
		if ent.role == roleFchdir && c.ret.ok() {
			// The directory comes from the descriptor's absolute -y name, so
			// the call is not cwd-dependent, but it is a change (D46).
			if !p.cwd.cwdOrdered(startLine, rec.line, false, true) {
				return fail(reasonCwdRace)
			}
			p.cwd.path = ev.Path
		}
	case roleFork:
		// Nothing to record beyond the result (the child pid).  A fork COPIES
		// the cwd, so it is a cwd-dependent call (D46).
		if c.ret.ok() && !p.cwd.cwdOrdered(startLine, rec.line, true, false) {
			return fail(reasonCwdRace)
		}
	case roleClone:
		var share bool
		ev.Flags, share, r = analyzeClone(c.args)
		if r != "" && filterAnswered("clone", c.ret) && cloneOnlyForbidden(c.args) {
			r = "" // the filter's EPERM: nothing was spawned (see cloneOnlyForbidden)
		}
		if r != "" {
			return fail(r)
		}
		// A spawn without CLONE_FS copies the cwd (D46); with it, it shares.
		if !share && c.ret.ok() && !p.cwd.cwdOrdered(startLine, rec.line, true, false) {
			return fail(reasonCwdRace)
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
		if !p.cwd.cwdOrdered(startLine, rec.line, ev.Addr.cwdRelative(), false) {
			return fail(reasonCwdRace)
		}
	case roleMsg:
		socks, r := findSockaddrs(arg(ent.addr))
		if r != "" {
			return fail(r)
		}
		seen := map[string]bool{}
		var evs []Event
		relative := false
		for _, s := range socks {
			if seen[s] {
				continue
			}
			seen[s] = true
			e2 := ev
			if e2.Addr, r = parseSockaddr(s, false); r != "" {
				return fail(r)
			}
			relative = relative || e2.Addr.cwdRelative()
			evs = append(evs, e2)
		}
		// One ordering check for the whole call (D46).
		if !p.cwd.cwdOrdered(startLine, rec.line, relative, false) {
			return fail(reasonCwdRace)
		}
		if len(evs) == 0 {
			evs = append(evs, ev)
		}
		return evs, nil
	default: // rolePath, roleCreat, roleChdir, roleExec, roleSymlink
		if drop, r := statxNullEfault(c, ent); r != "" {
			return fail(r)
		} else if drop {
			return nil, nil
		}
		if r := m.fillPath(&ev, ent, c.args); r != "" {
			return fail(r)
		}
		if ent.role == roleCreat {
			ev.Flags = "O_CREAT|O_WRONLY|O_TRUNC"
		}
		// D46: a sharer's call that ran AT THE SAME TIME as this one makes the
		// order ambiguous, whoever shares the cell (threads, CLONE_FS children).
		// A fork child has its own copy and is never on this cell.  A failed
		// chdir changes nothing, but its own relative path still reads the cwd.
		// Every exec is dependent: an absolute script whose #! names a relative
		// interpreter resolves it against the cwd inside the same call.
		mod := ent.role == roleChdir && c.ret.ok()
		if !p.cwd.cwdOrdered(startLine, rec.line, cwdDependsOn(&ev, ent) || ent.role == roleExec, mod) {
			return fail(reasonCwdRace)
		}
		if mod {
			n, ok := joinCwd(cwdAtCall, ev.Path)
			if !ok {
				return fail(reasonCwdTooLong)
			}
			p.cwd.path = n
		}
		if ent.role == roleExec {
			ai := 2
			if c.name == "execveat" {
				ai = 3
			}
			ev.Argv = parseArgv(c.args[ai-1])
		}
		if ent.role == roleExec && c.ret.ok() {
			p.LastExec = ev.Path
			if ev.Path == "" {
				p.LastExec = ev.DirfdPath
			}
		}
		if ent.kind == KindOpen && c.ret.ok() {
			if !c.ret.hasAnn {
				if !bareReadOnlyOpenCheck(c, ent) {
					return fail(reasonFDPath)
				}
				m.trace.BareOpens++ // ReturnFDPath stays empty (D50 rule 2c)
			} else {
				ev.ReturnFDPath, r = fdPathText(c.ret.ann, false)
				if r != "" {
					return fail(r)
				}
			}
		}
	}
	return []Event{ev}, nil
}

// The two D50 predicates are called through variables only so that a test can
// swap in a MUTANT (an errno check dropped, a write flag added to the closed
// set, a hex path accepted) and show the refusal corpus kills it
// (d50_mutant_test.go).  Nothing else assigns them.
var (
	restartedSpawnCheck   = restartedSpawn
	bareReadOnlyOpenCheck = bareReadOnlyOpen
)

// restartedSpawn is KILN04 D50 rule 2b: a fork, vfork or clone whose result
// is exactly `?` with the errno name ERESTARTNOINTR created no child.
//
// Why that is sound.  ERESTARTNOINTR is a kernel-internal code, never returned
// to user space: it means "restart this system call unconditionally, whatever
// the signal disposition".  In the kernel's fork path (copy_process) it is
// returned when a signal is pending and the fork is abandoned before the child
// is made visible; the half-built task is freed on that same path.  So no child exists, and none can have run.  The tracee then runs its
// signal handler (if any) and the kernel re-executes the call itself: that
// second execution is a fresh line, judged like any other, and is NOT paired
// with this one (pairing would hide a spawn that really happened or invent one
// that did not).  strace prints the unfinished call's result as `?` because
// the tracee never reached user space with a value.
//
// What stays refused: any other ERESTART* name (ERESTARTSYS and the like are
// restarted only under some dispositions, and the spawn's fate is then not
// shown by this line), a bare `?` (the task may have died inside the call, so
// a child may exist), `? <unavailable>`, ERESTARTNOINTR on a call that is not
// a spawn (it says nothing about paths or children there), and clone3 (its
// flags are not visible to the filter and it is refused as a whole).  For a
// clone the flags must still pass analyzeClone: a CLONE_UNTRACED or CLONE_NEW*
// call that the kernel then restarted might be restarted into one that works,
// and an unknown flag token is a flag we cannot name.
func restartedSpawn(c *rawCall, ent entry) bool {
	if c.ret.errno != "ERESTARTNOINTR" || !c.ret.spaced || c.ret.unavailable || c.ret.hasVal || c.ret.hasAnn {
		return false
	}
	switch ent.role {
	case roleFork:
		return true
	case roleClone:
		if _, _, r := analyzeClone(c.args); r != "" {
			return false
		}
		// analyzeClone lets any CLONE_* spelling through (a real clone that
		// worked is judged by its child); a call that created nothing has no
		// child to judge, so its flags must all be names we know.
		return cloneOnlyForbiddenFree(c.args)
	}
	return false
}

// cloneOnlyForbiddenFree says every token of the clone's flags= is in
// knownCloneToken and none is forbidden (analyzeClone has already refused the
// forbidden ones; this is the closed-vocabulary half).
func cloneOnlyForbiddenFree(args []string) bool {
	for _, a := range args {
		v, ok := strings.CutPrefix(a, "flags=")
		if !ok {
			continue
		}
		for _, tok := range strings.Split(v, "|") {
			if !knownCloneToken[strings.TrimSpace(tok)] {
				return false
			}
		}
		return true
	}
	return false
}

// bareReadOnlyOpenFlags is the closed set of flag tokens a read-only open may
// carry for rule 2c.  None of them can create, truncate, write or change which
// object the name leads to beyond what the walk already models.
var bareReadOnlyOpenFlags = map[string]bool{
	"O_RDONLY": true, "O_CLOEXEC": true, "O_NONBLOCK": true, "O_NDELAY": true,
	"O_DIRECTORY": true, "O_NOFOLLOW": true, "O_LARGEFILE": true, "O_NOCTTY": true,
	"O_NOATIME": true, "O_PATH": true,
}

// bareReadOnlyOpen is KILN04 D50 rule 2c: a SUCCESSFUL open or openat that
// carries no descriptor annotation, with a flag set that provably only reads.
//
// Why that is sound.  The -y annotation exists to say where the kernel really
// went; it is what lets the classifier compare the disk walk with the kernel
// (fd-path-differs).  Without it the classifier walks the path the action
// named and records the read against that walked name under the strict
// stability rule, exactly as it does for a descriptor that names a pipe or an
// anonymous inode (openCall, a non-absolute ReturnFDPath).  What could go
// wrong is a hidden WRITE (the annotation is absent, so nothing else proves the
// open only read) or an object the walk cannot name; the flag closure rules
// out the first (no O_WRONLY, O_RDWR, O_CREAT, O_TRUNC, O_TMPFILE, O_EXCL,
// O_APPEND or any other token, and no number that could hide a bit), and a
// /proc magic link or a descriptor path is refused later by the classifier,
// which needs an annotation to believe those.  The path must be a quoted
// string (a hex address is an unreadable buffer), the result a plain
// non-negative decimal descriptor with no pid-namespace note, and O_RDONLY or
// O_PATH must be present (the access mode is explicit).  An annotation that is
// present but does not parse never reaches here (it is refused by the line
// reader).  openat2 and creat stay refused (a struct of resolve flags, an
// implicit write), and every LATER call on the descriptor that lacks its own
// annotation is refused by its own rule (fd-path).
func bareReadOnlyOpen(c *rawCall, ent entry) bool {
	if c.name != "open" && c.name != "openat" {
		return false
	}
	// Defence in depth: parseResult accepts `= 3 FOO (msg)`, so the predicate does
	// not lean on the caller's ok() guard.
	if c.ret.errno != "" || c.ret.unknown || c.ret.hasAnn || c.ret.hex || c.ret.hasHost || c.ret.unavailable || !c.ret.hasVal || c.ret.value < 0 || c.ret.value > 1<<30 {
		return false
	}
	if p := c.args[ent.pth-1]; len(p) < 2 || p[0] != '"' {
		return false
	}
	access := false
	for _, tok := range strings.Split(c.args[ent.flags-1], "|") {
		if !bareReadOnlyOpenFlags[tok] { // no TrimSpace, no number: an exact token
			return false
		}
		access = access || tok == "O_RDONLY" || tok == "O_PATH"
	}
	return access
}

// statxNullEfault (KILN04 D45) checks whether a completed statx call is a
// NULL-pathname probe that failed with EFAULT without AT_EMPTY_PATH (such as
// Rust std's statx availability probe).
//
// In Linux (fs/stat.c: SYSCALL_DEFINE5(statx, ...) -> getname_uflags ->
// getname_flags), the kernel copies the pathname pointer from user memory
// before do_statx inspects dfd or looks up any file or descriptor.  A NULL
// pointer fails with -EFAULT having observed nothing on the filesystem and
// without writing to the statx buffer.
//
// Fail-closed guards:
//   - dirfd must still parse cleanly through dirfdPath;
//   - flags must decode cleanly through hasEmptyPath and MUST NOT carry
//     AT_EMPTY_PATH (including numeric 0x1000 / 4096), keeping path-arg /
//     flags refusals intact;
//   - only completed calls with !ret.OK && ret.Errno == "EFAULT" qualify;
//   - non-NULL addresses (e.g. 0x1) and dying calls stay refused.
func statxNullEfault(c *rawCall, ent entry) (drop bool, reason string) {
	if c.name != "statx" || c.ret.ok() || c.ret.errno != "EFAULT" || c.args[ent.pth-1] != "NULL" {
		return false, ""
	}
	if _, _, r := dirfdPath(c.args[ent.dirfd-1]); r != "" {
		return false, r
	}
	empty, r := hasEmptyPath(c.args[ent.flags-1])
	if r != "" {
		return false, r
	}
	if empty {
		return false, reasonPathArg
	}
	return true, ""
}

// fillPath extracts dirfd, path, dirfd2, path2, flags and symlink target
// according to the table entry.
// shortMknod reports a mknod/mknodat in its short form (no dev argument) whose
// mode names a character or block device.  strace prints dev for exactly those
// two file types, so such a line is not a call we understand.  The short form
// of every other type (S_IFREG, S_IFIFO, S_IFSOCK, or a bare permission mask,
// which is a regular file) is what strace prints.  A mode that is not
// symbolic is judged by its tokens: an unrecognised token also refuses.
func shortMknod(c *rawCall) bool {
	var modeIdx int
	switch c.name {
	case "mknod":
		modeIdx = 1
	case "mknodat":
		modeIdx = 2
	default:
		return false
	}
	if len(c.args) != modeIdx+1 {
		return false // the long form, judged by the ordinary arity check
	}
	for _, tok := range strings.Split(c.args[modeIdx], "|") {
		tok = strings.TrimSpace(tok)
		switch tok {
		case "S_IFREG", "S_IFIFO", "S_IFSOCK":
			continue
		}
		if isOctalToken(tok) && permBitsOnly(tok) {
			continue
		}
		return true // S_IFCHR, S_IFBLK, a mode with file-type bits, or anything unrecognised
	}
	return false
}

// isOctalToken reports a permission mask as strace prints it: a leading 0 and
// octal digits ("0600", "0").
func isOctalToken(s string) bool {
	if s == "" || s[0] != '0' {
		return false
	}
	for i := 1; i < len(s); i++ {
		if s[i] < '0' || s[i] > '7' {
			return false
		}
	}
	return true
}

// permBitsOnly says an octal token carries permission bits alone (at most
// 07777).  strace prints a mode's file type symbolically (S_IFCHR|0600), so an
// octal literal with type bits (020600, a character device) is not strace's
// own spelling of a non-device and must not pass for one.
func permBitsOnly(s string) bool {
	v, err := strconv.ParseUint(s, 8, 32)
	return err == nil && v <= 0o7777
}

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
	if ev.Path == "" && ev.DirfdPath != "" && ent.flags > 0 {
		empty, r := hasEmptyPath(args[ent.flags-1])
		if r != "" {
			return r
		}
		if empty {
			// fexecve, fstat-by-statx and friends: the descriptor itself is the
			// object, so its -y path is the path.
			ev.Path, ev.DirfdPath = ev.DirfdPath, ""
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

// hasFlag reports whether the "A|B|C" flag text names flag.
func hasFlag(text, flag string) bool {
	for _, f := range strings.Split(text, "|") {
		if strings.TrimSpace(f) == flag {
			return true
		}
	}
	return false
}

// hasEmptyPath decodes a flags argument for AT_EMPTY_PATH (0x1000).  strace
// prints known bits by name, but a numeric or hex leftover can hide the bit,
// so numbers are decoded; anything else is not understood and is an error.
func hasEmptyPath(text string) (bool, string) {
	found := false
	for _, tok := range strings.Split(text, "|") {
		tok = strings.TrimSpace(tok)
		switch {
		case tok == "AT_EMPTY_PATH":
			found = true
		case strings.HasPrefix(tok, "0x"):
			v, err := strconv.ParseUint(tok[2:], 16, 64)
			if err != nil {
				return false, reasonFlags
			}
			found = found || v&0x1000 != 0
		case tok != "" && tok[0] >= '0' && tok[0] <= '9':
			v, err := strconv.ParseUint(tok, 10, 64)
			if err != nil {
				return false, reasonFlags
			}
			found = found || v&0x1000 != 0
		case isErrnoName(tok):
			// a symbolic flag: not AT_EMPTY_PATH
		default:
			return false, reasonFlags
		}
	}
	return found, ""
}

// unshareHasFS reports whether an unshare flags argument includes CLONE_FS
// (0x200), spelled by name or as a number.  Anything it cannot decode is not
// CLONE_FS, so a later chdir stays a cwd-race: the safe direction.
func unshareHasFS(text string) bool {
	for _, tok := range strings.Split(text, "|") {
		tok = strings.TrimSpace(tok)
		if tok == "CLONE_FS" {
			return true
		}
		if n, err := strconv.ParseUint(tok, 0, 64); err == nil && n&0x200 != 0 {
			return true
		}
	}
	return false
}
