package main

import (
	"bytes"
	"fmt"
	"regexp"
	"strconv"
	"strings"
	"testing"

	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/trace"
)

var (
	bareOpenatUnknownResume = regexp.MustCompile(`^([0-9]+) +<\.\.\. openat resumed>\) = \?$`)
	traceSpawnResult        = regexp.MustCompile(`^([0-9]+) +(clone3?|fork|vfork)\(.*\) = ([0-9]+)(?: /\* ([0-9]+) in strace's PID NS \*/)?$`)
	traceTerminalPrefix     = regexp.MustCompile(`^[0-9]+ +\+\+\+ `)
)

// laterBareOpenatExit returns the line number of a parsed terminal exit for
// the exact refused openat PID, or zero if the refusal cannot be tied to the
// process's terminal event. This is a test oracle only; the production parser
// still refuses the incomplete trace.
func laterBareOpenatExit(raw []byte, refusal *trace.Error) int {
	if refusal == nil || (refusal.Reason != "unresumed-call" && refusal.Reason != "unparsable-line" && refusal.Reason != "result") || len(raw) == 0 {
		return 0
	}
	if refusal.Line < 1 {
		return 0
	}
	target := bareOpenatUnknownResume.FindSubmatch(traceLineAt(raw, refusal.Line))
	if target == nil {
		return 0
	}
	pid, err := strconv.Atoi(string(target[1]))
	if err != nil || pid <= 0 {
		return 0
	}
	terminalLine := 0
	visitTraceLines(raw, refusal.Line+1, func(lineNo int, line []byte) bool {
		if spawnedTracePID(line) == pid {
			// A later process generation makes this numeric PID ambiguous.
			return false
		}
		if !traceTerminalPrefix.Match(line) {
			return true
		}
		if parsedTerminalTracePID(line) == pid {
			terminalLine = lineNo
			return false
		}
		return true
	})
	return terminalLine
}

// spawnedTracePID reads only the exact numeric spawn result and optional
// namespace translation that strace prints. It is used solely to reject an
// ambiguous PID generation in this test oracle.
func spawnedTracePID(line []byte) int {
	match := traceSpawnResult.FindSubmatch(line)
	if match == nil {
		return 0
	}
	parent, err := strconv.Atoi(string(match[1]))
	if err != nil || parent <= 0 {
		return 0
	}
	value := match[3]
	if len(match) > 4 && len(match[4]) != 0 {
		value = match[4]
	}
	pid, err := strconv.Atoi(string(value))
	if err != nil || pid <= 0 {
		return 0
	}
	// Validate the candidate with the production trace parser in an isolated
	// minimal parent/child trace. A substring that merely resembles a spawn
	// result is not PID-generation evidence.
	mini := fmt.Sprintf("%s\n%d +++ exited with 0 +++\n%d +++ exited with 0 +++\n", line, parent, pid)
	parsed, err := trace.Parse(strings.NewReader(mini), "/", trace.Limits{})
	if err != nil || parsed == nil || parsed.Procs[pid] == nil || parsed.Procs[pid].Parent != parent {
		return 0
	}
	return pid
}

// parsedTerminalTracePID uses the actual trace parser on the single candidate
// terminal record, so arbitrary lines containing an exit-looking substring do
// not become evidence for the test oracle.
func parsedTerminalTracePID(line []byte) int {
	parsed, err := trace.Parse(bytes.NewReader(append(bytes.Clone(line), '\n')), "/", trace.Limits{})
	if err != nil || parsed == nil || len(parsed.Exits) != 1 {
		return 0
	}
	return parsed.Exits[0].Pid
}

// dyingRefusalDiagnostic prints only the refused line and the first later
// same-PID terminal candidate. Each line and the overall output are bounded.
func dyingRefusalDiagnostic(raw []byte, refusalLine, pid int) string {
	var out strings.Builder
	appendLine := func(i int, line []byte) {
		if len(line) > 300 {
			line = append(bytes.Clone(line[:300]), []byte("...")...)
		}
		fmt.Fprintf(&out, "%d: %s\n", i+1, line)
	}
	if refusalLine > 0 {
		if line := traceLineAt(raw, refusalLine); line != nil {
			appendLine(refusalLine-1, line)
		}
	}
	visitTraceLines(raw, refusalLine+1, func(lineNo int, line []byte) bool {
		if traceTerminalPrefix.Match(line) && linePID(line) == pid {
			appendLine(lineNo-1, line)
			return false
		}
		return true
	})
	if out.Len() == 0 {
		return "no refusal/terminal context available"
	}
	return out.String()
}

func traceLineAt(raw []byte, wanted int) []byte {
	if wanted < 1 {
		return nil
	}
	var found []byte
	visitTraceLines(raw, wanted, func(lineNo int, line []byte) bool {
		if lineNo == wanted {
			found = line
			return false
		}
		return true
	})
	return found
}

// visitTraceLines visits newline-delimited lines from the requested 1-based
// line number without retaining a second slice for every trace line.
func visitTraceLines(raw []byte, startLine int, visit func(int, []byte) bool) {
	if startLine < 1 {
		return
	}
	lineNo, start := 1, 0
	for start < len(raw) {
		relativeEnd := bytes.IndexByte(raw[start:], '\n')
		end := len(raw)
		next := len(raw)
		if relativeEnd >= 0 {
			end = start + relativeEnd
			next = end + 1
		}
		if lineNo >= startLine && !visit(lineNo, raw[start:end]) {
			return
		}
		lineNo++
		start = next
	}
}

func linePID(line []byte) int {
	space := bytes.IndexByte(line, ' ')
	if space <= 0 {
		return 0
	}
	pid, err := strconv.Atoi(string(line[:space]))
	if err != nil || pid <= 0 {
		return 0
	}
	return pid
}

func TestLaterBareOpenatExitRequiresExactRefusalAndPID(t *testing.T) {
	const (
		pid       = 202
		otherPID  = 303
		parentPID = 101
	)
	valid := fmt.Sprintf("%d <... openat resumed>) = ?\n%d +++ exited with 0 +++\n", pid, pid)
	validKilled := fmt.Sprintf("%d <... openat resumed>) = ?\n%d +++ killed by SIGKILL +++\n", pid, pid)
	validAfterReuse := fmt.Sprintf("%d +++ exited with 0 +++\n%d fork() = %d\n%d <... openat resumed>) = ?\n%d +++ exited with 0 +++\n", pid, parentPID, pid, pid, pid)
	interveningReuse := fmt.Sprintf("%d <... openat resumed>) = ?\n%d fork() = %d\n%d +++ exited with 0 +++\n", pid, parentPID, pid, pid)

	cases := []struct {
		name     string
		raw      string
		reason   string
		line     int
		wantLine int
	}{
		{name: "later same-pid parsed exit", raw: valid, reason: "unparsable-line", line: 1, wantLine: 2},
		{name: "later same-pid parsed signal exit", raw: validKilled, reason: "unresumed-call", line: 1, wantLine: 2},
		{name: "old pid generation does not count", raw: validAfterReuse, reason: "unresumed-call", line: 3, wantLine: 4},
		{name: "different pid exit", raw: fmt.Sprintf("%d <... openat resumed>) = ?\n%d +++ exited with 0 +++\n", pid, otherPID), reason: "unresumed-call", line: 1},
		{name: "prior exit only", raw: fmt.Sprintf("%d +++ exited with 0 +++\n%d <... openat resumed>) = ?\n", pid, pid), reason: "unresumed-call", line: 2},
		{name: "missing exit", raw: fmt.Sprintf("%d <... openat resumed>) = ?\n", pid), reason: "unresumed-call", line: 1},
		{name: "intervening same-pid generation", raw: interveningReuse, reason: "unresumed-call", line: 1},
		{name: "non-resumed call", raw: fmt.Sprintf("%d openat(AT_FDCWD, \"x\", O_RDONLY) = ?\n%d +++ exited with 0 +++\n", pid, pid), reason: "unresumed-call", line: 1},
		{name: "other syscall", raw: fmt.Sprintf("%d <... read resumed>) = ?\n%d +++ exited with 0 +++\n", pid, pid), reason: "unresumed-call", line: 1},
		{name: "arbitrary unknown syscall", raw: fmt.Sprintf("%d <... mystery resumed>) = ?\n%d +++ exited with 0 +++\n", pid, pid), reason: "unparsable-line", line: 1},
		{name: "trailing text", raw: fmt.Sprintf("%d <... openat resumed>) = ? unexpected\n%d +++ exited with 0 +++\n", pid, pid), reason: "unresumed-call", line: 1},
		{name: "wrong refusal reason", raw: valid, reason: "size", line: 1},
		{name: "out of range refusal line", raw: valid, reason: "unresumed-call", line: 0},
		{name: "unrelated unknown shape", raw: fmt.Sprintf("%d <... openat resumed>) = -1 (errno 999)\n%d +++ exited with 0 +++\n", pid, pid), reason: "result", line: 1},
	}
	for _, tc := range cases {
		t.Run(tc.name, func(t *testing.T) {
			got := laterBareOpenatExit([]byte(tc.raw), &trace.Error{Reason: tc.reason, Line: tc.line})
			if got != tc.wantLine {
				t.Fatalf("laterBareOpenatExit() = %d, want %d", got, tc.wantLine)
			}
		})
	}
}

func TestLaterBareOpenatExitRejectsMalformedTerminal(t *testing.T) {
	const pid = 202
	cases := []struct {
		name string
		raw  string
	}{
		{name: "malformed terminal", raw: fmt.Sprintf("%d <... openat resumed>) = ?\n%d +++ exited with nonsense +++\n", pid, pid)},
		{name: "terminal trailing junk", raw: fmt.Sprintf("%d <... openat resumed>) = ?\n%d +++ exited with 0 +++ extra\n", pid, pid)},
	}
	for _, tc := range cases {
		t.Run(tc.name, func(t *testing.T) {
			if got := laterBareOpenatExit([]byte(tc.raw), &trace.Error{Reason: "unparsable-line", Line: 1}); got != 0 {
				t.Fatalf("laterBareOpenatExit() = %d, want 0", got)
			}
		})
	}
}

func TestTraceSpawnPIDHonorsNamespaceTranslation(t *testing.T) {
	if got := spawnedTracePID([]byte("101 clone(child_stack=NULL, flags=CLONE_VM|CLONE_VFORK|SIGCHLD) = 2 /* 202 in strace's PID NS */")); got != 202 {
		t.Fatalf("spawnedTracePID() = %d, want translated host pid 202", got)
	}
	if got := spawnedTracePID([]byte("101 fork() = 202")); got != 202 {
		t.Fatalf("spawnedTracePID() = %d, want 202", got)
	}
	for _, line := range []string{
		"101 fork() = -1 EAGAIN (Resource temporarily unavailable)",
		"101 fork() = 0",
		"101 openat(...) = 202",
		"101 fork() = 2 /* malformed */",
	} {
		if got := spawnedTracePID([]byte(line)); got != 0 {
			t.Errorf("spawnedTracePID(%q) = %d, want 0", line, got)
		}
	}
}

func TestLaterBareOpenatExitDiagnosticsAreBounded(t *testing.T) {
	raw := []byte("202 " + strings.Repeat("x", 2000) + "\n202 +++ exited with 0 +++ " + strings.Repeat("y", 2000) + "\n")
	diagnostic := dyingRefusalDiagnostic(raw, 1, 202)
	if len(diagnostic) > 700 || !strings.Contains(diagnostic, "1: ") || !strings.Contains(diagnostic, "2: ") || !strings.Contains(diagnostic, "...") {
		t.Fatalf("diagnostic was not bounded refusal/terminal context: %q", diagnostic)
	}
}
