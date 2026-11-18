package record

import (
	"os"
	"path/filepath"
	"testing"
)

// TestParseHelperTraceReportsTheRefusedLine: a trace that does not parse names
// the refused line as a separate, display-only result, and the reason string
// (which feeds the sidecar's parse_reasons) carries no line number.
func TestParseHelperTraceReportsTheRefusedLine(t *testing.T) {
	p := filepath.Join(t.TempDir(), "walk.trace")
	// Line 3 is a thread-death line both readers refuse.
	body := "100  execve(\"/bin/true\", [\"true\"], []) = 0\n" +
		"100  exit_group(0) = ?\n" +
		"100  wait4(-1, <unfinished ...>) = ?\n"
	if err := os.WriteFile(p, []byte(body), 0o600); err != nil {
		t.Fatal(err)
	}
	tr, why, line := parseHelperTrace(p, "/kiln", t.TempDir(), "sha", 0, false, probeLimits())
	if tr != nil || why == "" {
		t.Fatalf("the trace must be refused: tr=%v why=%q", tr, why)
	}
	if line <= 0 {
		t.Errorf("the refused line is not reported: %d (reason %q)", line, why)
	}
	if why != "trace-incomplete:unparsable-line" {
		t.Errorf("the reason must stay stable and line-free: %q", why)
	}
	// Failures not tied to a line report 0.
	if _, why, line := parseHelperTrace(filepath.Join(t.TempDir(), "absent"), "/kiln", "/", "sha", 0, false, probeLimits()); why == "" || line != 0 {
		t.Errorf("an unopenable trace: why=%q line=%d", why, line)
	}
}
