package trace

import (
	"bytes"
	"io"
	"runtime"
	"runtime/debug"
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

// ---------------------------------------------------------------------------
// The reference kernels: rulers for the host (gate G1, KILN04 section 4.9)
// ---------------------------------------------------------------------------
//
// GitHub's runner fleet varies about 1.75x in CPU speed, so a raw MB/s floor on
// the grammar path gives false failures on the slow hosts.  To tell "the reader
// got slower" from "this host is slower", every calibrated measurement also times
// two small, fixed kernels, and the test rescales the reader's rate by them.
//
// Why two?  They are CANDIDATES.  Kernel A is a pure clock-frequency meter: a
// serial chain of dependent multiplies that knows nothing about branch
// prediction, out-of-order width, caches or the allocator, which are what drive
// the parser.  Kernel B is parser-shaped: it splits lines, tokenizes, allocates,
// dispatches through a switch, hashes strings into a map and keeps lines live.
// The calibration (KILN04 section 4.9) logs k_a and k_b on every leg; the one
// whose k varies LESS across hosts is frozen as the ruler and the other is
// deleted.
//
// *** FROZEN.  Both kernels, their tables, their pass counts (refARepeats,
// *** refBRepeats) and the nominal constant calibrated to them (refNominalMBs in
// *** sandbox_tracesize_linux_amd64_test.go) must never change: edit one and the
// *** ruler moves silently, making the nominal constant wrong.  If a kernel must
// *** change, recalibrate (KILN04 section 4.9) in the same PR.
//
// Both are allocation-free apart from what B deliberately allocates, and both
// write their result to a package-level sink so the compiler cannot delete them.
const (
	refStates   = 8
	refClasses  = 8
	refARepeats = 8 // kernel A passes per timed sample: FROZEN
	refBRepeats = 2 // kernel B passes per timed sample: FROZEN
	fnvOffset   = 14695981039346656037
	fnvPrime    = 1099511628211
)

// Byte classes shared by both kernels: 0 other, 1 identifier, 2 digit, 3 quote,
// 4 escape, 5 bracket, 6 comma, 7 space.
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

// refSink keeps the kernels' results alive: a value stored in a package-level
// variable cannot be proved dead.  refSinkS and refRing do the same for the
// strings and line slices kernel B builds.
var (
	refSink  uint64
	refSinkS string
)

// ---- Kernel A: the clock-frequency meter ----------------------------------

// refNext[state][class] is kernel A's transition table.  The numbers are
// arbitrary but fixed.
var refNext = [refStates][refClasses]uint8{
	{0, 1, 2, 3, 0, 4, 5, 7},
	{1, 1, 1, 3, 6, 4, 7, 7},
	{2, 1, 2, 3, 0, 4, 7, 7},
	{3, 3, 3, 7, 6, 3, 3, 3},
	{4, 1, 2, 3, 0, 4, 5, 0},
	{5, 1, 2, 3, 0, 4, 5, 7},
	{3, 3, 3, 3, 3, 3, 3, 3},
	{0, 1, 2, 3, 4, 5, 6, 0},
}

// refKernelA is a tokenizer-shaped state machine with a serial FNV-1a hash per
// byte (state 7 is the "accept" state and takes a data-dependent branch).  It
// allocates nothing.  Its cost is dominated by the dependent multiplies, so it
// measures clock speed and multiplier latency more than anything else.
func refKernelA(data []byte) uint64 {
	h := uint64(fnvOffset)
	state := uint8(0)
	for _, b := range data {
		state = refNext[state&(refStates-1)][refClass[b]&(refClasses-1)]
		if state == 7 {
			h ^= uint64(b) << 8
			h *= fnvPrime
		}
		h ^= uint64(state)
		h *= fnvPrime
		h ^= uint64(b)
		h *= fnvPrime
	}
	return h
}

func refSampleA(data []byte) time.Duration {
	t0 := time.Now()
	var h uint64
	for i := 0; i < refARepeats; i++ {
		h ^= refKernelA(data)
	}
	d := time.Since(t0)
	refSink ^= h
	return d
}

// ---- Kernel B: the parser-shaped candidate --------------------------------

// refTok is one token of kernel B, a heap node like the parser's records: its
// kind (a byte class, or 8 for a quoted string), byte range, the token's text
// copied to the heap, and a numeric value.
type refTok struct {
	kind       uint8
	start, end int32
	text       string
	val        uint64
	aux        uint64
	extra      uint64
}

// refLine is what kernel B keeps live per line: the token pointers (grown by
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

// refKernelB splits data on '\n'; per line it finds token boundaries with the
// class table, dispatches on the token kind with a switch (different work per
// arm), looks identifiers up in refDict after copying them to a heap string,
// and appends every token to a slice that starts at capacity 4 (so most lines
// grow it).  It hashes once per TOKEN, never per byte, so it has no serial
// multiply chain; its time is spent in branches, table loads, the allocator and
// the map, like the parser's.
func refKernelB(data []byte) uint64 {
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

func refSampleB(data []byte) time.Duration {
	t0 := time.Now()
	var h uint64
	for i := 0; i < refBRepeats; i++ {
		h ^= refKernelB(data)
	}
	d := time.Since(t0)
	refSink ^= h
	refSinkS = refRing[0].name
	return d
}

// RefBytes is how many bytes one reference sample processes for data under
// kernel A and under kernel B: divide by the best reference time to get the
// host's reference rate.
func RefBytes(data []byte) (a, b int) { return refARepeats * len(data), refBRepeats * len(data) }

// Calibrated is what BestPass1Calibrated measured: the minimum time of each of
// the grammar pass, reference sample A and reference sample B, and the refusal
// of the pass ("" when it read every line).
type Calibrated struct {
	Best, RefA, RefB time.Duration
	Refusal          string
}

// BestPass1Calibrated is BestPass1 plus the host rulers.  Each iteration of the
// usual loop (at least repeats iterations spanning at least 250 ms, at most 200,
// collector off and restored for a small trace) times one sample of kernel A,
// one of kernel B and one grammar pass, in that order, so all three see the same
// stretches of host noise; the minimum of each is kept (noise only adds time).
// The untimed refusal-check pass doubles as the grammar warm-up; each kernel has
// one untimed warm-up sample.  CI/test use only (see BestPass1).
func BestPass1Calibrated(data []byte, helperRoot bool, repeats int) Calibrated {
	lim := DefaultLimits()
	var r Calibrated
	_, _, e, _ := readRecordsAt(bytes.NewReader(data), lim, helperRoot, true) // refusal check and warm-up
	if e != nil {
		r.Refusal = e.Reason
	}
	refSampleA(data)
	refSampleB(data)
	if len(data) <= smallTrace {
		runtime.GC()
		defer debug.SetGCPercent(debug.SetGCPercent(-1))
	}
	start := time.Now()
	for i := 0; i < repeats || (time.Since(start) < minSpan && i < maxPasses); i++ {
		if d := refSampleA(data); i == 0 || d < r.RefA {
			r.RefA = d
		}
		if d := refSampleB(data); i == 0 || d < r.RefB {
			r.RefB = d
		}
		t0 := time.Now()
		readRecordsAt(bytes.NewReader(data), lim, helperRoot, true)
		if d := time.Since(t0); i == 0 || d < r.Best {
			r.Best = d
		}
	}
	return r
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
