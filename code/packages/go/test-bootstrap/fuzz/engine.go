package fuzz

import (
	"errors"
	"fmt"
	"reflect"
	"runtime"
	"strings"

	"github.com/adhithyan15/coding-adventures/code/packages/go/test-bootstrap/prng"
)

// Campaign ceilings and defaults (CWE-400: nothing a caller controls is
// unbounded).
const (
	// MaxCrashes is how many distinct crash signatures a report holds; further
	// novel ones only increment Report.CrashesDropped.
	MaxCrashes = 1000
	// MaxInitialCorpus bounds the number of seeds handed to a campaign.
	MaxInitialCorpus = 10_000
	// MaxStepBudget and MaxMemBudget are the ceilings of the two cooperative
	// budgets a target can observe.
	MaxStepBudget = 100_000_000
	MaxMemBudget  = 1 << 30
	// MaxMinimizeBudget caps the executions spent minimising ONE crash.
	MaxMinimizeBudget = 100_000
	// DefaultMinimizeBudget, DefaultStepBudget and DefaultMemBudget are the
	// DefaultOptions values.
	DefaultMinimizeBudget = 2000
	DefaultStepBudget     = 200_000
	DefaultMemBudget      = 64 << 20
)

// Coverage tier labels.  The label reflects what was actually collected.
const (
	// TierReported: at least one edge was reported by the target.
	TierReported = "caller-reported"
	// TierBlackBox: no edge was collected, so there was no feedback.
	TierBlackBox = "black-box-uninstrumented"
)

// ErrOptions is wrapped by every validation failure of Options, the corpus or
// an outcome.
var ErrOptions = errors.New("fuzz: invalid argument")

func invalid(format string, args ...any) error {
	return fmt.Errorf("%w: %s", ErrOptions, fmt.Sprintf(format, args...))
}

// -----------------------------------------------------------------------------
// Options
// -----------------------------------------------------------------------------

// Options bounds a campaign; every field is validated.
//
// MinimizeBudget is the most target executions spent minimising ONE novel crash
// (ceiling MaxMinimizeBudget).  On top of it the whole campaign spends at most
// MaxIterations minimisation executions in total, so minimisation can at most
// double a campaign's work however many distinct crashes it finds.  A crash
// found after that total is spent is still recorded, with Minimized == false.
// The Rust lane has neither cap (VER06e section 9).
type Options struct {
	MaxIterations  int
	Seed           uint64
	MaxInputLen    int
	BaseEnergy     int
	MinimizeBudget int
	// StepBudget is how many h.Tick() calls one execution may make before it is
	// a timeout.  MemBudget is how many bytes one execution may h.Reserve.
	StepBudget int
	MemBudget  int
}

// DefaultOptions is the standard campaign: 1000 iterations, seed 42, inputs up
// to 4096 bytes.
func DefaultOptions() Options {
	return Options{
		MaxIterations: 1000, Seed: 42, MaxInputLen: 4096, BaseEnergy: 10,
		MinimizeBudget: DefaultMinimizeBudget, StepBudget: DefaultStepBudget, MemBudget: DefaultMemBudget,
	}
}

// Validate checks every field against its range.
func (o Options) Validate() error {
	for _, c := range []struct {
		name  string
		v, hi int
	}{
		{"MaxIterations", o.MaxIterations, MaxIterations},
		{"MaxInputLen", o.MaxInputLen, MaxInputBytes},
		{"BaseEnergy", o.BaseEnergy, MaxIterations},
		{"MinimizeBudget", o.MinimizeBudget, MaxMinimizeBudget},
		{"StepBudget", o.StepBudget, MaxStepBudget},
		{"MemBudget", o.MemBudget, MaxMemBudget},
	} {
		if c.v < 0 || c.v > c.hi {
			return invalid("%s must be in [0, %d]", c.name, c.hi)
		}
	}
	return nil
}

// -----------------------------------------------------------------------------
// Outcomes
// -----------------------------------------------------------------------------

// OutcomeKind says how one execution ended.
type OutcomeKind int

// The four outcomes.
const (
	OutcomePass OutcomeKind = iota
	OutcomeCrash
	OutcomeTimeout
	OutcomeOOM
)

// Outcome is the result of one execution.  CrashKind and Frames are set only
// for OutcomeCrash.
type Outcome struct {
	Kind      OutcomeKind
	CrashKind string
	Frames    []string
}

// ExecFunc runs the code under test on data and reports the edges it executed
// into cov (the campaign resets cov before each call).  Run builds one from a
// Target; tests and other front ends may supply their own.
type ExecFunc func(data []byte, cov *Bitmap) Outcome

// -----------------------------------------------------------------------------
// The target-side handle and the executor
// -----------------------------------------------------------------------------

// Target is the code under test.  It receives its own copy of the input and a
// Handle for reporting coverage and observing budgets.  A panic is a crash.
type Target func(data []byte, h *Handle)

// Handle is what a Target sees: coverage reporting and two deterministic
// budgets.  It is reused across executions and must not be retained.
type Handle struct {
	cov                 *Bitmap
	steps, stepBudget   int
	reserved, memBudget int
	deep                []uintptr // lazily allocated stack buffer for deep crashes
}

// EdgeID derives a stable edge id from a name (FNV-1a), so a campaign is
// deterministic across runs and machines (no pointers, no hash randomisation).
func EdgeID(label string) uint64 { return prng.FNV1a64String(label) }

// Edge reports that the target executed the program point with this id.  Ids
// are reduced modulo 65,536.
func (h *Handle) Edge(id uint64) { h.cov.Record(id) }

// Tick counts one unit of work.  After StepBudget ticks the execution is a
// timeout: Tick panics with a private sentinel, and keeps panicking on every
// later call, so a target that swallows one recover cannot keep ticking past
// the budget.  A target that loops without ever ticking cannot be stopped (Go
// cannot pre-empt a goroutine; VER06e section 5), so CI needs an external
// wall-clock limit.
func (h *Handle) Tick() {
	if h.steps >= h.stepBudget {
		panic(stepBudgetExceeded{h})
	}
	h.steps++
}

// Reserve declares an allocation of n bytes about to be made on behalf of the
// input.  When the running total of one execution exceeds MemBudget the
// execution is an out-of-memory outcome.  (Real Go OOM is a fatal error nothing
// can recover.)  A negative n reserves nothing.
func (h *Handle) Reserve(n int) {
	if n < 0 {
		return
	}
	if n > h.memBudget-h.reserved {
		panic(memBudgetExceeded{h})
	}
	h.reserved += n
}

// The sentinels carry the Handle that raised them so an inner campaign never
// claims an outer campaign's budget panic.
type (
	stepBudgetExceeded struct{ h *Handle }
	memBudgetExceeded  struct{ h *Handle }
)

// harnessName is the runtime name of runTarget, used to cut stacks.
var harnessName string

// init resolves harnessName in an init function (a package-level initialiser
// would be an initialisation cycle: runTarget uses it).
func init() { harnessName = runtime.FuncForPC(reflect.ValueOf(runTarget).Pointer()).Name() }

const (
	// stackDepth is how many program counters the first, stack-allocated capture
	// holds: enough for any ordinary crash.
	stackDepth = 128
	// maxWalkPCs is the second, heap capture used when the first filled up (deep
	// recursion).  Beyond it the stack is not inspected, so a recursion deeper
	// than this hashes without its callers' frames (VER06e section 5).
	maxWalkPCs = 4096
)

// deepStack captures up to maxWalkPCs program counters, starting at the
// deferred handler that called it (skip 2: Callers, deepStack).  The buffer is
// allocated once per Handle, on the first deep crash.
func (h *Handle) deepStack() []uintptr {
	if h.deep == nil {
		h.deep = make([]uintptr, maxWalkPCs)
	}
	return h.deep[:runtime.Callers(2, h.deep)]
}

// runTarget runs one execution under recover.  It must not be inlined away or
// renamed carelessly: its name delimits the target's frames (harnessName).
func runTarget(target Target, data []byte, h *Handle) (out Outcome) {
	finished := false
	defer func() {
		if finished {
			return
		}
		r := recover()
		var first [stackDepth]uintptr
		pcs := first[:runtime.Callers(1, first[:])]
		if len(pcs) == len(first) { // possibly truncated: walk deeper
			pcs = h.deepStack()
		}
		var claimed bool
		out, claimed = classify(r, h, pcs)
		if !claimed {
			panic(r) // another harness's budget sentinel: let it unwind to its owner
		}
	}()
	target(append([]byte(nil), data...), h)
	finished = true
	return Outcome{Kind: OutcomePass}
}

// classify turns a recovered panic value into an outcome.  claimed is false for
// a budget sentinel that belongs to a different Handle.
func classify(r any, h *Handle, pcs []uintptr) (out Outcome, claimed bool) {
	switch s := r.(type) {
	case stepBudgetExceeded:
		return Outcome{Kind: OutcomeTimeout}, s.h == h
	case memBudgetExceeded:
		return Outcome{Kind: OutcomeOOM}, s.h == h
	}
	kind := "nil"
	if r != nil {
		kind = normaliseKind(fmt.Sprintf("%T", r))
	}
	return Outcome{Kind: OutcomeCrash, CrashKind: kind, Frames: framesOf(pcs)}, true
}

// framesOf normalises a captured stack: innermost first, runtime and harness
// frames dropped, cut at the executor's own frame.
func framesOf(pcs []uintptr) []string {
	var frames []string
	prevRaw := ""
	it := runtime.CallersFrames(pcs)
	for {
		f, more := it.Next()
		switch {
		case f.Function == harnessName:
			return collapse(frames)
		case f.Function == "", strings.HasPrefix(f.Function, "runtime."),
			strings.HasPrefix(f.Function, harnessName+".func"):
			// engine / runtime plumbing, not the target's
		default:
			// Stream-collapse on the raw name (a recursive crasher is thousands
			// of identical frames: normalise each distinct run once) and stop
			// once maxFrames distinct frames are held.
			if len(frames) == 0 || f.Function != prevRaw {
				prevRaw = f.Function
				frames = append(frames, normaliseFrame(f.Function))
				if len(frames) >= maxFrames {
					return collapse(frames)
				}
			}
		}
		if !more {
			return collapse(frames)
		}
	}
}

// Run runs a bounded coverage-guided campaign against target.  See RunOutcomes
// for the contracts.  The coverage tier is TierReported when the target called
// h.Edge at least once during the campaign, TierBlackBox otherwise.
func Run(initialCorpus [][]byte, opts Options, target Target) (*Report, error) {
	if target == nil {
		return nil, invalid("target is nil")
	}
	if err := opts.Validate(); err != nil {
		return nil, err
	}
	h := &Handle{stepBudget: opts.StepBudget, memBudget: opts.MemBudget}
	return RunOutcomes(initialCorpus, opts, func(data []byte, cov *Bitmap) Outcome {
		h.cov, h.steps, h.reserved = cov, 0, 0
		return runTarget(target, data, h)
	})
}

// -----------------------------------------------------------------------------
// Reports
// -----------------------------------------------------------------------------

// CrashRecord is one deduplicated crash.  The minimal input is PRIVATE
// (VER06 section 6.3): it is reachable only through MinimalInput, never through
// a default rendering, and public output goes through FormatTAPFailure only.
//
// Privacy caveat: the safe rendering comes from the Format/String methods.
// Reflection-based printers that do not call them (encoding/json aside, which
// only sees exported fields) and a %v of a caller's struct that holds this value
// in an UNEXPORTED field bypass Format and would print the private fields.  Do
// not embed it in such a struct; print it directly or use TAPFailures.
type CrashRecord struct {
	Signature string
	Kind      string
	Frames    []string
	Hits      int
	// Minimized is false when minimisation was skipped or cut short by a budget,
	// so the minimal input is only the shortest input seen, not a fixed point.
	Minimized bool
	minimal   []byte
}

// MinimalInput returns a copy of the shortest input known to reproduce the
// signature.  PRIVATE: never log it.
func (c CrashRecord) MinimalInput() []byte { return append([]byte(nil), c.minimal...) }

// String renders the signature, kind, frames and counts, never input bytes.
func (c CrashRecord) String() string {
	return fmt.Sprintf("fuzz.CrashRecord{sig=%s kind=%s frames=%v hits=%d minimized=%t}",
		c.Signature, c.Kind, c.Frames, c.Hits, c.Minimized)
}

// GoString is String, so %#v is as private as %v.
func (c CrashRecord) GoString() string { return c.String() }

// Format routes every fmt verb to String: the unexported minimal input would
// otherwise be printed by %v, %+v and %#v of the struct.
func (c CrashRecord) Format(f fmt.State, _ rune) { _, _ = f.Write([]byte(c.String())) }

// Report is what a campaign found.  Every execution lands in exactly one
// bucket.  Iterations counts MUTATION executions (triage is not counted);
// Crashes/Timeouts/OOMs count triage and mutation executions; anything that
// passed is in the corpus or was redundant.
//
// Privacy caveat: the safe rendering comes from the Format/String methods.
// Reflection-based printers that do not call them (encoding/json aside, which
// only sees exported fields) and a %v of a caller's struct that holds this value
// in an UNEXPORTED field bypass Format and would print the private fields.  Do
// not embed it in such a struct; print it directly or use TAPFailures.
type Report struct {
	Iterations    int
	AdmittedSeeds int
	TotalEdges    int
	Crashes       []CrashRecord
	Timeouts      int
	OOMs          int
	// CrashesDropped counts novel signatures beyond MaxCrashes.
	CrashesDropped int
	CoverageTier   string
	// seed and corpus are PRIVATE (VER06 section 6): unexported, so no default
	// rendering or encoder can reach them.
	seed   uint64
	corpus [][]byte
}

// IsClean reports whether the campaign can be read as "found nothing": at least
// one mutation iteration ran and no execution, triage included, crashed, timed
// out or ran out of memory.  A campaign that never got past triage, or ran zero
// iterations, fuzzed nothing.
func (r Report) IsClean() bool {
	return r.Iterations > 0 && len(r.Crashes) == 0 && r.Timeouts == 0 && r.OOMs == 0 && r.CrashesDropped == 0
}

// UniqueCrashes is the number of distinct crash signatures seen.
func (r Report) UniqueCrashes() int { return len(r.Crashes) + r.CrashesDropped }

// Seed returns the campaign seed.  PRIVATE.
func (r Report) Seed() uint64 { return r.seed }

// Corpus returns copies of the admitted seeds.  PRIVATE, like MinimalInput.
func (r Report) Corpus() [][]byte {
	out := make([][]byte, len(r.corpus))
	for i, c := range r.corpus {
		out[i] = append([]byte(nil), c...)
	}
	return out
}

// String renders counts, never seeds or bytes.
func (r Report) String() string {
	return fmt.Sprintf("fuzz.Report{iterations=%d admitted=%d edges=%d crashes=%d timeouts=%d ooms=%d dropped=%d tier=%s clean=%t}",
		r.Iterations, r.AdmittedSeeds, r.TotalEdges, len(r.Crashes), r.Timeouts, r.OOMs, r.CrashesDropped, r.CoverageTier, r.IsClean())
}

// GoString is String, so %#v is as private as %v.
func (r Report) GoString() string { return r.String() }

// Format routes every fmt verb to String (see CrashRecord.Format).
func (r Report) Format(f fmt.State, _ rune) { _, _ = f.Write([]byte(r.String())) }

// -----------------------------------------------------------------------------
// Minimisation (VER06 section 5.3)
// -----------------------------------------------------------------------------

func sameCrash(o Outcome, sig string) bool {
	return o.Kind == OutcomeCrash && CrashSignature(o.CrashKind, o.Frames) == sig
}

// minimize shrinks data while it keeps the exact signature.  It returns the
// bytes, the executions spent and whether it finished (false: the budget
// stopped it).
func minimize(data []byte, sig string, test func([]byte) Outcome, budget int) ([]byte, int, bool) {
	cur := append([]byte(nil), data...)
	spent := 0
	reproduces := func(candidate []byte) bool {
		spent++
		return sameCrash(test(candidate), sig)
	}
	if len(cur) == 0 {
		return cur, 0, true
	}
	// Pass 1: delete blocks, halving the block size.  Starting at max(1, len/2)
	// (not len/2) is what makes the empty input reachable from a 1-byte crasher.
	for chunk := max(1, len(cur)/2); chunk > 0; chunk /= 2 {
		for offset := 0; offset < len(cur); {
			if spent >= budget {
				return cur, spent, false
			}
			end := offset + min(chunk, len(cur)-offset)
			candidate := append(append([]byte(nil), cur[:offset]...), cur[end:]...)
			if reproduces(candidate) {
				cur = candidate // same offset again: the tail slid left
				continue
			}
			offset += chunk
		}
	}
	// Pass 2: zero individual non-essential bytes.
	for i := range cur {
		if cur[i] == 0 {
			continue
		}
		if spent >= budget {
			return cur, spent, false
		}
		original := cur[i]
		cur[i] = 0
		if !reproduces(cur) {
			cur[i] = original
		}
	}
	return cur, spent, true
}

// MinimizeCrashingInput shrinks data while it keeps the EXACT crash signature:
// pass 1 deletes blocks, pass 2 zeroes bytes.  A candidate that crashes
// differently (another signature) is rejected, so minimisation never trades one
// bug for another.  budget caps the executions of test; it is always finite
// (0..MaxMinimizeBudget, ErrOptions otherwise).
func MinimizeCrashingInput(data []byte, sig string, test func([]byte) Outcome, budget int) ([]byte, error) {
	if budget < 0 || budget > MaxMinimizeBudget {
		return nil, invalid("budget must be in [0, %d]", MaxMinimizeBudget)
	}
	if test == nil {
		return nil, invalid("test is nil")
	}
	out, _, _ := minimize(data, sig, test, budget)
	return out, nil
}

// -----------------------------------------------------------------------------
// The campaign (VER06 section 7, contracts of section 9.1)
// -----------------------------------------------------------------------------

type seedEntry struct {
	data       []byte
	newEdges   int
	generation int
}

type campaign struct {
	exec       ExecFunc
	opts       Options
	rng        *prng.SplitMix64
	step       Bitmap // edges of the current execution
	cumulative Bitmap
	corpus     []seedEntry
	crashes    []CrashRecord
	bySig      map[string]int
	timeouts   int
	ooms       int
	dropped    int
	// minimizeLeft is the campaign-wide minimisation allowance.
	minimizeLeft int
}

// execute runs one input and validates the outcome kind.  The edges of the run
// are left in c.step.
func (c *campaign) execute(data []byte) (Outcome, error) {
	c.step.Reset()
	out := c.exec(data, &c.step)
	if out.Kind < OutcomePass || out.Kind > OutcomeOOM {
		return Outcome{}, invalid("exec returned an unknown outcome kind %d", int(out.Kind))
	}
	return out, nil
}

// probe is the minimiser's re-run: coverage is discarded and nothing counted.
func (c *campaign) probe(data []byte) Outcome {
	out, err := c.execute(data)
	if err != nil {
		return Outcome{Kind: OutcomePass} // an invalid kind is surfaced by the main loop
	}
	return out
}

// file puts one non-passing execution in its bucket.
func (c *campaign) file(out Outcome, data []byte) {
	switch out.Kind {
	case OutcomeTimeout:
		c.timeouts++
	case OutcomeOOM:
		c.ooms++
	case OutcomeCrash:
		c.recordCrash(out, data)
	}
}

// admit puts a passing input in the corpus iff it covered a new edge.  The first
// passing seed is always admitted, even with no coverage, so there is something
// to mutate in black-box mode.
func (c *campaign) admit(data []byte, generation int) {
	hasNew, n := c.cumulative.Admit(c.step.edges)
	if hasNew || (generation == 0 && len(c.corpus) == 0) {
		c.corpus = append(c.corpus, seedEntry{append([]byte(nil), data...), n, generation})
	}
}

func (c *campaign) recordCrash(out Outcome, data []byte) {
	sig := CrashSignature(out.CrashKind, out.Frames)
	if i, seen := c.bySig[sig]; seen {
		rec := &c.crashes[i]
		rec.Hits++
		if len(data) < len(rec.minimal) {
			rec.minimal = append([]byte(nil), data...)
		}
		return
	}
	if len(c.crashes) >= MaxCrashes {
		c.dropped++
		return
	}
	allowance := min(c.opts.MinimizeBudget, c.minimizeLeft)
	minimal, spent, finished := minimize(data, sig, c.probe, allowance)
	c.minimizeLeft -= spent
	c.bySig[sig] = len(c.crashes)
	c.crashes = append(c.crashes, CrashRecord{
		Signature: sig, Kind: out.CrashKind, Frames: append([]string(nil), out.Frames...),
		Hits: 1, Minimized: finished, minimal: minimal,
	})
}

// RunOutcomes runs a bounded coverage-guided campaign over exec.
//
// Contracts (VER06 section 9.1), each pinned by a test:
//
//   - a pure function of (corpus, options, exec): same inputs, same report;
//   - with a live corpus Iterations == opts.MaxIterations exactly, and no mutated
//     candidate exceeds opts.MaxInputLen (initial seeds run as given, up to
//     MaxInputBytes each);
//   - an empty corpus is replaced by one 00 seed; seeds that crash, time out or
//     exhaust memory are never mutated, so a corpus of crashers fuzzes nothing
//     (Iterations == 0);
//   - every execution lands in Crashes, Timeouts, OOMs or the corpus; IsClean
//     needs at least one iteration and no failure at all;
//   - one CrashRecord per signature; a novel crash is minimised.
//
// Per iteration the stream draws: Bounded(len(corpus)) for the seed; per energy
// slot 1+Bounded(4) steps, one NextU64 sub-seed, then, if len(corpus) > 1,
// Bounded(4) and on 0 a donor index and two split draws (as Rust and Python).
func RunOutcomes(initialCorpus [][]byte, opts Options, exec ExecFunc) (*Report, error) {
	if err := opts.Validate(); err != nil {
		return nil, err
	}
	if exec == nil {
		return nil, invalid("exec is nil")
	}
	if len(initialCorpus) > MaxInitialCorpus {
		return nil, invalid("initial corpus exceeds %d seeds", MaxInitialCorpus)
	}
	for i, s := range initialCorpus {
		if len(s) > MaxInputBytes {
			return nil, invalid("initial corpus seed %d exceeds %d bytes", i, MaxInputBytes)
		}
	}
	seeds := initialCorpus
	if len(seeds) == 0 {
		seeds = [][]byte{{0}}
	}
	c := &campaign{
		exec: exec, opts: opts, rng: prng.New(opts.Seed),
		bySig: map[string]int{}, minimizeLeft: opts.MaxIterations,
	}

	// 1. Triage: run every seed once.
	for _, data := range seeds {
		out, err := c.execute(data)
		if err != nil {
			return nil, err
		}
		if out.Kind == OutcomePass {
			c.admit(data, 0)
		} else {
			c.file(out, data)
		}
	}

	// 2. The mutation loop.
	iterations := 0
	for iterations < opts.MaxIterations && len(c.corpus) > 0 {
		seed := c.corpus[c.rng.Bounded(uint64(len(c.corpus)))]
		energy := CalculateEnergy(opts.BaseEnergy, len(seed.data), seed.newEdges, c.cumulative.Count())
		for slot := 0; slot < energy && iterations < opts.MaxIterations; slot++ {
			iterations++
			steps := 1 + int(c.rng.Bounded(4))
			sub := c.rng.NextU64()
			candidate, _ := MutateSequence(seed.data, sub, steps, MaxInputBytes) // steps <= 4: cannot fail
			if len(c.corpus) > 1 && c.rng.Bounded(4) == 0 {
				donor := c.corpus[c.rng.Bounded(uint64(len(c.corpus)))].data
				ts := int64(c.rng.Bounded(uint64(len(candidate) + 1)))
				ds := int64(c.rng.Bounded(uint64(len(donor) + 1)))
				candidate = Splice(candidate, donor, ts, ds)
			}
			if len(candidate) > opts.MaxInputLen {
				candidate = candidate[:opts.MaxInputLen]
			}
			out, err := c.execute(candidate)
			if err != nil {
				return nil, err
			}
			if out.Kind == OutcomePass {
				c.admit(candidate, seed.generation+1)
			} else {
				c.file(out, candidate)
			}
		}
	}

	total := c.cumulative.Count()
	tier := TierBlackBox
	if total > 0 {
		tier = TierReported
	}
	corpus := make([][]byte, len(c.corpus))
	for i, s := range c.corpus {
		corpus[i] = s.data
	}
	return &Report{
		Iterations: iterations, AdmittedSeeds: len(c.corpus), TotalEdges: total,
		Crashes: c.crashes, Timeouts: c.timeouts, OOMs: c.ooms, CrashesDropped: c.dropped,
		CoverageTier: tier, seed: opts.Seed, corpus: corpus,
	}, nil
}
