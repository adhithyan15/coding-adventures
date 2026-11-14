//go:build unix

package record

// Tests of `kiln record collect`, part 3: the checkout after the build (KILN04
// section 4.8 and 7.1): VerifyTree with the pinned commit, `git fsck`, Inventory,
// and the two header counts the merge turns into a run-void (M2b and M3).

import (
	"errors"
	"fmt"
	"os"
	"path/filepath"
	"strings"
	"testing"

	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/recfmt"
	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/source"
)

// collectWith runs Collect with the options changed by mod.
func (f *cfix) collectWith(mod func(*CollectOptions)) (*CollectResult, *recfmt.Shard) {
	f.t.Helper()
	o := f.opts()
	mod(&o)
	res, err := Collect(o)
	if err != nil {
		f.t.Fatalf("Collect: %v", err)
	}
	return res, f.decode()
}

// writesOf rewrites a's record.json with the raw record's lists changed.
func (f *cfix) writesOf(tracked, writes []string) {
	f.t.Helper()
	os.Remove(RecordFile(f.p.dir, ca))
	f.verdict(ca, 1, func(c *ClassifiedRecord) {
		c.Record.TrackedWrites, c.Record.Writes = sortedCopy(tracked), sortedCopy(writes)
	})
}

func (f *cfix) edit(rel, content string) {
	f.t.Helper()
	full := filepath.Join(f.p.root, filepath.FromSlash(rel))
	os.MkdirAll(filepath.Dir(full), 0o755)
	if err := os.WriteFile(full, []byte(content), 0o644); err != nil {
		f.t.Fatal(err)
	}
}

func requireVoid(t *testing.T, h recfmt.ShardHeader, tracked, inventory int, void ...string) {
	t.Helper()
	if h.TrackedChangedUnexplained != tracked || h.PostInventoryUnexplained != inventory || strings.Join(h.Void, ",") != strings.Join(void, ",") {
		t.Fatalf("header %+v, want tracked %d inventory %d void %v", h, tracked, inventory, void)
	}
}

// A tracked file changed by something the traces do not name is a write nobody
// saw (M-2): it counts, and the shard is void. One the traces DO name is only
// the shard-level void.
func TestCollectPostBuildTreeErrors(t *testing.T) {
	const src = "code/packages/x/a/src.txt"
	for _, c := range []struct {
		name    string
		tracked []string
		want    int
	}{
		{"unexplained", nil, 1},
		{"named by a tracked write", []string{src}, 0},
		{"named by an ancestor directory", []string{"code/packages/x/a"}, 0},
		{"a sibling is not an ancestor", []string{"code/packages/x/a/src"}, 1},
		{"a name that merely starts the same", []string{"code/packages/x/a/src.tx"}, 1},
		{"a descendant is not an ancestor", []string{src + "/deeper"}, 1},
	} {
		t.Run(c.name, func(t *testing.T) {
			f := newCfix(t)
			f.good()
			f.writesOf(c.tracked, []string{ca + "/out"})
			f.edit(src, "changed by something nobody traced\n")
			_, sh := f.collect()
			requireVoid(t, sh.Header, c.want, 0, "tracked-changed")
			// And the merge voids the run exactly when the count is not zero.
			if m := mergeOf(f, sh); c.want > 0 && m.RunVoidCount == 0 {
				t.Fatalf("run-void %v for a count of %d", m.RunVoid, c.want)
			}
		})
	}

	t.Run("more changed files than an error names", func(t *testing.T) {
		files := map[string]string{}
		for i := 0; i < 40; i++ {
			files[fmt.Sprintf("code/packages/x/a/many/f%02d", i)] = "x\n"
		}
		f := newCfixWith(t, files, nil)
		f.good()
		for i := 0; i < 40; i++ {
			f.edit(fmt.Sprintf("code/packages/x/a/many/f%02d", i), "y\n")
		}
		// 37 of them are named; three are not.
		var named []string
		for i := 3; i < 40; i++ {
			named = append(named, fmt.Sprintf("code/packages/x/a/many/f%02d", i))
		}
		f.writesOf(named, []string{ca + "/out"})
		_, sh := f.collect()
		requireVoid(t, sh.Header, 3, 0, "tracked-changed")
	})

	t.Run("a deleted tracked file", func(t *testing.T) {
		f := newCfix(t)
		f.good()
		os.Remove(filepath.Join(f.p.root, "code/packages/x/a/src.txt"))
		_, sh := f.collect()
		requireVoid(t, sh.Header, 1, 0, "tracked-changed")
	})

	// HEAD moved (a BUILD ran `git commit`): Git itself reports the pinned commit
	// is gone; the paths cannot be listed, so none can be shown explained.
	t.Run("moved head", func(t *testing.T) {
		f := newCfix(t)
		f.good()
		runGit(t, f.p.root, "commit", "-q", "--allow-empty", "-m", "a BUILD did this")
		_, sh := f.collect()
		requireVoid(t, sh.Header, 1, 0, "tracked-changed")
	})
}

// The seams stand for each way the checks can fail, without a repository to
// break.
func TestCollectPostBuildSeams(t *testing.T) {
	boom := errors.New("git: exit status 128")
	for _, c := range []struct {
		name      string
		mod       func(*CollectOptions, *cfix)
		tracked   int
		inventory int
	}{
		{"a plain git error from VerifyTree", func(o *CollectOptions, f *cfix) {
			o.VerifyTree = func(*source.Git, string, string) (Result, error) { return Result{}, boom }
		}, 1, 0},
		{"a TreeError with no path list", func(o *CollectOptions, f *cfix) {
			o.VerifyTree = func(*source.Git, string, string) (Result, error) {
				return Result{}, &TreeError{What: "working tree is not HEAD", Total: 3}
			}
		}, 3, 0},
		{"a TreeError whose problem is not a path", func(o *CollectOptions, f *cfix) {
			o.VerifyTree = func(*source.Git, string, string) (Result, error) {
				return Result{}, &TreeError{What: "working tree is not HEAD", Total: 1, Paths: []string{"."}}
			}
		}, 1, 0},
		{"a TreeError with a zero total", func(o *CollectOptions, f *cfix) {
			o.VerifyTree = func(*source.Git, string, string) (Result, error) { return Result{}, &TreeError{} }
		}, 1, 0},
		{"a VerifyTree that vouches for another commit", func(o *CollectOptions, f *cfix) {
			o.VerifyTree = func(*source.Git, string, string) (Result, error) {
				return Result{Commit: strings.Repeat("0", 40)}, nil
			}
		}, 1, 0},
		{"a failing fsck", func(o *CollectOptions, f *cfix) {
			o.Fsck = func(*source.Git) error { return boom }
		}, 1, 0},
		{"an inventory that cannot be taken", func(o *CollectOptions, f *cfix) {
			o.Inventory = func(*source.Git, string) (recfmt.Inventory, error) { return recfmt.Inventory{}, boom }
		}, 0, 1},
	} {
		t.Run(c.name, func(t *testing.T) {
			f := newCfix(t)
			f.good()
			res, sh := f.collectWith(func(o *CollectOptions) { c.mod(o, f) })
			// Not a crash: every record was written, and the failure is the void and the
			// count (a fact the shard cannot say "nothing changed" about).
			if res.Records != 3 {
				t.Fatalf("%d records", res.Records)
			}
			requireVoid(t, sh.Header, c.tracked, c.inventory, "tracked-changed")
			for _, a := range []string{ca, cb, cz} {
				requireKnown(t, sh, a)
			}
		})
	}
	t.Run("the checks are handed the pinned commit", func(t *testing.T) {
		f := newCfix(t)
		f.good()
		var got string
		f.collectWith(func(o *CollectOptions) {
			o.VerifyTree = func(g *source.Git, root, want string) (Result, error) {
				got = want
				return VerifyTreeWith(g, root, want)
			}
		})
		if got != f.p.sha || got != f.pre.Binding.Revision {
			t.Fatalf("expectedCommit %q, want the binding's revision %q", got, f.p.sha)
		}
	})
}

func TestCollectFsckFailure(t *testing.T) {
	f := newCfix(t)
	f.good()
	// Remove a blob the tree names. VerifyTree compares the disk with the ids and
	// does not need it; only a connectivity check notices.
	id := runGit(t, f.p.root, "rev-parse", "HEAD:code/packages/x/a/src.txt")
	obj := filepath.Join(f.p.root, ".git", "objects", id[:2], id[2:])
	os.Chmod(filepath.Dir(obj), 0o755)
	if err := os.Remove(obj); err != nil {
		t.Skipf("the object is not loose: %v", err)
	}
	_, sh := f.collect()
	requireVoid(t, sh.Header, 1, 0, "tracked-changed")
}

// Fsck is `git fsck --connectivity-only`, run by path with exactly the allowlist
// environment and nothing of the process's.
func TestCollectFsckRunsPinned(t *testing.T) {
	d := t.TempDir()
	args, env := filepath.Join(d, "args"), filepath.Join(d, "env")
	bin := filepath.Join(d, "git")
	os.WriteFile(bin, []byte("#!/bin/sh\nprintf '%s\\n' \"$@\" > '"+args+"'\nenv | sort > '"+env+"'\n"), 0o755)
	t.Setenv("LD_PRELOAD", "/tmp/evil.so")
	t.Setenv("GIT_DIR", "/tmp/elsewhere")
	g := source.NewGitAt(d, bin, []string{"HOME=/h", "PATH=/usr/bin:/bin"})
	if err := Fsck(g); err != nil {
		t.Fatal(err)
	}
	a, _ := os.ReadFile(args)
	if !strings.Contains(string(a), "fsck\n--connectivity-only\n--no-dangling\n--no-progress") || !strings.Contains(string(a), "-C\n"+d) {
		t.Fatalf("arguments:\n%s", a)
	}
	e, _ := os.ReadFile(env)
	for _, bad := range []string{"LD_PRELOAD", "GIT_DIR=/tmp/elsewhere", "evil.so"} {
		if strings.Contains(string(e), bad) {
			t.Fatalf("the environment leaked %s:\n%s", bad, e)
		}
	}
	for _, want := range []string{"HOME=/h", "PATH=/usr/bin:/bin", "GIT_CONFIG_GLOBAL=", "GIT_NO_REPLACE_OBJECTS=1"} {
		if !strings.Contains(string(e), want) {
			t.Fatalf("the environment lacks %s:\n%s", want, e)
		}
	}
	// And a git that fails is an error, not a pass.
	os.WriteFile(bin, []byte("#!/bin/sh\nexit 2\n"), 0o755)
	if err := Fsck(g); err == nil {
		t.Fatal("a failing fsck passed")
	}
}

// post_inventory_unexplained: what is on the disk now, was not before the build and
// no record's writes cover (M3).
func TestCollectHeaderCounts(t *testing.T) {
	for _, c := range []struct {
		name   string
		make   func(f *cfix)
		writes []string
		want   int
	}{
		{"an untracked file nobody wrote", func(f *cfix) { f.edit("code/packages/x/a/new.txt", "x") }, nil, 1},
		{"the same but written", func(f *cfix) { f.edit("code/packages/x/a/new.txt", "x") }, []string{"code/packages/x/a/new.txt"}, 0},
		{"a collapsed directory that is the write root", func(f *cfix) { f.edit("code/packages/x/a/newdir/f", "x") }, []string{"code/packages/x/a/newdir"}, 0},
		{"a directory above a deeper write", func(f *cfix) { f.edit("code/packages/x/a/nd/deep/f", "x") }, []string{"code/packages/x/a/nd/deep"}, 0},
		{"a name that merely starts the same", func(f *cfix) { f.edit("code/packages/x/a/new.txt", "x") }, []string{"code/packages/x/a/ne"}, 1},
		{"two new paths and one written", func(f *cfix) {
			f.edit("code/packages/x/a/one.txt", "x")
			f.edit("code/packages/x/a/two.txt", "x")
		}, []string{"code/packages/x/a/one.txt"}, 1},
		{"a leftover that was there before the build", func(f *cfix) {}, nil, 0},
	} {
		t.Run(c.name, func(t *testing.T) {
			f := newCfixWith(t, nil, func(root string) {
				os.WriteFile(filepath.Join(root, "code/packages/x/a/leftover.txt"), []byte("before"), 0o644)
			})
			f.good()
			f.writesOf(nil, append([]string{ca + "/out"}, c.writes...))
			c.make(f)
			_, sh := f.collect()
			if sh.Header.PostInventoryUnexplained != c.want || sh.Header.TrackedChangedUnexplained != 0 || len(sh.Header.Void) != 0 {
				t.Fatalf("header %+v, want %d unexplained paths", sh.Header, c.want)
			}
			if m := mergeOf(f, sh); (m.RunVoidCount > 0) != (c.want > 0) {
				t.Fatalf("run-void %v for a count of %d", m.RunVoid, c.want)
			}
		})
	}
}

// A tracked file replaced by a directory breaks both checks, and neither may
// stop the other, or the records.
func TestCollectATrackedFileReplacedByADirectory(t *testing.T) {
	f := newCfix(t)
	f.good()
	p := filepath.Join(f.p.root, "code/packages/x/a/src.txt")
	os.Remove(p)
	os.Mkdir(p, 0o755)
	os.WriteFile(filepath.Join(p, "inside"), []byte("x"), 0o644)
	res, sh := f.collect()
	if res.Records != 3 {
		t.Fatal(res)
	}
	requireVoid(t, sh.Header, 1, 1, "tracked-changed")
}

// A sealed inventory that is not what the binding names is refused before
// anything is compared with it (VerifyPre), and one that cannot be decoded by the
// post-build comparison is a count, not a crash.
func TestCollectNewUnexplainedRefusesAnUnreadableInventory(t *testing.T) {
	f := newCfix(t)
	f.good()
	c := &collector{dir: f.p.dir, pre: f.pre}
	f.pre.Sums["inventory.json"] = strings.Repeat("0", 64)
	if _, err := c.newUnexplained(recfmt.Inventory{}, nil); err == nil {
		t.Fatal("an inventory that is not the sealed one was used")
	}
}

func TestUnexplainedPathsAndCoverage(t *testing.T) {
	tw := map[string]bool{"a/b": true}
	for _, c := range []struct {
		p    string
		want bool
	}{{"a/b", true}, {"a/b/c", true}, {"a/b/c/d", true}, {"a/bc", false}, {"a", false}, {".", false}, {"", false}, {"x", false}} {
		if got := coveredByAncestor(c.p, tw); got != c.want {
			t.Errorf("%q: %v", c.p, got)
		}
	}
	if n := unexplainedPaths(&TreeError{Total: 2, Paths: []string{"a/b/c", "z"}}, tw); n != 1 {
		t.Errorf("%d", n)
	}
}
