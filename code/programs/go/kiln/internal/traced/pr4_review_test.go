package traced

import (
	"go/ast"
	"go/parser"
	"go/token"
	"os"
	"testing"

	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/trace"
)

// Regression tests for the adversarial review of PR 4 (writes_complete).  The
// names of the proofs of concept are kept.

// M1: a NOFOLLOW metadata write with a trailing slash FOLLOWS a symlink to a
// directory (the kernel's lookup of a final name ending in "/" forces
// LOOKUP_FOLLOW|LOOKUP_DIRECTORY): it changes the directory, not the link.
func TestPOCTrailingSlashNofollow(t *testing.T) {
	h := newHarness(t)
	h.tree.link("code/p/a/dl", "../b") // a tracked link to the tracked directory code/p/b
	for _, sys := range []string{"lchown", "lsetxattr", "lremovexattr"} {
		rec := h.wc(write(sys, abs("code/p/a/dl/")))
		eq(t, sys+" tracked writes", rec.TrackedWrites, "code/p/b")
		wantComplete(t, sys, rec, true)
	}
	for _, sys := range []string{"utimensat", "fchownat", "fchmodat2"} {
		e := write(sys, abs("code/p/a/dl/"))
		e.Flags = "AT_SYMLINK_NOFOLLOW"
		rec := h.wc(e)
		eq(t, sys+" tracked writes", rec.TrackedWrites, "code/p/b")
	}
	// Without the slash the call changes the link itself.
	eq(t, "lchown dl", h.wc(write("lchown", abs("code/p/a/dl"))).TrackedWrites, "code/p/a/dl")
	// A followed call with the slash was always right.
	eq(t, "chown dl/", h.wc(write("chown", abs("code/p/a/dl/"))).TrackedWrites, "code/p/b")

	// The worst variant: a link the action made itself, in an output directory,
	// so no tracked write and no reason shows the damage.
	h2 := newHarness(t)
	h2.noSettle = true
	h2.fs.dir(abs("code/p/a/out")).link(abs("code/p/a/out/dl"), "../../b")
	e := write("fchmodat2", abs("code/p/a/out/dl/"))
	e.Flags = "AT_SYMLINK_NOFOLLOW"
	rec := h2.wc(mkdir(abs("code/p/a/out")), symlink("../../b", abs("code/p/a/out/dl")), e)
	eq(t, "writes", rec.Writes, "code/p/a/out", "code/p/b")
	eq(t, "tracked writes", rec.TrackedWrites, "code/p/b")
	wantComplete(t, "worst variant", rec, true)
	// mkdir and friends keep stripping the slash: they act on the NAME.
	rec = newHarness(t).wc(mkdir(abs("code/p/a/newdir/")))
	eq(t, "mkdir writes", rec.Writes, "code/p/a/newdir")
}

// M2: re-opening a pipe, socket or anonymous inode is not an unknown write.
func TestPOCDevStderr(t *testing.T) {
	h := newHarness(t)
	h.fs.link("/dev/stderr", "/proc/self/fd/2")
	h.fs.link("/dev/stdout", "/proc/self/fd/1")
	wantComplete(t, "> /dev/stderr", h.wc(open("/dev/stderr", "O_WRONLY|O_CREAT|O_TRUNC", "pipe:[77]")), true)
	wantComplete(t, ">> /dev/stdout", h.wc(open("/dev/stdout", "O_WRONLY|O_APPEND", "pipe:[77]")), true)
	wantComplete(t, "/proc/self/fd/1 -> socket", h.wc(open("/proc/self/fd/1", "O_WRONLY", "socket:[5]")), true)
	wantComplete(t, "/proc/self/fd/1 -> anon inode", h.wc(open("/proc/self/fd/1", "O_WRONLY", "anon_inode:[eventfd]")), true)
	wantComplete(t, "/dev/null", h.wc(open("/dev/null", "O_WRONLY|O_CREAT|O_TRUNC", "/dev/null")), true)
	// Anything else that is not an absolute path stays unknown, and so does
	// any other virtual name.
	wantComplete(t, "unknown annotation", h.wc(open("/proc/self/fd/1", "O_WRONLY", "weird:[5]")), false)
	wantComplete(t, "empty annotation", h.wc(open("/proc/self/fd/1", "O_WRONLY", "")), false)
	wantComplete(t, "/proc/self/oom_score_adj", h.wc(open("/proc/self/oom_score_adj", "O_WRONLY", "/proc/42/oom_score_adj")), false)
	wantComplete(t, "/proc/self/task/9/comm", h.wc(open("/proc/self/task/9/comm", "O_RDWR", "/proc/42/task/9/comm")), false)
	// A pipe annotation on a NAME that is not an fd link is still W5.
	wantComplete(t, "pipe on a /proc name", h.wc(open("/proc/self/cwd/x", "O_WRONLY", "pipe:[1]")), false)
}

// M3 (a): an unknown write set always leaves a trace-incomplete reason, so a
// merge that ignores or lacks the writes_complete key still sees it.
func TestPOCReasonless(t *testing.T) {
	h := newHarness(t)
	nb := base("bind", trace.KindNetwork, "")
	orphan := step(mkdir(abs("code/p/a/out")))
	orphan.Procs[9] = &trace.Process{Pid: 9, Parent: 1}
	killed := step(mkdir(abs("code/p/a/out")))
	killed.Procs[1].Exit = &trace.Exit{Pid: 1, Killed: true, Signal: "SIGKILL"}
	core := step(mkdir(abs("code/p/a/out")))
	core.Procs[7] = &trace.Process{Pid: 7, Parent: 1, Exit: &trace.Exit{Pid: 7, Killed: true, Signal: "SIGSEGV", CoreDumped: true}}
	core.Exits = []trace.Exit{{Pid: 7, Killed: true, Signal: "SIGSEGV", CoreDumped: true}, {Pid: 1}}
	h.fs.file("/var/run/docker.sock")
	cases := map[string]*RawRecord{
		"chmod root":                     h.wc(write("chmod", testRoot)),
		"creating open via /proc":        h.wc(openW("/proc/self/cwd/code/p/a/o", abs("code/p/a/o"))),
		"failed mkdir past ..":           h.wc(fail(mkdir("code/p/a/nodir/../x"), "ENOENT")),
		"failed chmod empty":             h.wc(fail(write("chmod", ""), "ENOENT")),
		"bind via /proc":                 h.wc(unix("bind", "/proc/self/cwd/s")),
		"bind nil addr":                  h.wc(nb),
		"proc with no exit":              h.runSteps(orphan),
		"killed step":                    h.runSteps(killed),
		"core dump":                      h.runSteps(core),
		"daemon over a host unix socket": h.wc(unix("connect", "/var/run/docker.sock")),
	}
	for name, rec := range cases {
		wantComplete(t, name, rec, false)
		if !hasReason(rec, rWritesUnknown) && !hasKindOf(rec, "trace-incomplete") {
			t.Errorf("%s: an unknown write set with no trace-incomplete reason (reasons %q)", name, rec.Reasons)
		}
	}
	// The fallback reason is added only when nothing else says it.
	rec := h.wc(mkdir("code/p/a/nodir/../x"))
	wantReason(t, rec, rUnresolvableOrUnstable(rec))
	if hasReason(rec, rWritesUnknown) {
		t.Errorf("an unplaceable write already has its reason: %q", rec.Reasons)
	}
	// A complete record never carries it.
	if rec := h.wc(mkdir(abs("code/p/a/out"))); hasReason(rec, rWritesUnknown) {
		t.Errorf("a complete record carries writes-unknown: %q", rec.Reasons)
	}
	// The reason survives the per-kind cap: sixteen other reasons of another
	// kind do not push it out, and sixteen trace-incomplete ones make it
	// unnecessary.
	var evs []trace.Event
	for i := 0; i < 40; i++ {
		evs = append(evs, openAbs("/opt/x"+string(rune('a'+i%26))+string(rune('a'+i/26))))
	}
	evs = append(evs, write("chmod", testRoot))
	rec = h.wc(evs...)
	wantComplete(t, "many reasons", rec, false)
	if !hasReason(rec, rWritesUnknown) {
		t.Errorf("writes-unknown lost under the cap: %q", rec.Reasons)
	}
}

func hasKindOf(rec *RawRecord, kind string) bool {
	for _, r := range rec.Reasons {
		if len(r) >= len(kind) && r[:len(kind)] == kind && (len(r) == len(kind) || r[len(kind)] == ':') {
			return true
		}
	}
	return false
}

// rUnresolvableOrUnstable is the reason a write past ".." raises itself.
func rUnresolvableOrUnstable(rec *RawRecord) string {
	for _, r := range rec.Reasons {
		if r == rUnstable || r == rUnresolvable || r == rLoop || r == rSize {
			return r
		}
	}
	return rUnstable
}

// L1: a process that dumps core, or dies of any signal, leaves writes no
// system call shows.
func TestPOCCoreDump(t *testing.T) {
	h := newHarness(t)
	mk := func(ex trace.Exit) *trace.Trace {
		st := step(mkdir(abs("code/p/a/out")))
		st.Procs[7] = &trace.Process{Pid: 7, Parent: 1, Exit: &ex}
		ex.Pid = 7
		st.Exits = []trace.Exit{ex, {Pid: 1}}
		return st
	}
	wantComplete(t, "core dumped", h.runSteps(mk(trace.Exit{Killed: true, Signal: "SIGSEGV", CoreDumped: true})), false)
	wantComplete(t, "SIGPIPE child (yes | head)", h.runSteps(mk(trace.Exit{Killed: true, Signal: "SIGPIPE"})), true) // M5: a killed process hides no write
	wantComplete(t, "a child that exited", h.runSteps(mk(trace.Exit{Code: 3})), true)
	// The exit line alone, without a process entry, is enough.
	st := step(mkdir(abs("code/p/a/out")))
	st.Exits = []trace.Exit{{Pid: 7, CoreDumped: true}, {Pid: 1}}
	wantComplete(t, "exit line only", h.runSteps(st), false)
}

// L2: a successful connect to an untraced peer (a daemon) may write the
// repository for it.
func TestPOCDaemon(t *testing.T) {
	h := newHarness(t)
	h.fs.file("/var/run/docker.sock")
	rec := h.wc(unix("connect", "/var/run/docker.sock"))
	wantReason(t, rec, "daemon:/var/run/docker.sock")
	wantComplete(t, "host unix daemon", rec, false)
	wantComplete(t, "loopback daemon (own netns, M4)", h.wc(inet("connect", "127.0.0.1", 5432)), true)
	ab := unix("connect", "dbus")
	ab.Addr.Abstract = true
	wantComplete(t, "abstract daemon (own netns, M4)", h.wc(ab), true)
	// A peer the action bound itself is traced.
	wantComplete(t, "own server", h.runNet(unix("bind", "/tmp/kiln-1/s"), unix("connect", "/tmp/kiln-1/s")), true)
	// An expected failure is host state the image fixes.
	wantComplete(t, "refused", h.wc(fail(inet("connect", "127.0.0.1", 5432), "ECONNREFUSED")), true)
}

// L3: write lookups are judged on their own; probing a name, then creating it,
// is not an unknown write.
func TestPOCProbeThenCreate(t *testing.T) {
	h := newHarness(t)
	h.noSettle = true
	h.fs.dir(abs("code/p/a/out")).file(abs("code/p/a/out/x"))
	rec := h.wc(
		fail(stat(abs("code/p/a/out/x")), "ENOENT"),
		mkdir(abs("code/p/a/out")),
		openW(abs("code/p/a/out/x"), abs("code/p/a/out/x")),
	)
	wantReason(t, rec, rUnstable) // the read of out/x went through a directory made later
	wantComplete(t, "probe child, mkdir, create", rec, true)
	rec = h.wc(
		fail(stat(abs("code/p/a/out")), "ENOENT"),
		mkdir(abs("code/p/a/out")),
		openW(abs("code/p/a/out/x"), abs("code/p/a/out/x")),
	)
	wantComplete(t, "probe dir, mkdir, create", rec, true)
	// A write that really went through an unstable component stays unknown:
	// out is replaced by a rename after the write placed a file in it.
	h2 := newHarness(t)
	h2.noSettle = true
	h2.fs.dir(abs("code/p/a/out")).file(abs("code/p/a/out/x")).file(abs("code/p/a/out/y"))
	rec = h2.wc(
		mkdir(abs("code/p/a/tmp")),
		openW(abs("code/p/a/tmp/x"), abs("code/p/a/tmp/x")),
		rename(abs("code/p/a/tmp"), abs("code/p/a/out")),
		openW(abs("code/p/a/out/y"), abs("code/p/a/out/y")),
	)
	wantReason(t, rec, rUnstable)
	wantComplete(t, "write through a directory replaced by rename", rec, false)
}

// State links and misc placements the review probed.
func TestPOCStateLink(t *testing.T) {
	h := newHarness(t)
	h.noSettle = true
	h.fs.dir("/home/runner/.cargo").dir("/home/runner/.cargo/registry").link("/home/runner/.cargo/registry/L", abs("code/p/b"))
	wantComplete(t, "write via a state-root link", h.wc(mkdir("/home/runner/.cargo/registry/L/x")), false)
	h.fs.dir("/opt").dir("/opt/x").link("/opt/x/L", abs("code/p/b"))
	rec := h.wc(mkdir("/opt/x/L/y"))
	eq(t, "writes via a host link", rec.Writes, "code/p/b/y")
	wantComplete(t, "write via a host link", rec, true)
}

func TestPOCMisc(t *testing.T) {
	h := newHarness(t)
	h.noSettle = true
	tmp := "/tmp/kiln-1"
	wantComplete(t, "tmp link in a renamed dir", h.wc(mkdir(tmp+"/d"), symlink(abs("code/p/b"), tmp+"/d/L"), rename(tmp+"/d", tmp+"/e"), mkdir(tmp+"/e/L/x")), false)
	rec := h.wc(symlink(abs("code/p/b"), tmp+"/L"), mkdir(tmp+"/L/x"))
	eq(t, "tmp link writes", rec.Writes, "code/p/b/x")
	ex := rename(tmp+"/d", tmp+"/e")
	ex.Syscall, ex.Flags = "renameat2", "RENAME_EXCHANGE"
	wantComplete(t, "tmp exchange", h.wc(mkdir(tmp+"/d"), mkdir(tmp+"/d/sub"), mkdir(tmp+"/e"), symlink(abs("code/p/b"), tmp+"/e/sub"), ex, mkdir(tmp+"/d/sub/x")), false)
	wantComplete(t, "stale tmp link across steps",
		h.runSteps(step(symlink(abs("code/p/b"), tmp+"/L")), step(mkdir(tmp+"/L"), mkdir(tmp+"/L/x"))), false)
	// fchmod of a descriptor that is a pipe: nothing in the repository.
	wantComplete(t, "fchmod of an empty path (AT_EMPTY_PATH)", h.wc(write("fchownat", "")), false)
}

// Security-review INFO: every write into a stability map goes through a
// lookupSet method, which is where the write-only copy is kept in step.  A
// direct assignment anywhere else would let the two diverge.
func TestStabilityMapsAreOnlyWrittenByLookupSetMethods(t *testing.T) {
	maps := map[string]bool{"seen": true, "seenAbsent": true, "seenFile": true, "seenFinal": true,
		"relFinal": true, "relAbs": true, "gone": true, "ownSeen": true}
	fset := token.NewFileSet()
	entries, err := os.ReadDir(".")
	if err != nil {
		t.Fatal(err)
	}
	checked := 0
	for _, e := range entries {
		name := e.Name()
		if len(name) < 3 || name[len(name)-3:] != ".go" || (len(name) > 8 && name[len(name)-8:] == "_test.go") {
			continue
		}
		f, err := parser.ParseFile(fset, name, nil, 0)
		if err != nil {
			t.Fatal(err)
		}
		for _, d := range f.Decls {
			fn, ok := d.(*ast.FuncDecl)
			if !ok || fn.Body == nil {
				continue
			}
			inSet := false
			if fn.Recv != nil && len(fn.Recv.List) == 1 {
				if st, ok := fn.Recv.List[0].Type.(*ast.StarExpr); ok {
					if id, ok := st.X.(*ast.Ident); ok && id.Name == "lookupSet" {
						inSet = true
					}
				}
			}
			ast.Inspect(fn.Body, func(n ast.Node) bool {
				var targets []ast.Expr
				switch x := n.(type) {
				case *ast.AssignStmt:
					targets = x.Lhs
				case *ast.CallExpr:
					if id, ok := x.Fun.(*ast.Ident); ok && id.Name == "delete" && len(x.Args) > 0 {
						targets = x.Args[:1]
					}
				}
				for _, l := range targets {
					if ix, ok := l.(*ast.IndexExpr); ok {
						l = ix.X
					}
					if sel, ok := l.(*ast.SelectorExpr); ok && maps[sel.Sel.Name] && !inSet {
						t.Errorf("%s: %s writes the stability map %q outside a lookupSet method", fset.Position(n.Pos()), fn.Name.Name, sel.Sel.Name)
					}
					checked++
				}
				return true
			})
		}
	}
	if checked == 0 {
		t.Fatal("the scan found nothing")
	}
}

// The mutated stability maps of a write call and of every call stay in step:
// a write lookup is in both sets, a read lookup in the first only.
func TestWriteLookupsAreKeptInBothSets(t *testing.T) {
	h := newHarness(t)
	p, err := prepare(h.cfg)
	if err != nil {
		t.Fatal(err)
	}
	c := newClassifier(p)
	c.noteTraversal("/a")
	c.writing(func() {
		c.noteTraversal("/b")
		c.noteFinal("/b")
		c.noteMissing("/c", false)
		c.noteMissing("/d", true)
		c.noteRel("/e", false)
		c.noteRel("/f", true)
		c.noteGone("/g")
		c.created["/h"] = pos{0, 1}
		c.start, c.cur = pos{0, 5}, pos{0, 6}
		c.isCreated("/h/x")
	})
	if _, ok := c.seen["/a"]; !ok {
		t.Error("read lookup missing from the full set")
	}
	if _, ok := c.wr.seen["/a"]; ok {
		t.Error("a read lookup must not be in the write set")
	}
	for name, ok := range map[string][2]bool{
		"seen":       {hasPos(c.seen, "/b"), hasPos(c.wr.seen, "/b")},
		"seenFinal":  {hasPos(c.seenFinal, "/b"), hasPos(c.wr.seenFinal, "/b")},
		"seenAbsent": {hasSpan(c.seenAbsent, "/c"), hasSpan(c.wr.seenAbsent, "/c")},
		"seenFile":   {c.seenFile["/d"], c.wr.seenFile["/d"]},
		"relFinal":   {hasSpan(c.relFinal, "/e"), hasSpan(c.wr.relFinal, "/e")},
		"relAbs":     {hasSpan(c.relAbs, "/f"), hasSpan(c.wr.relAbs, "/f")},
		"gone":       {hasGone(c.gone, "/g"), hasGone(c.wr.gone, "/g")},
		"ownSeen":    {hasSpan(c.ownSeen, "/h"), hasSpan(c.wr.ownSeen, "/h")},
	} {
		if !ok[0] || !ok[1] {
			t.Errorf("%s: in full set %v, in write set %v", name, ok[0], ok[1])
		}
	}
}

func hasPos(m map[string]pos, k string) bool   { _, ok := m[k]; return ok }
func hasSpan(m map[string]span, k string) bool { _, ok := m[k]; return ok }
func hasGone(m map[string]goneLook, k string) bool {
	_, ok := m[k]
	return ok
}

// M4: a test server on an ephemeral port (Go httptest) is the action's own;
// the loopback and abstract namespaces are the step's own, so only the plain
// daemon reason remains.
func TestR2EphemeralPort(t *testing.T) {
	h := newHarness(t)
	rec := h.wc(sockEv("socket:[1]", "SOCK_STREAM"), inet("bind", "127.0.0.1", 0), inet("connect", "127.0.0.1", 41234), mkdir(abs("code/p/a/out")))
	wantReason(t, rec, "daemon:127.0.0.1:41234")
	wantComplete(t, "bind :0, connect :41234", rec, true)
	// A pathname peer inside the repository or the scratch directories is
	// another action's, not a host daemon.
	h.fs.file(abs("code/p/b/live.sock"))
	wantComplete(t, "in-repo pathname peer", h.wc(unix("connect", abs("code/p/b/live.sock"))), true)
	// A host pathname peer that answered is untraced code with write access.
	h.fs.file("/var/run/docker.sock")
	wantComplete(t, "host pathname peer", h.wc(unix("connect", "/var/run/docker.sock")), false)
}

// L5: "the action bound this socket itself" is believed only while the bound
// name is stable: swapping it for a link to a host daemon's socket between
// bind and connect, and back, must not pass.
func TestR2DaemonSpoof(t *testing.T) {
	for _, P := range []string{abs("code/p/a/s"), "/tmp/kiln-1/s"} {
		h := newHarness(t)
		h.noSettle = true
		h.fs.file("/var/run/docker.sock")
		h.fs.info[P] = FileInfo{Kind: KindOther, Ctime: oldCtime}
		rec := h.wc(
			sockEv("socket:[1]", "SOCK_STREAM"),
			unix("bind", P),
			write("unlink", P),
			symlink("/var/run/docker.sock", P),
			unix("connect", P), // the kernel follows the link: dockerd answers
			write("unlink", P),
			unix("bind", P),
		)
		wantComplete(t, "spoofed self-bound peer "+P, rec, false)
	}
	// The honest shape stays complete.
	h := newHarness(t)
	wantComplete(t, "honest own server", h.runNet(unix("bind", "/tmp/kiln-1/s"), unix("connect", "/tmp/kiln-1/s")), true)
}
