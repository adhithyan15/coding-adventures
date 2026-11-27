package record

// Tests for the merge job (KILN04 section 7.1, stage 4 PR 3a).
//
// WHAT IS AND IS NOT TESTED HERE
// ------------------------------
// The RULES are `recmerge.Merge`: binding B1-B6 and the cross-action rules
// M1-M15 are tested there, including against a brute-force oracle under
// `FuzzMerge`. This file does not re-test them; it tests the job around them,
// which is where a mistake would be silent:
//
//   - the option checks that must fail BEFORE any work (a run that cannot
//     write its ledger must not compute a verdict first);
//   - B1: the tree is the revision the shards claim;
//   - reading a shard artifact: a symlink, a missing file, a duplicate index
//     and a duplicate directory are refused, never followed or merged;
//   - the plan projection, including the name -> path bridge the steps export
//     needs;
//   - the keys adapter: ONE legacy.Traced per distinct facts, a refusal is a
//     verdict about the action (not an error), and the strong key is built
//     from the same Traced as the weak one.
//
// An end-to-end test that EMITS requires a real shard, and a real shard
// requires `kiln record prepare`, which refuses anything but a trusted
// linux/amd64 run. That test belongs with stage-4 PR 3b's CI job; it cannot be
// faked here without weakening what it would prove.

import (
	"os"
	"path/filepath"
	"testing"
	"time"

	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/ledger"
	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/legacy"
	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/recfmt"
	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/recmerge"
	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/source"
)

const (
	mergeRev  = "0123456789abcdef0123456789abcdef01234567"
	testKlien = "kiln/test traced/2 recorder/13"
)

// fakeAuthority is an Authority for tests: both flags are explicit so a test
// states whether it is a trusted run.
type fakeAuthority struct{ trusted, local bool }

func (f fakeAuthority) Trusted() bool           { return f.trusted }
func (f fakeAuthority) WritesLocalLedger() bool { return f.local }

func localOnly() ledger.Authority { return fakeAuthority{local: true} }
func untrusted() ledger.Authority { return fakeAuthority{} }

// mergeByArgs runs only the argument checks, which must fail for every case
// below before the tree, the exports or a shard are touched: `Root` names a
// directory that does not exist, so reaching any of them would surface a
// different error.
func mergeByArgs(t *testing.T, o MergeOptions) error {
	t.Helper()
	if o.Root == "" {
		o.Root = filepath.Join(t.TempDir(), "does-not-exist")
	}
	if o.Source == nil {
		o.Source = source.NewGit(o.Root)
	}
	_, err := Merge(o)
	return err
}

func TestMergeRefusesIncompleteOptions(t *testing.T) {
	graph, steps, plan := writeExports(t)
	base := func() MergeOptions {
		return MergeOptions{
			Root:       t.TempDir(),
			GraphPath:  graph,
			StepsPath:  steps,
			PlanPath:   plan,
			ShardCount: 1,
			Shards:     []ShardArtifact{{Index: 0, Dir: t.TempDir()}},
			Run:        recmergeRun(),
			Kiln:       testKlien,
			Now:        time.Unix(1700000000, 0).UTC(),
			DryRun:     true,
		}
	}
	cases := map[string]func(*MergeOptions){
		"no root":         func(o *MergeOptions) { o.Root = "" },
		"no graph":        func(o *MergeOptions) { o.GraphPath = "" },
		"no steps":        func(o *MergeOptions) { o.StepsPath = "" },
		"no plan":         func(o *MergeOptions) { o.PlanPath = "" },
		"no source":       func(o *MergeOptions) { o.Source = nil },
		"no kiln id":      func(o *MergeOptions) { o.Kiln = "" },
		"no repository":   func(o *MergeOptions) { o.Run.Repository = "" },
		"no run id":       func(o *MergeOptions) { o.Run.ID = "" },
		"no revision":     func(o *MergeOptions) { o.Run.Revision = "" },
		"shard count 0":   func(o *MergeOptions) { o.ShardCount = 0 },
		"shard count big": func(o *MergeOptions) { o.ShardCount = recfmt.MaxShards + 1 },
	}
	for name, mut := range cases {
		o := base()
		mut(&o)
		// A nil Source is the one case the helper must not fill in.
		if name == "no source" {
			if _, err := Merge(o); err == nil {
				t.Errorf("%s: accepted", name)
			}
			continue
		}
		if err := mergeByArgs(t, o); err == nil {
			t.Errorf("%s: accepted", name)
		}
	}
}

// A run that may not write its ledger must be refused BEFORE the tree is
// verified: otherwise the job computes a whole verdict it can never record.
func TestMergeRefusesAnUnauthorizedWriteUpFront(t *testing.T) {
	graph, steps, plan := writeExports(t)
	dir := t.TempDir()
	for name, auth := range map[string]ledger.Authority{
		"untrusted shared": untrusted(),
		"local shared":     localOnly(),
		"nil shared":       nil,
	} {
		_, err := Merge(MergeOptions{
			Root: dir, Source: source.NewGit(dir),
			GraphPath: graph, StepsPath: steps, PlanPath: plan,
			ShardCount: 1, Shards: []ShardArtifact{{Index: 0, Dir: dir}},
			Run: recmergeRun(), Kiln: testKlien, Now: time.Unix(1700000000, 0).UTC(),
			LedgerDir: filepath.Join(dir, "ledger"), Scope: ledger.Shared, Authority: auth,
		})
		if err == nil {
			t.Errorf("%s: accepted", name)
			continue
		}
		if err != ledger.ErrUntrusted {
			// Any other error means it got past the authority check.
			t.Errorf("%s: got %v, want ErrUntrusted", name, err)
		}
	}
}

// A dry run needs no ledger and no authority, which is what makes the job
// usable as a report.
func TestMergeDryRunNeedsNoLedger(t *testing.T) {
	graph, steps, plan := writeExports(t)
	dir := t.TempDir()
	// The failure must be B1 (not a ledger complaint): the directory is not a
	// repository at the required revision.
	_, err := Merge(MergeOptions{
		Root: dir, Source: source.NewGit(dir),
		GraphPath: graph, StepsPath: steps, PlanPath: plan,
		ShardCount: 1, Shards: []ShardArtifact{{Index: 0, Dir: dir}},
		Run: recmergeRun(), Kiln: testKlien, Now: time.Unix(1700000000, 0).UTC(),
		DryRun: true,
	})
	if err == nil || !contains(err.Error(), "B1") {
		t.Fatalf("got %v, want a B1 error", err)
	}
}

// decodeShards refuses everything it cannot read as a shard, instead of
// treating a missing artifact as "no observations" (which would emit).
func TestDecodeShardsRefusesBadArtifacts(t *testing.T) {
	mk := func(t *testing.T, index int, dir string) ShardArtifact {
		return ShardArtifact{Index: index, Dir: dir}
	}
	t.Run("missing directory", func(t *testing.T) {
		_, err := decodeShards([]ShardArtifact{mk(t, 0, filepath.Join(t.TempDir(), "nope"))})
		if err == nil {
			t.Fatal("accepted a missing artifact")
		}
	})
	t.Run("missing records", func(t *testing.T) {
		dir := t.TempDir()
		for _, n := range []string{ShardBindingFile, ShardHostFactsFile, ShardInventoryFile} {
			if err := os.WriteFile(filepath.Join(dir, n), []byte("{}"), 0o600); err != nil {
				t.Fatal(err)
			}
		}
		_, err := decodeShards([]ShardArtifact{mk(t, 0, dir)})
		if err == nil {
			t.Fatal("accepted an artifact with no records")
		}
	})
	t.Run("symlinked binding", func(t *testing.T) {
		dir := t.TempDir()
		real := filepath.Join(dir, "real.json")
		if err := os.WriteFile(real, []byte("{}"), 0o600); err != nil {
			t.Fatal(err)
		}
		if err := os.Symlink(real, filepath.Join(dir, ShardBindingFile)); err != nil {
			t.Skipf("symlinks unavailable: %v", err)
		}
		_, err := decodeShards([]ShardArtifact{mk(t, 0, dir)})
		if err == nil || !contains(err.Error(), "regular file") {
			t.Fatalf("got %v, want a regular-file refusal", err)
		}
	})
	t.Run("duplicate index", func(t *testing.T) {
		a, b := t.TempDir(), t.TempDir()
		_, err := decodeShards([]ShardArtifact{mk(t, 1, a), mk(t, 1, b)})
		if err == nil {
			t.Fatal("accepted two shards with one index")
		}
	})
	t.Run("duplicate directory", func(t *testing.T) {
		d := t.TempDir()
		_, err := decodeShards([]ShardArtifact{mk(t, 0, d), mk(t, 1, d)})
		if err == nil {
			t.Fatal("accepted one directory as two shards")
		}
	})
}

// ParsePartition is the in-memory half of ReadPartition: the merge job reads the
// plan's bytes once (so the digest covers what is parsed) and projects them onto
// the shard plans here.
func TestParsePartitionProjectsThePlan(t *testing.T) {
	_, _, planPath := writeExports(t)
	data, err := os.ReadFile(planPath)
	if err != nil {
		t.Fatal(err)
	}
	p, err := ParsePartition(data, 1)
	if err != nil {
		t.Fatal(err)
	}
	if len(p.Shards) != 1 || p.Shards[0].Index != 0 {
		t.Fatalf("shards: %+v", p.Shards)
	}
	if got := p.Shards[0].Assigned; len(got) != 1 || got[0] != "code/packages/go/a" {
		t.Fatalf("assigned: %v", got)
	}
	if _, err := p.Digest(); err != nil {
		t.Fatalf("digest: %v", err)
	}
	if _, err := ParsePartition(data, 2); err == nil {
		t.Fatal("accepted the plan at the wrong shard count")
	}
}

// ---------------------------------------------------------------------------
// The keys adapter.

func TestTracedKeysBuildsOneTracedPerDistinctFacts(t *testing.T) {
	k, builds := newTestKeys(t)
	f := facts("image-a")
	if _, _, err := k.Weak(f, onlyKeyable); err != nil {
		t.Fatalf("weak: %v", err)
	}
	if _, _, err := k.Weak(f, onlyKeyable); err != nil {
		t.Fatalf("weak: %v", err)
	}
	if *builds != 1 {
		t.Fatalf("built %d Traced values for one facts value, want 1", *builds)
	}
	// A different image is a different weak key, so it must be a different
	// Traced: sharing one would key the second shard's actions with the first
	// image id.
	if _, _, err := k.Weak(facts("image-b"), "code/packages/go/a"); err != nil {
		t.Fatalf("weak: %v", err)
	}
	if *builds != 2 {
		t.Fatalf("built %d Traced values for two facts values, want 2", *builds)
	}
}

// A refusal is a verdict about the ACTION, not a malfunction: it is returned
// as a reason so the merge charges it to that action instead of voiding the
// run.
func TestTracedKeysReturnsARefusalAsAReason(t *testing.T) {
	k, _ := newTestKeys(t)
	key, refusal, err := k.Weak(facts("image-a"), "code/packages/go/missing")
	if err != nil {
		t.Fatalf("a refusal must not be an error: %v", err)
	}
	if refusal == "" {
		t.Fatal("a refusal reason is empty")
	}
	if key != [32]byte{} {
		t.Fatal("a refused action has a key")
	}
}

// The strong key is built from the SAME Traced as the weak one, so the two can
// never disagree about the tree.
func TestTracedKeysStrongNeedsAKnownAction(t *testing.T) {
	k, _ := newTestKeys(t)
	if _, err := k.Strong(facts("image-a"), "code/packages/go/missing", nil); err == nil {
		t.Fatal("built a strong key for a refused action")
	}
}

func TestNewTracedKeysRefusesMissingPieces(t *testing.T) {
	if _, err := NewTracedKeys(TracedKeysOptions{Source: source.NewGit(t.TempDir()), Root: "/x"}); err == nil {
		t.Error("accepted a nil graph")
	}
	if _, err := NewTracedKeys(TracedKeysOptions{Graph: &legacy.Graph{}, Root: "/x"}); err == nil {
		t.Error("accepted a nil source")
	}
	if _, err := NewTracedKeys(TracedKeysOptions{Graph: &legacy.Graph{}, Source: source.NewGit(t.TempDir())}); err == nil {
		t.Error("accepted an empty root")
	}
}

func TestCountReasonsSumsPerReason(t *testing.T) {
	got := countReasons([]recmerge.Outcome{
		{Reasons: []string{"b", "a"}},
		{Reasons: []string{"a"}},
		{},
	})
	want := []ReasonCount{{"a", 2}, {"b", 1}}
	if len(got) != len(want) {
		t.Fatalf("got %v", got)
	}
	for i := range want {
		if got[i] != want[i] {
			t.Fatalf("got %v, want %v", got, want)
		}
	}
}

// ---------------------------------------------------------------------------
// Helpers.

// fakeTraced is a Traced that answers for exactly one keyable action and
// refuses everything else, so the adapter's own logic is what is under test.
type fakeTraced struct{}

// onlyKeyable is the one action fakeTraced keys.
const onlyKeyable = "code/packages/go/a"

func (fakeTraced) WeakKey(rel string) (*legacy.WeakKey, error) {
	if rel != onlyKeyable {
		return nil, &legacy.Refusal{Reason: "no-such-package"}
	}
	return &legacy.WeakKey{Key: [32]byte{7}}, nil
}

func (fakeTraced) StrongKey(w *legacy.WeakKey, names []string) (*legacy.StrongKey, error) {
	return &legacy.StrongKey{Key: w.Key, Entries: nil}, nil
}

// newTestKeys builds a TracedKeys whose Traced values are fakeTraced, counting
// how many were constructed.
func newTestKeys(t *testing.T) (*TracedKeys, *int) {
	t.Helper()
	built := 0
	k, err := NewTracedKeys(TracedKeysOptions{
		Graph: &legacy.Graph{}, Source: source.NewGit(t.TempDir()), Root: "/repo",
		Arch: "x64",
		newTraced: func(*legacy.Graph, legacy.Options, string, []string) (Traced, error) {
			built++
			return fakeTraced{}, nil
		},
	})
	if err != nil {
		t.Fatal(err)
	}
	return k, &built
}

func recmergeRun() recmerge.Run {
	return recmerge.Run{Repository: "o/n", ID: "1", Attempt: 1, Revision: mergeRev}
}

func facts(image string) recfmt.HostFacts {
	return recfmt.HostFacts{
		Schema: recfmt.SchemaHostFacts, Image: image, Root: "/repo", Home: "/home/u",
		Tmpdir: "/tmp", ExecutorArgs: nil, Env: []string{"LANG=C.UTF-8"},
	}
}

// writeExports writes a minimal, valid graph/steps/plan triple: one package
// `go/a` at code/packages/go/a, assigned to and run by one shard.
func writeExports(t *testing.T) (graph, steps, plan string) {
	t.Helper()
	dir := t.TempDir()
	graph = filepath.Join(dir, "graph.json")
	steps = filepath.Join(dir, "steps.json")
	plan = filepath.Join(dir, "build-plan.json")
	write := func(p, s string) {
		if err := os.WriteFile(p, []byte(s), 0o600); err != nil {
			t.Fatal(err)
		}
	}
	write(graph, `{"schema":"kiln-legacy-graph/1","packages":[{"name":"go/a","rel_path":"code/packages/go/a","language":"go","build_files":{"linux":"BUILD"},"deps":{"linux":[]},"dev_deps":[],"repo_inputs":[],"repo_input_globs":[],"extra_toolchains":[]}]}`)
	write(steps, `{"schema":"kiln-legacy-steps/1","os":"linux","packages":[{"package":"go/a","steps":["go test ./..."]}]}`)
	write(plan, `{"packages":[{"name":"go/a","rel_path":"code/packages/go/a"}],"shards":[{"index":0,"assigned_packages":["go/a"],"package_names":["go/a"]}]}`)
	return graph, steps, plan
}

// contains is strings.Contains without importing strings in a test that would
// otherwise not need it.
func contains(s, sub string) bool {
	for i := 0; i+len(sub) <= len(s); i++ {
		if s[i:i+len(sub)] == sub {
			return true
		}
	}
	return false
}

// writeRegular is openRegular's subject; the merge refuses a FIFO or a device
// at any of its four input names.
func TestReadRegularRefusesADirectory(t *testing.T) {
	if _, err := readRegular(t.TempDir(), 1024, "graph"); err == nil {
		t.Fatal("read a directory")
	}
}

func TestReadRegularEnforcesTheCap(t *testing.T) {
	p := filepath.Join(t.TempDir(), "f")
	if err := os.WriteFile(p, []byte("0123456789"), 0o600); err != nil {
		t.Fatal(err)
	}
	if _, err := readRegular(p, 10, "graph"); err != nil {
		t.Fatalf("refused a file at the cap: %v", err)
	}
	if _, err := readRegular(p, 9, "graph"); err == nil {
		t.Fatal("accepted a file over the cap")
	}
}

func TestSum256IsHexSHA256(t *testing.T) {
	// The empty string's SHA-256, which is the one value worth pinning.
	if got := sum256(nil); got != "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855" {
		t.Fatalf("sum256(nil) = %q", got)
	}
}
