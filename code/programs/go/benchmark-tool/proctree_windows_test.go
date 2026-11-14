//go:build windows

package main

import (
	"os"
	"testing"
	"time"
	"unsafe"
)

// treeCommand starts a long-lived grandchild that inherits the service log as its stderr.
func treeCommand(_ string) string { return `ping -n 600 127.0.0.1` }

// waitForTreeChild gives cmd.exe time to start the child before it is stopped.
func waitForTreeChild(t *testing.T, _ string) { time.Sleep(1500 * time.Millisecond) }

// assertTreeKilled uses the symptom the real failure had: a surviving child keeps the
// log open, so the log cannot be deleted. Once the whole tree is dead it can.
func assertTreeKilled(t *testing.T, _ string, logPath string) {
	t.Helper()
	deadline := time.Now().Add(10 * time.Second)
	var err error
	for time.Now().Before(deadline) {
		if err = os.Remove(logPath); err == nil {
			return
		}
		time.Sleep(50 * time.Millisecond)
	}
	t.Fatalf("the service log is still held open after Stop (a child survived): %v", err)
}

// The Job Object structure is passed to the kernel by size; a wrong layout would silently
// not set KILL_ON_JOB_CLOSE. JOBOBJECT_EXTENDED_LIMIT_INFORMATION is 144 bytes on x64.
func TestJobObjectLayoutMatchesWindows(t *testing.T) {
	if unsafe.Sizeof(uintptr(0)) != 8 {
		t.Skip("layout is checked for 64-bit Windows")
	}
	if got := unsafe.Sizeof(jobExtendedLimit{}); got != 144 {
		t.Fatalf("sizeof(jobExtendedLimit) = %d, want 144", got)
	}
	if got := unsafe.Sizeof(jobBasicLimit{}); got != 64 {
		t.Fatalf("sizeof(jobBasicLimit) = %d, want 64", got)
	}
}
