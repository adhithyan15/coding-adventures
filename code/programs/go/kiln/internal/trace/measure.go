package trace

import (
	"bytes"
	"fmt"
	"io"
	"os"
	"runtime"
	"runtime/debug"
	"strings"
	"time"
)

// Measurement is what Measure found out about one trace.
type Measurement struct {
	Bytes   int           // size of the trace
	Lines   int           // number of lines
	OldTime time.Duration // pass 1 with the hand-written scanner
	NewTime time.Duration // pass 1 with the grammar path (one shot: census only, noisy on small traces)
	// NewBest is the MINIMUM pass-1 time of the grammar path over Repeats timed
	// passes after one warm-up pass (0 when no repeats were asked for).  The
	// minimum is the right statistic for a capability floor: noise from the
	// timer, the scheduler or a neighbour only ever adds time, never removes it.
	// Gate G1 (KILN04 section 4.9) uses this, not NewTime.
	NewBest time.Duration
	Repeats int
	// OldAllocs and NewAllocs are the heap allocations of each pass (a
	// process-wide MemStats.Mallocs delta: indicative only when other goroutines
	// or parallel tests allocate at the same time).
	OldAllocs uint64
	NewAllocs uint64
	OldError  string // "" or the refusal reason
	NewError  string
	// NewWindow is, when only the grammar path refused, a bounded, sanitised
	// window of the lines around the line it refused.
	NewWindow string
}

// Measure reads a trace (at most MaxTotalBytes of it) and times pass 1, the line
// reader, with each implementation.  It exists for D24's default-flip gate
// (KILN04 section 4.9): the kiln-sandbox CI job logs the sizes of the traces real
// strace produces and the grammar path's throughput on them, so the gate is
// stated from measurements.  It changes nothing and returns nothing a caller can
// trust as a parse: use Parse for that.
func Measure(r io.Reader, helperRoot bool) (Measurement, error) {
	return MeasureBest(r, helperRoot, 0)
}

// MeasureBest and BestPass1 are for the kiln-sandbox CI job and tests ONLY.  For
// a small trace they switch the garbage collector off for the duration with
// debug.SetGCPercent, which is process-global and not goroutine-safe: they must
// not be called concurrently with each other or with anything that cares about
// the collector.  (TestMeasureIsNotUsedByProductionCode keeps them out of
// non-test code.)  The figure they give is taken with the collector OFF,
// production runs with it on, so it flatters the reader by a few percent.
//
// MeasureBest is Measure plus, when repeats > 0, one untimed warm-up pass and
// then at least repeats timed passes (spanning at least 250 ms, at most 200 passes)
// of the grammar path's pass 1, keeping the minimum
// in NewBest.  NewTime stays the first, single-shot figure.
func MeasureBest(r io.Reader, helperRoot bool, repeats int) (Measurement, error) {
	lim := DefaultLimits()
	data, err := io.ReadAll(io.LimitReader(r, lim.MaxTotalBytes))
	if err != nil {
		return Measurement{}, err
	}
	m := measureData(data, helperRoot, lim)
	if repeats > 0 {
		m.NewBest, m.Repeats = bestPass1(data, helperRoot, lim, repeats), repeats
	}
	return m, nil
}

// BestPass1 is the minimum grammar-path pass-1 time over at least repeats passes
// spanning at least 250 ms (after one warm-up pass) on data already in memory, and the refusal reason of the
// pass ("" when it read every line).
func BestPass1(data []byte, helperRoot bool, repeats int) (time.Duration, string) {
	lim := DefaultLimits()
	_, _, e, _ := readRecordsAt(bytes.NewReader(data), lim, helperRoot, true)
	reason := ""
	if e != nil {
		reason = e.Reason
	}
	return bestPass1(data, helperRoot, lim, repeats), reason
}

const (
	smallTrace = 128 << 10
	minSpan    = 250 * time.Millisecond
	maxPasses  = 200
)

func bestPass1(data []byte, helperRoot bool, lim Limits, repeats int) time.Duration {
	readRecordsAt(bytes.NewReader(data), lim, helperRoot, true) // warm-up: caches, page faults
	if len(data) <= smallTrace {
		// A small trace's pass makes under a megabyte of garbage, so a collection
		// during it is not the reader's cost: it is a stop-the-world wait, which on
		// a busy machine can take tens of milliseconds.  Switch the collector off
		// for the (bounded: at most maxPasses passes) measurement, after one
		// collection to start from a clean heap.  A large trace keeps the
		// collector on: its passes are long enough that a pause is a small share,
		// and its garbage is too big to pile up.
		runtime.GC()
		defer debug.SetGCPercent(debug.SetGCPercent(-1))
	}
	// At least `repeats` passes AND at least minSpan of wall time: on a small trace
	// (about 2 ms a pass) seven passes fit inside one scheduler quantum, so a
	// neighbour that holds the CPU for 50 ms would slow every one of them.  Spread
	// over 250 ms, one descheduling cannot cover all the passes.
	var best time.Duration
	start := time.Now()
	for i := 0; i < repeats || (time.Since(start) < minSpan && i < maxPasses); i++ {
		t0 := time.Now()
		readRecordsAt(bytes.NewReader(data), lim, helperRoot, true)
		if d := time.Since(t0); i == 0 || d < best {
			best = d
		}
	}
	return best
}

func measureData(data []byte, helperRoot bool, lim Limits) Measurement {
	m := Measurement{Bytes: len(data), Lines: bytes.Count(data, []byte{'\n'})}
	var ms0, ms1 runtime.MemStats
	runtime.ReadMemStats(&ms0)
	t0 := time.Now()
	_, _, e := readRecords(bytes.NewReader(data), lim, helperRoot, false)
	m.OldTime = time.Since(t0)
	runtime.ReadMemStats(&ms1)
	m.OldAllocs = ms1.Mallocs - ms0.Mallocs
	if e != nil {
		m.OldError = e.Reason
	}
	runtime.ReadMemStats(&ms0)
	t0 = time.Now()
	_, _, e, at := readRecordsAt(bytes.NewReader(data), lim, helperRoot, true)
	m.NewTime = time.Since(t0)
	runtime.ReadMemStats(&ms1)
	m.NewAllocs = ms1.Mallocs - ms0.Mallocs
	if e != nil {
		m.NewError = e.Reason
		m.NewWindow = sanitisedWindow(data, at)
	}
	return m
}

// ---------------------------------------------------------------------------
// Diagnose: what to log when a real trace is refused and then lost
// ---------------------------------------------------------------------------

// maxDiagnoseBytes caps Diagnose's output.  A CI log is not the place for an
// unbounded dump, and the window can hold lines a pull request chose.  (A
// variable only so a test can lower it: the worst case a real window reaches,
// seven 160-byte lines quoted at up to four bytes a byte, sits near the cap.)
var maxDiagnoseBytes = 4096

// Diagnose explains, for a CI log, why pass 1 refuses a trace.  Like Measure it
// is for tests and the kiln-sandbox CI job ONLY (TestMeasureIsNotUsedByProductionCode
// keeps it out of production code).
//
// The incident that motivates it (D32): `kiln record prepare` refused a real
// strace trace as "unparsable-line", the trace lived in a t.TempDir that was gone
// by the time anyone looked, and the children that ran prepare had hand-built
// environments without KILN_TRACE_DIFF, so the parse was grammar-only and never
// cross-checked.  Nobody could say WHICH line was refused, or whether the old
// scanner would have refused it too.  Diagnose answers both from the bytes:
//
//   - it runs pass 1 with BOTH readers (the hand-written scanner and the grammar
//     path) and reports each one's reason, its Error line and its stop line (the
//     line being processed when it refused);
//   - the grammarPanics delta of the grammar run (it must be 0);
//   - the total line count and whether the data ends in a newline (a trace cut
//     off mid-line is refused at its last line for that reason alone);
//   - a window of +-3 lines around the EARLIER stop line, and the last earlier
//     line from the same pid as the stop line, because a refused "resumed" line
//     pairs with a pending half that can be far above the window.
//
// Reasons and line numbers never hold path text, so they are always printed.
// The window quotes trace lines, and real traces hold paths a pull request
// chose, so it is printed only when KILN_TRACE_DIFF=1 in the environment, the
// same policy as the cross-check's window (sanitisedWindow: each line cut to 160
// bytes and Go-quoted, so no raw control character and no workflow-command line
// start).  The whole result is at most maxDiagnoseBytes.
func Diagnose(data []byte, helperRoot bool) string {
	lim := DefaultLimits()
	_, _, eOld, atOld := readRecordsAt(bytes.NewReader(data), lim, helperRoot, false)
	before := grammarPanics.Load()
	_, _, eNew, atNew := readRecordsAt(bytes.NewReader(data), lim, helperRoot, true)
	panics := grammarPanics.Load() - before

	lines := bytes.Count(data, []byte{'\n'})
	endsNL := len(data) == 0 || data[len(data)-1] == '\n'
	if !endsNL {
		lines++
	}
	var b strings.Builder
	fmt.Fprintf(&b, "  trace: %d bytes, %d lines, ends with newline: %v, helper root: %v\n",
		len(data), lines, endsNL, helperRoot)
	describe := func(name string, e *Error, at int) {
		if e == nil {
			fmt.Fprintf(&b, "  %s: accepted pass 1 (stop line %d)\n", name, at)
			return
		}
		fmt.Fprintf(&b, "  %s: refused %q at error line %d, stop line %d\n", name, e.Reason, e.Line, at)
	}
	describe("old scanner", eOld, atOld)
	describe("grammar reader", eNew, atNew)
	fmt.Fprintf(&b, "  grammar panics recovered: %d\n", panics)

	// The earlier of the stop lines of the readers that refused.
	stop := 0
	for _, r := range []struct {
		e  *Error
		at int
	}{{eOld, atOld}, {eNew, atNew}} {
		if r.e != nil && r.at > 0 && (stop == 0 || r.at < stop) {
			stop = r.at
		}
	}
	if stop > 0 && os.Getenv("KILN_TRACE_DIFF") == "1" {
		fmt.Fprintf(&b, "  window around line %d:\n%s\n", stop, sanitisedWindowR(data, stop, 3))
		if prev, pid := lastLineOfSamePid(data, stop); prev > 0 && prev < stop-3 {
			fmt.Fprintf(&b, "  last earlier line of pid %d:\n%s\n", pid, sanitisedWindowR(data, prev, 0))
		}
	}
	out := b.String()
	if len(out) > maxDiagnoseBytes {
		// The window is quoted ASCII, so a byte cut cannot split a character.
		const mark = "\n  ...(truncated)"
		out = out[:max(0, maxDiagnoseBytes-len(mark))] + mark
	}
	return strings.TrimRight(out, "\n")
}

// lastLineOfSamePid finds the last line before line stop (1-based) that carries
// the same pid prefix as line stop, in one pass over the data.  It returns 0
// when line stop has no pid prefix or no earlier line shares it.
func lastLineOfSamePid(data []byte, stop int) (line, pid int) {
	pid, _, ok := parsePid(lineAt(data, stop))
	if !ok {
		return 0, 0
	}
	// The last earlier line with that pid.
	rest := data
	for i := 1; i < stop && len(rest) > 0; i++ {
		var l []byte
		if k := bytes.IndexByte(rest, '\n'); k < 0 {
			l, rest = rest, nil
		} else {
			l, rest = rest[:k], rest[k+1:]
		}
		if p, _, found := parsePid(string(l)); found && p == pid {
			line = i
		}
	}
	return line, pid
}
