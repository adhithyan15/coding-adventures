package traced

// KILN04 decision D38: /proc/sys/vm/overcommit_memory and /proc/version_signature
// are exact ephemeral entries.  These tests use the REAL host-roots.json and
// pin what the new entries must NOT cover.

import (
	"testing"

	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/trace"
)

func TestProcTuningAndVersionWithTheRealPolicy(t *testing.T) {
	const oc, vs = "/proc/sys/vm/overcommit_memory", "/proc/version_signature"
	tests := []struct {
		name  string
		event trace.Event
		want  string // "" means the record is kept
	}{
		{"open overcommit", openAbs(oc), ""},
		{"open version_signature", openAbs(vs), ""},
		{"stat overcommit", stat(oc), ""},
		{"stat version_signature", stat(vs), ""},
		// Siblings are not covered: the entries are exact.
		{"open swappiness", openAbs("/proc/sys/vm/swappiness"), "trace-incomplete:proc-path"},
		{"open max_map_count", openAbs("/proc/sys/vm/max_map_count"), "trace-incomplete:proc-path"},
		{"open version", openAbs("/proc/version"), "trace-incomplete:proc-path"},
		{"open version_signature/x", openAbs(vs + "/x"), "trace-incomplete:proc-path"},
		{"open overcommit/x", openAbs(oc + "/x"), "trace-incomplete:proc-path"},
		{"stat overcommit/x", stat(oc + "/x"), "trace-incomplete:proc-path"},
	}
	for _, tc := range tests {
		t.Run(tc.name, func(t *testing.T) {
			h := newHarness(t)
			withRealLists(t, h)
			rec := h.run(tc.event)
			if tc.want == "" {
				wantNoReasons(t, rec)
			} else {
				wantReason(t, rec, tc.want)
			}
		})
	}
}

// A write open of a tuning file is never allowed by the ephemeral list: a
// writing open of a virtual name that is not an fd link makes the write set
// unknown (W5), whatever the list says, and the record says so.
func TestProcTuningWriteOpenStaysRefused(t *testing.T) {
	for _, p := range []string{"/proc/sys/vm/overcommit_memory", "/proc/version_signature"} {
		for _, flags := range []string{"O_WRONLY", "O_RDWR", "O_WRONLY|O_TRUNC", "O_WRONLY|O_CREAT"} {
			t.Run(p+" "+flags, func(t *testing.T) {
				h := newHarness(t)
				withRealLists(t, h)
				rec := h.run(open(p, flags, p))
				if rec.WritesComplete {
					t.Fatalf("write open of %s (%s) left the write set complete: %q", p, flags, rec.Reasons)
				}
				wantReason(t, rec, rWritesUnknown)
			})
		}
	}
}
