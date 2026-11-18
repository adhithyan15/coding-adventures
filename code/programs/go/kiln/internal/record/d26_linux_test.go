//go:build linux

package record

// D26 on the real kernel: the sealed value (this process's own start time) and
// the comparands (other processes' start times) are one field of one file, in
// one unit, on one epoch. The tests below pin that, which no fake /proc can.

import (
	"fmt"
	"os"
	"os/exec"
	"testing"
)

// startOfChild starts `sleep 1` and returns its start time as the S3 gate reads
// it, /proc/<pid>/stat through startTicks.
func startOfChild(t *testing.T) int64 {
	t.Helper()
	child := exec.Command("sleep", "1")
	if err := child.Start(); err != nil {
		t.Skipf("no sleep: %v", err)
	}
	t.Cleanup(func() { child.Process.Kill(); child.Wait() })
	stat, err := os.ReadFile(fmt.Sprintf("/proc/%d/stat", child.Process.Pid))
	if err != nil {
		t.Fatal(err)
	}
	got, err := startTicks(stat)
	if err != nil {
		t.Fatal(err)
	}
	return got
}

func TestSelfStartTicksAndAChildAreOnTheSameClock(t *testing.T) {
	self, err := ReadSelfStartTicks()
	if err != nil {
		t.Fatal(err)
	}
	got := startOfChild(t)
	// The child started after we did, and not by a wild margin: a test binary
	// lives seconds, not minutes, before it forks `sleep`. A units or epoch
	// mismatch (seconds against ticks, uptime against boot) would miss by far
	// more than this.
	if got < self {
		t.Fatalf("a child started at tick %d, before its parent at %d", got, self)
	}
	if got-self > 30*60*ClockTicksPerSecond {
		t.Fatalf("a child started %d ticks after its parent", got-self)
	}
	if again, _ := ReadSelfStartTicks(); again != self {
		t.Fatalf("our own start time changed: %d then %d", self, again)
	}
}

// The S3 bound derived from our own start counts a process started after us and
// (when the host has an older pid 1) does not count one started long before.
func TestStartBoundSeparatesBeforeFromAfterOnTheRealKernel(t *testing.T) {
	self, err := ReadSelfStartTicks()
	if err != nil {
		t.Fatal(err)
	}
	bound := self - SurvivorSlackTicks
	if got := startOfChild(t); got < bound {
		t.Fatalf("a process started after prepare falls before the bound: %d < %d", got, bound)
	}
	init1, err := os.ReadFile("/proc/1/stat")
	if err != nil {
		t.Skip("cannot read /proc/1/stat")
	}
	t1, err := startTicks(init1)
	if err != nil || t1 >= bound {
		t.Skipf("pid 1 is not older than this test by the slack (start %d, bound %d): %v", t1, bound, err)
	}
}
