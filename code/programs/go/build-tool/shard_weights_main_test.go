package main

import (
	"flag"
	"io"
	"os"
	"path/filepath"
	"strings"
	"testing"

	"github.com/adhithyan15/coding-adventures/code/programs/go/build-tool/internal/plan"
)

// runShardWeightsCLI runs the real flag path with -shard-weights against a
// two-package repo and returns the exit code, stderr and the written plan.
func runShardWeightsCLI(t *testing.T, weightsBody string) (int, string, *plan.BuildPlan) {
	t.Helper()
	root := t.TempDir()
	for _, name := range []string{"slow", "fast"} {
		dir := filepath.Join(root, "code", "packages", "go", name)
		if err := os.MkdirAll(dir, 0o755); err != nil {
			t.Fatal(err)
		}
		if err := os.WriteFile(filepath.Join(dir, "BUILD"), []byte("go test ./...\n"), 0o644); err != nil {
			t.Fatal(err)
		}
		if err := os.WriteFile(filepath.Join(dir, "go.mod"), []byte("module example.com/"+name+"\n\ngo 1.26\n"), 0o644); err != nil {
			t.Fatal(err)
		}
	}
	weightsArg := filepath.Join("code", "durations.json") // repo-relative on purpose
	if weightsBody != "" {
		if err := os.WriteFile(filepath.Join(root, weightsArg), []byte(weightsBody), 0o644); err != nil {
			t.Fatal(err)
		}
	}
	planPath := filepath.Join(root, "plan.json")

	originalArgs, originalFlags, originalStderr, originalStdout := os.Args, flag.CommandLine, os.Stderr, os.Stdout
	reader, writer, err := os.Pipe()
	if err != nil {
		t.Fatal(err)
	}
	devnull, err := os.OpenFile(os.DevNull, os.O_WRONLY, 0)
	if err != nil {
		t.Fatal(err)
	}
	t.Cleanup(func() {
		os.Args, flag.CommandLine, os.Stderr, os.Stdout = originalArgs, originalFlags, originalStderr, originalStdout
		reader.Close()
		writer.Close()
		devnull.Close()
	})
	flag.CommandLine = flag.NewFlagSet("build-tool-test", flag.ContinueOnError)
	os.Args = []string{
		"build-tool", "-root", root, "-force", "-validate-build-files=false", "-ci-gates", "",
		"-emit-plan", planPath, "-shard-count", "2", "-shard-weights", weightsArg,
	}
	os.Stderr = writer
	os.Stdout = devnull

	code := run()
	writer.Close()
	os.Stderr, os.Stdout = originalStderr, originalStdout
	stderr, _ := io.ReadAll(reader)

	var bp *plan.BuildPlan
	if code == 0 {
		if bp, err = plan.Read(planPath); err != nil {
			t.Fatal(err)
		}
	}
	return code, string(stderr), bp
}

func TestShardWeightsFlagSelectsMeasuredSharding(t *testing.T) {
	code, stderr, bp := runShardWeightsCLI(t,
		`{"schema":"ci-package-durations-v1","unit":"seconds","durations":{"go/slow":600,"go/fast":5}}`)
	if code != 0 {
		t.Fatalf("exit %d: %s", code, stderr)
	}
	costs := map[string]int{}
	for _, shard := range bp.Shards {
		for _, name := range shard.AssignedPackages {
			costs[name] = shard.EstimatedCost
		}
	}
	// v1 would cost both at heuristic 2; v2 costs shards in measured seconds.
	if costs["go/slow"] != 600 || costs["go/fast"] != 5 {
		t.Fatalf("expected measured second costs, got %v (%#v)", costs, bp.Shards)
	}
}

func TestShardWeightsFlagFailsLoudlyOnABadFile(t *testing.T) {
	code, stderr, _ := runShardWeightsCLI(t, "")
	if code != 1 || !strings.Contains(stderr, "SHARD_WEIGHTS_UNREADABLE") {
		t.Fatalf("missing weights file: exit %d, stderr %q", code, stderr)
	}
	code, stderr, _ = runShardWeightsCLI(t, `{"schema":"nope"}`)
	if code != 1 || !strings.Contains(stderr, "SHARD_WEIGHTS_INVALID") {
		t.Fatalf("invalid weights file: exit %d, stderr %q", code, stderr)
	}
}
