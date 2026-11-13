package trace

import (
	"bufio"
	"errors"
	"io"
	"strconv"
	"strings"
)

// This file is pass 1: turn the byte stream into an ordered list of COMPLETED
// records (a system call with its result, or a process exit).  It is the part
// that knows strace's line shapes.
//
// # What strace -f --quiet=attach -y -s 4096 prints
//
// With -o <file> and -f every line starts with the pid of the process (strictly
// speaking the thread) that produced it, followed by two spaces.  Five shapes
// matter:
//
//	1234  openat(AT_FDCWD, "/etc/passwd", O_RDONLY|O_CLOEXEC) = 3</etc/passwd>
//	1234  openat(AT_FDCWD, "/nope", O_RDONLY) = -1 ENOENT (No such file or directory)
//	1234  +++ exited with 0 +++
//	1234  +++ killed by SIGKILL +++
//	1234  --- SIGCHLD {si_signo=SIGCHLD, si_code=CLD_EXITED, si_pid=1235} ---
//
// and, when two processes make calls at the same time, a call is split:
//
//	1234  wait4(-1,  <unfinished ...>
//	1235  openat(AT_FDCWD, "/x", O_RDONLY) = 3</x>
//	1234  <... wait4 resumed>[{WIFEXITED(s) && WEXITSTATUS(s) == 0}], 0, NULL) = 1235
//
// The state machine below keeps, per pid, the text printed before
// "<unfinished ...>", and when the matching "<... NAME resumed>" arrives it
// glues the two halves into one ordinary line and parses that.  A resume with
// nothing pending, a resume with the wrong name, a second call while one is
// pending, or a call still pending at end of input are all errors.
//
// Two further lines are special.  A personality change is printed without a
// pid, as "[ Process PID=1234 runs in 32 bit mode. ]": the process switched
// system-call ABI and the syscall names we classify can no longer be trusted,
// so it is an error.  Signal lines ("--- ... ---") carry no path and are
// skipped.
//
// Events are recorded in the order calls COMPLETE (when the return value is
// known), which for a split call is the position of its "resumed" line.

// Limits caps how much input Parse accepts.  A zero field takes the default.
type Limits struct {
	MaxLineBytes  int   // longest accepted line, including its newline
	MaxTotalBytes int64 // total input bytes
	MaxEvents     int   // number of events produced
}

// DefaultLimits returns the caps used for zero fields.
//
// They are a RESIDENT-MEMORY bound for the parser, not the recorder's 2 GiB
// disk cap.  Pass 1 keeps every non-ignored record until pass 2, and each kept
// record retains its whole source line.  So memory is roughly
// MaxTotalBytes (the lines) plus about 200 bytes and a few slice headers per
// record (MaxEvents of them): with the defaults below, a few hundred MiB at
// the very worst, which a CI runner can afford.  A caller that has measured a
// bigger trace and a bigger budget raises them explicitly.
func DefaultLimits() Limits {
	return Limits{MaxLineBytes: 1 << 20, MaxTotalBytes: 64 << 20, MaxEvents: 1_000_000}
}

func (l Limits) withDefaults() Limits {
	d := DefaultLimits()
	if l.MaxLineBytes <= 0 {
		l.MaxLineBytes = d.MaxLineBytes
	}
	if l.MaxTotalBytes <= 0 {
		l.MaxTotalBytes = d.MaxTotalBytes
	}
	if l.MaxEvents <= 0 {
		l.MaxEvents = d.MaxEvents
	}
	return l
}

// result is a parsed "= ..." tail.
type result struct {
	value   int64
	hasVal  bool // a number was printed (as opposed to "?")
	errno   string
	ann     []byte // -y annotation on a returned descriptor
	hasAnn  bool
	unknown bool // "= ?"
}

func (r result) ok() bool { return !r.unknown && r.errno == "" }

// rawCall is a completed call: name, argument texts, result.
type rawCall struct {
	name string
	args []string
	ret  result
}

// record is one completed observation, in trace order.
type record struct {
	line  int
	pid   int
	call  *rawCall // nil for exit lines and start markers
	exit  *Exit
	start bool // a handled call began (its "<unfinished ...>" line); call is nil
	split bool // this completed call was split: its cwd was sampled at the start record
}

// lineState is what the line handler carries between lines.
type lineState struct {
	pend map[int]*pending
	root int // pid of the first line
	// helper is HelperRoot: name classification (unknown, refused) is DEFERRED
	// to pass 2, because whether a name is refused depends on which process
	// made the call and whether it has exec'd, and that is only known once the
	// whole spawn structure has been read.
	helper bool
}

type pending struct {
	name   string
	prefix string
	line   int
}

// maxUnknownItems caps the arguments of a name outside the handled table.
const maxUnknownItems = 16

func validName(n string) bool {
	if n == "" || len(n) > 64 {
		return false
	}
	for i := 0; i < len(n); i++ {
		c := n[i]
		if !((c >= 'a' && c <= 'z') || (c >= '0' && c <= '9') || c == '_') {
			return false
		}
	}
	return true
}

// parseCall parses "name(args) = result" or "name(args <unfinished ...>".
func parseCall(s string) (call rawCall, unfinished bool, prefix string, reason string) {
	j := strings.IndexByte(s, '(')
	if j < 0 || !validName(s[:j]) {
		return call, false, "", reasonUnparsable
	}
	call.name = s[:j]
	// A handled call has a known arity; anything else gets a generous global
	// cap.  Either way the bound is enforced WHILE scanning.
	maxItems := maxUnknownItems
	if ent := lookup(call.name); ent.class == classHandled {
		maxItems = ent.max
	}
	res, r := scanItems(s, j+1, true, maxItems)
	if r != "" {
		return call, false, "", r
	}
	if res.unfinished {
		p := s[:res.end]
		p = strings.TrimSuffix(p, " ")
		return call, true, p, ""
	}
	call.args = res.items
	ret, r := parseResult(s[res.end+1:])
	if r != "" {
		return call, false, "", r
	}
	call.ret = ret
	return call, false, "", ""
}

func isErrnoName(s string) bool {
	if s == "" || s[0] < 'A' || s[0] > 'Z' {
		return false
	}
	for i := 0; i < len(s); i++ {
		c := s[i]
		if !((c >= 'A' && c <= 'Z') || (c >= '0' && c <= '9') || c == '_') {
			return false
		}
	}
	return true
}

// parseResult parses what follows the closing parenthesis:
//
//	= 3</etc/passwd>
//	= 0
//	= -1 ENOENT (No such file or directory)
//	= 0x7f1234567000
//	= ?
//	= ? ERESTARTSYS (To be restarted if SA_RESTART is set)
//
// strace pads the '=' to a column, so any run of spaces may precede it.  After
// the value only an errno name and a parenthesised message (free of quotes,
// brackets and '<' '>') may follow; anything else could be a path pretending
// to be the end of the call, so it is an error.
func parseResult(s string) (result, string) {
	var r result
	s = strings.TrimLeft(s, " ")
	if !strings.HasPrefix(s, "=") {
		return r, reasonResult
	}
	s = strings.TrimLeft(s[1:], " ")
	switch {
	case strings.HasPrefix(s, "?"):
		r.unknown = true
		s = s[1:]
	case strings.HasPrefix(s, "0x"):
		k := 2
		for k < len(s) {
			if _, ok := hexVal(s[k]); !ok {
				break
			}
			k++
		}
		if k == 2 {
			return r, reasonResult
		}
		v, err := strconv.ParseUint(s[2:k], 16, 64)
		if err != nil {
			return r, reasonResult
		}
		r.value, r.hasVal = int64(v), true
		s = s[k:]
	default:
		k := 0
		if k < len(s) && s[k] == '-' {
			k++
		}
		d := k
		for k < len(s) && s[k] >= '0' && s[k] <= '9' {
			k++
		}
		if k == d || k-d > 18 {
			return r, reasonResult
		}
		v, err := strconv.ParseInt(s[:k], 10, 64)
		if err != nil {
			return r, reasonResult
		}
		r.value, r.hasVal = v, true
		s = s[k:]
		if strings.HasPrefix(s, "<") {
			if v < 0 {
				return r, reasonResult
			}
			body, end, rs := scanAnnotation(s, 0)
			if rs != "" {
				return r, rs
			}
			if strings.TrimSpace(s[end:]) != "" {
				return r, reasonAmbiguousPath
			}
			r.ann, r.hasAnn = body, true
			s = s[end:]
		}
	}
	s = strings.TrimLeft(s, " ")
	if s != "" && s[0] != '(' {
		k := strings.IndexByte(s, ' ')
		name := s
		if k >= 0 {
			name = s[:k]
		}
		if !isErrnoName(name) {
			return r, reasonResult
		}
		r.errno = name
		s = strings.TrimLeft(s[len(name):], " ")
	}
	if s != "" {
		if s[0] != '(' || s[len(s)-1] != ')' || strings.ContainsAny(s[1:len(s)-1], "()<>\"[]{}\\") {
			return r, reasonResult
		}
	}
	if r.hasVal && r.value < 0 {
		if r.value != -1 || r.errno == "" {
			return r, reasonResult
		}
	}
	if r.unknown && r.errno == "" {
		r.errno = "?"
	}
	return r, ""
}

// parseExit parses the inside of "+++ ... +++".
func parseExit(body string, pid int) (*Exit, bool) {
	body = strings.TrimSpace(body)
	if rest, ok := strings.CutPrefix(body, "exited with "); ok {
		if rest == "" || len(rest) > 9 {
			return nil, false
		}
		n, err := strconv.Atoi(rest)
		if err != nil || n < 0 {
			return nil, false
		}
		return &Exit{Pid: pid, Code: n}, true
	}
	if rest, ok := strings.CutPrefix(body, "killed by "); ok {
		core := false
		if r2, ok2 := strings.CutSuffix(rest, " (core dumped)"); ok2 {
			rest, core = r2, true
		}
		if !strings.HasPrefix(rest, "SIG") || !isErrnoName(rest) {
			return nil, false
		}
		return &Exit{Pid: pid, Killed: true, Signal: rest, CoreDumped: core}, true
	}
	return nil, false
}

// parsePid strips the pid prefix: "1234  rest" or "[pid 1234] rest".
func parsePid(line string) (int, string, bool) {
	if rest, ok := strings.CutPrefix(line, "[pid "); ok {
		k := strings.IndexByte(rest, ']')
		if k < 0 {
			return 0, "", false
		}
		pid, ok := atoiPid(rest[:k])
		if !ok {
			return 0, "", false
		}
		return pid, strings.TrimLeft(rest[k+1:], " "), true
	}
	k := 0
	for k < len(line) && line[k] >= '0' && line[k] <= '9' {
		k++
	}
	if k == 0 || k >= len(line) || line[k] != ' ' {
		return 0, "", false
	}
	pid, ok := atoiPid(line[:k])
	if !ok {
		return 0, "", false
	}
	return pid, strings.TrimLeft(line[k:], " "), true
}

func atoiPid(s string) (int, bool) {
	if s == "" || len(s) > 9 || (len(s) > 1 && s[0] == '0') {
		return 0, false
	}
	n, err := strconv.Atoi(s)
	if err != nil || n <= 0 {
		return 0, false
	}
	return n, true
}

// readRecords is pass 1.  It also returns the pid of the first line, which is
// the root of the process tree.  Every record (calls, start markers and exit
// lines alike) counts against MaxEvents.
func readRecords(r io.Reader, lim Limits, helper bool) ([]record, int, *Error) {
	br := bufio.NewReaderSize(r, 64<<10)
	var recs []record
	st := &lineState{pend: map[int]*pending{}, helper: helper}
	var total int64
	lineNo := 0
	for {
		line, err := readLine(br, lim.MaxLineBytes)
		if err != nil && !errors.Is(err, io.EOF) {
			var e *Error
			if errors.As(err, &e) {
				e.Line = lineNo + 1
				return nil, 0, e
			}
			return nil, 0, &Error{Reason: reasonRead, Line: lineNo + 1, Err: err}
		}
		if errors.Is(err, io.EOF) {
			if line == "" {
				break
			}
			// The last line has no newline: strace was cut off (or the file
			// was truncated) and the line may be a prefix of a different one.
			return nil, 0, incomplete(lineNo+1, reasonUnparsable)
		}
		lineNo++
		total += int64(len(line))
		if total > lim.MaxTotalBytes {
			return nil, 0, incomplete(lineNo, reasonSize)
		}
		text := strings.TrimSuffix(line, "\n")
		if st.root == 0 {
			if pid, _, ok := parsePid(text); ok {
				st.root = pid
			}
		}
		rec, e := handleLine(text, lineNo, st)
		if e != nil {
			return nil, 0, e
		}
		if rec != nil {
			recs = append(recs, *rec)
			if len(recs) > lim.MaxEvents {
				return nil, 0, incomplete(lineNo, reasonSize)
			}
		}
	}
	var first *pending
	for _, p := range st.pend {
		if first == nil || p.line < first.line {
			first = p
		}
	}
	if first != nil {
		return nil, 0, incomplete(first.line, reasonUnresumed)
	}
	return recs, st.root, nil
}

func isExecName(n string) bool { return n == "execve" || n == "execveat" }

// readLine reads one line (including its '\n') of at most max bytes.
func readLine(br *bufio.Reader, max int) (string, error) {
	var buf []byte
	for {
		chunk, err := br.ReadSlice('\n')
		if len(buf)+len(chunk) > max {
			return "", incomplete(0, reasonSize)
		}
		buf = append(buf, chunk...)
		switch {
		case err == nil:
			return string(buf), nil
		case errors.Is(err, bufio.ErrBufferFull):
			continue
		default:
			return string(buf), err
		}
	}
}

// handleLine classifies one line.  It returns the completed record (nil when
// the line completes nothing, e.g. an unfinished call or a signal line).
func handleLine(text string, lineNo int, st *lineState) (*record, *Error) {
	pend := st.pend
	if text == "" {
		return nil, incomplete(lineNo, reasonUnparsable)
	}
	for i := 0; i < len(text); i++ {
		if c := text[i]; c < 0x20 || c == 0x7f {
			// strace escapes every control character; a raw one means the line
			// is not strace's own output.
			return nil, incomplete(lineNo, reasonUnparsable)
		}
	}
	if strings.HasPrefix(text, "[ Process PID=") {
		return nil, incomplete(lineNo, reasonPersonality)
	}
	pid, body, ok := parsePid(text)
	if !ok || body == "" {
		return nil, incomplete(lineNo, reasonUnparsable)
	}
	switch {
	case strings.HasPrefix(body, "+++ "):
		if !strings.HasSuffix(body, " +++") || len(body) < 8 {
			return nil, incomplete(lineNo, reasonUnparsable)
		}
		ex, ok := parseExit(body[4:len(body)-4], pid)
		if !ok {
			return nil, incomplete(lineNo, reasonUnparsable)
		}
		if p := pend[pid]; p != nil {
			// A process killed or exiting inside a call that has no effect on
			// paths is normal (wait4 interrupted by SIGKILL).  A path call that
			// never returned is not: its result is unknown.
			if c := lookup(p.name); c.class != classIgnored {
				return nil, incomplete(p.line, reasonUnresumed)
			}
			delete(pend, pid)
		}
		return &record{line: lineNo, pid: pid, exit: ex}, nil
	case strings.HasPrefix(body, "--- "):
		if !strings.HasSuffix(body, " ---") || len(body) < 8 {
			return nil, incomplete(lineNo, reasonUnparsable)
		}
		return nil, nil
	case strings.HasPrefix(body, "<... "):
		rest := body[len("<... "):]
		k := strings.Index(rest, " resumed>")
		if k < 0 {
			return nil, incomplete(lineNo, reasonUnparsable)
		}
		name := rest[:k]
		p := pend[pid]
		if p == nil || p.name != name {
			return nil, incomplete(lineNo, reasonUnmatched)
		}
		delete(pend, pid)
		call, unfinished, _, r := parseCall(p.prefix + rest[k+len(" resumed>"):])
		if r != "" {
			return nil, incomplete(lineNo, r)
		}
		if unfinished {
			return nil, incomplete(lineNo, reasonUnparsable)
		}
		return finishCall(pid, lineNo, call, true, st.helper)
	}
	call, unfinished, prefix, r := parseCall(body)
	if r != "" {
		return nil, incomplete(lineNo, r)
	}
	if pend[pid] != nil {
		return nil, incomplete(lineNo, reasonInterleaved)
	}
	if unfinished {
		if !st.helper {
			if e := classifyName(call.name, lineNo); e != nil {
				return nil, e
			}
		}
		pend[pid] = &pending{name: call.name, prefix: prefix, line: lineNo}
		if lookup(call.name).class == classHandled {
			// The working directory a split call runs in is the one at its
			// START; pass 2 samples it when it meets this marker.
			return &record{line: lineNo, pid: pid, start: true}, nil
		}
		return nil, nil
	}
	return finishCall(pid, lineNo, call, false, st.helper)
}

// finishCall checks the closed table and keeps the call unless it is ignored.
//
// With HelperRoot, unknown and refused names are kept and judged in pass 2
// (deferClass), where the sandbox helper's setup calls can be told apart.
func finishCall(pid, lineNo int, call rawCall, split, deferClass bool) (*record, *Error) {
	if !deferClass {
		if e := classifyName(call.name, lineNo); e != nil {
			return nil, e
		}
	}
	if lookup(call.name).class == classIgnored {
		return nil, nil
	}
	return &record{line: lineNo, pid: pid, call: &call, split: split}, nil
}
