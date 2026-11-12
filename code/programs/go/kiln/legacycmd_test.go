package main

import (
	"bytes"
	"os"
	"os/exec"
	"path/filepath"
	"strings"
	"testing"
)

const cmdGraph = `{"schema":"kiln-legacy-graph/1","packages":[
 {"name":"go/a","rel_path":"code/packages/go/a","language":"go",
  "build_files":{"linux":"BUILD","darwin":"BUILD","windows":"BUILD"},
  "deps":{"linux":[],"darwin":[],"windows":[]},
  "dev_deps":[],"repo_inputs":[],"repo_input_globs":[],"extra_toolchains":[]}]}`

// cmdRepo writes a minimal repository and returns its directory.
func cmdRepo(t *testing.T) string {
	t.Helper()
	dir := t.TempDir()
	for p, c := range map[string]string{
		"code/kiln/legacy/pinned.json":                                            `{"languages": []}`,
		"code/kiln/legacy/never.json":                                             `{"packages": []}`,
		"code/specs/fixtures/build-tool-v1/repository-source-input-boundary.json": `{"schema_version":1,"boundaries":[]}`,
		"code/programs/go/build-tool/go.mod":                                      "module bt\n",
		"code/programs/go/build-tool/main.go":                                     "package main\n",
		"code/packages/go/a/BUILD":                                                "echo a\n",
	} {
		full := filepath.Join(dir, filepath.FromSlash(p))
		if err := os.MkdirAll(filepath.Dir(full), 0o755); err != nil {
			t.Fatal(err)
		}
		if err := os.WriteFile(full, []byte(c), 0o644); err != nil {
			t.Fatal(err)
		}
	}
	return dir
}

func runCLI(args ...string) (int, string, string) {
	var out, errb bytes.Buffer
	code := run(args, &out, &errb)
	return code, out.String(), errb.String()
}

func TestLegacyPlanAndExplain(t *testing.T) {
	repo := cmdRepo(t)
	work := t.TempDir()
	graph := filepath.Join(work, "graph.json")
	plan := filepath.Join(work, "plan.json")
	if err := os.WriteFile(graph, []byte(cmdGraph), 0o644); err != nil {
		t.Fatal(err)
	}
	code, out, errs := runCLI("legacy-plan", "--graph", graph, "--out", plan, "--root", repo, "--source", "folder", "--os", "linux")
	if code != 0 || !strings.Contains(out, "ok: 1 packages") {
		t.Fatalf("code %d out %q err %q", code, out, errs)
	}
	first, _ := os.ReadFile(plan)
	if code, _, errs := runCLI("legacy-plan", "--graph", graph, "--out", plan, "--root", repo, "--source", "folder", "--os", "linux"); code != 0 {
		t.Fatalf("second run: %s", errs)
	}
	if second, _ := os.ReadFile(plan); !bytes.Equal(first, second) {
		t.Fatal("two runs wrote different plans")
	}
	code, out, errs = runCLI("explain", "--plan", plan, "go/a")
	if code != 0 || !strings.Contains(out, `"key"`) || !strings.Contains(out, `"closure_size": 1`) {
		t.Fatalf("code %d out %q err %q", code, out, errs)
	}
	if code, _, _ := runCLI("explain", "--plan", plan, "nope"); code != 1 {
		t.Fatalf("unknown package: code %d", code)
	}
	if code, _, _ := runCLI("explain", "--plan", filepath.Join(work, "none"), "go/a"); code != 1 {
		t.Fatalf("missing plan: code %d", code)
	}
}

func TestLegacyPlanGitSource(t *testing.T) {
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
	if code, _, errs := runCLI("legacy-plan", "--graph", graph, "--out", plan, "--root", repo, "--os", "linux"); code != 0 {
		t.Fatalf("git source: %s", errs)
	}
	_ = os.WriteFile(filepath.Join(repo, "stray.txt"), []byte("x"), 0o644)
	if code, _, errs := runCLI("legacy-plan", "--graph", graph, "--out", plan, "--root", repo, "--os", "linux"); code != 1 || !strings.Contains(errs, "not clean") {
		t.Fatalf("dirty: code %d %s", code, errs)
	}
}

func TestLegacyPlanErrors(t *testing.T) {
	repo := cmdRepo(t)
	work := t.TempDir()
	graph := filepath.Join(work, "graph.json")
	plan := filepath.Join(work, "plan.json")
	_ = os.WriteFile(graph, []byte(cmdGraph), 0o644)
	bad := filepath.Join(work, "bad.json")
	_ = os.WriteFile(bad, []byte("{}"), 0o644)
	for name, tc := range map[string]struct {
		args []string
		code int
	}{
		"no flags":      {[]string{"legacy-plan"}, 2},
		"bad flag":      {[]string{"legacy-plan", "--nope"}, 2},
		"extra arg":     {[]string{"legacy-plan", "--graph", graph, "--out", plan, "x"}, 2},
		"bad source":    {[]string{"legacy-plan", "--graph", graph, "--out", plan, "--source", "tar"}, 2},
		"bad os":        {[]string{"legacy-plan", "--graph", graph, "--out", plan, "--root", repo, "--source", "folder", "--os", "plan9"}, 1},
		"missing graph": {[]string{"legacy-plan", "--graph", filepath.Join(work, "x"), "--out", plan, "--root", repo, "--source", "folder"}, 1},
		"bad graph":     {[]string{"legacy-plan", "--graph", bad, "--out", plan, "--root", repo, "--source", "folder"}, 1},
		"bad root":      {[]string{"legacy-plan", "--graph", graph, "--out", plan, "--root", filepath.Join(work, "gone"), "--source", "folder", "--os", "linux"}, 1},
		"bad out":       {[]string{"legacy-plan", "--graph", graph, "--out", filepath.Join(work, "no", "dir", "p"), "--root", repo, "--source", "folder", "--os", "linux"}, 1},
		"explain none":  {[]string{"explain"}, 2},
		"explain flag":  {[]string{"explain", "--bogus"}, 2},
		"explain extra": {[]string{"explain", "--plan", plan, "a", "b"}, 2},
	} {
		if code, _, errs := runCLI(tc.args...); code != tc.code {
			t.Errorf("%s: code %d (want %d): %s", name, code, tc.code, errs)
		}
	}
}
