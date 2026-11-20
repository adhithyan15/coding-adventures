package traced

import (
	"testing"

	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/trace"
)

// Tests of KILN04 D40, tmpfs containment: while the action has made no
// symbolic link inside the fresh HOME or TMPDIR, the stability rules do not
// apply to paths there.  Each positive case has its negative twin: the same
// sequence with ONE symlink made somewhere under the tmpfs, which must keep
// today's verdicts (unstable-path, writes incomplete).

const (
	tHome = "/home/runner"
	tTmp  = "/tmp/kiln-1"
)

// unlink is a successful unlink of path.
func unlink(path string) trace.Event { return write("unlink", path) }

// gccSequence is what gcc does with $TMPDIR/ccXXXXXX.o: an exclusive create,
// a stat, an unlink, a re-create WITHOUT O_EXCL, and a final unlink.
func gccSequence() []trace.Event {
	o := tTmp + "/ccAbCdEf.o"
	return []trace.Event{
		open(o, "O_WRONLY|O_CREAT|O_EXCL", o),
		stat(o),
		unlink(o),
		open(o, "O_WRONLY|O_CREAT|O_TRUNC", o),
		stat(o),
		unlink(o),
	}
}

// goCacheSequence is what go does with its build cache: an absent probe of
// the entry, a temporary file written then renamed into place, and an open.
func goCacheSequence() []trace.Event {
	dir := tHome + "/.cache/go-build/ab"
	f := dir + "/abcd-a"
	tmp := dir + "/abcd-a123"
	return []trace.Event{
		mkdir(tHome + "/.cache"),
		mkdir(tHome + "/.cache/go-build"),
		mkdir(dir),
		fail(open(f, "O_RDONLY", ""), "ENOENT"),
		open(tmp, "O_WRONLY|O_CREAT|O_EXCL", tmp),
		rename(tmp, f),
		open(f, "O_RDONLY", f),
		stat(f),
		open(f, "O_WRONLY|O_CREAT|O_TRUNC", f),
	}
}

func hasUnstable(rec *RawRecord) bool { return hasReason(rec, rUnstable) }

func TestTmpContainmentGccScratchIsStable(t *testing.T) {
	h := newHarness(t)
	rec := h.wc(gccSequence()...)
	wantNoReasons(t, rec)
	wantComplete(t, "gcc scratch under TMPDIR", rec, true)
	eq(t, "writes", rec.Writes)
}

func TestTmpContainmentGoCacheIsStable(t *testing.T) {
	h := newHarness(t)
	// $HOME has a declared state root beneath it in this harness, so fresh()
	// does not call HOME itself fresh and the walk asks the disk about it, as it
	// does for a real host's HOME.  Only what is INSIDE it is contained.
	h.fs.dir(tHome)
	rec := h.wc(goCacheSequence()...)
	wantNoReasons(t, rec)
	wantComplete(t, "go build cache under HOME", rec, true)
	eq(t, "writes", rec.Writes)
}

// One symlink anywhere under the tmpfs keeps today's verdicts.
func TestTmpContainmentASymlinkKeepsTheOldVerdicts(t *testing.T) {
	for _, link := range []string{tTmp + "/elsewhere", tHome + "/.cache/lnk", tHome + "/lnk"} {
		for name, seq := range map[string][]trace.Event{"gcc": gccSequence(), "gocache": goCacheSequence()} {
			h := newHarness(t)
			h.fs.dir(tHome)
			evs := append([]trace.Event{mkdir(tHome + "/.cache"), symlink("/etc", link)}, seq...)
			rec := h.wc(evs...)
			if !hasUnstable(rec) {
				t.Errorf("%s with a symlink at %s: no unstable-path in %q", name, link, rec.Reasons)
			}
			// Today's write verdicts, unchanged: the gcc sequence only touches
			// its own scratch names with writes that stay stable, the cache
			// sequence's renamed-into-place entry does not.
			wantComplete(t, name+" with a symlink at "+link, rec, name == "gcc")
		}
	}
}

// A symlink in the repository is not a tmp link.
func TestTmpContainmentRepoSymlinkDoesNotSetIt(t *testing.T) {
	h := newHarness(t)
	evs := append([]trace.Event{symlink("src", abs("code/p/a/out-link"))}, gccSequence()...)
	rec := h.wc(evs...)
	if hasUnstable(rec) {
		t.Errorf("a repository symlink disabled containment: %q", rec.Reasons)
	}
	// The classifier-level fact too.
	p, err := prepare(h.cfg)
	if err != nil {
		t.Fatal(err)
	}
	c := newClassifier(p)
	c.noteTmpLink(abs("code/p/a/out-link"))
	if c.tmpLinked {
		t.Error("noteTmpLink set tmpLinked for a path in the repository")
	}
	c.noteTmpLink("/etc/x")
	if c.tmpLinked {
		t.Error("noteTmpLink set tmpLinked for a path outside the tmpfs")
	}
	c.noteTmpLink(tTmp + "/x")
	if !c.tmpLinked {
		t.Error("noteTmpLink ignored a link inside the tmpfs")
	}
}

// A rename that carries a symlink into the tmpfs sets it.
func TestTmpContainmentRenameCarryingALinkSetsIt(t *testing.T) {
	h := newHarness(t)
	evs := append([]trace.Event{
		symlink("src", abs("code/p/a/out-link")),
		rename(abs("code/p/a/out-link"), tTmp+"/moved"),
	}, gccSequence()...)
	rec := h.wc(evs...)
	if !hasUnstable(rec) {
		t.Errorf("a link renamed into the tmpfs did not disable containment: %q", rec.Reasons)
	}

	// A rename of a plain tmp file does not.
	h = newHarness(t)
	rec = h.wc(append([]trace.Event{
		open(tTmp+"/a", "O_WRONLY|O_CREAT|O_EXCL", tTmp+"/a"),
		rename(tTmp+"/a", tTmp+"/b"),
	}, gccSequence()...)...)
	wantNoReasons(t, rec)
	wantComplete(t, "plain rename inside tmpfs", rec, true)
}

// Paths outside the tmpfs keep today's rules: the same re-create/unlink
// sequence under the checkout is still unstable.
func TestTmpContainmentOutsideTheTmpfsKeepsTheRules(t *testing.T) {
	h := newHarness(t)
	o := abs("code/p/a/out/cc.o")
	h.fs.dir(abs("code/p/a/out"))
	rec := h.wc(
		mkdir(abs("code/p/a/out")),
		open(o, "O_WRONLY|O_CREAT|O_EXCL", o),
		stat(o),
		unlink(o),
		open(o, "O_WRONLY|O_CREAT|O_TRUNC", o),
		stat(o),
		unlink(o),
	)
	if !hasUnstable(rec) {
		t.Errorf("a re-created repository file stopped being unstable: %q", rec.Reasons)
	}
}
