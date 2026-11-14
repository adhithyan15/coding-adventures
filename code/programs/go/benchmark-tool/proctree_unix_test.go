//go:build !windows

package main

import (
	"os"
	"path/filepath"
	"strconv"
	"strings"
	"syscall"
	"testing"
	"time"
)

// treeCommand starts a long-lived grandchild (the "server") and records its pid.
func treeCommand(dir string) string {
	return `sleep 600 & echo $! > ` + filepath.Join(dir, "child.pid") + `; wait`
}

func readChildPID(t *testing.T, dir string) int {
	t.Helper()
	deadline := time.Now().Add(10 * time.Second)
	for time.Now().Before(deadline) {
		if raw, err := os.ReadFile(filepath.Join(dir, "child.pid")); err == nil {
			if pid, err := strconv.Atoi(strings.TrimSpace(string(raw))); err == nil && pid > 0 {
				return pid
			}
		}
		time.Sleep(10 * time.Millisecond)
	}
	t.Fatal("the grandchild never recorded its pid")
	return 0
}

func waitForTreeChild(t *testing.T, dir string) { readChildPID(t, dir) }

// assertTreeKilled requires the grandchild to be gone (or a zombie awaiting reaping, which
// is what a killed child looks like when nothing in the container reaps orphans).
func assertTreeKilled(t *testing.T, dir, _ string) {
	t.Helper()
	pid := readChildPID(t, dir)
	deadline := time.Now().Add(10 * time.Second)
	for time.Now().Before(deadline) {
		if syscall.Kill(pid, 0) == syscall.ESRCH {
			return
		}
		if raw, err := os.ReadFile("/proc/" + strconv.Itoa(pid) + "/stat"); err == nil {
			if i := strings.LastIndex(string(raw), ") "); i >= 0 && strings.HasPrefix(string(raw)[i+2:], "Z") {
				return
			}
		}
		time.Sleep(20 * time.Millisecond)
	}
	t.Fatalf("grandchild %d is still running after Stop", pid)
}
