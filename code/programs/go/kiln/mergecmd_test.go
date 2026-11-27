package main

// The `record-merge` CLI's contract (KILN04 section 7.1, stage 4 PR 3a).
//
// The exit status is the report, so it is the thing to test here: 2 for usage,
// 3 for a run that may not record, 1 for a broken input, and 4 for a verdict
// of "void" (which is a result, not a malfunction). The verdict itself belongs
// to internal/record and internal/recmerge.

import (
	"bytes"
	"path/filepath"
	"strings"
	"testing"

	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/recmerge"
	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/record"
)

// cliMerge runs `kiln record-merge` with args and returns its exit code and
// stderr.
func cliMerge(t *testing.T, args ...string) (int, string) {
	t.Helper()
	var out, errb bytes.Buffer
	code := runRecordMerge(args, &out, &errb)
	return code, errb.String()
}

func TestRecordMergeUsageErrors(t *testing.T) {
	// The CLI falls back to $GITHUB_REPOSITORY, $GITHUB_RUN_ID and $GITHUB_SHA,
	// and they ARE set on a CI runner. The "no run metadata" case below is only
	// a usage error when they are absent, so clear them here rather than letting
	// the suite pass on a laptop and fail on CI.
	for _, k := range []string{"GITHUB_REPOSITORY", "GITHUB_RUN_ID", "GITHUB_SHA"} {
		t.Setenv(k, "")
	}
	dir := t.TempDir()
	cases := map[string][]string{
		"no arguments":       {},
		"positional":         {dir},
		"missing root":       {"--graph", "g", "--steps", "s", "--plan", "p", "--shard-count", "1", "--shard", "0=" + dir},
		"missing graph":      {"--root", dir, "--steps", "s", "--plan", "p", "--shard-count", "1", "--shard", "0=" + dir},
		"missing shard":      {"--root", dir, "--graph", "g", "--steps", "s", "--plan", "p", "--shard-count", "1"},
		"missing shardcount": {"--root", dir, "--graph", "g", "--steps", "s", "--plan", "p", "--shard", "0=" + dir},
		"bad shard spec":     {"--root", dir, "--graph", "g", "--steps", "s", "--plan", "p", "--shard-count", "1", "--shard", "nope"},
		"negative index":     {"--root", dir, "--graph", "g", "--steps", "s", "--plan", "p", "--shard-count", "1", "--shard", "-1=" + dir},
		"no ledger":          {"--root", dir, "--graph", "g", "--steps", "s", "--plan", "p", "--shard-count", "1", "--shard", "0=" + dir},
		"bad scope":          {"--root", dir, "--graph", "g", "--steps", "s", "--plan", "p", "--shard-count", "1", "--shard", "0=" + dir, "--ledger", dir, "--scope", "elsewhere"},
		"unknown flag":       {"--root", dir, "--nope"},
		"no run metadata":    {"--root", dir, "--graph", "g", "--steps", "s", "--plan", "p", "--shard-count", "1", "--shard", "0=" + dir, "--dry-run"},
	}
	for name, args := range cases {
		if code, _ := cliMerge(t, args...); code != 2 {
			t.Errorf("%s: exit %d, want 2", name, code)
		}
	}
}

// An untrusted run must be refused with 3 before anything is read: a developer
// machine (or a fork's CI) may not write the shared ledger.
func TestRecordMergeRefusesAnUntrustedSharedWrite(t *testing.T) {
	dir := t.TempDir()
	// No CI markers: host.Detect answers Local, which writes only the local
	// ledger.
	t.Setenv("GITHUB_ACTIONS", "")
	t.Setenv("CI", "")
	code, stderr := cliMerge(t, "--root", dir, "--graph", "g", "--steps", "s", "--plan", "p",
		"--shard-count", "1", "--shard", "0="+dir, "--ledger", filepath.Join(dir, "ledger"), "--scope", "shared",
		"--repository", "o/n", "--run-id", "1", "--revision", "abc")
	if code != 3 {
		t.Fatalf("exit %d, want 3 (stderr: %s)", code, stderr)
	}
	if !strings.Contains(stderr, "may not write the shared ledger") {
		t.Fatalf("stderr does not say why: %q", stderr)
	}
}

// A dry run needs no ledger and no trust, and fails at B1 (the directory is not
// a repository at the required revision) rather than at the CLI: exit 1, not 2.
func TestRecordMergeDryRunReachesTheJob(t *testing.T) {
	dir := t.TempDir()
	code, stderr := cliMerge(t, "--root", dir, "--graph", filepath.Join(dir, "g"),
		"--steps", filepath.Join(dir, "s"), "--plan", filepath.Join(dir, "p"),
		"--shard-count", "1", "--shard", "0="+dir, "--dry-run", "--revision", "abc",
		"--repository", "o/n", "--run-id", "1")
	if code != 1 {
		t.Fatalf("exit %d, want 1 (stderr: %s)", code, stderr)
	}
	if !strings.Contains(stderr, "B1") {
		t.Fatalf("stderr does not name B1: %q", stderr)
	}
}

// parsePrintable keeps a control character out of a log line, whatever an
// argument contains.
func TestPrintableArgSanitisesControlCharacters(t *testing.T) {
	if got := printableArg("a\x1b[31mb"); strings.ContainsRune(got, 0x1b) {
		t.Fatalf("escape survived: %q", got)
	}
	long := strings.Repeat("x", 200)
	if got := printableArg(long); len(got) > 72 {
		t.Fatalf("long argument not capped: %d bytes", len(got))
	}
}

func TestMergeShardFlagParsesIndexAndDir(t *testing.T) {
	var f mergeShardFlags
	if err := f.Set("2=/tmp/a"); err != nil {
		t.Fatal(err)
	}
	if err := f.Set("0=/tmp/b"); err != nil {
		t.Fatal(err)
	}
	if len(f.arts) != 2 || f.arts[0].Index != 2 || f.arts[0].Dir != "/tmp/a" || f.arts[1].Index != 0 {
		t.Fatalf("got %+v", f.arts)
	}
	if err := f.Set("=/tmp/c"); err == nil {
		t.Error("accepted an empty index")
	}
	if err := f.Set("3="); err == nil {
		t.Error("accepted an empty directory")
	}
}

// The report is the only thing a job log shows, so a void run must say so and
// list its causes: a bare "ok" on a refused run is how a green job would hide
// a soundness failure.
func TestWriteMergeReportIsExplicitAboutVoid(t *testing.T) {
	rep := &record.MergeReport{
		Revision: "abc", PlanPackages: 3, Actions: 2, Recordable: 0,
		Verdict: recmerge.Result{
			Emitted:      false,
			RunVoid:      []string{"reads-product:code/packages/go/a"},
			RunVoidCount: 2,
			ShardVoid:    map[int][]string{1: {"survivor"}},
		},
		Reasons: []record.ReasonCount{{Reason: "reads-product", Count: 2}},
	}
	var b bytes.Buffer
	writeMergeReport(&b, rep, false)
	got := b.String()
	for _, want := range []string{"run VOID", "reads-product", "1 more not shown", "shard 1 void", "survivor", "recordable 0 of 2"} {
		if !strings.Contains(got, want) {
			t.Errorf("report does not contain %q:\n%s", want, got)
		}
	}
	if strings.Contains(got, "run OK") {
		t.Errorf("a void run reported OK:\n%s", got)
	}
}

// --language is validated like a binding's language, as a usage error (2), and
// "all" is the default spelling.
func TestRecordMergeLanguageFlagIsValidated(t *testing.T) {
	base := []string{"--root", "/x", "--repository", "o/n", "--run-id", "1", "--revision", "abc", "--dry-run"}
	for _, bad := range []string{"Go", "a b", "-x", "../x"} {
		if code, errs := cliMerge(t, append(base, "--language", bad)...); code != 2 || !strings.Contains(errs, "--language") {
			t.Errorf("%q: exit %d, stderr %q", bad, code, errs)
		}
	}
	// A valid name gets past the flag check (and stops at the next usage check:
	// no shards were given).
	for _, ok := range []string{"go", "all"} {
		if _, errs := cliMerge(t, append(base, "--language", ok)...); strings.Contains(errs, "--language") {
			t.Errorf("%q refused: %q", ok, errs)
		}
	}
}

// Under a language selection the report says which one the shards sealed and
// how many actions lost their record to the foreign-package rule: a count, no
// path. Under the default there is no such line.
func TestWriteMergeReportCountsForeignDrops(t *testing.T) {
	rep := &record.MergeReport{Revision: "abc", Language: "go", ForeignDropped: 7, Verdict: recmerge.Result{Emitted: true}}
	var b bytes.Buffer
	writeMergeReport(&b, rep, true)
	if !strings.Contains(b.String(), "language selection go: 7 action(s)") {
		t.Fatalf("report: %s", b.String())
	}
	b.Reset()
	writeMergeReport(&b, &record.MergeReport{Revision: "abc", Verdict: recmerge.Result{Emitted: true}}, true)
	if strings.Contains(b.String(), "language selection") {
		t.Fatalf("the default report mentions a selection: %s", b.String())
	}
}

// An emitted dry run says it wrote nothing.
func TestWriteMergeReportSaysDryRunWroteNothing(t *testing.T) {
	rep := &record.MergeReport{Revision: "abc", Manifests: 4, Passes: 4, Verdict: recmerge.Result{Emitted: true}}
	var b bytes.Buffer
	writeMergeReport(&b, rep, true)
	if !strings.Contains(b.String(), "would be written") || strings.Contains(b.String(), "wrote 4") {
		t.Fatalf("dry run report: %s", b.String())
	}
}
