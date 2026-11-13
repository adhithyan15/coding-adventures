package main

import (
	"encoding/json"
	"flag"
	"io"
	"os"
	"path/filepath"
	"reflect"
	"runtime"
	"strings"
	"testing"

	"github.com/adhithyan15/coding-adventures/code/programs/go/build-tool/internal/discovery"
	"github.com/adhithyan15/coding-adventures/code/programs/go/build-tool/internal/executor"
)

// runBuildTool runs the front door with args and returns the exit code and
// what it wrote to stderr. It swaps the global flag set and executor state
// the same way TestRunReportsStablePackageHashFailure does.
func runBuildTool(t *testing.T, args ...string) (int, string) {
	t.Helper()
	originalArgs, originalFlags, originalStderr := os.Args, flag.CommandLine, os.Stderr
	reader, writer, err := os.Pipe()
	if err != nil {
		t.Fatal(err)
	}
	defer func() {
		os.Args, flag.CommandLine, os.Stderr = originalArgs, originalFlags, originalStderr
		executor.ExecWrapper, executor.ExecWrapperRepoRoot = nil, ""
		executor.PruneLeafCargoTargets = false
	}()
	flag.CommandLine = flag.NewFlagSet("build-tool-test", flag.ContinueOnError)
	flag.CommandLine.SetOutput(io.Discard)
	os.Args = append([]string{"build-tool"}, args...)
	os.Stderr = writer

	code := run()
	_ = writer.Close()
	os.Stderr = originalStderr
	out, _ := io.ReadAll(reader)
	_ = reader.Close()
	return code, string(out)
}

// kilnFixture makes a repo with one Go package per entry in builds (name ->
// BUILD text) and returns its root.
func kilnFixture(t *testing.T, builds map[string]string) string {
	t.Helper()
	root := t.TempDir()
	for name, build := range builds {
		dir := filepath.Join(root, "code", "packages", "go", name)
		if err := os.MkdirAll(dir, 0o755); err != nil {
			t.Fatal(err)
		}
		if err := os.WriteFile(filepath.Join(dir, "BUILD"), []byte(build), 0o644); err != nil {
			t.Fatal(err)
		}
		if err := os.WriteFile(filepath.Join(dir, "main.go"), []byte("package "+name+"\n"), 0o644); err != nil {
			t.Fatal(err)
		}
	}
	return root
}

// fakeWrapper writes a wrapper script that records its argv and the KILN_*
// variables it received, then runs the wrapped command. The first argument
// to the script is the record file.
func fakeWrapper(t *testing.T) (script string, record string) {
	t.Helper()
	if runtime.GOOS == "windows" {
		t.Skip("fake wrapper is a POSIX shell script")
	}
	dir := t.TempDir()
	script = filepath.Join(dir, "wrap.sh")
	record = filepath.Join(dir, "record.txt")
	body := `#!/bin/sh
rec=$1
shift
{
  echo "ARGC=$#"
  for a in "$@"; do printf 'ARG=%s\n' "$a"; done
  echo "ENV=$KILN_ACTION|$KILN_STEP|$KILN_STEP_COUNT"
} >> "$rec"
exec "$@"
`
	if err := os.WriteFile(script, []byte(body), 0o755); err != nil {
		t.Fatal(err)
	}
	return script, record
}

func wrapperJSON(t *testing.T, argv ...string) string {
	t.Helper()
	b, err := json.Marshal(argv)
	if err != nil {
		t.Fatal(err)
	}
	return string(b)
}

func readResults(t *testing.T, path string) kilnLegacyResults {
	t.Helper()
	data, err := os.ReadFile(path)
	if err != nil {
		t.Fatal(err)
	}
	var doc kilnLegacyResults
	if err := json.Unmarshal(data, &doc); err != nil {
		t.Fatalf("results are not JSON: %v\n%s", err, data)
	}
	return doc
}

func TestExecWrapperPrefixesEveryStepWithExactArgvAndEnv(t *testing.T) {
	script, record := fakeWrapper(t)
	root := kilnFixture(t, map[string]string{
		"demo": "echo one\nprintf 'a  b' > out.txt\n# not a step\ntrue && echo \"quoted $HOME\"\n",
	})
	results := filepath.Join(t.TempDir(), "results.json")

	code, stderr := runBuildTool(t, "-root", root, "-force", "-validate-build-files=false",
		"-exec-wrapper", wrapperJSON(t, script, record), "-emit-results", results)
	if code != 0 {
		t.Fatalf("exit %d: %s", code, stderr)
	}

	got, err := os.ReadFile(record)
	if err != nil {
		t.Fatal(err)
	}
	want := strings.Join([]string{
		"ARGC=3", "ARG=sh", "ARG=-c", "ARG=echo one", "ENV=code/packages/go/demo|0|3",
		"ARGC=3", "ARG=sh", "ARG=-c", "ARG=printf 'a  b' > out.txt", "ENV=code/packages/go/demo|1|3",
		"ARGC=3", "ARG=sh", "ARG=-c", `ARG=true && echo "quoted $HOME"`, "ENV=code/packages/go/demo|2|3",
		"",
	}, "\n")
	if string(got) != want {
		t.Fatalf("wrapper saw:\n%s\nwant:\n%s", got, want)
	}
	// The step still ran, with the package directory as its cwd.
	out, err := os.ReadFile(filepath.Join(root, "code", "packages", "go", "demo", "out.txt"))
	if err != nil || string(out) != "a  b" {
		t.Fatalf("step did not run through the wrapper: %q %v", out, err)
	}
	doc := readResults(t, results)
	wantPkgs := []kilnResultPackage{{Package: "code/packages/go/demo", Status: "built", StepsRun: 3, StepsTotal: 3}}
	if doc.Schema != "kiln-legacy-results/1" || !reflect.DeepEqual(doc.Packages, wantPkgs) {
		t.Fatalf("results = %+v", doc)
	}
}

func TestExecWrapperEnvIsNotLeakedToTheStepAndCallerValuesAreReplaced(t *testing.T) {
	script, record := fakeWrapper(t)
	t.Setenv("KILN_STEP", "forged")
	root := kilnFixture(t, map[string]string{"demo": "echo done\n"})

	code, stderr := runBuildTool(t, "-root", root, "-force", "-validate-build-files=false",
		"-exec-wrapper", wrapperJSON(t, script, record))
	if code != 0 {
		t.Fatalf("exit %d: %s", code, stderr)
	}
	got, _ := os.ReadFile(record)
	if !strings.Contains(string(got), "ENV=code/packages/go/demo|0|1") {
		t.Fatalf("forged KILN_STEP survived: %s", got)
	}
}

func TestEmitResultsReportsMidPackageFailureAndSkips(t *testing.T) {
	root := kilnFixture(t, map[string]string{
		"aaa-good": "echo ok\n",
		"bad":      "true\nfalse\necho never\n",
	})
	results := filepath.Join(t.TempDir(), "results.json")

	// Alone (no wrapper): a failure in step 2 of 3.
	code, stderr := runBuildTool(t, "-root", root, "-force", "-validate-build-files=false", "-emit-results", results)
	if code != 1 {
		t.Fatalf("exit %d, want 1: %s", code, stderr)
	}
	doc := readResults(t, results)
	want := []kilnResultPackage{
		{Package: "code/packages/go/aaa-good", Status: "built", StepsRun: 1, StepsTotal: 1},
		{Package: "code/packages/go/bad", Status: "failed", StepsRun: 2, StepsTotal: 3},
	}
	if !reflect.DeepEqual(doc.Packages, want) {
		t.Fatalf("results = %+v, want %+v", doc.Packages, want)
	}

	// A second run without -force finds both up to date or failed-cached:
	// the good package is skipped, with 0 steps run and its BUILD's total.
	code, stderr = runBuildTool(t, "-root", root, "-validate-build-files=false", "-emit-results", results)
	_ = code
	doc = readResults(t, results)
	for _, p := range doc.Packages {
		if p.Package == "code/packages/go/aaa-good" {
			if p.Status != "skipped" || p.StepsRun != 0 || p.StepsTotal != 1 {
				t.Fatalf("skipped package = %+v (stderr %s)", p, stderr)
			}
		}
	}
}

func TestEmitResultsDryRunWouldBuild(t *testing.T) {
	root := kilnFixture(t, map[string]string{"demo": "echo a\necho b\n"})
	results := filepath.Join(t.TempDir(), "results.json")
	code, stderr := runBuildTool(t, "-root", root, "-force", "-dry-run", "-validate-build-files=false", "-emit-results", results)
	if code != 0 {
		t.Fatalf("exit %d: %s", code, stderr)
	}
	doc := readResults(t, results)
	want := []kilnResultPackage{{Package: "code/packages/go/demo", Status: "would-build", StepsRun: 0, StepsTotal: 2}}
	if !reflect.DeepEqual(doc.Packages, want) {
		t.Fatalf("results = %+v", doc.Packages)
	}
}

func TestExecWrapperUsageErrors(t *testing.T) {
	root := kilnFixture(t, map[string]string{"demo": "echo a\n"})
	base := []string{"-root", root, "-force", "-dry-run", "-validate-build-files=false"}
	cases := []struct {
		name string
		args []string
		want string
	}{
		{"empty value", []string{"-exec-wrapper", ""}, "JSON array"},
		{"not json", []string{"-exec-wrapper", "strace -f"}, "JSON array"},
		{"not strings", []string{"-exec-wrapper", `[1,2]`}, "JSON array"},
		{"empty array", []string{"-exec-wrapper", `[]`}, "empty array"},
		{"empty element", []string{"-exec-wrapper", `["a",""]`}, "element 1 is empty"},
		{"NUL element", []string{"-exec-wrapper", `["a\u0000b"]`}, "NUL"},
		{"trailing data", []string{"-exec-wrapper", `["a"] ["b"]`}, "trailing data"},
		{"with clippy", []string{"-exec-wrapper", `["a"]`, "-clippy"}, "cannot be combined"},
		{"with prune", []string{"-exec-wrapper", `["a"]`, "-prune-leaf-targets"}, "cannot be combined"},
		{"empty emit-results", []string{"-emit-results", ""}, "needs a path"},
	}
	for _, tc := range cases {
		t.Run(tc.name, func(t *testing.T) {
			code, stderr := runBuildTool(t, append(append([]string{}, base...), tc.args...)...)
			if code != 1 || !strings.Contains(stderr, tc.want) {
				t.Fatalf("exit %d, stderr %q; want exit 1 mentioning %q", code, stderr, tc.want)
			}
		})
	}
}

func TestEmitResultsRefusesSymlinkOutput(t *testing.T) {
	if runtime.GOOS == "windows" {
		t.Skip("symlink creation needs privileges on Windows")
	}
	root := kilnFixture(t, map[string]string{"demo": "echo a\n"})
	dir := t.TempDir()
	target := filepath.Join(dir, "target.json")
	if err := os.WriteFile(target, []byte("keep"), 0o644); err != nil {
		t.Fatal(err)
	}
	link := filepath.Join(dir, "link.json")
	if err := os.Symlink(target, link); err != nil {
		t.Fatal(err)
	}
	code, stderr := runBuildTool(t, "-root", root, "-force", "-validate-build-files=false", "-emit-results", link)
	if code != 1 || !strings.Contains(stderr, "symlink") {
		t.Fatalf("exit %d, stderr %q", code, stderr)
	}
	if data, _ := os.ReadFile(target); string(data) != "keep" {
		t.Fatalf("symlink target was overwritten: %q", data)
	}
	// The writer refuses too, independent of the early check.
	if err := writeKilnResults(nil, root, nil, link); err == nil || !strings.Contains(err.Error(), "symlink") {
		t.Fatalf("writeKilnResults through symlink: %v", err)
	}
}

func TestCheckResultsPathRefusesNonRegular(t *testing.T) {
	if err := checkResultsPath(t.TempDir()); err == nil {
		t.Fatal("a directory output path must be refused")
	}
}

func TestBuildKilnResultsSortedAndUnsupportedTotals(t *testing.T) {
	root := t.TempDir()
	pk := func(name string, cmds ...string) discovery.Package {
		return discovery.Package{Name: name, Path: filepath.Join(root, name), BuildCommands: cmds}
	}
	pkgs := []discovery.Package{pk("z", "a"), pk("b", "a", "b"), pk("m", "a")}
	res := map[string]executor.BuildResult{
		"z": {Status: "built", StepsRun: 1, StepsTotal: 1},
		"b": {Status: "dep-skipped"},
		// "m" absent: not listed.
	}
	doc, err := buildKilnResults(pkgs, root, res)
	if err != nil {
		t.Fatal(err)
	}
	want := []kilnResultPackage{
		{Package: "b", Status: "dep-skipped", StepsRun: 0, StepsTotal: 2},
		{Package: "z", Status: "built", StepsRun: 1, StepsTotal: 1},
	}
	if !reflect.DeepEqual(doc.Packages, want) {
		t.Fatalf("got %+v", doc.Packages)
	}
}
