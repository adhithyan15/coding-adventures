package traced

import (
	"strings"

	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/key"
	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/trace"
)

// Reasons this package adds.  Reasons of section 6 that carry a path are built
// where the path is known; these are the fixed ones.  Every trace-incomplete
// detail is free of path text, so it can be printed in a CI log.
const (
	rUnstable       = "trace-incomplete:unstable-path"
	rLoop           = "trace-incomplete:symlink-loop"
	rUnresolvable   = "trace-incomplete:unresolvable-path"
	rProc           = "trace-incomplete:proc-path"
	rEmptyPath      = "trace-incomplete:empty-path"
	rBadPath        = "trace-incomplete:bad-path"
	rSize           = "trace-incomplete:size"
	rVCSWrite       = "trace-incomplete:vcs-write"
	rResolveFlags   = "trace-incomplete:resolve-flags"
	rExecUnreadable = "trace-incomplete:exec-unreadable"
	rExecFormat     = "trace-incomplete:exec-format"
	rExecDepth      = "trace-incomplete:exec-depth"
	rExecELF        = "trace-incomplete:exec-elf"
	rRecipe         = "trace-incomplete:recipe-mismatch"
	rVCSRead        = "vcs-read"
	rFailed         = "failed"
	rDaemonLeft     = "daemon-left"
)

// pos orders events across the steps of one action: the step, then the line.
type pos struct{ step, line int }

func (a pos) before(b pos) bool {
	return a.step < b.step || (a.step == b.step && a.line < b.line)
}

// mod is one namespace change the action made to a path.  clean marks the
// three creations whose result the trace fully determines: mkdir, symlink and
// open with O_CREAT|O_EXCL.
type mod struct {
	at    pos
	clean bool
	made  bool // the change may have created the name (any creation)
}

type absKey struct {
	path string
	k    int
}

// classifier is the state of one action across all of its steps.
//
// The sets that make "written, then read" work (created, absent) are kept in
// TRACE ORDER: a path is the action's own only from the moment the action made
// it.  The rest is the record being built.
type classifier struct {
	cfg *prepared

	entries, writes, tracked, untracked, absentObs, reasons map[string]bool

	// created holds the absolute paths this action created from empty so far
	// (section 5.2).  A path is the action's own if it, or any directory above
	// it, is in the set.
	created map[string]bool
	// absent holds paths the action probed and found missing by an
	// unambiguous ENOENT on the final component.
	absent map[string]bool
	// links and dirs are the symlinks and directories this action made.  They
	// answer lookups under $HOME and TMPDIR, where the post-run disk is never
	// asked.
	links map[string]string
	dirs  map[string]bool
	// cache holds post-run lookups, which do not change during classification.
	cache map[string]node

	// mods logs, per absolute path, each namespace change the action made to
	// it.  hard holds paths that were ever a link()/linkat() source or
	// destination: hard links make aliases the trace cannot follow.
	mods map[string][]mod
	hard map[string]bool
	// wlog logs every write to a path, for the exec-then-rewrite rule.
	wlog map[string][]pos
	// seen maps each component the action's paths went through to the START
	// of the earliest lookup that went through it.
	seen map[string]pos
	// seenAbsent holds the places a walk stopped because the disk showed
	// nothing (or a file) there; see noteMissing.
	seenAbsent map[absKey]pos
	seenFile   map[string]bool
	// seenFinal holds followed final components, with the start of the earliest
	// lookup that followed them.
	seenFinal map[string]pos
	// execReads maps an untracked file whose head was read from the post-run
	// disk (for a "#!" line or an ELF header) to the exec that read it.
	execReads map[string]pos

	step   int
	cur    pos // completion of the current call
	start  pos // start of the current call
	callOK bool
	// noFinals is set while resolving the path of a successful open: the
	// descriptor's -y path already says what was opened, so the final
	// component's link-ness is not in question.
	noFinals  bool
	steps     int // walk steps spent
	sockTypes map[string]string

	boundInet []boundAddr
	boundUnix map[string]string // path (or @name) -> socket type
}

func newClassifier(p *prepared) *classifier {
	return &classifier{
		cfg:     p,
		entries: map[string]bool{}, writes: map[string]bool{}, tracked: map[string]bool{},
		untracked: map[string]bool{}, absentObs: map[string]bool{}, reasons: map[string]bool{},
		created: map[string]bool{}, absent: map[string]bool{}, links: map[string]string{},
		dirs: map[string]bool{}, cache: map[string]node{}, mods: map[string][]mod{},
		hard: map[string]bool{}, wlog: map[string][]pos{}, seen: map[string]pos{}, seenAbsent: map[absKey]pos{}, seenFile: map[string]bool{}, seenFinal: map[string]pos{},
		execReads: map[string]pos{}, sockTypes: map[string]string{}, boundUnix: map[string]string{},
	}
}

func (c *classifier) reason(r string) { c.reasons[r] = true }

// runTrace classifies one step's trace.
func (c *classifier) runTrace(t *trace.Trace) {
	for i := range t.Events {
		c.event(t, &t.Events[i])
	}
	c.stepStatus(t)
	c.step++
}

func (c *classifier) event(t *trace.Trace, ev *trace.Event) {
	c.cur = pos{c.step, ev.Line}
	c.start = pos{c.step, ev.StartLine}
	if ev.StartLine == 0 {
		c.start = c.cur
	}
	c.callOK = ev.Result.OK
	switch ev.Kind {
	case trace.KindOpen:
		c.open(ev)
	case trace.KindMetadata:
		if tg, ok := c.targetOf(ev, metaFollow(ev)); ok {
			c.observeResult(tg, ev.Result)
		}
	case trace.KindListing:
		// The path is the descriptor's -y annotation: the kernel's own, so
		// there is nothing to walk.
		c.observe(ev.Path, true, true)
	case trace.KindDirectory:
		if ev.Syscall == "chdir" {
			if tg, ok := c.targetOf(ev, true); ok {
				c.observeResult(tg, ev.Result)
			}
		}
		// fchdir needs nothing: the open that made the descriptor was observed.
	case trace.KindWrite:
		c.write(ev)
	case trace.KindProcess:
		if ev.Syscall == "execve" || ev.Syscall == "execveat" {
			c.exec(ev)
		}
	case trace.KindNetwork:
		c.network(t, ev)
	}
}

// stepStatus adds the reasons that come from how the step ended.
func (c *classifier) stepStatus(t *trace.Trace) {
	if ex := t.StepExit(); ex == nil || ex.Killed || ex.Code != 0 {
		c.reason(rFailed)
	}
	// Daemons (section 4.3): the init child is PID 1 of the sandbox's PID
	// namespace, so when it exits the kernel kills every survivor.  A process
	// whose exit line says SIGKILL and comes AFTER the init child's was such a
	// survivor.
	if t.InitPid == 0 {
		return
	}
	seenInit := false
	for _, ex := range t.Exits {
		if seenInit && ex.Killed && ex.Signal == "SIGKILL" {
			c.reason(rDaemonLeft)
		}
		if ex.Pid == t.InitPid {
			seenInit = true
		}
	}
}

// targetOf resolves an event's primary path.
func (c *classifier) targetOf(ev *trace.Event, follow bool) (target, bool) {
	if ev.Path == "" {
		// open("") fails with ENOENT before any lookup; anything else with no
		// path is not a call we understand.
		if ev.Result.OK {
			c.reason(rEmptyPath)
		}
		return target{}, false
	}
	return c.resolve(join(ev.Cwd, ev.DirfdPath, ev.Path), follow)
}

// presence reads a call's result as "was the path there": ENOENT and ENOTDIR
// say it was not, EEXIST and success say it was, and any other failure
// (EACCES, EISDIR, ...) still means something is there.
func presence(r trace.Result) bool {
	return r.OK || (r.Errno != "ENOENT" && r.Errno != "ENOTDIR")
}

// observeResult observes the target of a call that only looked at it.  The
// path is remembered as absent only after an ENOENT on a name the walk really
// looked up: an ENOENT can also mean a missing parent, or come from a call
// that was about something else.
func (c *classifier) observeResult(tg target, r trace.Result) {
	present := presence(r)
	if !present && r.Errno == "ENOENT" && tg.reached {
		c.absent[tg.abs] = true
	}
	c.observe(tg.abs, present, false)
}

// isCreated says whether the action made abs: it, or a directory above it.
func (c *classifier) isCreated(abs string) bool {
	for p := abs; p != ""; p = parentOf(p) {
		if c.created[p] {
			return true
		}
	}
	return false
}

// observe records that the action looked at abs.  present says whether it
// found something there; listing says the call was getdents.
//
//	virtual (/proc, /dev/fd)   -> the /proc rules
//	made by this action        -> nothing: it is the action's own output
//	inside the repository      -> a pkg:, path: or list: entry
//	anywhere else              -> the host-root rules
func (c *classifier) observe(abs string, present, listing bool) {
	switch {
	case isVirtual(abs):
		c.observeVirtual(abs)
	case c.isCreated(abs):
	default:
		if rel, ok := c.rel(abs); ok {
			c.observeRepo(rel, present, listing)
		} else {
			c.observeHost(abs, present)
		}
	}
}

// observeRepo is section 5.2's table.
func (c *classifier) observeRepo(rel string, present, listing bool) {
	if segs := strings.Split(rel, "/"); rel != "" {
		for i, s := range segs {
			if s != ".git" {
				continue
			}
			// ".git" itself, by stat or access, is nothing; looking inside it
			// (or listing it) reads a commit-dependent answer no key covers.
			if i == len(segs)-1 && !listing {
				return
			}
			c.reason(rVCSRead)
			return
		}
	}
	if rel == "" && !listing {
		return // the repository root is a directory by definition
	}
	if rel != "" {
		if err := key.ValidatePath(rel); err != nil {
			c.reason(rBadPath)
			return
		}
	}
	nd := c.cfg.Tree.Stat(rel)
	switch q, inPkg := c.cfg.Packages.Enclosing(rel); {
	case listing:
		// Listings are never folded into pkg: a listing also sees the names
		// of nested packages (section 5.2).
		if rel == "" {
			c.entries["list:."] = true
		} else {
			c.entries["list:"+rel] = true
		}
	case inPkg:
		c.entries["pkg:"+q] = true
	default:
		c.entries["path:"+c.firstAbsent(rel)] = true
	}
	if present && nd.Kind == KindAbsent {
		c.untracked[rel] = true
		if len(c.untracked) > maxListed {
			c.reason(rSize)
		}
	}
	if !present && !listing {
		// An absence the tracked tree does not cover: another action may
		// create it first (section 14).  The merge job checks these.
		if f := c.firstAbsent(rel); c.cfg.Tree.Stat(f).Kind == KindAbsent {
			c.absentObs[f] = true
			if len(c.absentObs) > maxListed {
				c.reason(rSize)
			}
		}
	}
}

// firstAbsent is the shortest prefix of rel that is not a tracked directory:
// where the tracked tree stops agreeing with the path.  An absent path is
// recorded there, because that is the one name whose digest changes the
// moment something appears (section 5.2).
func (c *classifier) firstAbsent(rel string) string {
	for i := 0; i <= len(rel); i++ {
		if i < len(rel) && rel[i] != '/' {
			continue
		}
		p := rel[:i]
		if p == "" {
			continue
		}
		if c.cfg.Tree.Stat(p).Kind != KindDir {
			return p
		}
	}
	return rel
}

// coarsen turns a written path into its write root (section 7): the highest
// ancestor holding no tracked file, or the path itself.  A tracked path is
// returned as it is.
func (c *classifier) coarsen(rel string) string {
	for i := 0; i <= len(rel); i++ {
		if i < len(rel) && rel[i] != '/' {
			continue
		}
		p := rel[:i]
		if p != "" && c.cfg.Tree.Stat(p).Kind == KindAbsent {
			return p
		}
	}
	return rel
}

// writePath records a write to abs.
func (c *classifier) writePath(abs string) {
	if isVirtual(abs) {
		return
	}
	c.wlog[abs] = append(c.wlog[abs], c.cur)
	rel, ok := c.rel(abs)
	if !ok {
		c.writeHost(abs)
		return
	}
	if rel == "" {
		return
	}
	if err := key.ValidatePath(rel); err != nil {
		c.reason(rBadPath)
		return
	}
	for _, s := range strings.Split(rel, "/") {
		if s == ".git" {
			c.reason(rVCSWrite)
			return
		}
	}
	if c.cfg.Tree.Stat(rel).Kind != KindAbsent {
		c.tracked[rel] = true
		c.reason("writes-tracked:" + sanitize(rel))
	}
	c.writes[c.coarsen(rel)] = true
}

// creating records the absence a creation proves: before the action makes abs,
// nothing of the same name was there, and that is an observation (section 5.2)
// like any ENOENT probe.  It is skipped when a directory above abs is the
// action's own, because then nothing outside the action could have been there.
func (c *classifier) creating(abs string) {
	if isVirtual(abs) || c.isCreated(parentOf(abs)) {
		return
	}
	if rel, ok := c.rel(abs); ok {
		c.observeRepo(rel, false, false)
	}
}

// nsChange logs a namespace change to abs.
func (c *classifier) nsChange(abs string, clean bool) {
	c.mods[abs] = append(c.mods[abs], mod{at: c.cur, clean: clean, made: true})
}

// noteFinal records a followed final component that the disk shows present.
func (c *classifier) noteFinal(abs string) {
	if p, ok := c.seenFinal[abs]; !ok || c.start.before(p) {
		c.seenFinal[abs] = c.start
	}
}

// noteMissing records that a walk stopped at abs because the disk had nothing
// there (or a file).  A final name that is simply absent is an answer the
// call's own result confirms, but the disk's "absent" is the END state: if the
// action made abs before the lookup and removed it afterwards, the call may
// have gone through a link or directory that is gone.  So an absent stop is
// unstable when the last change to it before the lookup was a creation; a file
// there is unstable if the action touched it at all.
func (c *classifier) noteMissing(abs string, isFile bool) {
	if isFile {
		c.seenFile[abs] = true
		return
	}
	k := absKey{abs, len(c.mods[abs])}
	if p, ok := c.seenAbsent[k]; !ok || c.start.before(p) {
		c.seenAbsent[k] = c.start
	}
}

// markCreated records that the action made abs, from empty, now.
func (c *classifier) markCreated(abs string, clean bool) {
	c.creating(abs)
	c.created[abs] = true
	delete(c.absent, abs)
	delete(c.links, abs)
	delete(c.dirs, abs)
	c.nsChange(abs, clean)
}

// markRemoved records that the action removed (or moved away) abs.
func (c *classifier) markRemoved(abs string) {
	delete(c.created, abs)
	delete(c.links, abs)
	delete(c.dirs, abs)
	c.absent[abs] = true
	c.mods[abs] = append(c.mods[abs], mod{at: c.cur})
}

// noteTraversal records that a path went through the component abs, at the
// start of the current call.  Only the earliest lookup matters: if the rule
// holds for it, it holds for every later one.
func (c *classifier) noteTraversal(abs string) {
	if p, ok := c.seen[abs]; !ok || c.start.before(p) {
		c.seen[abs] = c.start
	}
}

// touchedAfter reports a namespace change to abs or to any directory above it
// that completed after `after` (the zero pos means "ever").
func (c *classifier) touchedAfter(abs string, after pos, ever bool) bool {
	for p := abs; p != ""; p = parentOf(p) {
		for _, m := range c.mods[p] {
			if ever || after.before(m.at) {
				return true
			}
		}
	}
	return false
}

// stable is the stability rule of section 5.1.  The post-run disk can vouch
// for a component only if nothing could have changed it between the call and
// the end of the run, and the strict rule accepts exactly two shapes:
//
//	(a) a tracked component that no namespace change touched, nor any ancestor
//	    of it;
//	(b) a component this action created exactly once (mkdir, symlink or
//	    O_CREAT|O_EXCL), whose creation COMPLETED before the lookup STARTED,
//	    that was never a hard-link source or destination, and that no
//	    namespace change has touched since, nor any ancestor of it.
//
// Outside the repository the post-run disk is trusted unless a traced
// namespace change touched the component or an ancestor.  Under $HOME and
// TMPDIR only (b) applies.  Everything else, including an untracked leftover
// of an earlier action, is unstable here: the cross-action rules of section 7
// can only say "reads-product" about what a trace reached by a stable path.
func (c *classifier) stable(abs string, lookup pos) bool {
	if c.underTmp(abs) {
		return c.createdOnce(abs, lookup)
	}
	if rel, ok := c.rel(abs); ok && rel != "" {
		if c.cfg.Tree.Stat(rel).Kind != KindAbsent {
			return !c.touchedAfter(abs, pos{}, true)
		}
		return c.createdOnce(abs, lookup)
	}
	return !c.touchedAfter(abs, pos{}, true)
}

func (c *classifier) createdOnce(abs string, lookup pos) bool {
	m := c.mods[abs]
	if len(m) != 1 || !m[0].clean || !m[0].at.before(lookup) || c.hard[abs] {
		return false
	}
	return !c.touchedAfterParents(abs, m[0].at)
}

// touchedAfterParents is touchedAfter for the proper ancestors of abs.
func (c *classifier) touchedAfterParents(abs string, after pos) bool {
	p := parentOf(abs)
	return p != "" && c.touchedAfter(p, after, false)
}

// checkStability applies the rule to every component any path went through,
// and the exec rule: a file read for its "#!" line or ELF header must not be
// changed afterwards, or the post-run bytes are not the bytes the kernel read.
func (c *classifier) checkStability() {
	for p, at := range c.seen {
		if !c.stable(p, at) {
			c.reason(rUnstable)
		}
	}
	for k, at := range c.seenAbsent {
		var last *mod
		for i := range c.mods[k.path] {
			if m := &c.mods[k.path][i]; m.at.before(at) {
				last = m
			}
		}
		if last != nil && last.made {
			c.reason(rUnstable)
		}
	}
	for p, at := range c.seenFinal {
		// A path the action never changed is what the disk says (an untracked
		// leftover is then an untracked observation, for the merge job); one it
		// changed must satisfy the creation rule.
		if len(c.mods[p]) > 0 && !c.createdOnce(p, at) {
			c.reason(rUnstable)
		}
	}
	for p := range c.seenFile {
		if c.touchedAfter(p, pos{}, true) {
			c.reason(rUnstable)
		}
	}
	for p, at := range c.execReads {
		bad := c.touchedAfter(p, at, false)
		for _, w := range c.wlog[p] {
			bad = bad || at.before(w)
		}
		if bad {
			c.reason(rExecUnreadable)
		}
	}
}

// record assembles the output.
func (c *classifier) record(pkg string) *RawRecord {
	trav := map[string]bool{}
	for p := range c.seen {
		if rel, ok := c.rel(p); ok && rel != "" {
			trav[rel] = true
		}
	}
	if len(trav) > maxListed {
		c.reason(rSize)
	}
	return &RawRecord{
		Package:               pkg,
		Entries:               sortedKeys(c.entries),
		Writes:                sortedKeys(c.writes),
		TrackedWrites:         sortedKeys(c.tracked),
		UntrackedObservations: sortedKeys(c.untracked),
		AbsentObservations:    sortedKeys(c.absentObs),
		Traversed:             sortedKeys(trav),
		Reasons:               capReasons(c.reasons),
	}
}
