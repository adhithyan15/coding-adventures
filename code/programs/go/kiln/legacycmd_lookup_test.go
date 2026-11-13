package main

import (
	"os"
	"os/exec"
	"path/filepath"
	"strings"
	"testing"
)

// Stage 5a on the command line: lookup is opt-in, refused on a plain folder,
// and shows up in the plan and in `kiln explain`.

func TestLegacyPlanLookupFlags(t *testing.T) {
	repo := cmdRepo(t)
	work := t.TempDir()
	graph := filepath.Join(work, "graph.json")
	plan := filepath.Join(work, "plan.json")
	_ = os.WriteFile(graph, []byte(cmdGraph), 0o644)
	led := filepath.Join(work, "ledger")
	base := []string{"legacy-plan", "--graph", graph, "--out", plan, "--root", repo, "--os", "linux"}
	with := func(extra ...string) []string { return append(append([]string(nil), base...), extra...) }

	// The folder source cannot vouch for a clean tree: refused, nothing written.
	code, _, errs := runCLI(with("--source", "folder", "--ledger", led, "--scope", "local")...)
	if code != 2 || !strings.Contains(errs, "refusing --ledger with --source folder") {
		t.Fatalf("folder: code %d %s", code, errs)
	}
	if _, err := os.Stat(plan); err == nil {
		t.Fatal("a refused run wrote a plan")
	}
	for name, args := range map[string][]string{
		"ledger without scope": with("--ledger", led),
		"scope without ledger": with("--scope", "local"),
		"bad scope":            with("--ledger", led, "--scope", "global"),
		"flags without ledger": with("--executor-arg", "-x=1"),
	} {
		if code, _, _ := runCLI(args...); code != 2 {
			t.Errorf("%s: code %d", name, code)
		}
	}
}

func TestLegacyPlanLookupAndExplain(t *testing.T) {
	if _, err := exec.LookPath("git"); err != nil {
		t.Skip("git not installed")
	}
	repo := cmdRepo(t)
	for _, args := range [][]string{{"init", "-q"}, {"config", "user.email", "k@example.com"}, {"config", "user.name", "k"},
		{"config", "commit.gpgsign", "false"}, {"add", "-A"}, {"commit", "-q", "-m", "x"}} {
		if out, err := exec.Command("git", append([]string{"-C", repo}, args...)...).CombinedOutput(); err != nil {
			t.Fatalf("git %v: %v %s", args, err, out)
		}
	}
	work := t.TempDir()
	graph := filepath.Join(work, "graph.json")
	plan := filepath.Join(work, "plan.json")
	_ = os.WriteFile(graph, []byte(cmdGraph), 0o644)
	led := filepath.Join(work, "ledger") // does not exist: an empty store

	for _, scope := range []string{"local", "shared"} {
		code, out, errs := runCLI("legacy-plan", "--graph", graph, "--out", plan, "--root", repo, "--os", "linux",
			"--ledger", led, "--scope", scope, "--executor-arg", "-exec-wrapper=/k")
		if code != 0 || !strings.Contains(out, "lookup ("+scope+", shadow only)") {
			t.Fatalf("%s: code %d out %q err %q", scope, code, out, errs)
		}
	}
	data, _ := os.ReadFile(plan)
	if !strings.Contains(string(data), `"lookup"`) || !strings.Contains(string(data), `"no-manifest"`) {
		t.Fatalf("plan lacks the lookup result: %s", data)
	}
	code, out, errs := runCLI("explain", "--plan", plan, "go/a")
	if code != 0 || !strings.Contains(out, `"result": "no-manifest"`) || !strings.Contains(out, `"lookup_info"`) ||
		!strings.Contains(out, "shadow only") || !strings.Contains(out, `"weak"`) {
		t.Fatalf("explain: code %d out %s err %s", code, out, errs)
	}

	// A truncated manifest is said to be truncated in `kiln explain`.
	tr := t.TempDir()
	plp := filepath.Join(tr, "p.json")
	_ = os.WriteFile(plp, []byte(`{"schema":"kiln-legacy-plan/1","os":"linux","arch":"x64","host":"h","env":[],"packages":[{"name":"go/a","key":"","class":"uncacheable","reasons":[],"hints":[],"closure_size":1,"tools":[],"components":{"own":"","reads":"","consumed":"","recipe":"","tools":"","env":""},"lookup":{"result":"miss","manifests":[{"id":"i","run":"1","at":"t","revision":"r","entry_count":999,"entries_truncated":true,"entries":[]}]}}],"histogram":{"classes":{},"reasons":{},"hints":{},"closure_sizes":{"min":0,"median":0,"p90":0,"max":0}}}`), 0o644)
	if code, out, errs := runCLI("explain", "--plan", plp, "go/a"); code != 0 || !strings.Contains(out, "lookup_note") || !strings.Contains(out, "entry_count") {
		t.Fatalf("truncation note: code %d %s %s", code, out, errs)
	}

	// Without --ledger the plan and the explanation carry no lookup at all.
	if code, _, _ := runCLI("legacy-plan", "--graph", graph, "--out", plan, "--root", repo, "--os", "linux"); code != 0 {
		t.Fatal("plain plan failed")
	}
	if _, out, _ := runCLI("explain", "--plan", plan, "go/a"); strings.Contains(out, "lookup") {
		t.Fatalf("explain shows lookup without --ledger: %s", out)
	}

	// A malformed executor flag is an error, not a silent mismatch.
	if code, _, _ := runCLI("legacy-plan", "--graph", graph, "--out", plan, "--root", repo, "--os", "linux",
		"--ledger", led, "--scope", "local", "--executor-arg", "bad"); code != 1 {
		t.Fatalf("bad executor flag: code %d", code)
	}
}
