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
}

func newExplain() ExplainSummary {
	return ExplainSummary{
		RecordReasonKinds: map[string]int{}, RecordParseReasons: map[string]int{},
		ParseRefusals: map[string]int{}, RecordFailed: map[string]int{},
		Sidecar: SidecarFacts{Truncated: map[string]int{}, Unclean: map[string]int{}},
	}
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
var uncleanCodes = []string{UncleanNotStarted, UncleanSignalled, UncleanTruncated, UncleanStdout, UncleanStderr, UncleanPipesHeld}

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
