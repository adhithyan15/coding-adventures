package record

import (
	"strings"
	"testing"
)

// Every function of census_explain.go maps free text to a member of a closed
// list: hostile text can choose which member, never become one.
func TestExplainCodesAreFixed(t *testing.T) {
	for in, want := range map[string]string{
		"failed":                                "failed",
		"vcs-read":                              "vcs-read",
		"daemon-left":                           "daemon-left",
		"daemon:tcp":                            "daemon",
		"host-read:/etc/passwd":                 "host-read",
		"host-read:setup-modified:/x":           "host-read:setup-modified",
		"trace-incomplete:unstable-path:/a/b":   "trace-incomplete:unstable-path",
		"trace-incomplete:recipe-mismatch":      "trace-incomplete:recipe-mismatch",
		"trace-incomplete:unstable":             "trace-incomplete:unstable",
		"trace-incomplete:evil\n::error::x":     "trace-incomplete:other",
		"writes-tracked:code/packages/go/x":     "writes-tracked",
		"daemon-leftover":                       "other",
		"::set-output name=x::y":                "other",
		"host-readX":                            "other",
		"network:unknown":                       "network",
		"trace-incomplete:exec-unreadable:/bin": "trace-incomplete:exec-unreadable",
	} {
		if got := reasonKindCode(in); got != want {
			t.Errorf("reasonKindCode(%q) = %q, want %q", in, got, want)
		}
	}
	for in, want := range map[string]string{
		"step 0: trace-incomplete:unparsable-line":              "unparsable-line",
		"trace-incomplete:unresumed-call":                       "unresumed-call",
		"trace-incomplete:size":                                 "size",
		"trace-incomplete:unknown-syscall:foo":                  "unknown-syscall",
		"sandbox-refused":                                       "sandbox-refused",
		"step 3: unclean:strace-signalled":                      "unclean:strace-signalled",
		"step 3: unclean:/etc/x":                                "unclean:other",
		"step 1: the step did not start":                        "step-not-started",
		"the trace cannot be opened":                            "trace-unopenable",
		"the step's own execve is not in the trace":             "no-step-exec",
		"the root's exit line is not the sidecar's root_status": "root-status-mismatch",
		"step 2: step 2: ::warning::x":                          "other",
		"trace-incomplete:unparsable-line-but-evil":             "other",
	} {
		if got := parseReasonCode(in); got != want {
			t.Errorf("parseReasonCode(%q) = %q, want %q", in, got, want)
		}
	}
	for in, want := range map[string]string{
		"timeout: the classifier ran longer than 10m0s and was killed": "timeout",
		"traces-too-large: 5 bytes over 4":                             "traces-too-large",
		"the classifier failed: exit status 2: panic: x":               "panic",
		"the classifier failed: signal: killed: ":                      "classifier-failed",
		"the classifier could not be started: x":                       "classifier-not-started",
		"the classifier's record.json does not decode: y":              "record-invalid",
		"record-too-large: record.json is 5 bytes":                     "record-too-large",
		"::error::pwn": "other",
		"":             "other",
	} {
		if got := failedCode(in); got != want {
			t.Errorf("failedCode(%q) = %q, want %q", in, got, want)
		}
	}
	// The set of keys is closed: nothing from the input is ever in an output.
	if got := reasonKindCode("host-read:" + strings.Repeat("x", 1<<16)); got != "host-read" {
		t.Errorf("a long tail changed the code: %q", got)
	}
}
