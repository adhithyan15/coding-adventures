//go:build !windows

package main

import (
	"os"
	"strings"
	"syscall"
	"testing"
	"time"
)

// A real SIGINT delivered to the process during a run kills the hung BUILD
// command and makes the run exit 130, never 0.
func TestRunInterruptExitsNonZero(t *testing.T) {
	resetCommandTimeout(t)
	root := kilnFixture(t, map[string]string{"hang": "sleep 60\n"})
	go func() {
		time.Sleep(700 * time.Millisecond)
		_ = syscall.Kill(os.Getpid(), syscall.SIGINT)
	}()
	began := time.Now()
	code, stderr := runBuildTool(t, "-root", root, "-force", "-validate-build-files=false")
	if took := time.Since(began); took > 30*time.Second {
		t.Fatalf("interrupt did not stop the run: %v", took)
	}
	if code != 130 || !strings.Contains(stderr, "Interrupted") {
		t.Fatalf("exit = %d, stderr = %s; want 130 and an Interrupted notice", code, stderr)
	}
}
