package traced

// KILN04 decision D37: /proc/self/fd and /proc/self/fd/* are ephemeral (clang
// readlinks one per header it opens), and four exact paths are absent-ok
// (Go's hugepage probe, python's /pyvenv.cfg, the compiler search paths /opt/rh
// and /include).  These tests use the REAL host-roots.json and pin what the
// new entries must NOT cover.

import (
	"testing"

	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/trace"
)

func readlinkOf(path string) trace.Event { return base("readlink", trace.KindMetadata, path) }
func accessOf(path string) trace.Event   { return base("access", trace.KindMetadata, path) }

func TestProcSelfFdWithTheRealPolicy(t *testing.T) {
	tests := []struct {
		name  string
		event trace.Event
		want  string // "" means the record is kept
	}{
		{"readlink fd dir", readlinkOf("/proc/self/fd"), ""},
		{"access fd dir", accessOf("/proc/self/fd"), ""},
		{"stat fd dir", stat("/proc/self/fd"), ""},
		{"readlink fd 3", readlinkOf("/proc/self/fd/3"), ""},
		{"access fd 3", accessOf("/proc/self/fd/3"), ""},
		{"stat fd 3", stat("/proc/self/fd/3"), ""},
		// (b) through the magic link: procMagicBeyond runs before ephemeralMatch.
		{"through fd 3", readlinkOf("/proc/self/fd/3/x"), "trace-incomplete:proc-path"},
		{"stat through fd 3", stat("/proc/self/fd/3/x"), "trace-incomplete:proc-path"},
		// (c) another pid is not "self".
		{"other pid fd dir", readlinkOf("/proc/1234/fd"), "trace-incomplete:proc-path"},
		{"other pid fd 3", readlinkOf("/proc/1234/fd/3"), "trace-incomplete:proc-path"},
		{"other pid stat fd 3", stat("/proc/1234/fd/3"), "trace-incomplete:proc-path"},
		{"thread-self fd 3", readlinkOf("/proc/thread-self/fd/3"), "trace-incomplete:proc-path"},
		// (a) an open is classified by the descriptor's own -y path.
		{"open fd 3 of a host file", open("/proc/self/fd/3", "O_RDONLY", "/srv/secret/key"), "host-read:/srv/secret/key"},
		{"open fd 3 of a proc file", open("/proc/self/fd/3", "O_RDONLY", "/proc/1234/environ"), "trace-incomplete:proc-path"},
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

func TestAbsentOKEntriesWithTheRealPolicy(t *testing.T) {
	const hp = "/sys/kernel/mm/transparent_hugepage/hpage_pmd_size"
	for _, p := range []string{hp, "/pyvenv.cfg", "/opt/rh", "/include"} {
		t.Run(p, func(t *testing.T) {
			// An absent probe is allowed.
			for _, ev := range []trace.Event{fail(stat(p), "ENOENT"), fail(accessOf(p), "ENOENT"), fail(open(p, "O_RDONLY", ""), "ENOENT")} {
				h := newHarness(t)
				withRealLists(t, h)
				wantNoReasons(t, h.run(ev))
			}
			// (d) a present read is still a host-read.
			for _, ev := range []trace.Event{openAbs(p), stat(p), accessOf(p)} {
				h := newHarness(t)
				withRealLists(t, h)
				wantReason(t, h.run(ev), "host-read:"+p)
			}
		})
	}
	// The siblings are tunable, so they are not covered, absent or not.
	for _, p := range []string{
		"/sys/kernel/mm/transparent_hugepage/enabled",
		"/sys/kernel/mm/transparent_hugepage/defrag",
		"/opt/rh/x", "/include/stdio.h", "/pyvenv.cfg/x",
	} {
		h := newHarness(t)
		withRealLists(t, h)
		wantReason(t, h.run(openAbs(p)), "host-read:"+p)
	}
}
