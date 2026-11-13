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
	rExecUnreadable = "trace-incomplete:exec-unreadable"
	rExecFormat     = "trace-incomplete:exec-format"
	rExecDepth      = "trace-incomplete:exec-depth"
	rExecELF        = "trace-incomplete:exec-elf"
	rRecipe         = "trace-incomplete:recipe-mismatch"
	rVCSRead        = "vcs-read"
	rFailed         = "failed"
	rDaemonLeft     = "daemon-left"
)

// classifier is the state of one action across all of its steps.
//
// The sets that make "written, then read" work (created, absent, links) are
// kept in TRACE ORDER: a path is the action's own only from the moment the
// action made it.  The rest are the record being built.
type classifier struct {
	cfg *prepared

	entries, writes, tracked, untracked, reasons map[string]bool

	// created holds the absolute paths this action created from empty so far
	// (section 5.2).  A path is the action's own if it, or any directory above
	// it, is in the set.
	created map[string]bool
	// absent holds paths the action probed and found missing: a later create
	// of one provably started from empty.
	absent map[string]bool
	// links is the symlinks this action created and has not removed: their
	// targets are known from the trace, whatever the post-run disk says.
	links map[string]string
	// dirs is the directories this action made and has not removed; gone is
	// paths it removed (or moved away) and has not made again.  With links they
	// let a lookup answer from the trace, exactly as things stood at the time,
	// where the post-run disk only knows how they ended.
	dirs map[string]bool
	gone map[string]bool
	// cache holds post-run lookups, which do not change during classification.
	cache map[string]node

	// mods logs, per absolute path, each namespace change the action made to
	// it, in order: true for a clean creation, false for a removal or other
	// change.  The stability rule reads it after the whole trace is seen.
	mods map[string][]bool
	// seen is the set of (component, number of changes made to it so far,
	// whether the trace knew it exactly) the action's paths went through.  Two
	// traversals with the same key cannot differ in stability, so this stays
	// small however many events there are.
	seen map[travKey]bool

	boundInet []boundAddr
	boundUnix map[string]bool
}

type travKey struct {
	path  string
	k     int
	exact bool
}

func newClassifier(p *prepared) *classifier {
	return &classifier{
		cfg:     p,
		entries: map[string]bool{}, writes: map[string]bool{}, tracked: map[string]bool{},
		untracked: map[string]bool{}, reasons: map[string]bool{},
		created: map[string]bool{}, absent: map[string]bool{}, links: map[string]string{},
		dirs: map[string]bool{}, gone: map[string]bool{},
		cache: map[string]node{}, mods: map[string][]bool{}, seen: map[travKey]bool{},
		boundUnix: map[string]bool{},
	}
}

func (c *classifier) reason(r string) { c.reasons[r] = true }

// runTrace classifies one step's trace.
func (c *classifier) runTrace(t *trace.Trace) {
	for i := range t.Events {
		c.event(t, &t.Events[i])
	}
	c.stepStatus(t)
}

func (c *classifier) event(t *trace.Trace, ev *trace.Event) {
	switch ev.Kind {
	case trace.KindOpen:
		c.open(ev)
	case trace.KindMetadata:
		if tg, ok := c.targetOf(ev, metaFollow(ev)); ok {
			c.observeResult(tg.abs, ev.Result)
		}
	case trace.KindListing:
		// The path is the descriptor's -y annotation: the kernel's own, so
		// there is nothing to walk.
		c.observe(ev.Path, true, true)
	case trace.KindDirectory:
		if ev.Syscall == "chdir" {
			if tg, ok := c.targetOf(ev, true); ok {
				c.observeResult(tg.abs, ev.Result)
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

// observeResult observes the target of a call that only looked at it.
func (c *classifier) observeResult(abs string, r trace.Result) {
	present := presence(r)
	if !present {
		c.absent[abs] = true
	}
	c.observe(abs, present, false)
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

// markCreated records that the action made abs, from empty, now.
func (c *classifier) markCreated(abs string) {
	c.created[abs] = true
	delete(c.absent, abs)
	delete(c.links, abs)
	delete(c.dirs, abs)
	delete(c.gone, abs)
	c.mods[abs] = append(c.mods[abs], true)
}

// markRemoved records that the action removed (or moved away) abs.
func (c *classifier) markRemoved(abs string) {
	delete(c.created, abs)
	delete(c.links, abs)
	delete(c.dirs, abs)
	c.gone[abs] = true
	c.absent[abs] = true
	c.mods[abs] = append(c.mods[abs], false)
}

// noteTraversal records that a path went through the component abs.
func (c *classifier) noteTraversal(abs string, exact bool) {
	c.seen[travKey{abs, len(c.mods[abs]), exact}] = true
}

// checkStability applies the stability rule of section 5.1 once the whole
// trace is known.  The post-run disk can only vouch for what it still shows,
// so a component the action's paths went through is stable when:
//
//   - the trace itself knew it at that moment (a link or directory the action
//     made, or a path it removed): the walk used the trace, not the disk;
//   - the action never changed it (another action that did is the merge job's
//     business, which is why Traversed is exposed);
//   - its LAST change was a clean creation and the traversal came after it, so
//     the disk shows the same thing the call saw;
//   - it did not exist yet when the call ran (the action made it later, and
//     the call never reached it).
//
// Anything else, such as a tracked link the action replaced after reading
// through it, means the disk may disagree with what the call saw:
// unstable-path.
func (c *classifier) checkStability() {
	for k := range c.seen {
		m := c.mods[k.path]
		n := len(m)
		switch {
		case k.exact, n == 0:
		case k.k == n && m[n-1]:
		case k.k == 0 && m[0] && !c.trackedAbs(k.path):
		default:
			c.reason(rUnstable)
		}
	}
}

func (c *classifier) trackedAbs(abs string) bool {
	rel, ok := c.rel(abs)
	return ok && c.cfg.Tree.Stat(rel).Kind != KindAbsent
}

// record assembles the output.
func (c *classifier) record(pkg string) *RawRecord {
	trav := map[string]bool{}
	for k := range c.seen {
		if rel, ok := c.rel(k.path); ok && rel != "" {
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
		Traversed:             sortedKeys(trav),
		Reasons:               capReasons(c.reasons),
	}
}
