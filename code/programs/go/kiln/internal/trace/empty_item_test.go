package trace

import (
	"strings"
	"testing"
)

// H1 (review): an item that is empty after strings.TrimSpace and is followed by
// a comma before "<unfinished ...>" is flushed, and refused, by the old scanner.
// The grammar path must not drop it as "the item before the marker".
func TestEmptyItemBeforeCommaAndMarker(t *testing.T) {
	for _, c := range []struct {
		line   string
		accept bool
	}{
		{"2  openat( ,  <unfinished ...>", false},
		{"2  openat( ,  <unfinished ...>", false},
		{"1  wait4(-1,  , <unfinished ...>", false},
		{"1  wait4(-1,  , <unfinished ...>", false},
		{"1  wait4(-1,   <unfinished ...>", true},
		{"1  wait4(-1,   <unfinished ...>", true},
	} {
		_, eo := handleLine(c.line, 1, &lineState{pend: map[int]*pending{}})
		_, en := handleLineG(c.line, 1, &lineState{pend: map[int]*pending{}})
		if (eo == nil) != c.accept || (en == nil) != c.accept {
			t.Errorf("%q: old %v, new %v, want accept=%v", c.line, eo, en, c.accept)
		}
	}
	// The whole-trace repro (the cross-check hook compares both parsers).
	tr := "1  execve(\"/bin/x\", [\"x\"], 0x1 /* 1 var */) = 0\n1  wait4( ,  <unfinished ...>\n1  +++ exited with 0 +++\n"
	if _, err := Parse(strings.NewReader(tr), "/w", Limits{}); err == nil {
		t.Error("accepted")
	}
}
