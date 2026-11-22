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
		// A small trace's pass makes garbage in proportion to its size (the grammar
		// path allocates about 46 bytes per input byte: up to about 6 MB for a
		// 128 KB trace), so a collection during it is not the reader's cost: it is
		// a stop-the-world wait, which on a busy machine can take tens of
		// milliseconds.  Switch the collector off for the (bounded: at most
		// maxPasses passes) measurement, after one collection to start from a
		// clean heap.  The bound on the heap that piles up is therefore about
		// maxPasses x 6 MB = 1.2 GB in the worst case (BestPass1Calibrated adds the
		// reference kernel's roughly 28 bytes per byte on top, over a 64 KB slice:
		// a few MB per iteration); the usual 250 ms span ends the loop after a few
		// dozen passes, a few hundred MB.  A large trace keeps the
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

// ---------------------------------------------------------------------------
// The reference kernel: a ruler for the host (gate G1, KILN04 section 4.9)
// ---------------------------------------------------------------------------
//
// GitHub's runner fleet varies about 1.75x in CPU speed, so a raw MB/s floor on
// the grammar path gives false failures on the slow hosts.  To tell "the reader
// got slower" from "this host is slower", every calibrated measurement also times
// a small, fixed, PARSER-SHAPED kernel (refKernel), and the test rescales the
// reader's rate by it: norm_mb_s = best_mb_s * refNominalMBs / ref_mb_s.
//
// The kernel is deliberately not a pure clock meter (a first candidate, a serial
// multiply chain, was rejected at a 1.39x cross-host spread): it does the kinds
// of work the parser does, so it slows down on a host in the same way the parser
// does.  It splits the input on newlines; per line it makes a token slice of
// capacity 4 and appends heap token nodes (so the slice grows and the allocator
// runs); it finds token boundaries with a byte-class table and dispatches with a
// `switch` on the class (different work per arm); it copies each token to a
// string and looks it up in a small prefilled map; it keeps the last 64 lines
// live in a ring; and it hashes once per TOKEN, never per byte, so there is no
// serial multiply chain.  It is NOT allocation-free: it allocates about 28 bytes
// per input byte (the grammar path about 46).
//
// *** FROZEN.  The kernel, its tables, its dictionary, its pass count
// *** (refRepeats) and the nominal constant calibrated to it (refNominalMBs in
// *** sandbox_tracesize_linux_amd64_test.go, 72.0) must never change: edit any of
// *** it and the ruler moves silently, making the nominal constant wrong.  If the
// *** kernel must change, recalibrate (KILN04 section 4.9) in the same PR.
//
// The result goes to package-level sinks so the compiler cannot delete the work.
const (
	refRepeats = 2 // kernel passes per timed sample: FROZEN
	fnvOffset  = 14695981039346656037
	fnvPrime   = 1099511628211
)

// Byte classes: 0 other, 1 identifier, 2 digit, 3 quote, 4 escape, 5 bracket,
// 6 comma, 7 space.
const (
	clsOther = iota
	clsIdent
	clsDigit
	clsQuote
	clsEscape
	clsBracket
	clsComma
	clsSpace
)

// refClass maps a byte to its class.  Built once at init, read-only afterwards.
var refClass = func() (t [256]uint8) {
	for c := 0; c < 256; c++ {
		switch {
		case c >= 'a' && c <= 'z', c >= 'A' && c <= 'Z', c == '_':
			t[c] = clsIdent
		case c >= '0' && c <= '9':
			t[c] = clsDigit
		case c == '"':
			t[c] = clsQuote
		case c == '\\':
			t[c] = clsEscape
		case c == '(' || c == ')' || c == '[' || c == ']' || c == '{' || c == '}':
			t[c] = clsBracket
		case c == ',':
			t[c] = clsComma
		case c == ' ' || c == '\t' || c == '\n':
			t[c] = clsSpace
		}
	}
	return
}()

// refSink and refSinkS keep the kernel's results alive: a value stored in a
// package-level variable cannot be proved dead.
var (
	refSink  uint64
	refSinkS string
)

// refTok is one token of the kernel, a heap node like the parser's records: its
// kind (a byte class, or 8 for a quoted string), byte range, the token's text
// copied to the heap, and numeric values.
type refTok struct {
	kind       uint8
	start, end int32
	text       string
	val        uint64
	aux        uint64
	extra      uint64
}

// refLine is what the kernel keeps live per line: the token pointers (grown by
// append, so it allocates) and the last identifier.
type refLine struct {
	toks []*refTok
	name string
}

// refRing holds the last 64 lines so their allocations stay live, as a parser's
// records do.
var refRing [64]refLine

// refDict is the small prefilled map identifier tokens are looked up in.
var refDict = func() map[string]int {
	m := make(map[string]int)
	for i, w := range []string{
		"openat", "read", "write", "close", "execve", "mmap", "munmap", "stat",
		"fstat", "lstat", "access", "getcwd", "readlink", "clone", "wait4",
		"exit_group", "O_RDONLY", "O_CLOEXEC", "AT_FDCWD", "ENOENT", "PROT_READ",
		"MAP_PRIVATE", "NULL", "resumed", "unfinished", "pid",
	} {
		m[w] = i + 1
	}
	return m
}()

// refKernel is the frozen reference workload: one pass over data (see the
// section comment for what it does and why).
func refKernel(data []byte) uint64 {
	h := uint64(fnvOffset)
	depth, commas := 0, 0
	lineNo := 0
	for len(data) > 0 {
		end := bytes.IndexByte(data, '\n')
		var line []byte
		if end < 0 {
			line, data = data, nil
		} else {
			line, data = data[:end], data[end+1:]
		}
		toks := make([]*refTok, 0, 4)
		name := ""
		for i := 0; i < len(line); {
			c := refClass[line[i]]
			s := i
			i++
			var kind uint8 = c
			switch c {
			case clsIdent:
				for i < len(line) && (refClass[line[i]] == clsIdent || refClass[line[i]] == clsDigit) {
					i++
				}
				name = string(line[s:i]) // heap copy: kept live in the ring
				h = (h ^ uint64(refDict[name]+1)) * fnvPrime
			case clsDigit:
				v := uint64(line[s] - '0')
				for i < len(line) && refClass[line[i]] == clsDigit {
					v = v*10 + uint64(line[i]-'0')
					i++
				}
				h = (h ^ v) * fnvPrime
			case clsQuote:
				kind = 8
				for i < len(line) && line[i] != '"' {
					if line[i] == '\\' {
						i++
					}
					i++
				}
				if i < len(line) {
					i++
				}
				h = (h ^ uint64(i-s)) * fnvPrime
			case clsBracket:
				if line[s] == '(' || line[s] == '[' || line[s] == '{' {
					depth++
				} else {
					depth--
				}
				h = (h ^ uint64(depth&0xff)) * fnvPrime
			case clsComma:
				commas++
				h = (h ^ uint64(commas)) * fnvPrime
			case clsSpace:
				continue // whitespace is skipped, not a token
			default:
				h = (h ^ uint64(line[s])) * fnvPrime
			}
			toks = append(toks, &refTok{kind: kind, start: int32(s), end: int32(i), text: string(line[s:i]), val: h, aux: uint64(refDict[string(line[s:i])]), extra: uint64(depth)})
		}
		refRing[lineNo&(len(refRing)-1)] = refLine{toks: toks, name: name}
		lineNo++
	}
	return h ^ uint64(depth) ^ uint64(commas)
}

// refSample is one timed reference sample: refRepeats passes of the kernel.
func refSample(data []byte) time.Duration {
	t0 := time.Now()
	var h uint64
	for i := 0; i < refRepeats; i++ {
		h ^= refKernel(data)
	}
	d := time.Since(t0)
	refSink ^= h
	refSinkS = refRing[0].name
	return d
}

// RefBytes is how many bytes one reference sample processes for data: divide it
// by the best reference time to get the host's reference rate (ref_mb_s).
func RefBytes(data []byte) int { return refRepeats * len(data) }

// BestPass1Calibrated is BestPass1 plus the host ruler.  Each iteration of the
// usual loop (at least repeats iterations spanning at least 250 ms, at most 200,
// collector off and restored for a small trace) times one reference sample and
// then one grammar pass, so both see the same stretches of host noise; the
// minimum of each is kept (noise only adds time).  The untimed refusal-check
// pass doubles as the grammar warm-up; the kernel has one untimed warm-up
// sample.  It returns the best grammar pass time, the best reference sample
// time, and the refusal of the pass ("" when it read every line).  CI/test use
// only (see BestPass1).
func BestPass1Calibrated(data []byte, helperRoot bool, repeats int) (best, refBest time.Duration, refusal string) {
	lim := DefaultLimits()
	_, _, e, _ := readRecordsAt(bytes.NewReader(data), lim, helperRoot, true) // refusal check and warm-up
	if e != nil {
		refusal = e.Reason
	}
	refSample(data)
	if len(data) <= smallTrace {
		runtime.GC()
		defer debug.SetGCPercent(debug.SetGCPercent(-1))
	}
	start := time.Now()
	for i := 0; i < repeats || (time.Since(start) < minSpan && i < maxPasses); i++ {
		if d := refSample(data); i == 0 || d < refBest {
			refBest = d
		}
		t0 := time.Now()
		readRecordsAt(bytes.NewReader(data), lim, helperRoot, true)
		if d := time.Since(t0); i == 0 || d < best {
			best = d
		}
	}
	return best, refBest, refusal
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
// seven 160-byte lines quoted at up to four bytes a byte, is well under the cap.)
var maxDiagnoseBytes = 8192

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
//   - a window of +-3 lines around the EARLIER stop line, the last earlier
//     line from the same pid as the stop line (a refused "resumed" line pairs
//     with a pending half that can be far above the window), and each reader's
//     Error line when it differs from its stop line;
//
// Reasons and line numbers never hold path text, so they are always printed.
// The window quotes trace lines, and real traces hold paths a pull request
// chose, so it is printed only when KILN_TRACE_DIFF=1 in the environment, the
// same policy as the cross-check's window (sanitisedWindow: each line cut to 160
// bytes and Go-quoted, so no raw control character and no workflow-command line
// start).  The refused line, the stop line's pid's last earlier line and a
// reader's own Error line are key lines: shown with their first AND last 160
// bytes, and printed before the window.  The whole result is at most
// maxDiagnoseBytes (8 KB).
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
	// Diagnose is pass 1 (the line reader) only.  A trace both readers accept
	// here can still be refused later, by pass 2 (the model).
	fmt.Fprintf(&b, "  pass 1 only: if both readers accepted, any refusal is in pass 2 (the model), not shown here\n")
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
		// The key lines come BEFORE the window, so the size cap can only cut the
		// neighbours: each is shown with its head and its tail.
		//  - a reader's own Error line when it is not its stop line (an unresumed
		//    call at the end of the input is reported at the call's start);
		//  - the last earlier line of the stop line's pid (a resume's pending half).
		shown := map[int]bool{}
		for _, r := range []struct {
			name string
			e    *Error
			at   int
		}{{"old scanner", eOld, atOld}, {"grammar reader", eNew, atNew}} {
			if r.e != nil && r.e.Line > 0 && r.e.Line != r.at && !shown[r.e.Line] {
				shown[r.e.Line] = true
				fmt.Fprintf(&b, "  error line (%s):\n%s\n", r.name, windowOf(data, r.e.Line, 0, true))
			}
		}
		if prev, pid := lastLineOfSamePid(data, stop); prev > 0 && !shown[prev] {
			fmt.Fprintf(&b, "  last earlier line of pid %d:\n%s\n", pid, windowOf(data, prev, 0, true))
		}
		fmt.Fprintf(&b, "  window around line %d:\n%s\n", stop, windowOf(data, stop, 3, true))
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
