package traced

import "testing"

// A link an action creates ITSELF, in the repository, is resolved from the DISK
// at classification time, not from the link text the trace carries (that is used
// only under the fresh $HOME and TMPDIR), and a `mkdir` through it has no -y
// path to be checked against. So the write is placed wherever the link points
// when the classifier looks, and nothing says it moved. This is why the recorder
// classifies each action at its end, before a later action can retarget the link
// (KILN04 4.8, decision P13); the test pins the behaviour P13 works around.
//
// Trace: symlink("../b", a/s1); mkdir(a/s1/new)  (so the write is at b/new).
// Disk at classification: s1 -> "nested", as if a later action retargeted it.
func TestOwnLinkIsResolvedFromTheDiskNotTheTrace(t *testing.T) {
	h := newHarness(t)
	h.noSettle = true
	h.fs.link(abs("code/p/a/s1"), "nested")
	h.fs.dir(abs("code/p/a/nested/new"))
	rec := h.run(symlink("../b", "code/p/a/s1"), mkdir("code/p/a/s1/new"))
	found := false
	for _, w := range rec.Writes {
		if w == "code/p/a/nested/new" {
			found = true
		}
		if w == "code/p/b/new" {
			t.Errorf("the write was placed from the trace's link text: %q", rec.Writes)
		}
	}
	if !found {
		t.Fatalf("the write is not at the disk's target: writes %q reasons %q", rec.Writes, rec.Reasons)
	}
	if !rec.WritesComplete {
		t.Fatalf("the misplacement is silent (complete), which is the problem: reasons %q", rec.Reasons)
	}
}
