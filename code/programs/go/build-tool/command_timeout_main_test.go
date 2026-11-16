package main

import (
	"runtime"
	"strings"
	"testing"
	"time"

	"github.com/adhithyan15/coding-adventures/code/programs/go/build-tool/internal/executor"
)

// The -command-timeout flag: negative values are a usage error, and a real run
// with a tiny limit fails the package (exit 1) and names the hung command.

// resetCommandTimeout stops one run's -command-timeout from leaking into the
// next test through the executor's package-level setting.
func resetCommandTimeout(t *testing.T) {
	t.Helper()
	t.Cleanup(func() { executor.CommandTimeout = 0 })
}

func TestRunRejectsNegativeCommandTimeout(t *testing.T) {
	root := kilnFixture(t, map[string]string{"alpha": "echo hi\n"})
	code, stderr := runBuildTool(t, "-root", root, "-force", "-validate-build-files=false", "-command-timeout=-1s")
	if code != 1 {
		t.Fatalf("exit = %d, want 1; stderr=%s", code, stderr)
	}
	if !strings.Contains(stderr, "-command-timeout") {
		t.Errorf("stderr does not mention the flag: %s", stderr)
	}
}

func TestRunCommandTimeoutFailsHungPackage(t *testing.T) {
	if runtime.GOOS == "windows" {
		t.Skip("BUILD fixture uses POSIX sleep")
	}
	resetCommandTimeout(t)
	root := kilnFixture(t, map[string]string{"hang": "echo started\nsleep 60\n"})
	began := time.Now()
	code, stderr := runBuildTool(t, "-root", root, "-force", "-validate-build-files=false", "-command-timeout=400ms")
	if took := time.Since(began); took > 30*time.Second {
		t.Fatalf("run took %v; the timeout did not bound it", took)
	}
	if code == 0 {
		t.Fatalf("a timed-out package must fail the run; stderr=%s", stderr)
	}
	if executor.CommandTimeout != 400*time.Millisecond {
		t.Errorf("executor.CommandTimeout = %v, want 400ms", executor.CommandTimeout)
	}
}

func TestRunDefaultCommandTimeoutIsInstalled(t *testing.T) {
	resetCommandTimeout(t)
	root := kilnFixture(t, map[string]string{"alpha": "echo hi\n"})
	if code, stderr := runBuildTool(t, "-root", root, "-force", "-dry-run", "-validate-build-files=false"); code != 0 {
		t.Fatalf("exit = %d; stderr=%s", code, stderr)
	}
	if executor.CommandTimeout != executor.DefaultCommandTimeout {
		t.Errorf("executor.CommandTimeout = %v, want default %v", executor.CommandTimeout, executor.DefaultCommandTimeout)
	}
}
