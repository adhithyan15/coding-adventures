//go:build unix

package record

import (
	"os"
	"strings"
	"testing"
)

// The plan's side of "would emit": a gate or a missing weak key means the merge
// never emits the action; an action the plan does not know is never claimed.
func TestPlanVerdict(t *testing.T) {
	weak := strings.Repeat("a", 64)
	for name, tc := range map[string]struct {
		a                         Action
		in                        bool
		gated, unkeyed, mergeable bool
	}{
		"keyed":     {Action{Weak: weak}, true, false, false, true},
		"gated":     {Action{Weak: weak, Gate: "never:x"}, true, true, false, false},
		"unkeyed":   {Action{Refusal: "unknown-toolchain:y"}, true, false, true, false},
		"both":      {Action{Gate: "never:x"}, true, true, true, false},
		"not known": {Action{Weak: weak}, false, false, false, false},
	} {
		g, u, m := planVerdict(tc.a, tc.in)
		if g != tc.gated || u != tc.unkeyed || m != tc.mergeable {
			t.Errorf("%s: gated %v unkeyed %v mergeable %v", name, g, u, m)
		}
	}
}

// A census.json whose op counts could wrap an int64 when summed is invalid, and
// a trailing slash does not make the sealed home a different directory.
func TestCensusOpsSumCapAndTrailingSlash(t *testing.T) {
	f := censusDirFix(t)
	putCensusDoc(t, f, actZ, CensusDoc{Total: 1, Entries: []tracedEntry{{Kind: "host-read", Path: "/var/x", Count: 1}},
		Ops: map[string]map[string]int64{"host-read": {"stat": 1 << 62, "open": 1 << 62}}})
	_, s, _, _ := runCensus(t, f)
	if s.Problems["census_bad_entry"] != 1 || s.Actions.CensusInvalid != 1 {
		t.Errorf("problems %v actions %+v", s.Problems, s.Actions)
	}
	got := relabelRunner([]Redaction{{From: "/step/home/", To: "$HOME"}, {From: "/step/tmp", To: "$TMPDIR"}}, "/step/home", "/step/tmp/")
	if got[0].To != "$HOME" || got[1].To != "$TMPDIR" {
		t.Errorf("relabelRunner with trailing slashes: %+v", got)
	}
}

// writes_unknown and the strict-merge line.
func TestCensusWritesUnknownAndStrictMerge(t *testing.T) {
	f := censusDirFix(t)
	putRecord(t, f, actZ, func(r *ClassifiedRecord) { r.Record.WritesComplete, r.WritesComplete = false, false })
	_, s, _, mdPath := runCensus(t, f)
	if s.Actions.WritesUnknown != 1 || !s.Actions.StrictMergeVoidsShard || s.Actions.ShardVoid || s.Languages[0].WritesUnknown != 1 {
		t.Errorf("actions %+v", s.Actions)
	}
	md, _ := os.ReadFile(mdPath)
	if !strings.Contains(string(md), "strict merge would void this shard: yes") || !strings.Contains(string(md), "UPPER BOUND") {
		t.Error("the Markdown lacks the strict-merge line or the upper-bound wording")
	}
	// record.failed counts as an unknown write set too.
	writeBytes(t, RecordFailedFile(f.p.dir, actA), []byte("timeout: x"), 0o644)
	_, s, _, _ = runCensus(t, f)
	if s.Actions.WritesUnknown != 2 {
		t.Errorf("writes_unknown %d, want 2", s.Actions.WritesUnknown)
	}
	// An action with NO verdict at all (no record.json, no record.failed) has an
	// unknown write set too: the strict merge would void the shard for it.
	f = censusDirFix(t)
	if err := os.Remove(RecordFile(f.p.dir, actA)); err != nil {
		t.Fatal(err)
	}
	_, s, _, _ = runCensus(t, f)
	if s.Actions.NoVerdict != 1 || s.Actions.WritesUnknown != 1 || !s.Actions.StrictMergeVoidsShard || s.Languages[0].WritesUnknown != 1 {
		t.Errorf("actions %+v", s.Actions)
	}
	// Nothing unknown and no void: the strict merge would not void the shard.
	f = censusDirFix(t)
	_, s, _, mdPath = runCensus(t, f)
	md, _ = os.ReadFile(mdPath)
	if s.Actions.StrictMergeVoidsShard || !strings.Contains(string(md), "strict merge would void this shard: no") {
		t.Errorf("actions %+v", s.Actions)
	}
}
