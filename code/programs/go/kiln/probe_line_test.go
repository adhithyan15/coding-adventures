package main

import (
	"encoding/json"
	"fmt"
	"strings"
	"sync/atomic"
	"testing"
	"time"

	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/record"
)

// The shape of a "PROBE: trace" line, and the wall-time facts on it
// ===================================================================
//
// ci/assert_sandbox_tests.py reads these lines out of a `go test -json` stream
// with a regular expression.  Two properties of the line are therefore part of
// its contract, not decoration:
//
//  1. Every number the script reads comes BEFORE the two refusal fields.  A
//     refusal is a %q-quoted string of text the trace itself influenced (a
//     refused line, a path); if it came first, a refusal that read
//     `best_mb_s=99 gated=1` could sit in front of the real fields and be
//     mistaken for them.  The script reads key=value pairs only from the text
//     before " old_refusal=", first occurrence wins; putting the refusals LAST
//     makes "everything before the first refusal" exactly "everything we
//     wrote ourselves".
//
//  2. A wall time is a census fact for gate G2 (parse time at most 10% of the
//     step's wall time; KILN04 section 4.9).  A wall that is not positive is a
//     measurement that did not happen (a clock that stepped back, a sidecar
//     stamp that did not parse): it is DROPPED, never logged, never used as a
//     divisor, and counted, so the census can say how many it lost.

// droppedWalls counts the non-positive wall times probeTraceWall was handed.
var droppedWalls atomic.Int64

// noWall is what a caller passes when it has no wall time at all (the line then
// carries no wall facts and nothing is counted as dropped).
const noWall time.Duration = 0

// droppedWall is what wallFromSidecar returns for a wall that cannot be used.
const droppedWall time.Duration = -1

// wallFromSidecar is a record-step sidecar's wall time: ended_at minus
// started_at, both RFC 3339 with nanoseconds (kiln-record-step/2).  It returns
// droppedWall for a stamp that does not parse and for a difference that is not
// positive.
func wallFromSidecar(sc record.StepSidecar) time.Duration {
	a, err1 := time.Parse(time.RFC3339Nano, sc.StartedAt)
	b, err2 := time.Parse(time.RFC3339Nano, sc.EndedAt)
	if err1 != nil || err2 != nil {
		return droppedWall
	}
	if d := b.Sub(a); d > 0 {
		return d
	}
	return droppedWall
}

// wallFacts is the " step_wall_ms=.. parse_ms=.. parse_share=.." tail for a
// step that took wall and whose grammar pass 1 took parse, and "" when there is
// no usable wall.  dropped is true (and the caller should count it) when wall
// is negative, which is how a supplied but unusable wall arrives; noWall (0)
// means "none supplied" and is not a drop.
// The share is parse over wall; wall > 0 is checked first, so it never divides
// by zero.
func wallFacts(wall, parse time.Duration) (facts string, dropped bool) {
	switch {
	case wall == noWall:
		return "", false
	case wall < 0:
		return "", true
	}
	return fmt.Sprintf(" step_wall_ms=%d parse_ms=%.2f parse_share=%.4f",
		wall.Milliseconds(), float64(parse.Microseconds())/1000, parse.Seconds()/wall.Seconds()), false
}

// traceProbeLine assembles the payload of a "PROBE: trace" line: the numbers
// first, then the wall facts, then the dropped count, and the two refusals
// last.  head is everything the line says about the trace before the wall facts.
func traceProbeLine(head, wall string, droppedSoFar int64, oldRefusal, newRefusal string) string {
	dropped := ""
	if droppedSoFar > 0 {
		dropped = fmt.Sprintf(" walls_dropped=%d", droppedSoFar)
	}
	return fmt.Sprintf("%s%s%s old_refusal=%q grammar_refusal=%q", head, wall, dropped, oldRefusal, newRefusal)
}

func TestTraceProbeLineRefusalsComeLast(t *testing.T) {
	// A refusal that imitates the fields the CI script trusts.
	evil := "bad line best_mb_s=99 gated=1 step_wall_ms=1"
	facts, dropped := wallFacts(2*time.Second, 40*time.Millisecond)
	if dropped {
		t.Fatal("a positive wall was dropped")
	}
	line := traceProbeLine("PROBE: trace x bytes=1 lines=1 best_mb_s=25.0 gated=1", facts, 0, evil, evil)
	cut := strings.Index(line, " old_refusal=")
	if cut < 0 {
		t.Fatalf("no old_refusal in %q", line)
	}
	if strings.Contains(line[:cut], "99") || strings.Contains(line[:cut], "bad line") {
		t.Errorf("refusal text leaked in front of the first refusal field: %q", line[:cut])
	}
	for _, want := range []string{"step_wall_ms=2000", "parse_ms=40.00", "parse_share=0.0200"} {
		if !strings.Contains(line[:cut], want) {
			t.Errorf("%q is not before the refusals in %q", want, line)
		}
	}
	if strings.Count(line, " old_refusal=") != 1 || !strings.HasSuffix(line, fmt.Sprintf("grammar_refusal=%q", evil)) {
		t.Errorf("the refusals are not the last two fields: %q", line)
	}
}

func TestWallFactsDropsNonPositiveWalls(t *testing.T) {
	if f, d := wallFacts(noWall, time.Millisecond); f != "" || d {
		t.Errorf("no wall supplied: %q %v", f, d)
	}
	for _, w := range []time.Duration{droppedWall, -5 * time.Second} {
		if f, d := wallFacts(w, time.Millisecond); f != "" || !d {
			t.Errorf("wall %v: %q dropped=%v", w, f, d)
		}
	}
	// Even a wall of one nanosecond divides safely.
	f, d := wallFacts(time.Nanosecond, 0)
	if d || !strings.Contains(f, "parse_share=0.0000") || strings.Contains(f, "NaN") || strings.Contains(f, "Inf") {
		t.Errorf("tiny wall: %q %v", f, d)
	}
	if got := traceProbeLine("h", "", 3, "", ""); !strings.Contains(got, "walls_dropped=3") {
		t.Errorf("the dropped count is not on the line: %q", got)
	}
	if got := traceProbeLine("h", "", 0, "", ""); strings.Contains(got, "walls_dropped") {
		t.Errorf("a zero drop count was logged: %q", got)
	}
}

func TestWallFromSidecar(t *testing.T) {
	sc := record.StepSidecar{StartedAt: "2026-11-14T10:00:00Z", EndedAt: "2026-11-14T10:00:01.250000001Z"}
	if got := wallFromSidecar(sc); got != 1250000001*time.Nanosecond {
		t.Errorf("wall %v", got)
	}
	for name, bad := range map[string]record.StepSidecar{
		"equal stamps": {StartedAt: sc.StartedAt, EndedAt: sc.StartedAt},
		"clock back":   {StartedAt: sc.EndedAt, EndedAt: sc.StartedAt},
		"unparsed":     {StartedAt: "yesterday", EndedAt: sc.EndedAt},
		"empty":        {},
	} {
		if got := wallFromSidecar(bad); got != droppedWall {
			t.Errorf("%s: %v", name, got)
		}
	}
}

// Is a probe report free of timing?  A structural check, not a substring one
// =========================================================================
//
// pre/probes.json embeds temp-directory paths built from the test's name, so a
// search for the text "wall" in it finds the PATH, not a field.  What matters is
// the report's SHAPE: no object key anywhere is a timing key, and a run object
// has exactly the keys the sealed schema has (derived from the encoder on a
// zero-wall run, not written out here), so a field of any name added later
// fails too.

// sealedRunKeys is the set of keys of one run object as EncodeProbes writes it.
func sealedRunKeys(t testing.TB) map[string]bool {
	t.Helper()
	data, err := record.EncodeProbes(record.ProbeReport{Schema: record.ProbesSchema, Runs: []record.ProbeRunRecord{{Name: "x"}}})
	if err != nil {
		t.Fatal(err)
	}
	var doc struct {
		Runs []map[string]any `json:"runs"`
	}
	if err := json.Unmarshal(data, &doc); err != nil || len(doc.Runs) != 1 {
		t.Fatalf("cannot read the encoder's own output: %v %s", err, data)
	}
	keys := map[string]bool{}
	for k := range doc.Runs[0] {
		keys[k] = true
	}
	return keys
}

// timingKey says an object key names a duration.
func timingKey(k string) bool {
	k = strings.ToLower(k)
	return strings.HasPrefix(k, "wall") || k == "elapsed" || k == "duration"
}

// probeReportProblems lists what is wrong with a probes.json document: a timing
// key at any depth, or a run object whose keys are not exactly runKeys.
func probeReportProblems(data []byte, runKeys map[string]bool) []string {
	var doc any
	if err := json.Unmarshal(data, &doc); err != nil {
		return []string{"not JSON: " + err.Error()}
	}
	var problems []string
	var walk func(path string, v any)
	walk = func(path string, v any) {
		switch x := v.(type) {
		case map[string]any:
			for k, child := range x {
				if timingKey(k) {
					problems = append(problems, fmt.Sprintf("timing key %q at %s", k, path))
				}
				walk(path+"."+k, child)
			}
		case []any:
			for i, child := range x {
				walk(fmt.Sprintf("%s[%d]", path, i), child)
			}
		}
	}
	walk("$", doc)
	if top, ok := doc.(map[string]any); ok {
		runs, _ := top["runs"].([]any)
		for i, r := range runs {
			obj, _ := r.(map[string]any)
			for k := range obj {
				if !runKeys[k] {
					problems = append(problems, fmt.Sprintf("runs[%d] has the key %q, which the sealed schema does not", i, k))
				}
			}
			for k := range runKeys {
				if _, ok := obj[k]; !ok {
					problems = append(problems, fmt.Sprintf("runs[%d] lacks the key %q", i, k))
				}
			}
		}
	}
	return problems
}

func TestProbeReportProblemsIsStructural(t *testing.T) {
	keys := sealedRunKeys(t)
	rep := record.ProbeReport{Schema: record.ProbesSchema, Action: "/x/wall/elapsed", Shadowed: []string{"/tmp/wall_time"},
		Runs: []record.ProbeRunRecord{{Name: "wall", Argv: []string{"wall"}, Wall: time.Second}}}
	data, err := record.EncodeProbes(rep)
	if err != nil {
		t.Fatal(err)
	}
	// The text "wall" is all over the values, and that is not a problem.
	if p := probeReportProblems(data, keys); len(p) != 0 {
		t.Errorf("a clean report was flagged: %v", p)
	}
	for name, bad := range map[string]string{
		"wall key":      `{"runs":[{"name":"a","argv":[],"exit":0,"trace_bytes":1,"trace_sha256":"","wall_ns":5}]}`,
		"upper key":     `{"Elapsed":1,"runs":[]}`,
		"nested":        `{"runs":[],"toolchains":[{"tool":"t","duration":1}]}`,
		"unknown field": `{"runs":[{"name":"a","argv":[],"exit":0,"trace_bytes":1,"trace_sha256":"","took":5}]}`,
		"missing field": `{"runs":[{"name":"a"}]}`,
	} {
		if len(probeReportProblems([]byte(bad), keys)) == 0 {
			t.Errorf("%s: not flagged", name)
		}
	}
}
