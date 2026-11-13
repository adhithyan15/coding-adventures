package traced

import (
	"strings"
	"testing"

	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/trace"
)

func abs(rel string) string { return testRoot + "/" + rel }

func TestMappingInsideAndOutsidePackages(t *testing.T) {
	h := newHarness(t)
	rec := h.run(
		open("code/p/a/src/x.go", "O_RDONLY", abs("code/p/a/src/x.go")),
		open("code/p/a/nested/n.txt", "O_RDONLY", abs("code/p/a/nested/n.txt")), // the deepest package wins
		stat("go.work"),
		stat("code/scripts/gen.py"),
	)
	eq(t, "entries", rec.Entries, "path:code/scripts/gen.py", "path:go.work", "pkg:code/p/a", "pkg:code/p/a/nested")
	eq(t, "untracked", rec.UntrackedObservations)
	wantNoReasons(t, rec)
	if rec.Package != "code/p/a" {
		t.Fatal(rec.Package)
	}
}

func TestAbsence(t *testing.T) {
	h := newHarness(t)
	rec := h.run(
		fail(stat("code/p/a/missing.h"), "ENOENT"),
		fail(stat("code/scripts/nope.py"), "ENOENT"),
		fail(stat("newdir/x/y"), "ENOENT"),
		fail(stat("code/p/a/nested/new/f"), "ENOENT"),
		fail(stat("code/p/a/newpkg/BUILD"), "ENOENT"), // a would-be nested package still reads as pkg:a
		fail(stat("go.work/under-a-file"), "ENOTDIR"),
	)
	// Outside packages an absent path is recorded at its FIRST absent
	// component; inside a package it is the package, so a new tracked file or a
	// new nested package both change T(q).
	eq(t, "entries", rec.Entries,
		"path:code/scripts/nope.py", "path:go.work", "path:newdir", "pkg:code/p/a", "pkg:code/p/a/nested")
	eq(t, "untracked", rec.UntrackedObservations)
	wantNoReasons(t, rec)
}

func TestFailedCallsAreObservationsNotWrites(t *testing.T) {
	h := newHarness(t)
	rec := h.run(
		fail(mkdir("code/p/a/out"), "EEXIST"),               // present, untracked
		fail(write("unlink", "code/scripts/zzz"), "ENOENT"), // absent
		fail(open("code/p/b/x", "O_WRONLY|O_CREAT", ""), "EACCES"),
	)
	eq(t, "writes", rec.Writes)
	eq(t, "tracked", rec.TrackedWrites)
	eq(t, "untracked", rec.UntrackedObservations, "code/p/a/out", "code/p/b/x")
	eq(t, "entries", rec.Entries, "path:code/scripts/zzz", "pkg:code/p/a", "pkg:code/p/b")
	wantNoReasons(t, rec)
}

func TestListing(t *testing.T) {
	h := newHarness(t)
	rec := h.run(
		getdents(abs("code/p")),
		getdents(abs("code/p/a/src")),
		getdents(testRoot),
		getdents(abs("code/p/a/untracked-dir")),
	)
	// A listing is never folded into pkg: it also sees nested package names.
	eq(t, "entries", rec.Entries, "list:.", "list:code/p", "list:code/p/a/src", "list:code/p/a/untracked-dir")
	eq(t, "untracked", rec.UntrackedObservations, "code/p/a/untracked-dir")
	wantNoReasons(t, rec)

	// A directory the action made itself is not an input, listed or not.
	// (Its creation is an absence observation: before the mkdir, nothing was there.)
	rec = h.run(mkdir("code/p/a/out"), getdents(abs("code/p/a/out")))
	eq(t, "entries", rec.Entries, "pkg:code/p/a")
}

func TestGitIsNothingOrVCSRead(t *testing.T) {
	h := newHarness(t)
	rec := h.run(stat(".git"), fail(stat(".git"), "ENOENT"))
	eq(t, "entries", rec.Entries)
	wantNoReasons(t, rec)
	for _, p := range []string{".git/config", ".git/HEAD", "code/x/.git/config"} {
		rec = h.run(open(p, "O_RDONLY", abs(p)))
		wantReason(t, rec, "vcs-read")
		eq(t, "entries", rec.Entries)
	}
	wantReason(t, h.run(getdents(abs(".git"))), "vcs-read")
	// The repository root itself is a directory by definition.
	eq(t, "entries", h.run(stat(".")).Entries)
}

func TestWrittenThenRead(t *testing.T) {
	cases := map[string][]trace.Event{
		"O_EXCL":  {open("code/p/a/out.txt", "O_WRONLY|O_CREAT|O_EXCL", abs("code/p/a/out.txt"))},
		"O_TRUNC": {open("code/p/a/out.txt", "O_WRONLY|O_CREAT|O_TRUNC", abs("code/p/a/out.txt"))},
		"creat":   {write("creat", "code/p/a/out.txt")},
		"mkdir":   {mkdir("code/p/a/out.txt")},
		"probe then create": {
			fail(stat("code/p/a/out.txt"), "ENOENT"),
			open("code/p/a/out.txt", "O_WRONLY|O_CREAT", abs("code/p/a/out.txt")),
		},
		"link onto it":   {link("code/p/b/b.txt", "code/p/a/out.txt")},
		"rename onto it": {rename("code/p/b/b.txt", "code/p/a/out.txt")},
	}
	for name, evs := range cases {
		t.Run(name, func(t *testing.T) {
			h := newHarness(t)
			if name == "creat" {
				evs[0].Flags = "O_CREAT|O_WRONLY|O_TRUNC"
				evs[0].Kind = trace.KindOpen
				evs[0].ReturnFDPath = abs("code/p/a/out.txt")
			}
			evs = append(evs, open("code/p/a/out.txt", "O_RDONLY", abs("code/p/a/out.txt")), stat("code/p/a/out.txt"))
			evs = append(evs, open("code/p/a/out.txt/inner", "O_RDONLY", abs("code/p/a/out.txt/inner")))
			rec := h.run(evs...)
			if strings.HasPrefix(name, "link") || strings.HasPrefix(name, "rename") {
				// The source stays an observation, and so does the absence the
				// creation proves; the later reads of the new name record nothing.
				eq(t, "entries", rec.Entries, "pkg:code/p/a", "pkg:code/p/b")
			} else {
				eq(t, "entries", rec.Entries, "pkg:code/p/a")
			}
			eq(t, "untracked", rec.UntrackedObservations)
		})
	}
}

func TestSymlinkCreatedIsTheActionsOwn(t *testing.T) {
	h := newHarness(t)
	// The link itself is the action's; what it points at is not.
	h.fs.link(abs("code/p/a/mine"), "README")
	rec := h.run(symlink("README", "code/p/a/mine"), lstat("code/p/a/mine"),
		open("code/p/a/mine", "O_RDONLY", abs("code/p/a/README")))
	eq(t, "entries", rec.Entries, "pkg:code/p/a")
	eq(t, "untracked", rec.UntrackedObservations)
	wantNoReasons(t, rec)
}

func TestOpenWithoutProofOfCreationIsAnObservation(t *testing.T) {
	h := newHarness(t)
	rec := h.run(
		// Append, and a plain create, cannot tell a new file from an old one.
		open("code/p/a/log.txt", "O_WRONLY|O_APPEND", abs("code/p/a/log.txt")),
		open("code/p/a/touch.txt", "O_WRONLY|O_CREAT|O_NOCTTY", abs("code/p/a/touch.txt")),
		// A read after an ENOENT probe is not a creation: something appeared.
		fail(stat("code/p/b/late"), "ENOENT"),
		open("code/p/b/late", "O_RDONLY", abs("code/p/b/late")),
	)
	eq(t, "entries", rec.Entries, "pkg:code/p/a", "pkg:code/p/b")
	eq(t, "untracked", rec.UntrackedObservations, "code/p/a/log.txt", "code/p/a/touch.txt", "code/p/b/late")
	// Both writes are still writes.
	eq(t, "writes", rec.Writes, "code/p/a/log.txt", "code/p/a/touch.txt")
}

func TestRenameAndLinkSourceIsAnObservation(t *testing.T) {
	h := newHarness(t)
	// Renaming another package's file into place and reading it adds that package.
	rec := h.run(
		link("code/p/b/b.txt", "code/p/a/copy.txt"),
		open("code/p/a/copy.txt", "O_RDONLY", abs("code/p/a/copy.txt")),
	)
	eq(t, "entries", rec.Entries, "pkg:code/p/a", "pkg:code/p/b")
	// And, being a write to a tracked path, it is refused.
	wantReason(t, rec, "writes-tracked:code/p/b/b.txt")
	eq(t, "tracked", rec.TrackedWrites, "code/p/b/b.txt")

	// A failed rename observes both names.
	rec = h.run(fail(rename("code/p/b/gone", "code/scripts/new"), "ENOENT"))
	eq(t, "entries", rec.Entries, "path:code/scripts/new", "pkg:code/p/b")
	eq(t, "writes", rec.Writes)

	// RENAME_EXCHANGE creates nothing: both names existed.
	ex := rename("code/p/b/b.txt", "code/p/a/README")
	ex.Syscall, ex.Flags = "renameat2", "RENAME_EXCHANGE"
	rec = h.run(ex, open("code/p/a/README", "O_RDONLY", abs("code/p/a/README")))
	eq(t, "entries", rec.Entries, "pkg:code/p/a", "pkg:code/p/b")
}

func TestWritesAreCoarsenedToWriteRoots(t *testing.T) {
	h := newHarness(t)
	rec := h.run(
		mkdir("code/p/a/target"),
		open("code/p/a/target/debug/x.o", "O_WRONLY|O_CREAT|O_TRUNC", abs("code/p/a/target/debug/x.o")),
		open("code/p/a/new.txt", "O_WRONLY|O_CREAT|O_EXCL", abs("code/p/a/new.txt")),
		open("deep/er/out", "O_WRONLY|O_CREAT|O_TRUNC", abs("deep/er/out")),
		open("code/p/a/README", "O_WRONLY|O_TRUNC", abs("code/p/a/README")), // tracked
		write("chmod", "go.work"), // tracked
	)
	eq(t, "writes", rec.Writes, "code/p/a/README", "code/p/a/new.txt", "code/p/a/target", "deep", "go.work")
	eq(t, "tracked", rec.TrackedWrites, "code/p/a/README", "go.work")
	wantReason(t, rec, "writes-tracked:code/p/a/README")
	wantReason(t, rec, "writes-tracked:go.work")
}

func TestUnlinkRmdirAndChangesInPlace(t *testing.T) {
	h := newHarness(t)
	// A call that changes an object in place succeeded, so the object is on the
	// disk afterwards (a name the action never made, and the disk shows absent,
	// is a contradiction: see the "gone" rule in checkStability).
	h.fs.file(abs("code/p/a/t")).file(abs("code/p/a/x")).file(abs("code/p/a/u"))
	rec := h.run(
		write("unlink", "code/p/a/old.o"),
		write("rmdir", "code/p/a/olddir"),
		write("truncate", "code/p/a/t"),
		write("setxattr", "code/p/a/x"),
		write("utimensat", "code/p/a/u"),
		write("mknod", "code/p/a/fifo"),
	)
	eq(t, "writes", rec.Writes, "code/p/a/fifo", "code/p/a/old.o", "code/p/a/olddir", "code/p/a/t", "code/p/a/u", "code/p/a/x")
	eq(t, "entries", rec.Entries, "pkg:code/p/a") // each success observes (or, for mknod, proves absence)
	wantNoReasons(t, rec)
	// A recreated path is the action's own again.
	rec = h.run(mkdir("code/p/a/d"), write("rmdir", "code/p/a/d"), stat("code/p/a/d"), mkdir("code/p/a/d"), stat("code/p/a/d"))
	eq(t, "entries", rec.Entries, "pkg:code/p/a") // only the stat between rmdir and mkdir
}

func TestOutOfRepositoryWritesAreNotRecorded(t *testing.T) {
	h := newHarness(t)
	rec := h.run(
		open("/tmp/kiln-1/x", "O_WRONLY|O_CREAT|O_EXCL", "/tmp/kiln-1/x"),
		open("/tmp/kiln-1/x", "O_RDONLY", "/tmp/kiln-1/x"),
		write("unlink", "/tmp/kiln-1/x"),
	)
	eq(t, "writes", rec.Writes)
	wantNoReasons(t, rec)
}

func TestNestedPackageAppears(t *testing.T) {
	// The tracked tree has no package b/new yet; a package index that does is a
	// DIFFERENT tree.  An absent path under a is pkg:a on one, pkg:a/new on the
	// other: the entry name carries the boundary, and T(q) (digest stage) the
	// rest.
	h := newHarness(t)
	rec := h.run(fail(stat("code/p/a/fresh/BUILD"), "ENOENT"))
	eq(t, "entries", rec.Entries, "pkg:code/p/a")
	h.cfg.Packages = NewPackageIndex([]string{"code/p/a", "code/p/a/fresh"})
	rec = h.run(fail(stat("code/p/a/fresh/BUILD"), "ENOENT"))
	eq(t, "entries", rec.Entries, "pkg:code/p/a/fresh")
}

func TestMultipleStepsShareCreatedPaths(t *testing.T) {
	h := newHarness(t)
	s1 := step(mkdir("code/p/a/out"), open("code/p/a/out/f", "O_WRONLY|O_CREAT|O_EXCL", abs("code/p/a/out/f")))
	s2 := step(open("code/p/a/out/f", "O_RDONLY", abs("code/p/a/out/f")))
	h.fs.dir(abs("code/p/a/out")).file(abs("code/p/a/out/f"))
	rec := h.runSteps(s1, s2)
	eq(t, "entries", rec.Entries, "pkg:code/p/a")
	wantNoReasons(t, rec)
}

func TestStepStatus(t *testing.T) {
	h := newHarness(t)
	bad := step(stat("go.work"))
	bad.Procs[1].Exit.Code = 2
	wantReason(t, h.runSteps(bad), "failed")
	killed := step(stat("go.work"))
	killed.Procs[1].Exit.Killed = true
	wantReason(t, h.runSteps(killed), "failed")
	none := step(stat("go.work"))
	none.Procs[1].Exit = nil
	wantReason(t, h.runSteps(none), "failed")

	// A survivor killed after the init child exited is a daemon.
	d := step(stat("go.work"))
	d.InitPid = 1
	d.Procs[7] = &trace.Process{Pid: 7, Exit: &trace.Exit{Pid: 7, Killed: true, Signal: "SIGKILL"}}
	d.Exits = []trace.Exit{{Pid: 1}, {Pid: 7, Killed: true, Signal: "SIGKILL"}}
	wantReason(t, h.runSteps(d), "daemon-left")
	// Killed BEFORE the init child's exit is an ordinary kill.
	d.Exits = []trace.Exit{{Pid: 7, Killed: true, Signal: "SIGKILL"}, {Pid: 1}}
	if hasReason(h.runSteps(d), "daemon-left") {
		t.Fatal("killed before init exited")
	}
	d.Exits = []trace.Exit{{Pid: 1}, {Pid: 7, Killed: true, Signal: "SIGTERM"}}
	if hasReason(h.runSteps(d), "daemon-left") {
		t.Fatal("SIGTERM is not a survivor")
	}
}

func TestReasonsAreCappedPerKind(t *testing.T) {
	h := newHarness(t)
	var evs []trace.Event
	for i := 0; i < 40; i++ {
		evs = append(evs, openAbs("/nowhere/f"+string(rune('a'+i%26))+string(rune('a'+i/26))))
	}
	evs = append(evs, open("code/p/a/README", "O_WRONLY|O_TRUNC", abs("code/p/a/README")))
	rec := h.run(evs...)
	hostReads, other := 0, 0
	for _, r := range rec.Reasons {
		if strings.HasPrefix(r, "host-read:") {
			hostReads++
		} else {
			other++
		}
	}
	if hostReads != 16 || other != 1 {
		t.Fatalf("%d host-read, %d other: %q", hostReads, other, rec.Reasons)
	}
}

func TestOutputsAreSortedAndUnique(t *testing.T) {
	h := newHarness(t)
	rec := h.run(stat("go.work"), stat("go.work"), stat("code/scripts/gen.py"), stat("code/p/b/b.txt"), stat("code/p/a/README"))
	for _, s := range [][]string{rec.Entries, rec.Writes, rec.Reasons, rec.Traversed} {
		for i := 1; i < len(s); i++ {
			if s[i-1] >= s[i] {
				t.Fatalf("not strictly sorted: %q", s)
			}
		}
	}
}

func TestBadAndSensitivePaths(t *testing.T) {
	h := newHarness(t)
	// A repository path KILN02 cannot encode is refused, not recorded.
	rec := h.run(stat("code/scripts/a\xffb"), write("chmod", "code/scripts/\xff"))
	wantReason(t, rec, "trace-incomplete:bad-path")
	eq(t, "entries", rec.Entries)
	// Reasons never carry a raw newline from a filename.
	rec = h.run(write("chmod", "go.work"), openAbs("/no/where\nelse"))
	for _, r := range rec.Reasons {
		if strings.ContainsAny(r, "\n\x00") {
			t.Fatalf("reason %q", r)
		}
	}
	// Writes under .git are not something a sandboxed BUILD can do.
	wantReason(t, h.run(write("chmod", ".git/config")), "trace-incomplete:vcs-write")
	// A path call with no path that "succeeded" is not understood.
	e := stat("")
	wantReason(t, h.run(e), "trace-incomplete:empty-path")
	// ... but open("") failing ENOENT is just the kernel saying no.
	wantNoReasons(t, h.run(fail(stat(""), "ENOENT")))
}

func TestClassifyRejectsBadInput(t *testing.T) {
	h := newHarness(t)
	good := Input{Package: "code/p/a", Traces: []*trace.Trace{step(stat("go.work"))}, Recipe: []string{"step"}}
	if _, err := Classify(h.cfg, good); err != nil {
		t.Fatal(err)
	}
	for name, in := range map[string]Input{
		"no package": {Traces: good.Traces},
		"no traces":  {Package: "code/p/a", Recipe: []string{}},
		"no recipe":  {Package: "code/p/a", Traces: good.Traces},
		"nil trace":  {Package: "code/p/a", Traces: []*trace.Trace{nil}},
	} {
		if _, err := Classify(h.cfg, in); err == nil {
			t.Errorf("%s: no error", name)
		}
	}
	for name, mutate := range map[string]func(*Config){
		"relative root": func(c *Config) { c.Root = "w/repo" },
		"root slash":    func(c *Config) { c.Root = "/" },
		"unclean root":  func(c *Config) { c.Root = "/w/repo/" },
		"no packages":   func(c *Config) { c.Packages = nil },
		"no tree":       func(c *Config) { c.Tree = nil },
		"root through a link": func(c *Config) {
			c.FS = &linkedRoot{c.FS}
		},
		"no fs":      func(c *Config) { c.FS = nil },
		"bad home":   func(c *Config) { c.Home = "home" },
		"bad tmpdir": func(c *Config) { c.Tmpdir = "/tmp/" },
	} {
		cfg := h.cfg
		mutate(&cfg)
		if _, err := Classify(cfg, good); err == nil {
			t.Errorf("%s: no error", name)
		}
	}
}

// linkedRoot is a disk where the checkout's parent is a symlink.
type linkedRoot struct{ FS }

func (l *linkedRoot) Lstat(p string) (FileInfo, error) {
	if p == "/w" {
		return FileInfo{Kind: KindLink}, nil
	}
	return l.FS.Lstat(p)
}

func TestNoHostRootsMeansEverythingIsAHostRead(t *testing.T) {
	h := newHarness(t)
	h.cfg.HostRoots = nil
	rec := h.run(openAbs("/usr/lib/x"))
	wantReason(t, rec, "host-read:/usr/lib/x")
	wantReason(t, rec, "host-read:/usr/bin/sh") // even the shell
	// With $HOME unset, a root that needs it can never match.
	h = newHarness(t)
	h.cfg.Home = ""
	h.cfg.HostRoots.AbsentOK = []string{"$HOME/.config"}
	h.cfg.HostRoots.Roots = append(h.cfg.HostRoots.Roots, h.cfg.HostRoots.Roots[0])
	h.cfg.HostRoots.Roots[len(h.cfg.HostRoots.Roots)-1].Path = "$TMPDIR/x"
	h.cfg.Tmpdir = ""
	h.cfg.HostRoots.Ephemeral = []string{"$HOME/e"}
	rec = h.run(fail(stat("/home/runner/.config/x"), "ENOENT"))
	wantReason(t, rec, "host-read:/home/runner/.config/x")
}
