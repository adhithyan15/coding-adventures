package traced

import "testing"

// KILN04 D40: the recorder's derived state roots (~/.cargo/git here; the
// harness declares ~/.cargo/registry itself) are writable HOST binds, not fresh
// tmpfs, so containment must not skip the stability rules for them.
func TestHostBindsAreNotFreshTmpfs(t *testing.T) {
	h := newHarness(t)
	c := newClassifier(mustPrepare(t, h.cfg))
	if !c.underTmp("/home/runner/.cargo/git/x") {
		t.Fatal("without HostBinds the path is not taken for fresh tmpfs: the test has no teeth")
	}
	h.cfg.HostBinds = []string{"/home/runner/.cargo/git"}
	c = newClassifier(mustPrepare(t, h.cfg))
	for _, q := range []string{"/home/runner/.cargo/git", "/home/runner/.cargo/git/x", "/home/runner/.cargo"} {
		if c.underTmp(q) || c.contained(q) {
			t.Errorf("%s is taken for fresh, contained tmpfs", q)
		}
	}
	if !c.contained("/home/runner/.cache/x") {
		t.Error("an ordinary path under Home stopped being contained")
	}
	// An absent probe is still a constant (the root starts empty); a present
	// look is host state.
	wantNoReasons(t, h.run(fail(stat("/home/runner/.cargo/git/absent"), "ENOENT")))
	wantReason(t, h.run(openAbs("/home/runner/.cargo/git/db")), "host-read:$HOME/.cargo/git/db")
}
