package traced

import (
	"testing"

	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/trace"
)

// KILN04 D40, second condition: no DIRECTORY or FOREIGN rename in the tmpfs.
//
// The trace model keeps each process's cwd as TEXT.  A renamed directory leaves
// that text stale while the kernel resolves ".." physically, so a path can be
// redirected without any symbolic link:
//
//	chdir $HOME/a/b; rename $HOME/a/b -> $HOME/b2; mkdir $HOME/a/b
//	unlink ../work/repo/f          (lexically $HOME/a/work/repo/f, the action's
//	                                own scratch file; really $HOME/work/repo/f,
//	                                the checkout bound on a hosted runner)
//
// Before D40 the renamed directory failed the stability rules.  Containment
// skipped exactly those rules, so the rename itself must lift containment.

func staleCwdSeq() []trace.Event {
	h := "/home/runner"
	un := write("unlink", "../work/repo/f")
	un.Cwd = h + "/a/b"
	return []trace.Event{
		mkdir(h + "/a"), mkdir(h + "/a/b"), mkdir(h + "/a/work"), mkdir(h + "/a/work/repo"),
		open(h+"/a/work/repo/f", "O_WRONLY|O_CREAT|O_EXCL", h+"/a/work/repo/f"),
		rename(h+"/a/b", h+"/b2"), mkdir(h + "/a/b"),
		un,
	}
}

func TestStaleCwdAfterADirectoryRenameIsNotContained(t *testing.T) {
	h := newHarness(t)
	h.fs.dir("/home/runner")
	rec := h.wc(staleCwdSeq()...)
	if !hasUnstable(rec) {
		t.Errorf("a renamed directory under the tmpfs left no unstable-path: reasons %q", rec.Reasons)
	}
	wantComplete(t, "stale cwd after a directory rename", rec, false)
}

// The same, in the real-home layout where $HOME/work/repo is the checkout.
func TestStaleCwdReachingTheCheckoutIsRefused(t *testing.T) {
	h := realHomeHarness(t)
	home := "/home/runner"
	un := write("unlink", "../work/x/x/f")
	un.Cwd = home + "/a/b"
	rec := h.wc(
		mkdir(home+"/a"), mkdir(home+"/a/b"), mkdir(home+"/a/work"), mkdir(home+"/a/work/x"), mkdir(home+"/a/work/x/x"),
		open(home+"/a/work/x/x/f", "O_WRONLY|O_CREAT|O_EXCL", home+"/a/work/x/x/f"),
		rename(home+"/a/b", home+"/b2"), mkdir(home+"/a/b"),
		un,
	)
	if len(rec.Reasons) == 0 {
		t.Fatalf("a write that reached the checkout through a stale cwd has no reason; writes %q", rec.Writes)
	}
	wantComplete(t, "stale cwd reaching the checkout", rec, false)
}

// Moving a FILE the action created, into place, stays contained (Go's build
// cache, gcc, clang); moving a directory, a foreign name, or exchanging does not.
func TestTmpMoveTaintConditions(t *testing.T) {
	tmp := tTmp
	cases := []struct {
		name    string
		evs     []trace.Event
		tainted bool
	}{
		{"own file into place", []trace.Event{
			open(tmp+"/a", "O_WRONLY|O_CREAT|O_EXCL", tmp+"/a"), rename(tmp+"/a", tmp+"/b")}, false},
		{"own directory", []trace.Event{mkdir(tmp + "/d"), rename(tmp+"/d", tmp+"/e")}, true},
		{"foreign name", []trace.Event{rename(tmp+"/preloaded", tmp+"/e")}, true},
		{"exchange of own files", []trace.Event{
			open(tmp+"/a", "O_WRONLY|O_CREAT|O_EXCL", tmp+"/a"),
			open(tmp+"/b", "O_WRONLY|O_CREAT|O_EXCL", tmp+"/b"),
			exchange(tmp+"/a", tmp+"/b")}, false},
		{"exchange with a foreign name", []trace.Event{
			open(tmp+"/a", "O_WRONLY|O_CREAT|O_EXCL", tmp+"/a"), exchange(tmp+"/a", tmp+"/preloaded")}, true},
		{"repository rename", []trace.Event{mkdir(abs("code/p/a/d")), rename(abs("code/p/a/d"), abs("code/p/a/e"))}, false},
	}
	for _, tc := range cases {
		h := newHarness(t)
		p, err := prepare(h.cfg)
		if err != nil {
			t.Fatal(err)
		}
		c := newClassifier(p)
		c.runTrace(step(tc.evs...))
		if c.tmpMoved != tc.tainted {
			t.Errorf("%s: tmpMoved = %v, want %v", tc.name, c.tmpMoved, tc.tainted)
		}
	}
}
