package trace

import (
	"runtime"
	"strings"
	"testing"
)

// D34: ParseCrossChecked returns a disagreement as a value and counts the
// allowed stricter-only divergences of ONE parse.
func TestParseCrossCheckedReportsAllowedAsADelta(t *testing.T) {
	if runtime.GOOS == "windows" {
		t.Skip("strace fixtures with POSIX paths")
	}
	// The D31 row: a signal between split call halves is a stricter-only hit.
	const text = "2  chdir(a,  <unfinished ...>\n2  --- SIGCHLD {si_signo=SIGCHLD} ---\n2  <... chdir resumed> -1,  , b) = 0\n"
	_, err, m, allowed := ParseCrossChecked(strings.NewReader(text), "/work/repo", Options{})
	if m != nil || err == nil || allowed != 1 {
		t.Errorf("err=%v mismatch=%v allowed=%d, want a refusal, no mismatch, 1 allowed", err, m, allowed)
	}
	// A second call reports ITS delta, not the cumulative count.
	_, _, _, allowed = ParseCrossChecked(strings.NewReader("garbage\n"), "/work/repo", Options{})
	if allowed != 0 {
		t.Errorf("allowed = %d on a clean refusal, want 0", allowed)
	}
}
