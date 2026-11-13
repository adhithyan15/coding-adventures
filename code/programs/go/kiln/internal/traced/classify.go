package traced

import (
	"sort"
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
	rWritesUnknown  = "trace-incomplete:writes-unknown"
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
	at    pos // completion
	start pos // start of the call (the change took effect somewhere in [start, at])
	clean bool
	made  bool // the change may have created the name (any creation)
	// opened marks an O_CREAT open keyed by the descriptor's -y path, and
	// unlinked an unlink/unlinkat of a name.  Together they are the only
	// entries the open-only rule of section 5.5 accepts in a change log.
	opened, unlinked bool
}

// firstMade is the completion of the first entry that proves the action made
// the name itself: a clean creation or an O_CREAT open keyed by a kernel path.
func (c *classifier) firstMade(ms []mod) pos {
	first := posInf
	for _, m := range ms {
		if (m.clean || m.opened) && m.at.before(first) {
			first = m.at
		}
	}
	return first
}

// removedBeforeMade says some removal completed at or after from but before
// the action made the name: the lookup may have gone through what was there.
func removedBeforeMade(ms []mod, firstMade, from pos) bool {
	for _, m := range ms {
		if !m.made && m.at.before(firstMade) && !m.at.before(from) {
			return true
		}
	}
	return false
}

// goneLook describes lookups that followed a final name the post-run disk shows
// absent.
type goneLook struct {
	from    pos
	present bool
}

// span is the interval covered by a set of lookups: the earliest start and the
// latest completion.  Treating all lookups of a name as one wide interval only
// ever adds reasons.
type span struct{ s, e pos }

// lookupSet is every lookup the stability rule judges, keyed by the absolute
// path looked up.  It is the ONLY place these maps are written (the methods
// below), so the classifier can keep two of them in step: one for every call,
// one for the calls that can write.  A test scans the package to keep it so.
type lookupSet struct {
	// seen maps each component the action's paths went through to the START
	// of the earliest lookup that went through it.
	seen map[string]pos
	// seenAbsent holds the places a walk stopped because the disk showed
	// nothing there, with the interval of the lookups; see noteMissing.
	seenAbsent map[string]span
	// ownSeen holds the created paths a call treated as "the action's own"
	// (and so did not observe), with the interval of the calls that did.  A
	// change to such a path that overlaps one of those calls may have landed
	// first, so the call may have seen something else (see judge).
	ownSeen map[string]span
	// seenFile holds the places a walk stopped at a file.
	seenFile map[string]bool
	// seenFinal holds followed final components, with the start of the earliest
	// lookup that followed them.
	seenFinal map[string]pos
	// relFinal and relAbs hold the followed final names of a successful stat or
	// exec that the open-only rule (section 5.5) may accept: relFinal those the
	// post-run disk shows as a file, relAbs those it shows absent.  The span is
	// the earliest start and the latest completion of such lookups.
	relFinal, relAbs map[string]span
	// gone holds the followed final names the post-run disk shows absent, with
	// the start of the earliest lookup and whether any call found something
	// there (it succeeded, or failed with an errno that says so).  See judge.
	gone map[string]goneLook
}

func newLookupSet() lookupSet {
	return lookupSet{
		seen: map[string]pos{}, seenAbsent: map[string]span{}, ownSeen: map[string]span{},
		seenFile: map[string]bool{}, seenFinal: map[string]pos{},
		relFinal: map[string]span{}, relAbs: map[string]span{}, gone: map[string]goneLook{},
	}
}

// widen grows the interval of m[abs] to cover [start, cur].
func widen(m map[string]span, abs string, start, cur pos) {
	sp, ok := m[abs]
	if !ok || start.before(sp.s) {
		sp.s = start
	}
	if !ok || sp.e.before(cur) {
		sp.e = cur
	}
	m[abs] = sp
}

func (l *lookupSet) traversal(abs string, start pos) {
	if p, ok := l.seen[abs]; !ok || start.before(p) {
		l.seen[abs] = start
	}
}

func (l *lookupSet) final(abs string, start pos) {
	if p, ok := l.seenFinal[abs]; !ok || start.before(p) {
		l.seenFinal[abs] = start
	}
}

func (l *lookupSet) missing(abs string, isFile bool, start, cur pos) {
	if isFile {
		l.seenFile[abs] = true
		return
	}
	widen(l.seenAbsent, abs, start, cur)
}

func (l *lookupSet) rel(abs string, absent bool, start, cur pos) {
	if absent {
		widen(l.relAbs, abs, start, cur)
	} else {
		widen(l.relFinal, abs, start, cur)
	}
}

func (l *lookupSet) goneAt(abs string, start pos, present bool) {
	g, ok := l.gone[abs]
	if !ok || start.before(g.from) {
		g.from = start
	}
	g.present = g.present || present
	l.gone[abs] = g
}

func (l *lookupSet) own(p string, start, cur pos) { widen(l.ownSeen, p, start, cur) }

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
	created map[string]pos
	// absent holds paths the action probed and found missing by an
	// unambiguous ENOENT on the final component.
	absent map[string]pos
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
	// binds are the loopback or wildcard inet addresses the action bound, as
	// "<type>/<address>:<port>"; see RawRecord.Binds.
	binds map[string]bool
	// lookupSet (embedded) holds every lookup the stability rule judges (seen,
	// seenAbsent, ownSeen, seenFile, seenFinal, relFinal, relAbs, gone); wr
	// holds only the lookups made by calls that can WRITE.  The same rule is
	// applied to both: the first set explains the unstable-path reason, the
	// second says whether a write may have landed somewhere else (W8).
	lookupSet
	wr lookupSet
	// relax is set while a successful stat or exec target is resolved.
	relax bool
	// execReads maps an untracked file whose head was read from the post-run
	// disk (for a "#!" line or an ELF header) to the exec that read it.
	execReads map[string]pos

	// incomplete is the negation of RawRecord.WritesComplete: set the moment
	// something happens that could hide a write from Writes/TrackedWrites.
	// Every setter says which trigger of KILN04 section 5.5 it is.
	incomplete bool
	// inWrite is set while a call that can write is being processed.  Any
	// trace-incomplete reason raised then (a loop, a budget, an unresolvable
	// lookup, an unstable direct check) is a reason the WRITE could not be
	// placed, so it sets incomplete too (W8).
	inWrite bool
	step    int
	cur     pos // completion of the current call
	start   pos // start of the current call
	callOK  bool
	// callPresent says the call found something at its path (presence).
	callPresent bool
	// openRet is the -y path of the descriptor while the path of a successful
	// open is resolved: when the walk ends at exactly that path, the final name
	// was not a link at the moment of the open ("opens settle the final name",
	// section 5.5).
	openRet string
	// noOwnSeen is set while a call observes the very name it is about to
	// change: its own change is not an overlap with itself.
	noOwnSeen bool
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
		created: map[string]pos{}, absent: map[string]pos{}, links: map[string]string{},
		dirs: map[string]bool{}, cache: map[string]node{}, mods: map[string][]mod{},
		hard: map[string]bool{}, wlog: map[string][]pos{}, binds: map[string]bool{},
		execReads: map[string]pos{}, sockTypes: map[string]string{}, boundUnix: map[string]string{},
		lookupSet: newLookupSet(), wr: newLookupSet(),
	}
}

func (c *classifier) reason(r string) {
	c.reasons[r] = true
	// W8: a read-side problem is only a reason, but the same problem while a
	// write is being placed means the write may be somewhere else.
	if c.inWrite && strings.HasPrefix(r, "trace-incomplete:") {
		c.incomplete = true
	}
}

// writing runs f with inWrite set (restoring it after), for a call that can
// write.
func (c *classifier) writing(f func()) {
	old := c.inWrite
	c.inWrite = true
	defer func() { c.inWrite = old }()
	f()
}

// budgetOut raises the size reason when the stability pass runs out of
// budget.  The pass stops early, so nothing vouches for the components it did
// not reach: any of them may have been under a write (W9).
func (c *classifier) budgetOut() {
	c.reason(rSize)
	c.incomplete = true
}

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
	c.callPresent = presence(ev.Result)
	switch ev.Kind {
	case trace.KindOpen:
		c.open(ev)
	case trace.KindMetadata:
		follow := metaFollow(ev)
		c.relax = follow && ev.Result.OK
		tg, ok := c.targetOf(ev, follow)
		c.relax = false
		if ok {
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
		c.writing(func() { c.write(ev) })
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
	ex := t.StepExit()
	if ex == nil || ex.Killed || ex.Code != 0 {
		c.reason(rFailed)
	}
	// W10: no exit line for the step, or death by a signal, is a truncated or
	// interrupted trace.  A plain nonzero exit is not: the failed action
	// wrote what it wrote, and the merge handles the failure (section 7.1,
	// M-1).  A traced process with no exit line was still running when the
	// trace ended, or its line was lost: its later writes are unseen.
	if ex == nil || ex.Killed {
		c.incomplete = true
	}
	for _, p := range t.Procs {
		if p == nil || p.Exit == nil || p.Exit.Killed || p.Exit.CoreDumped {
			c.incomplete = true
		}
	}
	// L1: a process killed by a signal, whichever process it was, may have been
	// cut off with work unseen; and the kernel writes a core file for a
	// process that dumps core with NO system call, so no trace shows it.
	for _, ex := range t.Exits {
		if ex.Killed || ex.CoreDumped {
			c.incomplete = true
		}
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
			c.incomplete = true // W10: a survivor may have written before it was killed
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
		c.absent[tg.abs] = c.cur
	}
	c.observe(tg.abs, present, false)
}

// observeSelf observes abs for a call that is itself about to change it.
func (c *classifier) observeSelf(abs string, present bool) {
	c.noOwnSeen = true
	c.observe(abs, present, false)
	c.noOwnSeen = false
}

// isCreated says whether the action made abs: it, or a directory above it.
func (c *classifier) isCreated(abs string) bool {
	for p := abs; p != ""; p = parentOf(p) {
		if at, ok := c.created[p]; ok && at.before(c.start) {
			if !c.noOwnSeen {
				c.lookupSet.own(p, c.start, c.cur)
				if c.inWrite {
					c.wr.own(p, c.start, c.cur)
				}
			}
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
		c.incomplete = true // W5: the name says nothing about the object written
		return
	}
	c.wlog[abs] = append(c.wlog[abs], c.cur)
	rel, ok := c.rel(abs)
	if !ok {
		c.writeHost(abs)
		return
	}
	if rel == "" {
		c.incomplete = true // W7: the root has no write root to be listed under
		return
	}
	if err := key.ValidatePath(rel); err != nil {
		c.reason(rBadPath)
		c.incomplete = true // W6: a path the record cannot hold
		return
	}
	for _, s := range strings.Split(rel, "/") {
		if s == ".git" {
			c.reason(rVCSWrite)
			c.incomplete = true // W6
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
func (c *classifier) nsChange(abs string, clean, opened bool) {
	c.mods[abs] = append(c.mods[abs], mod{at: c.cur, start: c.start, clean: clean, made: true, opened: opened})
}

// noteRel records a followed final name of a successful stat or exec for the
// open-only rule: absent says the post-run disk shows it absent (relAbs), else
// it is a file (relFinal).
func (c *classifier) noteRel(abs string, absent bool) {
	c.lookupSet.rel(abs, absent, c.start, c.cur)
	if c.inWrite {
		c.wr.rel(abs, absent, c.start, c.cur)
	}
}

// openOnly is the open-only change-log rule of section 5.5.  A final name p
// that a successful stat or exec followed was a regular file throughout, and
// the post-run disk may stand in for the lookup, when ALL of these hold:
//
//   - every entry of p's change log is an O_CREAT open keyed by its -y path
//     (an O_CREAT open through a link is logged at the link's target, so an
//     open logged at p proves p was not a link when it ran; it never replaces
//     or moves a name), except that the log may end with ONE unlink;
//   - the first open COMPLETED before the earliest such lookup STARTED;
//   - the unlink, when there is one, STARTED after the latest lookup
//     COMPLETED (so no lookup overlaps it), and the disk agrees with it: p is
//     absent after an unlink and present without one (absent says which);
//   - p was never a link()/linkat() source or destination.
//
// The ancestors of p need no check here: every lookup went through them, so
// the ordinary rule of stable already judged each one.
func (c *classifier) openOnly(p string, sp span, absent bool) bool {
	ms := c.mods[p]
	n := len(ms)
	if n == 0 || c.hard[p] {
		return false
	}
	k := n
	if ms[n-1].unlinked {
		k = n - 1
		if !sp.e.before(ms[n-1].start) {
			return false
		}
	}
	if absent != (k < n) || k == 0 {
		return false
	}
	for _, m := range ms[:k] {
		if !m.opened {
			return false
		}
	}
	return ms[0].at.before(sp.s)
}

// noteFinal records a followed final component that the disk shows present.
func (c *classifier) noteFinal(abs string) {
	c.lookupSet.final(abs, c.start)
	if c.inWrite {
		c.wr.final(abs, c.start)
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
	c.lookupSet.missing(abs, isFile, c.start, c.cur)
	if c.inWrite {
		c.wr.missing(abs, isFile, c.start, c.cur)
	}
}

// noteGone records a followed final name that the post-run disk shows absent.
func (c *classifier) noteGone(abs string) {
	c.lookupSet.goneAt(abs, c.start, c.callPresent)
	if c.inWrite {
		c.wr.goneAt(abs, c.start, c.callPresent)
	}
}

// markCreated records that the action made abs, from empty, now.
func (c *classifier) markCreated(abs string, clean bool) {
	c.creating(abs)
	c.created[abs] = c.cur
	delete(c.absent, abs)
	delete(c.links, abs)
	delete(c.dirs, abs)
	c.nsChange(abs, clean, false)
}

// markRemoved records that the action removed (or moved away) abs.  unlink
// says the call was unlink or unlinkat without AT_REMOVEDIR.
func (c *classifier) markRemoved(abs string, unlink bool) {
	delete(c.created, abs)
	delete(c.links, abs)
	delete(c.dirs, abs)
	c.absent[abs] = c.cur
	c.mods[abs] = append(c.mods[abs], mod{at: c.cur, start: c.start, unlinked: unlink})
}

// noteTraversal records that a path went through the component abs, at the
// start of the current call.  Only the earliest lookup matters: if the rule
// holds for it, it holds for every later one.
func (c *classifier) noteTraversal(abs string) {
	c.lookupSet.traversal(abs, c.start)
	if c.inWrite {
		c.wr.traversal(abs, c.start)
	}
}

// touchedAfter reports a namespace change to abs or to any directory above it
// that completed after `after` (the zero pos means "ever").
func (c *classifier) touchedAfter(abs string, after pos, ever bool) bool {
	for p := abs; p != ""; p = parentOf(p) {
		// The log is in completion order, so the last entry decides.
		if ms := c.mods[p]; len(ms) > 0 && (ever || after.before(ms[len(ms)-1].at)) {
			return true
		}
	}
	return false
}

// touchedFrom is touchedAfter for a change that completed at or after `from`:
// one that may have taken effect while a call that began at `from` was running.
func (c *classifier) touchedFrom(abs string, from pos) bool {
	for p := abs; p != ""; p = parentOf(p) {
		if ms := c.mods[p]; len(ms) > 0 && !ms[len(ms)-1].at.before(from) {
			return true
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
	return !c.touchedAfterParents(abs, m[0].start)
}

// touchedAfterParents is touchedAfter for the proper ancestors of abs.
func (c *classifier) touchedAfterParents(abs string, after pos) bool {
	p := parentOf(abs)
	return p != "" && c.touchedFrom(p, after)
}

// checkStability applies the rule to every component any path went through,
// and the exec rule: a file read for its "#!" line or ELF header must not be
// changed afterwards, or the post-run bytes are not the bytes the kernel read.
//
// The rule runs twice, over the same change log: over every lookup (an
// unstable one is the unstable-path reason) and over the lookups of calls
// that can write alone (an unstable one means a write may have landed
// somewhere else: W8).  A lookup that only a read made, however unstable,
// costs a write nothing.  Judging the write lookups on their own intervals
// is as sound as judging them within the wider set: the verdicts are
// monotone in the interval (a shorter span, or a later first lookup, can only
// be safer), and what a write's placement depends on is exactly its own
// lookups.
func (c *classifier) checkStability() {
	idx := map[string]*modIndex{}
	c.judge(&c.lookupSet, idx, func() { c.reason(rUnstable) })
	c.judge(&c.wr, idx, func() { c.incomplete = true })
	for p, from := range c.execReads {
		// Measured from the FIRST exec's start: any change that completed after
		// it began may have altered the bytes the kernel read.
		bad := c.touchedFrom(p, from)
		if w := c.wlog[p]; len(w) > 0 && !w[len(w)-1].before(from) {
			bad = true
		}
		if bad {
			c.reason(rExecUnreadable)
		}
	}
}

// judge applies the stability rule to the lookups in l; bad is called once
// for each lookup that is unstable.
func (c *classifier) judge(l *lookupSet, idx map[string]*modIndex, bad func()) {
	for p, at := range l.seen {
		if !c.stable(p, at) {
			bad()
		}
	}
	for p, sp := range l.seenAbsent {
		c.steps += 8
		if c.steps > walkBudget {
			c.budgetOut()
			break
		}
		ms := c.mods[p]
		if len(ms) == 0 {
			continue
		}
		mi := idx[p]
		if mi == nil {
			mi = newModIndex(ms)
			idx[p] = mi
		}
		if mi.unsafe(ms, sp.s, sp.e) || removedBeforeMade(ms, c.firstMade(ms), sp.s) {
			bad()
		}
	}
	for p, at := range l.seenFinal {
		// A path the action never changed is what the disk says (an untracked
		// leftover is then an untracked observation, for the merge job); one it
		// changed must satisfy the creation rule.
		if len(c.mods[p]) > 0 && !c.createdOnce(p, at) {
			bad()
		}
	}
	// The followed finals of a successful stat or exec: the open-only rule,
	// else the strict rules they would have met as ordinary lookups.
	for p, sp := range l.relFinal {
		if len(c.mods[p]) > 0 && !c.openOnly(p, sp, false) && !c.createdOnce(p, sp.s) {
			bad()
		}
	}
	for p, sp := range l.relAbs {
		ms := c.mods[p]
		if len(ms) == 0 || c.openOnly(p, sp, true) {
			continue
		}
		c.steps += 8
		if c.steps > walkBudget {
			c.budgetOut()
			break
		}
		mi := idx[p]
		if mi == nil {
			mi = newModIndex(ms)
			idx[p] = mi
		}
		if mi.unsafe(ms, sp.s, sp.e) {
			bad()
		}
	}
	// A followed final name that the post-run disk shows absent was removed, or
	// never there.  A call that found something there needs the action to have
	// made the name: the creation must have COMPLETED before the earliest
	// lookup began.  And a removal of a name the action had NOT made yet, that
	// completed after the earliest lookup began, may have taken a link or file
	// the lookup went through (a pre-existing object the disk no longer shows).
	for p, g := range l.gone {
		ms := c.mods[p]
		firstMade := c.firstMade(ms)
		if g.present && !firstMade.before(g.from) {
			bad()
		}
		if removedBeforeMade(ms, firstMade, g.from) {
			bad()
		}
	}
	for p := range l.seenFile {
		if c.touchedAfter(p, pos{}, true) {
			bad()
		}
	}
	for p, sp := range l.ownSeen {
		// Anything that may have taken effect while such a call ran: a change
		// to the path or an ancestor that completed after the call began and
		// began before it ended.
		for q := p; q != ""; q = parentOf(q) {
			ms := c.mods[q]
			if len(ms) == 0 {
				continue
			}
			c.steps += 8
			if c.steps > walkBudget {
				c.budgetOut()
				break
			}
			mi := idx[q]
			if mi == nil {
				mi = newModIndex(ms)
				idx[q] = mi
			}
			if mi.overlaps(ms, sp.s, sp.e) {
				bad()
				break
			}
		}
	}
}

// modIndex answers, in O(log n), whether some change to a name may have been in
// effect while a lookup of it ran, using prefix maxima over the (completion
// ordered) change log.
type modIndex struct {
	madeAtMax   []pos // max completion of made changes among ms[:i]
	remStartMax []pos // max start of removals among ms[:i]
	madeStart   []pos // min start of made changes among ms[i:]
	anyStart    []pos // min start of any change among ms[i:]
}

var (
	posNone = pos{-1, -1}
	posInf  = pos{1 << 30, 0}
)

func newModIndex(ms []mod) *modIndex {
	n := len(ms)
	mi := &modIndex{make([]pos, n+1), make([]pos, n+1), make([]pos, n+1), make([]pos, n+1)}
	mi.madeAtMax[0], mi.remStartMax[0], mi.madeStart[n], mi.anyStart[n] = posNone, posNone, posInf, posInf
	for i, m := range ms {
		mi.madeAtMax[i+1], mi.remStartMax[i+1] = mi.madeAtMax[i], mi.remStartMax[i]
		if m.made && mi.madeAtMax[i].before(m.at) {
			mi.madeAtMax[i+1] = m.at
		}
		if !m.made && mi.remStartMax[i].before(m.start) {
			mi.remStartMax[i+1] = m.start
		}
	}
	for i := n - 1; i >= 0; i-- {
		mi.madeStart[i] = mi.madeStart[i+1]
		mi.anyStart[i] = mi.anyStart[i+1]
		if ms[i].start.before(mi.anyStart[i]) {
			mi.anyStart[i] = ms[i].start
		}
		if ms[i].made && ms[i].start.before(mi.madeStart[i]) {
			mi.madeStart[i] = ms[i].start
		}
	}
	return mi
}

// overlaps says some change completed at or after s and began before e.
func (mi *modIndex) overlaps(ms []mod, s, e pos) bool {
	j := sort.Search(len(ms), func(i int) bool { return !ms[i].at.before(s) })
	return mi.anyStart[j].before(e)
}

// unsafe says a lookup that ran over [s, e] and found the name absent on the
// post-run disk may have seen it present: some creation could have taken effect
// before the lookup ended (it began before e), and no removal that began after
// that creation completed also completed before the lookup began.
func (mi *modIndex) unsafe(ms []mod, s, e pos) bool {
	n := len(ms)
	i1 := sort.Search(n, func(i int) bool { return !ms[i].at.before(e) })
	if mi.madeStart[i1].before(e) {
		return true // a creation overlapping or following the lookup's start
	}
	made := mi.madeAtMax[i1]
	if made == posNone {
		return false
	}
	j := sort.Search(n, func(i int) bool { return !ms[i].at.before(s) })
	return !made.before(mi.remStartMax[j])
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
	// The invariant the merge job leans on (KILN04 7.1, G1): an unknown write
	// set always leaves a trace-incomplete reason in the record.  Several
	// triggers (a write to the root, a failed unplaceable call, a core dump, a
	// daemon peer) leave none of their own, so say it here.  The reason cap
	// keeps one reason of every kind, so it survives capReasons.
	if c.incomplete {
		has := false
		for r := range c.reasons {
			if strings.HasPrefix(r, "trace-incomplete:") {
				has = true
				break
			}
		}
		if !has {
			c.reasons[rWritesUnknown] = true
		}
	}
	// W9: the write lists are never truncated, but a record this large is
	// refused like the other lists (the size reason), and says its write set
	// is not to be trusted.
	if len(c.writes) > maxListed || len(c.tracked) > maxListed {
		c.reason(rSize)
		c.incomplete = true
	}
	return &RawRecord{
		Package:               pkg,
		Entries:               sortedKeys(c.entries),
		Writes:                sortedKeys(c.writes),
		TrackedWrites:         sortedKeys(c.tracked),
		UntrackedObservations: sortedKeys(c.untracked),
		AbsentObservations:    sortedKeys(c.absentObs),
		Binds:                 sortedKeys(c.binds),
		Traversed:             sortedKeys(trav),
		Reasons:               capReasons(c.reasons),
		WritesComplete:        !c.incomplete,
	}
}
