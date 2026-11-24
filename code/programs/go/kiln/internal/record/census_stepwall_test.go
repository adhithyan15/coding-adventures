package record

// Tests of the per-language step wall and the unknown-write-set reason codes
// (census_stepwall.go).  Portable: no file is touched.

import (
	"reflect"
	"strings"
	"testing"
)

// Walls arrive in milliseconds and leave as seconds.  The median is nearest
// rank: the lower middle for an even count.
func TestLangWallOf(t *testing.T) {
	for _, c := range []struct {
		name string
		ms   []float64
		want LangWall
	}{
		{"no samples", nil, LangWall{}},
		{"one sample", []float64{589000}, LangWall{N: 1, P50: 589, Max: 589, Sum: 589}},
		{"odd count, unsorted", []float64{3000, 1000, 2000}, LangWall{N: 3, P50: 2, Max: 3, Sum: 6}},
		{"even count takes the lower middle", []float64{4000, 1000, 3000, 2000}, LangWall{N: 4, P50: 2, Max: 4, Sum: 10}},
		{"sub-second rounds to three decimals", []float64{1, 2}, LangWall{N: 2, P50: 0.001, Max: 0.002, Sum: 0.003}},
	} {
		t.Run(c.name, func(t *testing.T) {
			in := append([]float64(nil), c.ms...)
			if got := langWallOf(c.ms); got != c.want {
				t.Errorf("got %+v, want %+v", got, c.want)
			}
			if !reflect.DeepEqual(in, append([]float64(nil), c.ms...)) {
				t.Error("the input was modified")
			}
		})
	}
}

// The census files each step's wall under its language, and failed steps
// (non-zero root status) also under a separate distribution.
func TestCensusLangWallByOutcome(t *testing.T) {
	a := newCensusAcc(nil)
	a.langs["rust"] = &langAcc{
		walls:       []float64{1000, 589000, 3000},
		failedWalls: []float64{589000},
	}
	a.langs["go"] = &langAcc{walls: []float64{2000, 4000}}
	s := a.summary(10)
	got := map[string]LangSummary{}
	for _, l := range s.Languages {
		got[l.Language] = l
	}
	r := got["rust"]
	if r.StepWallS != (LangWall{N: 3, P50: 3, Max: 589, Sum: 593}) || r.StepWallFailedS != (LangWall{N: 1, P50: 589, Max: 589, Sum: 589}) {
		t.Errorf("rust: %+v / failed %+v", r.StepWallS, r.StepWallFailedS)
	}
	g := got["go"]
	if g.StepWallS != (LangWall{N: 2, P50: 2, Max: 4, Sum: 6}) || g.StepWallFailedS != (LangWall{}) {
		t.Errorf("go: %+v / failed %+v", g.StepWallS, g.StepWallFailedS)
	}
	md := renderMarkdown(s)
	if !strings.Contains(md, "step wall (s) n 3, p50 3, max 589, sum 593; failed steps (s) n 1, p50 589, max 589, sum 589") {
		t.Errorf("markdown lacks the rust wall line")
	}
}

// A prefix up to the first colon picks the code; anything unknown is "other".
func TestUnknownReasonCode(t *testing.T) {
	for _, c := range []struct{ in, want string }{
		{"serial-violated: seq 3 was used twice", UnknownSerialViolated},
		{"serial-violated: traces/.seq is not a counter", UnknownSerialViolated},
		{"interrupted: a step started and was never accounted for (1 of 2)", UnknownInterrupted},
		{"not run: the build stopped before this step", UnknownNotRun},
		{"serial-violated", UnknownSerialViolated}, // no colon: the whole text is the prefix
		{"mystery: something new", UnknownOther},
		{"serial-violated-ish: no", UnknownOther}, // a longer word is not the prefix
		{"not-run: dashed is not the spelling", UnknownOther},
		{"", UnknownOther},
		{"free text with no colon", UnknownOther},
	} {
		if got := unknownReasonCode(c.in); got != c.want {
			t.Errorf("%q: got %q, want %q", c.in, got, c.want)
		}
	}
}

// Record lines are counted, reasons coded, and bad or over-long lines survive.
func TestScanRecordLines(t *testing.T) {
	long := `{"unknown_writes":"interrupted: x","pad":"` + strings.Repeat("a", censusRecordsReadChunk) + "\"}\n"
	in := `{"action":"a","unknown_writes":""}` + "\n" +
		`{"action":"b","unknown_writes":"serial-violated: seq 1 was used twice"}` + "\n" +
		`{"action":"c","unknown_writes":"interrupted: a step started"}` + "\n" +
		`{"action":"d","unknown_writes":"not run: the build stopped before this step"}` + "\n" +
		`{"action":"e","unknown_writes":"brand new"}` + "\n" +
		"not json\n" +
		// A nested empty reason beside a real top-level one: the top level decides.
		`{"action":"n","detail":{"unknown_writes":""},"unknown_writes":"serial-violated: x"}` + "\n" +
		// A nested non-empty reason with an empty top-level one: not a reason.
		`{"action":"m","detail":{"unknown_writes":"interrupted: nested"},"unknown_writes":""}` + "\n" +
		long +
		`{"action":"f","unknown_writes":"not run: again"}` + "\n" +
		`{"action":"unterminated","unknown_writes":"not run: never counted"}`
	codes := map[string]int{}
	n := scanRecordLines(strings.NewReader(in), 1<<40, codes)
	if n != 10 {
		t.Errorf("lines = %d, want 10", n)
	}
	want := map[string]int{UnknownSerialViolated: 2, UnknownInterrupted: 1, UnknownNotRun: 2, UnknownOther: 1}
	if !reflect.DeepEqual(codes, want) {
		t.Errorf("codes = %v, want %v", codes, want)
	}
	// The byte limit cuts the read.
	if n := scanRecordLines(strings.NewReader(in), 10, map[string]int{}); n != 0 {
		t.Errorf("limit 10: lines = %d", n)
	}
	if n := scanRecordLines(strings.NewReader(in), -5, map[string]int{}); n != 0 {
		t.Errorf("negative limit: lines = %d", n)
	}
}
