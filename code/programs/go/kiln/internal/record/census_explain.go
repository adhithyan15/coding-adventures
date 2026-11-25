package record

// Why an action is not recordable (KILN04, "`kiln record census`", the explain
// section)
// ====================================================================
//
// census.json explains the HOST-PATH reasons only.  An action can also be
// unrecordable for reasons that carry no path: the build failed, a trace did not
// parse, the classifier timed out, the tracer was not clean.  This file turns
// each of those into a count under a FIXED code.
//
// The rule that keeps it safe: a code is never a slice of the input.  Every
// function below maps free text (a reason string, the text of record.failed, a
// parse refusal) to a member of a closed list written in this file, and says
// "other" when nothing matches.  The input only chooses WHICH member, so no
// attacker text, and no path, can reach the report through here, and the number
// of distinct keys is bounded by the lists.

import (
	"regexp"
	"strings"

	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/exechead"
	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/trace"
)

// ExplainSummary counts actions (or traces) by the fixed code of why they are
// not recordable.
type ExplainSummary struct {
	// RecordReasonKinds: actions whose record.json carries a reason of this kind
	// (once per action per kind).
	RecordReasonKinds map[string]int `json:"record_reason_kinds"`
	// RecordParseReasons: actions whose record.json says the parse half failed,
	// by the fixed code of each parse reason (once per action per code).
	RecordParseReasons map[string]int `json:"record_parse_reasons"`
	// ParseRefusals: step traces THIS census's own timed parse refused, by code.
	ParseRefusals map[string]int `json:"parse_refusals"`
	// RecordParseReasonsByLanguage is RecordParseReasons split by the action's
	// language: language -> code -> actions.  Languages are the validated names
	// of the share-by-language section (at most maxCensusLanguages, then
	// "other"); codes are the closed list of parseReasonCode.
	RecordParseReasonsByLanguage CountTable `json:"record_parse_reasons_by_language"`
	// ParseRefusalsByLanguage is ParseRefusals split the same way.
	ParseRefusalsByLanguage CountTable `json:"parse_refusals_by_language"`
	// ParseRefusalSyscalls is the histogram reason x syscall of the refusals
	// of this census's cross-checked parse: code -> syscall -> traces.  The
	// syscall is a member of the parser's closed syscall table (refusalSyscall),
	// or "other" when the refusal is not tied to a modelled call (an
	// unparsable line, an unresumed call) or the call is not in the table.
	ParseRefusalSyscalls CountTable `json:"parse_refusal_syscalls"`
	// ParseRefusalSyscallsByLanguage is the same, split by language:
	// language -> code -> syscall -> traces.
	ParseRefusalSyscallsByLanguage CountTable3 `json:"parse_refusal_syscalls_by_language"`
	// RecordFailed: actions with a record.failed, by category.
	RecordFailed map[string]int `json:"record_failed"`
	// Sidecar: facts from every step sidecar that decoded.
	Sidecar SidecarFacts `json:"sidecar"`
}

// SidecarFacts are counts over the step sidecars.
type SidecarFacts struct {
	Steps       int            `json:"steps"`
	Truncated   map[string]int `json:"truncated"` // by truncated_reason
	NotClean    int            `json:"not_clean"`
	Unclean     map[string]int `json:"unclean"` // by the tracer's unclean reason
	Exit125     int            `json:"exit_125"`
	RootNonZero int            `json:"root_status_nonzero"`
	// StraceStderrCodes counts steps by ClassifyStraceStderr's code (the closed
	// list plus "unrecorded"); StraceSignals counts steps whose strace was killed
	// by a signal, by StraceSignalLabel.  Informational: neither fails a census.
	StraceStderrCodes map[string]int `json:"strace_stderr_codes"`
	// StraceStderrFailedSteps is the part of StraceStderrCodes whose step also
	// failed its build step (a started step with a non-zero root status, the
	// same signal as RootNonZero and builtOK's sidecar fallback), by the same
	// code.  It tells a strace race that costs one lost record from one that
	// takes the build step down and cascades into DEP-SKIPs.  Informational:
	// it is in no gate.  Keys are the closed code list plus "unrecorded".
	StraceStderrFailedSteps map[string]int `json:"strace_stderr_failed_steps"`
	StraceSignals           map[string]int `json:"strace_signals"`
	// StraceExits counts, among steps with stderr text, strace's exit status as
	// StraceExitLabel names it (a signalled strace has none).
	StraceExits map[string]int `json:"strace_exits"`
}

func newExplain() ExplainSummary {
	return ExplainSummary{
		RecordReasonKinds: map[string]int{}, RecordParseReasons: map[string]int{},
		ParseRefusals: map[string]int{}, RecordFailed: map[string]int{},
		RecordParseReasonsByLanguage: CountTable{}, ParseRefusalsByLanguage: CountTable{},
		ParseRefusalSyscalls: CountTable{}, ParseRefusalSyscallsByLanguage: CountTable3{},
		Sidecar: SidecarFacts{Truncated: map[string]int{}, Unclean: map[string]int{}, StraceStderrCodes: map[string]int{}, StraceStderrFailedSteps: map[string]int{}, StraceSignals: map[string]int{}, StraceExits: map[string]int{}},
	}
}

// CountTable is a two-level count, a -> b -> n.  Every key is a member of a
// closed list (a validated language name, a parseReasonCode, a syscall-table
// name), so its size is bounded by those lists.
type CountTable map[string]map[string]int

func (t CountTable) add(a, b string) {
	m := t[a]
	if m == nil {
		m = map[string]int{}
		t[a] = m
	}
	m[b]++
}

// CountTable3 is a three-level count, a -> b -> c -> n.
type CountTable3 map[string]CountTable

func (t CountTable3) add(a, b, c string) {
	m := t[a]
	if m == nil {
		m = CountTable{}
		t[a] = m
	}
	m.add(b, c)
}

// refusalSyscall names the call a parse refusal is about, from CLOSED data only.
//
//	refused:<name>   the name is the code's own, already checked against the
//	                 table by refusedCode
//	otherwise        trace.Error.Call, which the parser sets only to a table
//	                 name (cwd-race: chdir or fchdir; fd-path: fchmod, ...)
//	else             "other"
//
// It is looked up again here, so a hostile or stale error can never put its
// own text into the report.
func refusalSyscall(code string, e *trace.Error) string {
	if name, ok := strings.CutPrefix(code, "refused:"); ok && name != "other" && trace.SyscallClass(name) == "refused" {
		return name
	}
	if e != nil && e.Call != "" && trace.SyscallClass(e.Call) != "" {
		return e.Call
	}
	// D44: a dying call outside the table says only whether its name is
	// strace's unknown-number spelling, as one of two fixed codes.  The value
	// is compared to those constants, so a hostile error cannot put its own
	// text into the report.
	if code == "unresumed-call" && e != nil {
		switch e.UnknownNumber {
		case "unknown-number:yes", "unknown-number:no":
			return e.UnknownNumber
		}
	}
	return "other"
}

// matchCode returns the first member of codes that s equals or starts, as a
// whole word: followed by the end, ':' or ' '.
func matchCode(s string, codes []string) (string, bool) {
	for _, c := range codes {
		if s == c || strings.HasPrefix(s, c+":") || strings.HasPrefix(s, c+" ") {
			return c, true
		}
	}
	return "", false
}

// reasonCodes are the classifier's reason kinds (internal/traced), longest
// first so a more specific kind wins.
var reasonCodes = []string{
	"host-read:setup-modified", "host-read:tool-not-keyed", "host-read",
	"trace-incomplete:unstable-path", "trace-incomplete:proc-path", "trace-incomplete:exec-unreadable",
	"trace-incomplete:recipe-mismatch", "trace-incomplete:symlink-loop", "trace-incomplete:exec-elf",
	"trace-incomplete:unresolvable-path", "trace-incomplete:size", "trace-incomplete:exec-format",
	"trace-incomplete:vcs-write", "trace-incomplete:resolve-flags", "trace-incomplete:exec-depth",
	"trace-incomplete:empty-path", "trace-incomplete:bad-path", "trace-incomplete:writes-unknown",
	"trace-incomplete:unstable",
	"writes-tracked", "trace-visible", "state-write", "network", "daemon-left", "daemon", "vcs-read",
	"failed", "recipe-mismatch", "unknown", "truncated", "writes-too-many",
}

// reasonKindCode is the fixed kind of one classifier reason string.
func reasonKindCode(s string) string {
	if c, ok := matchCode(s, reasonCodes); ok {
		return c
	}
	if strings.HasPrefix(s, "trace-incomplete") {
		return "trace-incomplete:other"
	}
	return "other"
}

// traceReasonCodes are the parser's reason names (internal/trace/errors.go).
var traceReasonCodes = []string{
	"size", "ambiguous-path", "truncated", "non-utf8-path", "deleted-path", "unparsable-line", "personality",
	"interleaved-call", "unmatched-resume", "unresumed-call", "clone-flags", "arity", "path-arg", "dirfd",
	"fd-path", "sockaddr", "orphan-pid", "pid-reuse", "process-cycle", "missing-exit", "event-after-exit",
	"duplicate-exit", "empty", "initial-cwd", "cwd-too-long", "result", "read", "cwd-race", "pidns-untranslated",
	"helper-spawn", "helper-no-exec", "helper-no-init", "helper-activity", "helper-cwd", "helper-exe",
	"helper-config", "flags", "unknown-syscall", "refused",
}

// uncleanCodes are the tracer's unclean reasons (the Unclean* constants).
//
// The supervisor's own (KILN04 D48, internal/exechead) are here too: a step run
// with --exec-head can be unclean for them, and without a code of their own the
// census would file them all under `unclean:other`.
var uncleanCodes = []string{
	UncleanNotStarted, UncleanSignalled, UncleanTruncated, UncleanStdout, UncleanStderr, UncleanPipesHeld,
	exechead.UncleanHandover, exechead.UncleanReceiver, exechead.UncleanFailed,
	exechead.UncleanOverflow, exechead.UncleanStuck, exechead.UncleanStall,
}

// parsePhrases map the fixed sentences of parseHelperTrace and classifyAction to
// codes.
var parsePhrases = []struct{ contains, code string }{
	{"sandbox-refused", "sandbox-refused"},
	{"the step did not start", "step-not-started"},
	{"its sidecar is unreadable", "sidecar-unreadable"},
	{"run under another binding", "other-binding"},
	{"the trace cannot be opened", "trace-unopenable"},
	{"the trace is not a regular file", "trace-not-regular"},
	{"own execve is not in the trace", "no-step-exec"},
	{"root_status", "root-status-mismatch"},
	{"--config-sha256", "config-sha-mismatch"},
	{"--sha256", "init-config-mismatch"},
	{"another sealed", "other-binding"},
	// KILN04 D48 step 5: an exec-head file that cannot be trusted refuses the whole
	// action (exechead_read.go); one fixed phrase, one code.
	{"exechead-missing", "exechead-missing"},
	{"exechead-unclaimed", "exechead-unclaimed"},
	{"exechead-oversize", "exechead-oversize"},
	{"exechead-digest", "exechead-digest"},
	{"exechead-malformed", "exechead-malformed"},
}

var stepPrefixRE = regexp.MustCompile(`^step [0-9]+: `)

// parseReasonCode is the fixed code of a parse reason: the text parseStep
// returns, or an entry of record.json's parse_reasons ("step N: ...").
func parseReasonCode(s string) string {
	s = stepPrefixRE.ReplaceAllString(s, "")
	if rest, ok := strings.CutPrefix(s, "unclean:"); ok {
		if c, ok := matchCode(rest, uncleanCodes); ok {
			return "unclean:" + c
		}
		return "unclean:other"
	}
	rest := strings.TrimPrefix(s, "trace-incomplete:")
	if c, ok := matchCode(rest, traceReasonCodes); ok {
		if c == "refused" {
			return refusedCode(rest)
		}
		return c
	}
	for _, p := range parsePhrases {
		if strings.Contains(s, p.contains) {
			return p.code
		}
	}
	if rest == "writes-too-many" {
		return "writes-too-many"
	}
	return "other"
}

// failedCode is the category of a record.failed's text (the text before its first
// colon, as the epilogue writes it), from a closed set: timeout, traces-too-large,
// record-too-large, panic, classifier-failed, classifier-not-started,
// record-invalid, other.
func failedCode(text string) string {
	switch {
	case strings.HasPrefix(text, "timeout"):
		return "timeout"
	case strings.HasPrefix(text, "traces-too-large"):
		return "traces-too-large"
	case strings.HasPrefix(text, "record-too-large"):
		return "record-too-large"
	case strings.HasPrefix(text, "the classifier failed"):
		if strings.Contains(text, "panic") {
			return "panic"
		}
		return "classifier-failed"
	case strings.HasPrefix(text, "the classifier could not be started"):
		return "classifier-not-started"
	case strings.HasPrefix(text, "the classifier exited cleanly but left no record.json"),
		strings.HasPrefix(text, "the classifier's record.json"):
		return "record-invalid"
	}
	return "other"
}

// refusedCode turns the parser's `refused:<syscall>` into `refused:<syscall>`
// again, but only when <syscall> is a member of the parser's CLOSED syscall
// table in its refused class (trace.SyscallClass; internal/trace/table.go).
// The name is the one part of the reason an untrusted record.json could choose,
// so it is never copied: it is looked up, and the table's own spelling is what
// is returned.  Anything else (a name outside the table, a handled name, a
// missing name, a name with a newline in it) is `refused:other`, which keeps
// the number of distinct codes bounded by the table.
func refusedCode(rest string) string {
	name, ok := strings.CutPrefix(rest, "refused:")
	if !ok {
		return "refused:other"
	}
	// The reason may be followed by free text ("refused:ptrace some words"):
	// the syscall name is what precedes the first ':' or ' '.
	if i := strings.IndexAny(name, ": "); i >= 0 {
		name = name[:i]
	}
	if trace.SyscallClass(name) == "refused" {
		return "refused:" + name
	}
	return "refused:other"
}
