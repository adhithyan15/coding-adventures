package traced

import (
	"debug/elf"
	"encoding/binary"
	"io/fs"
	"strings"
	"testing"
	"time"
	"unicode"

	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/trace"
)

// Regression tests for the adversarial soundness review of the first
// classifier.  Each one reproduces a review scenario.

// watchFS fails the test if the classifier asks the disk about anything under
// the scrubbed HOME or TMPDIR.
type watchFS struct {
	FS
	t *testing.T
}

func (w watchFS) check(p string) {
	for _, d := range []string{"/home/runner", "/tmp/kiln-1"} {
		if strings.HasPrefix(p, d+"/") {
			w.t.Errorf("the disk was consulted at %s", p)
		}
	}
}

func (w watchFS) Lstat(p string) (FileInfo, error)  { w.check(p); return w.FS.Lstat(p) }
func (w watchFS) Readlink(p string) (string, error) { w.check(p); return w.FS.Readlink(p) }
func (w watchFS) ReadHead(p string, n int) ([]byte, error) {
	w.check(p)
	return w.FS.ReadHead(p, n)
}

func TestTmpAndHomeAreNeverReadFromTheDisk(t *testing.T) {
	h := newHarness(t)
	// A stale link the post-run disk would still show under TMPDIR.
	h.fs.link("/tmp/kiln-1/ghost", abs("code/p/b"))
	h.cfg.FS = watchFS{h.fs, t}
	rec := h.run(
		stat("/tmp/kiln-1"),
		getdents("/tmp/kiln-1"),
		stat("/tmp/kiln-1/ghost/b.txt"),
		mkdir("/tmp/kiln-1/d"),
		symlink(abs("code/p/b"), "/tmp/kiln-1/d/l"),
		stat("/tmp/kiln-1/d/l/b.txt"),
		stat("/home/runner"),
	)
	// The link the action made resolves from the trace; the stale one is absent.
	// (The post-run disk lacks d/ here, so the created component is unstable;
	// the point is that the disk was not asked.)
	eq(t, "entries", rec.Entries, "pkg:code/p/b")
	// The stale link is not there (a read of it is a host-read); the directories
	// themselves may be stat'ed and listed.
	wantReason(t, rec, "host-read:$TMPDIR/ghost/b.txt")
	// An exec of a file in TMPDIR cannot be read after the run.
	wantReason(t, h.run(open("/tmp/kiln-1/x", "O_WRONLY|O_CREAT|O_EXCL", "/tmp/kiln-1/x"), execve("/tmp/kiln-1/x", "x")),
		"trace-incomplete:exec-unreadable")
}

func TestStabilityAncestorsAndOutsideTheRepository(t *testing.T) {
	// A tracked link is unstable when ANY ancestor was renamed (F9).
	h := newHarness(t)
	rec := h.run(stat("code/p/a/lnk/../README"), rename("code/p", "code/q"))
	wantReason(t, rec, "trace-incomplete:unstable-path")
	// Outside the repository the disk is trusted unless a traced change touched
	// the component or an ancestor.
	h = newHarness(t)
	h.image("/etc/target")
	h.fs.link("/etc/ln", "/etc/target")
	wantNoReasons(t, h.run(stat("/etc/ln")))
	rec = h.run(lstat("/etc/ln"), write("unlink", "/etc/ln"))
	if hasReason(rec, "trace-incomplete:unstable-path") {
		t.Fatal("a final link that is not traversed is not a component")
	}
	rec = h.run(stat("/etc/ln/x"), write("unlink", "/etc/ln"))
	wantReason(t, rec, "trace-incomplete:unstable-path")
}

func TestDotDotAfterAMissingComponent(t *testing.T) {
	h := newHarness(t)
	// Failed call: what the kernel looked at last is the missing component.
	rec := h.run(fail(stat("code/p/a/missing/../README"), "ENOENT"), fail(stat("nodir/../x"), "ENOENT"))
	eq(t, "entries", rec.Entries, "path:nodir", "pkg:code/p/a")
	wantNoReasons(t, rec)
	eq(t, "absent", rec.AbsentObservations, "code/p/a/missing", "nodir")
	// Successful call: it went somewhere the walk cannot know.
	rec = h.run(stat("code/p/a/missing/../README"))
	wantReason(t, rec, "trace-incomplete:unstable-path")
	// Past a file the same holds (ENOTDIR on failure).
	rec = h.run(fail(stat("go.work/../x"), "ENOTDIR"))
	eq(t, "entries", rec.Entries, "path:go.work")
	// Not for /proc, which is lexical.
	h.image("/etc/x")
	wantNoReasons(t, h.run(stat("/proc/../etc/x")))
}

func TestAbsentIsOnlyRememberedFromAnUnambiguousENOENT(t *testing.T) {
	h := newHarness(t)
	reopen := func(path string) trace.Event {
		return open(path, "O_WRONLY|O_CREAT", abs(path))
	}
	// A trailing slash or "/." does not ask about the name itself.
	for _, p := range []string{"code/p/b/new/", "code/p/b/new/."} {
		rec := h.run(fail(stat(p), "ENOENT"), reopen("code/p/b/new"))
		eq(t, "untracked "+p, rec.UntrackedObservations, "code/p/b/new")
	}
	// A missing parent is not a missing name.
	rec := h.run(fail(mkdir("code/p/b/no/dir"), "ENOENT"), reopen("code/p/b/no/dir"))
	eq(t, "untracked", rec.UntrackedObservations, "code/p/b/no/dir")
	// Neither is a failed exec, rename or link.
	for name, e := range map[string]trace.Event{
		"exec":   fail(execve("code/p/b/x", "x"), "ENOENT"),
		"rename": fail(rename("code/p/b/x", "code/p/b/y"), "ENOENT"),
		"link":   fail(link("code/p/b/x", "code/p/b/y"), "ENOENT"),
	} {
		rec := h.run(e, reopen("code/p/b/x"), reopen("code/p/b/y"))
		if len(rec.UntrackedObservations) != 2 {
			t.Errorf("%s: %q", name, rec.UntrackedObservations)
		}
	}
	// A plain ENOENT on a final name inside the repository still proves it.
	rec = h.run(fail(stat("code/p/b/ok"), "ENOENT"), reopen("code/p/b/ok"))
	eq(t, "untracked", rec.UntrackedObservations)
	// Outside the repository the parent is not trusted: an ENOENT probe under
	// an image root does not make a later O_CREAT "provably from empty".
	rec = h.run(fail(stat("/etc/zz"), "ENOENT"), open("/etc/zz", "O_WRONLY|O_CREAT", "/etc/zz"))
	wantReason(t, rec, "host-read:/etc/zz")
}

func TestCreationIsAnAbsenceObservation(t *testing.T) {
	h := newHarness(t)
	rec := h.run(mkdir("newdir"), mkdir("newdir/inner"), symlink("x", "code/scripts/lnk"),
		open("code/p/b/out", "O_WRONLY|O_CREAT|O_EXCL", abs("code/p/b/out")), write("mknod", "code/p/a/fifo"))
	// One entry per creation with no own ancestor; inner is under the action's own dir.
	eq(t, "entries", rec.Entries, "path:code/scripts/lnk", "path:newdir", "pkg:code/p/a", "pkg:code/p/b")
	eq(t, "absent", rec.AbsentObservations, "code/p/a/fifo", "code/p/b/out", "code/scripts/lnk", "newdir")
	// The destination of rename and link too.
	rec = h.run(rename("code/p/b/b.txt", "elsewhere"), link("code/p/b/script.sh", "code/p/a/sh2"))
	eq(t, "entries", rec.Entries, "path:elsewhere", "pkg:code/p/a", "pkg:code/p/b")
	// bind of a Unix socket creates its path.
	rec = h.runNet(unix("bind", "sock.s"))
	eq(t, "entries", rec.Entries, "path:sock.s")
}

func TestWritesThroughDescriptorPathsAreRefused(t *testing.T) {
	h := newHarness(t)
	for _, sys := range []string{"chmod", "truncate", "unlink", "mkdir", "setxattr"} {
		wantReason(t, h.run(write(sys, "/dev/fd/3")), "trace-incomplete:proc-path")
		wantReason(t, h.run(write(sys, "/proc/self/fd/3")), "trace-incomplete:proc-path")
	}
	wantReason(t, h.run(link("/dev/fd/3", "code/p/a/x")), "trace-incomplete:proc-path")
	wantReason(t, h.run(link("code/p/b/b.txt", "/proc/self/fd/3")), "trace-incomplete:proc-path")
	wantReason(t, h.run(rename("/dev/fd/3", "code/p/a/x")), "trace-incomplete:proc-path")
	// A descriptor-based metadata write names its object by -y path.
	rec := h.run(write("fchmod", abs("code/p/b/b.txt")), write("futimens", abs("code/p/a/untracked")))
	eq(t, "tracked", rec.TrackedWrites, "code/p/b/b.txt")
	eq(t, "untracked", rec.UntrackedObservations, "code/p/a/untracked")
}

func TestExecOfUntrackedFileThatIsRewritten(t *testing.T) {
	setup := func() *harness {
		h := newHarness(t)
		h.fs.bytes(abs("code/p/a/tool"), buildELFStatic())
		h.fs.dir(abs("code/p/a/bin"))
		h.fs.bytes(abs("code/p/a/bin/tool"), buildELFStatic())
		return h
	}
	h := setup()
	// Left alone, an untracked program is read (it is an untracked observation).
	rec := h.run(execve("code/p/a/tool", "t"))
	wantNoReasons(t, rec)
	eq(t, "untracked", rec.UntrackedObservations, "code/p/a/tool")
	// Each later change to it makes the post-run bytes untrustworthy.
	for name, e := range map[string]trace.Event{
		"write-open": open("code/p/a/tool", "O_WRONLY|O_TRUNC", abs("code/p/a/tool")),
		"truncate":   write("truncate", "code/p/a/tool"),
		"rename":     rename("code/p/a/other", "code/p/a/tool"),
		"link":       link("code/p/b/b.txt", "code/p/a/tool"),
		"unlink":     write("unlink", "code/p/a/tool"),
	} {
		rec := setup().run(execve("code/p/a/tool", "t"), e)
		if !hasReason(rec, "trace-incomplete:exec-unreadable") {
			t.Errorf("%s: %q", name, rec.Reasons)
		}
	}
	// A namespace change to an ancestor counts as well.
	rec = setup().run(execve("code/p/a/bin/tool", "t"), rename("code/p/a/bin", "code/p/a/bin2"))
	wantReason(t, rec, "trace-incomplete:exec-unreadable")
	// A write BEFORE the exec (the compiler's output) is fine.
	rec = setup().run(open("code/p/a/tool", "O_WRONLY|O_CREAT|O_TRUNC", abs("code/p/a/tool")), execve("code/p/a/tool", "t"))
	if hasReason(rec, "trace-incomplete:exec-unreadable") {
		t.Fatalf("%q", rec.Reasons)
	}
	// A tracked program is read from the tree: later changes are writes-tracked.
	h = newHarness(t)
	h.fs.bytes("/bin/sh", buildELFStatic())
	h.image("/bin/sh")
	rec = h.run(execve("code/p/b/script.sh", "s"), write("chmod", "code/p/b/script.sh"))
	if hasReason(rec, "trace-incomplete:exec-unreadable") {
		t.Fatal("tracked programs are read from the tree")
	}
}

func TestSuccessfulChangesAreObservationsOfWhatWasThere(t *testing.T) {
	h := newHarness(t)
	rec := h.run(
		write("unlink", "code/p/a/leftover"),
		write("rmdir", "code/p/a/olddir"),
		write("chmod", "code/p/a/c"),
		write("truncate", "code/p/a/t"),
		write("utimes", "code/p/a/u"),
		write("setxattr", "code/p/a/x"),
	)
	eq(t, "untracked", rec.UntrackedObservations,
		"code/p/a/c", "code/p/a/leftover", "code/p/a/olddir", "code/p/a/t", "code/p/a/u", "code/p/a/x")
	// Not for what the action made itself.
	rec = h.run(open("code/p/a/mine", "O_WRONLY|O_CREAT|O_EXCL", abs("code/p/a/mine")),
		write("chmod", "code/p/a/mine"), write("unlink", "code/p/a/mine"))
	eq(t, "untracked", rec.UntrackedObservations)
}

func TestOpenat2ResolveFlags(t *testing.T) {
	h := newHarness(t)
	ok := open("code/p/b/b.txt", "{flags=O_RDONLY, resolve=0}", abs("code/p/b/b.txt"))
	ok.Syscall = "openat2"
	wantNoReasons(t, h.run(ok))
	for _, f := range []string{"{flags=O_RDONLY, resolve=RESOLVE_NO_SYMLINKS}", "{flags=O_RDONLY, resolve=RESOLVE_BENEATH|RESOLVE_IN_ROOT}"} {
		e := open("code/p/b/b.txt", f, abs("code/p/b/b.txt"))
		e.Syscall = "openat2"
		wantReason(t, h.run(e), "trace-incomplete:resolve-flags")
	}
	// A plain openat with the same text is not openat2.
	e := open("code/p/b/b.txt", "O_RDONLY", abs("code/p/b/b.txt"))
	wantNoReasons(t, h.run(e))
}

func TestWalkBudgetAndPathLength(t *testing.T) {
	h := newHarness(t)
	long := strings.Repeat("a/", 5000) + "x"
	wantReason(t, h.run(fail(stat(long), "ENOENT")), "trace-incomplete:size")
	old := walkBudget
	walkBudget = 6
	defer func() { walkBudget = old }()
	wantReason(t, h.run(stat("code/p/a/src/x.go")), "trace-incomplete:size")
	walkBudget = old
	wantNoReasons(t, h.run(stat("code/p/a/src/x.go")))
}

func TestImageAbsentProbeChecksTheNearestDirectory(t *testing.T) {
	h := newHarness(t)
	// A setup step added a file to /usr/lib after boot: its ctime moved, and
	// an absent probe there is no longer covered by the image.
	h.fs.info["/usr/lib"] = FileInfo{Kind: KindDir, Ctime: jobStart.Add(time.Minute)}
	rec := h.run(fail(stat("/usr/lib/absent"), "ENOENT"))
	eq(t, "reasons", rec.Reasons, "host-read:setup-modified:/usr/lib/absent")
	// The deepest EXISTING ancestor is the one that counts.
	rec = h.run(fail(stat("/usr/lib/a/b/c"), "ENOENT"))
	eq(t, "reasons", rec.Reasons, "host-read:setup-modified:/usr/lib/a/b/c")
	// An unmodified one is fine; an unknown ctime is not.
	h = newHarness(t)
	wantNoReasons(t, h.run(fail(stat("/usr/lib/absent"), "ENOENT")))
	h.fs.info["/usr/lib"] = FileInfo{Kind: KindDir}
	wantReason(t, h.run(fail(stat("/usr/lib/absent"), "ENOENT")), "host-read:/usr/lib/absent")
	// A root that does not exist at all is a host-read.
	h = newHarness(t)
	delete(h.fs.info, "/usr")
	delete(h.fs.info, "/usr/bin")
	delete(h.fs.info, "/usr/lib")
	wantReason(t, h.run(fail(stat("/usr/lib/absent"), "ENOENT")), "host-read:/usr/lib/absent")
	// A disk error on the way.
	h = newHarness(t)
	h.fs.errs["/usr/lib"] = fs.ErrPermission
	wantReason(t, h.run(fail(stat("/usr/lib/absent"), "ENOENT")), "host-read:/usr/lib/absent")
}

func TestRootMustBeRealDirectories(t *testing.T) {
	h := newHarness(t)
	good := Input{Package: "code/p/a", Traces: []*trace.Trace{step(stat("go.work"))}, Recipe: []string{"step"}}
	for _, p := range []string{"/", "/w", testRoot} {
		cfg := h.cfg
		cfg.FS = &kindFS{h.fs, p, KindLink}
		if _, err := Classify(cfg, good); err == nil {
			t.Errorf("%s as a link: no error", p)
		}
		cfg.FS = &kindFS{h.fs, p, KindFile}
		if _, err := Classify(cfg, good); err == nil {
			t.Errorf("%s as a file: no error", p)
		}
		cfg.FS = &kindFS{h.fs, p, KindAbsent}
		if _, err := Classify(cfg, good); err == nil {
			t.Errorf("%s missing: no error", p)
		}
	}
}

type kindFS struct {
	FS
	path string
	kind NodeKind
}

func (k *kindFS) Lstat(p string) (FileInfo, error) {
	if p == k.path {
		if k.kind == KindAbsent {
			return FileInfo{}, fs.ErrNotExist
		}
		return FileInfo{Kind: k.kind}, nil
	}
	return k.FS.Lstat(p)
}

func TestSanitizePrintableAndBounded(t *testing.T) {
	for _, in := range []string{"a\x1b[31mred", "a‮b", "a​b", "tab\there", "a\u0085b"} {
		out := sanitize(in)
		for _, r := range out {
			if r != ' ' && !isPrint(r) {
				t.Errorf("%q -> %q still has %U", in, out, r)
			}
		}
	}
	if got := sanitize("é" + strings.Repeat("é", 300)); len(got) > maxReasonText+3 {
		t.Errorf("%d bytes", len(got))
	}
	if got := sanitize(strings.Repeat("\U0001F600", 100)); len(got) > maxReasonText+3 || !strings.HasSuffix(got, "...") {
		t.Errorf("%q", got)
	}
	if sanitize("plain/path") != "plain/path" {
		t.Fatal("printable text changed")
	}
}

func TestShebangCarriageReturnIsNotTrimmed(t *testing.T) {
	// The kernel looks up "/bin/sh\r", which does not exist; so does the
	// classifier, and it fails closed instead of reading /bin/sh.
	h := newHarness(t)
	h.tree.file("code/p/b/crlf", "#!/bin/sh\r\n")
	h.fs.bytes("/bin/sh", buildELFStatic())
	h.image("/bin/sh")
	rec := h.run(execve("code/p/b/crlf", "c"))
	wantReason(t, rec, "trace-incomplete:exec-unreadable")
}

func TestNetworkStreamServerIsNotADatagramPeer(t *testing.T) {
	h := newHarness(t)
	b := unix("bind", "/tmp/kiln-1/s")
	c := unix("connect", "/tmp/kiln-1/s")
	c.SockPath = "socket:[9]"
	rec := h.run(sockEv("socket:[1]", "SOCK_STREAM"), sockEv("socket:[9]", "SOCK_DGRAM"), b, c)
	eq(t, "reasons", noUnknown(rec.Reasons), "daemon:$TMPDIR/s")
}

// buildELFStatic is an ELF head with no PT_INTERP.
func buildELFStatic() []byte {
	b := buildELF(loader, elf.ELFCLASS64, binary.LittleEndian)
	binary.LittleEndian.PutUint32(b[64+56:], uint32(elf.PT_NOTE))
	return b
}

func isPrint(r rune) bool { return unicode.IsPrint(r) }
