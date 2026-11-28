package main

import (
	"bytes"
	"crypto/sha256"
	"fmt"
	"regexp"
	"strconv"
	"strings"
	"testing"

	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/trace"
)

var (
	bareOpenatUnknownResume = regexp.MustCompile(`^([0-9]+) +<\.\.\. openat resumed>\) = \?$`)
	bareGetdentsUnknown     = regexp.MustCompile(`^([0-9]+) +getdents64\(.*\) = \? <unavailable>$`)
	traceSpawnResult        = regexp.MustCompile(`^([0-9]+) +(clone3?|fork|vfork)\(.*\) = ([0-9]+)(?: /\* ([0-9]+) in strace's PID NS \*/)?$`)
	traceSpawnStart         = regexp.MustCompile(`^([0-9]+) +(clone3?|fork|vfork)\(.* <unfinished \.\.\.>$`)
	traceSpawnResume        = regexp.MustCompile(`^([0-9]+) +<\.\.\. (clone3?|fork|vfork) resumed>(.*)$`)
	traceSpawnResumeResult  = regexp.MustCompile(`(?:\)|, [0-9]+\)) = ([0-9]+)(?: /\* ([0-9]+) in strace's PID NS \*/)?$`)
	traceTerminalPrefix     = regexp.MustCompile(`^[0-9]+ +\+\+\+ `)
)

// laterBareOpenatExit returns the line number of a parsed terminal exit for
// the exact refused openat PID, or zero if the refusal cannot be tied to the
// process's terminal event. This is a test oracle only; the production parser
// still refuses the incomplete trace.
func laterBareOpenatExit(raw []byte, refusal *trace.Error) int {
	return laterUnknownCallExit(raw, refusal, bareOpenatUnknownResume)
}

// laterBareGetdentsExit returns the line number of a parsed terminal exit for
// the exact refused getdents64 PID. This is a test oracle only: directory
// reads remain refused by the production parser because the trace omits their
// result and directory contents affect the package's observed inputs.
func laterBareGetdentsExit(raw []byte, refusal *trace.Error) int {
	return laterUnknownCallExit(raw, refusal, bareGetdentsUnknown)
}

// laterUnknownCallExit returns a later parsed terminal event only when it
// belongs to the exact PID generation whose unknown call was refused. The
// narrow target pattern is supplied by the caller; all process-generation
// checks are shared so this test oracle cannot accept an unrelated or reused
// numeric PID.
func laterUnknownCallExit(raw []byte, refusal *trace.Error, targetPattern *regexp.Regexp) int {
	if refusal == nil || (refusal.Reason != "unresumed-call" && refusal.Reason != "unparsable-line" && refusal.Reason != "result") || len(raw) == 0 {
		return 0
	}
	if refusal.Line < 1 {
		return 0
	}
	target := targetPattern.FindSubmatch(traceLineAt(raw, refusal.Line))
	if target == nil {
		return 0
	}
	pid, err := strconv.Atoi(string(target[1]))
	if err != nil || pid <= 0 {
		return 0
	}
	type spawnKey struct {
		pid  string
		name string
	}
	pending := make(map[spawnKey][][]byte)
	terminalLine := 0
	ambiguous := false
	refusalSeen := false
	visitTraceLines(raw, 1, func(lineNo int, line []byte) bool {
		if lineNo == refusal.Line {
			refusalSeen = true
			return true
		}
		if start := traceSpawnStart.FindSubmatch(line); start != nil {
			key := spawnKey{string(start[1]), string(start[2])}
			pending[key] = append(pending[key], line)
			return true
		}
		if resumed := traceSpawnResume.FindSubmatch(line); resumed != nil {
			key := spawnKey{string(resumed[1]), string(resumed[2])}
			starts := pending[key]
			delete(pending, key)
			for _, start := range starts {
				if parsedSplitSpawnPID(start, line) == pid && refusalSeen {
					ambiguous = true
				}
			}
			if ambiguous {
				return false
			}
			return true
		}
		if refusalSeen && spawnedTracePID(line) == pid {
			// A later process generation makes this numeric PID ambiguous.
			ambiguous = true
			return false
		}
		if !refusalSeen || !traceTerminalPrefix.Match(line) {
			return true
		}
		if parsedTerminalTracePID(line) == pid {
			if !ambiguous {
				terminalLine = lineNo
			}
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
	if bytes.Equal(match[2], []byte("clone3")) {
		if !parsedRefusedClone3(line, nil) {
			return 0
		}
		return parsedSpawnPID([]byte(fmt.Sprintf("%d clone(child_stack=NULL, flags=SIGCHLD) = %d", parent, pid)), nil, parent, pid)
	}
	return parsedSpawnPID(line, nil, parent, pid)
}

func dyingTraceDigest(raw []byte) string {
	digest := sha256.Sum256(raw)
	return fmt.Sprintf("trace_sha256=%x trace_bytes=%d", digest, len(raw))
}

// parsedSplitSpawnPID accepts only a matching unfinished/resumed spawn pair
// that the production parser turns into the expected parent/child processes.
func parsedSplitSpawnPID(start, resumed []byte) int {
	startMatch := traceSpawnStart.FindSubmatch(start)
	resumeMatch := traceSpawnResume.FindSubmatch(resumed)
	if startMatch == nil || resumeMatch == nil || !bytes.Equal(startMatch[1], resumeMatch[1]) || !bytes.Equal(startMatch[2], resumeMatch[2]) {
		return 0
	}
	parent, err := strconv.Atoi(string(startMatch[1]))
	if err != nil || parent <= 0 {
		return 0
	}
	result := traceSpawnResumeResult.FindSubmatch(resumeMatch[3])
	if result == nil {
		return 0
	}
	value := result[1]
	if len(result) > 2 && len(result[2]) != 0 {
		value = result[2]
	}
	child, err := strconv.Atoi(string(value))
	if err != nil || child <= 0 {
		return 0
	}
	if bytes.Equal(startMatch[2], []byte("clone3")) {
		if !parsedRefusedClone3(start, resumed) {
			return 0
		}
		return parsedSpawnPID([]byte(fmt.Sprintf("%d clone(child_stack=NULL, flags=SIGCHLD) = %d", parent, child)), nil, parent, child)
	}
	return parsedSpawnPID(start, resumed, parent, child)
}

// clone3 is intentionally refused by the production policy. Parsing that
// refusal still proves the resumed record's syscall/result shape; a normalized
// equivalent clone is then parsed to verify its parent/child PID relationship.
func parsedRefusedClone3(start, resumed []byte) bool {
	mini := append(bytes.Clone(start), '\n')
	wantLine := 1
	if resumed != nil {
		mini = append(mini, resumed...)
		mini = append(mini, '\n')
		wantLine = 2
	}
	_, err := trace.Parse(bytes.NewReader(mini), "/", trace.Limits{})
	refusal, ok := err.(*trace.Error)
	return ok && refusal.Reason == "refused:clone3" && refusal.Line == wantLine
}

// parsedSpawnPID validates a spawn record with the production parser in an
// isolated minimal parent/child trace. A substring that merely resembles a
// spawn result is not PID-generation evidence.
func parsedSpawnPID(start, resumed []byte, parent, expected int) int {
	mini := append(bytes.Clone(start), '\n')
	if resumed != nil {
		mini = append(mini, resumed...)
		mini = append(mini, '\n')
	}
	if expected > 0 {
		mini = append(mini, []byte(fmt.Sprintf("%d +++ exited with 0 +++\n", expected))...)
	}
	mini = append(mini, []byte(fmt.Sprintf("%d +++ exited with 0 +++\n", parent))...)
	parsed, err := trace.Parse(bytes.NewReader(mini), "/", trace.Limits{})
	if err != nil || parsed == nil {
		return 0
	}
	if child := parsed.Procs[expected]; child != nil && expected != parent && child.Parent == parent {
		return expected
	}
	return 0
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

const (
	maxDyingDiagnosticEvents = 18
	maxDyingDiagnosticBytes  = 4096
)

type safeDyingEvent struct {
	line   int
	kind   string
	name   string
	result string
	link   string
}

func (event safeDyingEvent) String() string {
	text := fmt.Sprintf("line=%d event=%s", event.line, event.kind)
	if event.name != "" {
		text += " name=" + event.name
	}
	if event.result != "" {
		text += " result=" + event.result
	}
	if event.link != "" {
		text += " link=" + event.link
	}
	return text
}

func safeDyingRefusalReason(reason string) string {
	switch reason {
	case "unresumed-call", "unparsable-line", "result":
		return reason
	default:
		return "other"
	}
}

// summarizeDyingTraceLine converts one same-PID strace line to a small
// whitelist of event, syscall, result, and linkage labels. It never returns
// arguments, paths, environment values, or buffer contents.
func summarizeDyingTraceLine(lineNo int, line []byte) (safeDyingEvent, bool) {
	pid := linePID(line)
	if pid <= 0 {
		return safeDyingEvent{}, false
	}
	space := bytes.IndexByte(line, ' ')
	if space < 0 || space+1 >= len(line) {
		return safeDyingEvent{}, false
	}
	rest := strings.TrimSpace(string(line[space+1:]))
	event := safeDyingEvent{line: lineNo}
	switch {
	case strings.HasPrefix(rest, "+++ exited with "):
		fields := strings.Fields(rest)
		if len(fields) == 5 && fields[0] == "+++" && fields[1] == "exited" && fields[2] == "with" && fields[4] == "+++" {
			if fields[3] == "0" {
				event.kind, event.result = "terminal", "exit-status-zero"
			} else if _, err := strconv.ParseUint(fields[3], 10, 32); err == nil {
				event.kind, event.result = "terminal", "exit-status-nonzero"
			}
		}
		return event, event.kind != ""
	case strings.HasPrefix(rest, "+++ killed by "):
		fields := strings.Fields(rest)
		if len(fields) == 5 && fields[0] == "+++" && fields[1] == "killed" && fields[2] == "by" && fields[4] == "+++" && safeSignal(fields[3]) {
			event.kind, event.name = "terminal", fields[3]
		}
		return event, event.kind != ""
	case strings.HasPrefix(rest, "--- SIG"):
		nameEnd := strings.IndexByte(rest, ' ')
		if nameEnd > 4 && safeSignal(rest[4:nameEnd]) {
			event.kind, event.name = "signal", rest[4:nameEnd]
		}
		return event, event.kind != ""
	case strings.HasPrefix(rest, "<... "):
		end := strings.Index(rest, " resumed>")
		if end < 0 || !safeSyscallName(rest[5:end]) {
			return safeDyingEvent{}, false
		}
		event.kind, event.name = "resume", rest[5:end]
		event.result = traceResultClass(rest[end+len(" resumed>"):])
		return event, true
	default:
		open := strings.IndexByte(rest, '(')
		if open <= 0 || !safeSyscallName(rest[:open]) {
			return safeDyingEvent{}, false
		}
		event.kind, event.name = "call", rest[:open]
		event.result = traceResultClass(rest)
		return event, true
	}
}

func safeSyscallName(name string) bool {
	if name == "" || len(name) > 32 {
		return false
	}
	for _, c := range name {
		if !(c >= 'a' && c <= 'z' || c >= 'A' && c <= 'Z' || c >= '0' && c <= '9' || c == '_') {
			return false
		}
	}
	return true
}

func safeSignal(name string) bool {
	if !strings.HasPrefix(name, "SIG") || len(name) > 16 {
		return false
	}
	for _, c := range name {
		if !(c >= 'A' && c <= 'Z' || c >= '0' && c <= '9') {
			return false
		}
	}
	return true
}

func traceResultClass(text string) string {
	if strings.HasSuffix(text, "<unfinished ...>") {
		return "unfinished"
	}
	if strings.HasSuffix(text, "= ? <unavailable>") {
		return "unavailable"
	}
	if strings.HasSuffix(text, "= ?") {
		return "unknown"
	}
	if traceErrnoResult.MatchString(text) {
		return "error-errno"
	}
	if traceNumericResult.MatchString(text) {
		return "numeric"
	}
	return "other"
}

var (
	traceErrnoResult   = regexp.MustCompile(`\) = -1 (?:[A-Z][A-Z0-9]* \([^\n]*\)|\(errno [0-9]+\))$`)
	traceNumericResult = regexp.MustCompile(`\) = -?[0-9]+$`)
)

// dyingRefusalDiagnostic reports bounded, sanitized facts from the full trace.
// The numeric-PID terminal candidate is explicitly not a process-generation
// proof; this output is diagnostic only and never changes the refusal oracle.
func dyingRefusalDiagnostic(raw []byte, refusalLine, pid int) string {
	if pid <= 0 {
		return "diagnostic unavailable: invalid PID"
	}
	var before []safeDyingEvent
	var selected []safeDyingEvent
	pending := map[string]int{}
	terminalLine := 0
	terminalKind := ""
	spawnLine := 0
	seenRefusal := false
	var totalSamePID int
	visitTraceLines(raw, 1, func(lineNo int, line []byte) bool {
		if linePID(line) != pid {
			if spawnedTracePID(line) == pid && lineNo > refusalLine && spawnLine == 0 {
				if event, ok := summarizeDyingTraceLine(lineNo, line); ok && event.kind == "call" && (event.name == "clone" || event.name == "clone3" || event.name == "fork" || event.name == "vfork") {
					spawnLine = lineNo
				}
			}
			return true
		}
		totalSamePID++
		event, ok := summarizeDyingTraceLine(lineNo, line)
		if !ok {
			event = safeDyingEvent{line: lineNo, kind: "unclassified"}
		}
		if event.kind == "resume" {
			if start, found := pending[event.name]; found {
				event.link = fmt.Sprintf("unfinished-call-line-%d", start)
				delete(pending, event.name)
			} else {
				event.link = "no-prior-unfinished-call"
			}
		}
		if event.kind == "call" && event.result == "unfinished" {
			pending[event.name] = lineNo
		}
		if event.kind == "terminal" && lineNo > refusalLine && terminalLine == 0 {
			terminalLine, terminalKind = lineNo, event.result
			if terminalKind == "" {
				terminalKind = event.name
			}
			event.link = "same-numeric-pid-only-generation-unproven"
		}
		if lineNo < refusalLine {
			before = append(before, event)
			if len(before) > 6 {
				before = before[len(before)-6:]
			}
			return true
		}
		if lineNo == refusalLine {
			seenRefusal = true
			selected = append(selected, event)
			return true
		}
		if seenRefusal && len(selected) < maxDyingDiagnosticEvents {
			selected = append(selected, event)
		}
		return true
	})
	var out strings.Builder
	fmt.Fprintf(&out, "%s refusal_line=%d pid=%d same_pid_events=%d\n", dyingTraceDigest(raw), refusalLine, pid, totalSamePID)
	for _, event := range before {
		out.WriteString(event.String())
		out.WriteByte('\n')
	}
	for _, event := range selected {
		out.WriteString(event.String())
		out.WriteByte('\n')
	}
	if terminalLine > 0 {
		fmt.Fprintf(&out, "terminal_candidate_line=%d class=%s linkage=same-numeric-pid-only-generation-unproven\n", terminalLine, terminalKind)
	} else {
		out.WriteString("terminal_candidate=none-observed\n")
	}
	if spawnLine > 0 {
		fmt.Fprintf(&out, "spawn_candidate_line=%d class=child-pid-matches-refusal linkage=numeric-pid-reuse-possible\n", spawnLine)
	}
	if out.Len() > maxDyingDiagnosticBytes {
		return out.String()[:maxDyingDiagnosticBytes]
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
	clone3 := "101 clone3({flags=CLONE_VM|CLONE_FS|CLONE_FILES|CLONE_SIGHAND|CLONE_THREAD|CLONE_SYSVSEM|CLONE_SETTLS|CLONE_PARENT_SETTID|CLONE_CHILD_CLEARTID, child_tid=0x1, parent_tid=0x1, exit_signal=0, stack=0x2, stack_size=0xffff80, tls=0x3} => {parent_tid=[2 /* 202 in strace's PID NS */]}, 88) = 2 /* 202 in strace's PID NS */"
	if got := spawnedTracePID([]byte(clone3)); got != 202 {
		t.Fatalf("spawnedTracePID(clone3) = %d, want translated host pid 202", got)
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

func TestLaterBareOpenatExitRejectsReusedPIDFromResumedSpawn(t *testing.T) {
	const target = 202
	cloneStart := "101 clone(child_stack=0x7f0000001000, flags=CLONE_VM|CLONE_FS|CLONE_FILES|CLONE_SIGHAND|CLONE_THREAD|CLONE_SYSVSEM|CLONE_SETTLS|CLONE_PARENT_SETTID|CLONE_CHILD_CLEARTID, parent_tid=[202], tls=0x7f0000001700, child_tidptr=0x7f0000001710 <unfinished ...>"
	clone3Tail := ", child_tid=0x7fada74bf990, parent_tid=0x7fada74bf990, exit_signal=0, stack=0x7fada64bf000, stack_size=0xffff80, tls=0x7fada74bf6c0}"
	clone3Start := "101 clone3({flags=CLONE_VM|CLONE_FS|CLONE_FILES|CLONE_SIGHAND|CLONE_THREAD|CLONE_SYSVSEM|CLONE_SETTLS|CLONE_PARENT_SETTID|CLONE_CHILD_CLEARTID" + clone3Tail + " <unfinished ...>"
	cases := []struct {
		name  string
		start string
		end   string
		want  bool
	}{
		{name: "clone resumed reuses pid", start: cloneStart, end: "101 <... clone resumed>) = 202", want: true},
		{name: "clone3 resumed reuses pid", start: clone3Start, end: "101 <... clone3 resumed> => {parent_tid=[202]}, 88) = 202", want: true},
		{name: "fork resumed reuses pid", start: "101 fork( <unfinished ...>", end: "101 <... fork resumed>) = 202", want: true},
		{name: "vfork resumed reuses pid", start: "101 vfork( <unfinished ...>", end: "101 <... vfork resumed>) = 202", want: true},
		{name: "resumed child is another pid", start: "101 fork( <unfinished ...>", end: "101 <... fork resumed>) = 303"},
		{name: "resumed parent mismatch", start: "102 fork( <unfinished ...>", end: "101 <... fork resumed>) = 202"},
		{name: "resumed syscall mismatch", start: "101 fork( <unfinished ...>", end: "101 <... vfork resumed>) = 202"},
		{name: "malformed resumed result", start: "101 fork( <unfinished ...>", end: "101 <... fork resumed>) = nonsense"},
		{name: "failed resumed result", start: "101 fork( <unfinished ...>", end: "101 <... fork resumed>) = -1 EAGAIN (Resource temporarily unavailable)"},
		{name: "unknown resumed result", start: "101 fork( <unfinished ...>", end: "101 <... fork resumed>) = ?"},
		{name: "unfinished without resumed result", start: "101 fork( <unfinished ...>"},
		{name: "clone3 namespace result uses host pid", start: clone3Start, end: "101 <... clone3 resumed> => {parent_tid=[2 /* 202 in strace's PID NS */]}, 88) = 2 /* 202 in strace's PID NS */", want: true},
	}
	for _, tc := range cases {
		t.Run(tc.name, func(t *testing.T) {
			lines := []string{fmt.Sprintf("%d <... openat resumed>) = ?", target), tc.start}
			if tc.end != "" {
				lines = append(lines, tc.end)
			}
			lines = append(lines, fmt.Sprintf("%d +++ exited with 0 +++", target))
			raw := []byte(strings.Join(lines, "\n") + "\n")
			got := laterBareOpenatExit(raw, &trace.Error{Reason: "unresumed-call", Line: 1})
			if tc.want && got != 0 {
				t.Fatalf("laterBareOpenatExit() = %d, want refusal due to reused PID", got)
			}
			if !tc.want && got != len(lines) {
				t.Fatalf("laterBareOpenatExit() = %d, want terminal line %d", got, len(lines))
			}
		})
	}
	// A matching start may precede the refusal and complete afterward, with
	// unrelated child activity interleaved as in real strace output.
	preTarget := fmt.Sprintf("101 fork( <unfinished ...>\n%d <... openat resumed>) = ?\n303 stat(\"unrelated\", 0x1) = 0\n101 <... fork resumed>) = %d\n%d +++ exited with 0 +++\n", target, target, target)
	if got := laterBareOpenatExit([]byte(preTarget), &trace.Error{Reason: "unresumed-call", Line: 2}); got != 0 {
		t.Fatalf("pre-refusal split spawn reused PID: laterBareOpenatExit() = %d, want 0", got)
	}
}

func TestLaterBareOpenatExitDiagnosticsAreBounded(t *testing.T) {
	raw := []byte("202 openat(" + strings.Repeat("x", 2000) + ") = 0\n202 +++ exited with 0 +++\n")
	diagnostic := dyingRefusalDiagnostic(raw, 1, 202)
	if len(diagnostic) > maxDyingDiagnosticBytes || !strings.Contains(diagnostic, "refusal_line=1 pid=202") || !strings.Contains(diagnostic, "terminal_candidate_line=2") {
		t.Fatalf("diagnostic was not bounded refusal/terminal context: %q", diagnostic)
	}
}

func TestDyingRefusalDiagnosticRedactsAndLinksEvents(t *testing.T) {
	raw := []byte(strings.Join([]string{
		`101 fork() = 202`,
		`202 openat(AT_FDCWD</private/checkout>, "secret-argv", O_RDONLY <unfinished ...>`,
		`303 read(4</private/other>, "secret-buffer", 12) = 12`,
		`202 <... openat resumed>) = ? <unavailable>`,
		`202 +++ exited with 0 +++`,
		`202 write(1, "secret-environment-value", 25) = 25`,
	}, "\n") + "\n")
	diagnostic := dyingRefusalDiagnostic(raw, 4, 202)
	for _, forbidden := range []string{"/private", "secret-argv", "secret-buffer", "secret-environment-value", "pid=303"} {
		if strings.Contains(diagnostic, forbidden) {
			t.Fatalf("diagnostic leaked %q: %s", forbidden, diagnostic)
		}
	}
	if len(diagnostic) > maxDyingDiagnosticBytes {
		t.Fatalf("diagnostic bytes=%d, max=%d", len(diagnostic), maxDyingDiagnosticBytes)
	}
	if !strings.Contains(diagnostic, dyingTraceDigest(raw)) ||
		!strings.Contains(diagnostic, "line=2 event=call name=openat result=unfinished") ||
		!strings.Contains(diagnostic, "line=4 event=resume name=openat result=unavailable link=unfinished-call-line-2") ||
		!strings.Contains(diagnostic, "terminal_candidate_line=5 class=exit-status-zero linkage=same-numeric-pid-only-generation-unproven") {
		t.Fatalf("diagnostic omitted required safe trace facts: %s", diagnostic)
	}
	if strings.Index(diagnostic, "line=2 event=call") > strings.Index(diagnostic, "line=4 event=resume") ||
		strings.Index(diagnostic, "line=4 event=resume") > strings.Index(diagnostic, "line=5 event=terminal") {
		t.Fatalf("diagnostic events are not ordered: %s", diagnostic)
	}
}

func TestDyingRefusalDiagnosticCapsEventCountAndOutput(t *testing.T) {
	var raw strings.Builder
	for i := 1; i <= 2000; i++ {
		fmt.Fprintf(&raw, "202 openat(AT_FDCWD</private/%d>, \"secret-%d\", O_RDONLY) = 0\n", i, i)
	}
	raw.WriteString("202 +++ exited with 0 +++\n")
	diagnostic := dyingRefusalDiagnostic([]byte(raw.String()), 1, 202)
	if len(diagnostic) > maxDyingDiagnosticBytes {
		t.Fatalf("diagnostic bytes=%d, max=%d", len(diagnostic), maxDyingDiagnosticBytes)
	}
	if strings.Count(diagnostic, "event=") > maxDyingDiagnosticEvents+6 {
		t.Fatalf("diagnostic emitted too many events: %s", diagnostic)
	}
	if strings.Contains(diagnostic, "/private/") || strings.Contains(diagnostic, "secret-") {
		t.Fatalf("diagnostic leaked trace payload: %s", diagnostic)
	}
	if !strings.Contains(diagnostic, "terminal_candidate_line=2001") {
		t.Fatalf("diagnostic did not find a terminal event outside the event window: %s", diagnostic)
	}
}

func TestDyingRefusalDiagnosticLabelsNumericPIDReuseAsAmbiguous(t *testing.T) {
	raw := []byte(strings.Join([]string{
		`202 openat("refused", O_RDONLY) = ? <unavailable>`,
		`101 clone(child_stack=NULL, flags=SIGCHLD) = 202`,
		`202 +++ exited with 0 +++`,
	}, "\n") + "\n")
	diagnostic := dyingRefusalDiagnostic(raw, 1, 202)
	if !strings.Contains(diagnostic, "terminal_candidate_line=3 class=exit-status-zero linkage=same-numeric-pid-only-generation-unproven") ||
		!strings.Contains(diagnostic, "spawn_candidate_line=2 class=child-pid-matches-refusal linkage=numeric-pid-reuse-possible") {
		t.Fatalf("diagnostic overstated numeric-PID identity or omitted reuse evidence: %s", diagnostic)
	}
}

func TestLaterBareGetdentsExitRequiresExactRefusalAndPID(t *testing.T) {
	const (
		pid      = 202
		otherPID = 303
	)
	unknown := fmt.Sprintf("%d getdents64(9</tmp/pkg>) = ? <unavailable>\n", pid)
	valid := unknown + fmt.Sprintf("%d +++ exited with 0 +++\n", pid)
	reused := unknown + fmt.Sprintf("101 fork() = %d\n%d +++ exited with 0 +++\n", pid, pid)
	cases := []struct {
		name     string
		raw      string
		reason   string
		line     int
		wantLine int
	}{
		{name: "later same-pid parsed exit", raw: valid, reason: "unresumed-call", line: 1, wantLine: 2},
		{name: "later same-pid parsed signal exit", raw: unknown + fmt.Sprintf("%d +++ killed by SIGKILL +++\n", pid), reason: "unresumed-call", line: 1, wantLine: 2},
		{name: "different pid exit", raw: unknown + fmt.Sprintf("%d +++ exited with 0 +++\n", otherPID), reason: "unresumed-call", line: 1},
		{name: "missing terminal", raw: unknown, reason: "unresumed-call", line: 1},
		{name: "same numeric pid reused after refusal", raw: reused, reason: "unresumed-call", line: 1},
		{name: "prior terminal is not evidence", raw: fmt.Sprintf("%d +++ exited with 0 +++\n", pid) + unknown, reason: "unresumed-call", line: 2},
		{name: "successful resumed getdents is not the unknown shape", raw: fmt.Sprintf("%d <... getdents64 resumed>) = 4\n%d +++ exited with 0 +++\n", pid, pid), reason: "unresumed-call", line: 1},
		{name: "wrong refusal reason", raw: valid, reason: "size", line: 1},
		{name: "malformed terminal", raw: unknown + fmt.Sprintf("%d +++ exited with nonsense +++\n", pid), reason: "unresumed-call", line: 1},
		{name: "terminal trailing junk", raw: unknown + fmt.Sprintf("%d +++ exited with 0 +++ extra\n", pid), reason: "unresumed-call", line: 1},
		{name: "trailing text on unknown result", raw: fmt.Sprintf("%d getdents64(9</tmp/pkg>) = ? <unavailable> extra\n%d +++ exited with 0 +++\n", pid, pid), reason: "unresumed-call", line: 1},
	}
	for _, tc := range cases {
		t.Run(tc.name, func(t *testing.T) {
			got := laterBareGetdentsExit([]byte(tc.raw), &trace.Error{Reason: tc.reason, Line: tc.line})
			if got != tc.wantLine {
				t.Fatalf("laterBareGetdentsExit() = %d, want %d", got, tc.wantLine)
			}
		})
	}
}

func TestUnavailableGetdentsStillFailsClosedInProductionParser(t *testing.T) {
	const pid = 202
	raw := []byte(fmt.Sprintf("%d getdents64(9</tmp/pkg>) = ? <unavailable>\n%d +++ exited with 0 +++\n", pid, pid))
	_, err := trace.Parse(bytes.NewReader(raw), "/", trace.Limits{})
	refusal, ok := err.(*trace.Error)
	if !ok || refusal.Reason != "unresumed-call" || refusal.Line != 1 {
		t.Fatalf("production parser error = %#v, want unresumed-call at line 1", err)
	}
	if got := laterBareGetdentsExit(raw, refusal); got != 2 {
		t.Fatalf("test-only same-PID oracle = %d, want parsed terminal line 2", got)
	}
}
