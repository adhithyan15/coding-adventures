package main

// The one strace failure the real-strace sandbox tests retry, and how they decide.
//
// strace has a known race on a dying thread: when a thread of the traced group
// is killed (exit_group) while strace is moving it out of a group-stop, strace's
// own PTRACE_LISTEN call fails and strace exits 1 with exactly
//
//	/usr/bin/strace: ptrace(PTRACE_LISTEN,pid:NNNN,sig:0): Input/output error
//
// It hits strace 6.8 and 6.19 alike.  The recorder already fails CLOSED on it in
// production (the step is not recorded; the census counts it under the closed
// code ptrace-listen-eio), so nothing about the race is hidden.  What it does to
// a TEST is different: TestSandboxStraceDyingThreads kills twelve workers twenty
// times per run, and CI repeats it with -count=5, so one CI run exposes the test
// to the race a hundred times.  About one CI run in six hit it, on unrelated
// branches, and the failure said nothing about Kiln.
//
// So the helper that runs strace classifies strace's OWN failure before the test
// asserts anything about the trace, and re-runs the whole step from a fresh
// fixture when, and only when, the failure is that race.  This file holds the
// decision as a pure function (it needs no Linux, so a table test pins it on
// every OS).  What stays a hard failure, unchanged:
//
//   - any strace failure whose stderr is not exactly the race (a different
//     ClassifyStraceStderr code, an extra line, empty stderr);
//   - a strace that exits 0 but wrote to stderr, and a strace killed by a signal;
//   - the third race hit in a row (stracePtraceListenAttempts attempts in all).
//
// Nothing here turns a failure into a skip, and neither the trace parser's
// rules nor the dying-shape list are touched: a retried run is judged exactly
// like a first run.

import (
	"bytes"

	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/record"
)

// stracePtraceListenAttempts is how many times in all a step is run when strace
// keeps hitting its PTRACE_LISTEN race: the first run and two retries.  Three in
// a row is not the known one-in-a-few race any more; it fails the test.
const stracePtraceListenAttempts = 3

// stracePtraceListenRace reports whether a strace run that ended with exit code
// exitCode (-1 when a signal killed it) and standard error stderr failed ONLY
// because of strace's PTRACE_LISTEN race.  It reuses record.ClassifyStraceStderr
// (the census's classifier, one source of truth for what the message is) and is
// stricter than it: the classifier matches by substring, so each non-blank line
// must itself be the race message, and the exit status must be strace's own 1.
func stracePtraceListenRace(exitCode int, stderr []byte) bool {
	if exitCode != 1 {
		return false
	}
	if record.ClassifyStraceStderr(stderr, 0) != record.StraceErrListenEIO {
		return false
	}
	seen := false
	for _, line := range bytes.Split(stderr, []byte("\n")) {
		line = bytes.TrimSpace(line)
		if len(line) == 0 {
			continue
		}
		if !bytes.Contains(line, []byte("ptrace(PTRACE_LISTEN")) || !bytes.HasSuffix(line, []byte("Input/output error")) {
			return false
		}
		seen = true
	}
	return seen
}

// straceShouldRetry decides whether attempt number attempt (1-based) is run
// again: only when it hit the race and an attempt remains.
func straceShouldRetry(attempt int, race bool) bool {
	return race && attempt < stracePtraceListenAttempts
}
