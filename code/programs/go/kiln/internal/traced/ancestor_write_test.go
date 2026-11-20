package traced

import (
	"strings"
	"testing"

	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/trace"
)

// KILN04 D39 covers LOOKS at an ancestor, never write-class calls.  With the
// checkout under Home (hosted runners) the sandbox makes the chain
// $HOME/work/x as plain directories of Home's tmpfs and binds the checkout on
// top, so an action could `mv ~/work ~/y` and then list ~/y/x, the checkout
// root, as "its own output" (D40 skips stability for the tmpfs).  A write to an
// ancestor is therefore host-read, as it was before D39.
func realHomeHarness(t *testing.T) *harness {
	t.Helper()
	const root = "/home/runner/work/x/x"
	h := newHarness(t)
	h.cfg.Root, h.fs.root = root, root
	h.fs.dir("/home/runner").dir("/home/runner/work").dir("/home/runner/work/x").dir(root)
	return h
}

func hasHostRead(rec *RawRecord) bool {
	for _, r := range rec.Reasons {
		if strings.HasPrefix(r, "host-read") {
			return true
		}
	}
	return false
}

func TestWriteClassCallsOnAnAncestorAreHostRead(t *testing.T) {
	const work = "/home/runner/work"
	for name, ev := range map[string]trace.Event{
		"rename src":  rename(work, "/home/runner/y"),
		"rename dst":  rename("/home/runner/tmpfile", work),
		"rmdir":       write("rmdir", work),
		"unlink":      write("unlink", work),
		"chmod":       write("chmod", work),
		"utimensat":   write("utimensat", work),
		"rename deep": rename(work+"/x", "/home/runner/y"),
	} {
		h := realHomeHarness(t)
		rec := h.run(ev)
		if !hasHostRead(rec) {
			t.Errorf("%s: reasons %q, want a host-read", name, rec.Reasons)
		}
	}
	// The same ancestor, only looked at, is still allowed (D39 unchanged).
	h := realHomeHarness(t)
	wantNoReasons(t, h.run(stat(work), lstat(work), openAbs(work), open(work, "O_RDONLY|O_DIRECTORY", work)))
}

// A write that FAILED changed nothing: `mkdir -p` calls mkdir on each existing
// prefix and gets EEXIST.  That stays a look; a successful write does not.
func TestFailedWriteOnAnAncestorIsALook(t *testing.T) {
	const work = "/home/runner/work"
	h := realHomeHarness(t)
	wantNoReasons(t, h.run(fail(mkdir(work), "EEXIST"), fail(mkdir("/home/runner"), "EEXIST")))
	// The negative: the same ancestor, successfully renamed, is host-read and
	// the write set is no longer trustworthy.
	h = realHomeHarness(t)
	rec := h.run(rename(work, "/home/runner/y"))
	if !hasHostRead(rec) {
		t.Errorf("a successful rename of an ancestor: reasons %q, want host-read", rec.Reasons)
	}
	wantComplete(t, "successful rename of an ancestor", rec, false)
}

func TestMovingTheCheckoutChainThenListingIsRefused(t *testing.T) {
	h := realHomeHarness(t)
	rec := h.run(
		rename("/home/runner/work", "/home/runner/y"),
		open("/home/runner/y/x/x", "O_RDONLY|O_DIRECTORY", "/home/runner/y/x/x"),
		getdents("/home/runner/y/x/x"),
	)
	if !hasHostRead(rec) {
		t.Fatalf("the moved checkout chain was listed with no host-read; reasons %q entries %q", rec.Reasons, rec.Entries)
	}
	wantComplete(t, "moved checkout chain", rec, false)
}
