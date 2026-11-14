//go:build unix

package record

// Tests of the small helpers `collect` is built from.

import (
	"os"
	"path/filepath"
	"strings"
	"testing"
)

func TestWallMS(t *testing.T) {
	sc := StepSidecar{StartedAt: "2026-11-14T10:00:00Z", EndedAt: "2026-11-14T10:00:01.25Z"}
	if got := wallMS(sc); got != 1250 {
		t.Errorf("%d", got)
	}
	// A clock that went back, or a stamp that does not parse, is 0 (the sidecar
	// validated both; this is only a guard).
	sc.EndedAt = "2026-11-14T09:59:00Z"
	if got := wallMS(sc); got != 0 {
		t.Errorf("%d", got)
	}
	sc.StartedAt = "not a time"
	if got := wallMS(sc); got != 0 {
		t.Errorf("%d", got)
	}
}

func TestReasonTextIsPrintableAndBounded(t *testing.T) {
	if got := reasonText("plain"); got != "plain" {
		t.Errorf("%q", got)
	}
	got := reasonText("a\nb\x1b[0m‮")
	if strings.ContainsAny(got, "\n\x1b‮") {
		t.Errorf("%q", got)
	}
	long := reasonText(strings.Repeat("é", 300))
	if len(long) > maxUnknownText+3 || strings.ContainsRune(long, '�') {
		t.Errorf("%d bytes: %q", len(long), long)
	}
	if unknownText(nil) != "" || unknownText([]string{"a", "b"}) != "a; b" {
		t.Errorf("unknownText")
	}
}

func TestReadBoundedFile(t *testing.T) {
	d := canonTemp(t)
	good := filepath.Join(d, "f")
	os.WriteFile(good, []byte("12345"), 0o600)
	if b, err := readBoundedFile(good, 5); err != nil || string(b) != "12345" {
		t.Fatalf("%q %v", b, err)
	}
	if _, err := readBoundedFile(good, 4); err == nil || !strings.Contains(err.Error(), "over") {
		t.Errorf("over the bound: %v", err)
	}
	if _, err := readBoundedFile(filepath.Join(d, "absent"), 5); err == nil {
		t.Error("absent")
	}
	if _, err := readBoundedFile(d, 5); err == nil {
		t.Error("a directory")
	}
	link := filepath.Join(d, "link")
	if err := os.Symlink(good, link); err == nil {
		if _, err := readBoundedFile(link, 5); err == nil {
			t.Error("a symlink was followed")
		}
	}
}

func TestScanActionOnAnUnreadableDirectory(t *testing.T) {
	if os.Geteuid() == 0 {
		t.Skip("root can read anything")
	}
	d := canonTemp(t)
	sub := filepath.Join(d, "act")
	os.Mkdir(sub, 0o700)
	os.WriteFile(filepath.Join(sub, "0.json"), nil, 0o600)
	os.Chmod(sub, 0)
	t.Cleanup(func() { os.Chmod(sub, 0o700) })
	s := scanAction(sub)
	if !s.exists || len(s.bad) != 1 || !strings.Contains(s.bad[0], "cannot be listed") {
		t.Fatalf("%+v", s)
	}
	// A directory whose parent cannot be searched: lstat fails with something
	// other than "does not exist".
	os.Chmod(d, 0)
	t.Cleanup(func() { os.Chmod(d, 0o700) })
	s = scanAction(sub)
	if !s.exists || len(s.bad) != 1 {
		t.Fatalf("%+v", s)
	}
}

func TestScanActionAbsentIsNotAnError(t *testing.T) {
	s := scanAction(filepath.Join(canonTemp(t), "none"))
	if s.exists || len(s.bad) != 0 || len(s.sidecars) != 0 {
		t.Fatalf("%+v", s)
	}
}

// A sealed file that is not the one SEALED lists is refused by the self-check,
// before the shard is decoded or written.
func TestCollectSelfCheckRefusesAChangedSealedFile(t *testing.T) {
	f := newCfix(t)
	f.good()
	c := &collector{dir: f.p.dir, pre: f.pre}
	f.pre.Sums["host-facts.json"] = strings.Repeat("0", 64)
	if err := c.selfCheck([]byte("x")); err == nil || !strings.Contains(err.Error(), "modified") {
		t.Fatalf("%v", err)
	}
}

func TestCollectSelfCheckRefusesAShardOfAnotherBindingOrSize(t *testing.T) {
	f := newCfix(t)
	f.good()
	f.collect() // a real shard to take lines from
	data, _ := os.ReadFile(f.out)
	c := &collector{dir: f.p.dir, pre: f.pre}
	if err := c.selfCheck(data); err != nil {
		t.Fatalf("a good shard: %v", err)
	}
	// Records for fewer actions than the plan has.
	lines := strings.SplitAfter(string(data), "\n")
	if err := c.selfCheck([]byte(strings.Join(lines[:2], ""))); err == nil || !strings.Contains(err.Error(), "records for 3 actions") {
		t.Fatalf("%v", err)
	}
	// Records of another binding.
	other := strings.ReplaceAll(string(data), f.pre.BindingID, strings.Repeat("ab", 32))
	if err := c.selfCheck([]byte(other)); err == nil {
		t.Fatal("a shard of another binding passed")
	}
}
