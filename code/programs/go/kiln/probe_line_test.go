package main

import (
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
