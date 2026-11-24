package pbt

import (
	"errors"
	"fmt"
	"strings"

	"github.com/adhithyan15/coding-adventures/code/packages/go/test-bootstrap/prng"
)

// =============================================================================
// Closed failure vocabulary (VER05 section 5.1)
// =============================================================================

const (
	// FailureCounterexample: the property was falsified; a minimal witness was found.
	FailureCounterexample = "property-counterexample"
	// FailureExhausted: nothing (or no valid input) was evaluated.
	FailureExhausted = "property-exhausted"
	// FailureTimeout is registered but never produced by this lane.
	FailureTimeout = "property-timeout"
	// FailurePanic: the property itself panicked.
	FailurePanic = "property-panic"
)

// FailureIDs is the closed set of ids, in registration order.
var FailureIDs = [4]string{FailureCounterexample, FailureExhausted, FailureTimeout, FailurePanic}

// IsPBTFailureID is true only for one of the four registered ids.
//
//	IsPBTFailureID("property-panic")   -> true
//	IsPBTFailureID("unregistered-id")  -> false
func IsPBTFailureID(candidate string) bool {
	for _, id := range FailureIDs {
		if candidate == id {
			return true
		}
	}
	return false
}

// =============================================================================
// Options and CI redaction (VER05 section 5.2)
// =============================================================================

// CIMode says whether output is public CI output.  The zero value redacts:
// the engine fails closed, because a seed plus a property is a working exploit
// reproducer (VER00 section 7.3) and this package cannot read the environment
// to find out where it is running.
type CIMode int

const (
	// CIUnset (the zero value) is treated as CI: witnesses and seeds are redacted.
	CIUnset CIMode = iota
	// CIOn redacts.
	CIOn
	// CIOff is local development: messages carry the seed and the witness.
	CIOff
)

// CIFromLookup is pure CI detection.  Pass os.LookupEnv: it reports true when
// CI is set to anything except empty, "0" or "false" (trimmed, any case).  The
// package calls no os function itself.
func CIFromLookup(lookup func(key string) (string, bool)) bool {
	value, found := lookup("CI")
	if !found {
		return false
	}
	switch strings.ToLower(strings.TrimSpace(value)) {
	case "", "0", "false":
		return false
	}
	return true
}

// Options configures CheckProperty.  A nil *Options means DefaultOptions.
type Options struct {
	// NumTests is how many cases to run.  0 is legal and is a failure: a
	// property that never ran proves nothing.
	NumTests int
	// MaxShrinkSteps is the shrink budget, 0..MaxShrinkSteps.
	MaxShrinkSteps int
	// CI selects redaction.
	CI CIMode
}

// DefaultOptions are 100 cases, 1000 shrink steps and CI redaction.
func DefaultOptions() Options {
	return Options{NumTests: 100, MaxShrinkSteps: 1000}
}

// Verdict is the outcome of CheckProperty.  In redacted mode Seed,
// Counterexample and Minimal are zero; only the message, iteration, step count
// and digest remain.
type Verdict struct {
	Passed         bool
	FailureID      string
	Message        string
	CasesRun       int
	Iteration      int
	ShrinkSteps    int
	Seed           *uint64
	Counterexample any
	Minimal        any
	Digest         string
}

// Failed is the negation of Passed.
func (v Verdict) Failed() bool { return !v.Passed }

// Failure is the error form of a failed verdict.
type Failure struct {
	ID      string
	Message string
}

func (f *Failure) Error() string { return "[" + f.ID + "] " + f.Message }

// Err is nil for a pass and a *Failure otherwise, for assertion-style tests:
//
//	v, err := pbt.CheckProperty(seed, gen, prop, nil)
//	if err != nil { t.Fatal(err) }
//	if e := v.Err(); e != nil { t.Fatal(e) }
func (v Verdict) Err() error {
	if v.Passed {
		return nil
	}
	return &Failure{ID: v.FailureID, Message: v.Message}
}

// =============================================================================
// The honest runner (VER05 section 7.1)
// =============================================================================

// CheckProperty runs prop over opts.NumTests candidates from seed.  prop
// returns true when the property holds.
//
// Verdict honesty (VER05 section 7.1):
//
//   - zero cases evaluated is property-exhausted, never a pass;
//   - a generator error or panic is property-exhausted;
//   - a property panic is property-panic;
//   - the reported minimal witness is re-verified: if the shrinker panics,
//     returns something that no longer fails, a value the property panics on,
//     or a nonsensical step count, the original candidate is reported with 0
//     steps;
//   - redacted mode never emits the witness or the seed.
//
// The error result is only for invalid options; the Verdict is then a failure.
func CheckProperty[T any](seed uint64, gen Gen[T], prop func(T) bool, opts *Options) (Verdict, error) {
	return CheckPropertyShrink(seed, gen, prop, opts, nil)
}

// CheckPropertyShrink is CheckProperty with an explicit shrinker that overrides
// the generator's own; it exists so a defective shrinker can be tested.
func CheckPropertyShrink[T any](seed uint64, gen Gen[T], prop func(T) bool, opts *Options, shrink ShrinkFunc[T]) (Verdict, error) {
	o := DefaultOptions()
	if opts != nil {
		o = *opts
	}
	notPass := Verdict{FailureID: FailureExhausted}
	if prop == nil {
		return notPass, argErr("nil property")
	}
	if o.NumTests < 0 || o.NumTests > MaxCases {
		return notPass, argErr("NumTests %d outside [0, %d]", o.NumTests, MaxCases)
	}
	if err := checkBudget(o.MaxShrinkSteps); err != nil {
		return notPass, err
	}
	redact := o.CI != CIOff

	if o.NumTests == 0 {
		return Verdict{
			FailureID: FailureExhausted,
			Message:   "no cases were evaluated (NumTests is 0); a property that never ran is not a pass",
		}, nil
	}

	rng := newRNG(seed)
	for iteration := 1; iteration <= o.NumTests; iteration++ {
		candidate, err := safeSample(gen, rng)
		if err != nil {
			return exhaustedVerdict(err, iteration, redact), nil
		}
		holds, panicked, panicValue := safeProp(prop, candidate)
		if panicked {
			return panicVerdict(seed, iteration, panicValue, redact), nil
		}
		if !holds {
			return falsified(seed, iteration, candidate, gen, prop, shrink, o, redact), nil
		}
	}
	return Verdict{Passed: true, CasesRun: o.NumTests}, nil
}

// generatorPanic wraps a recovered panic from a generator so the runner can
// treat it as a generator error.
type generatorPanic struct{ value any }

func (g generatorPanic) Error() string { return "generator panicked: " + safeString(g.value) }

func safeSample[T any](gen Gen[T], rng *prng.SplitMix64) (v T, err error) {
	defer func() {
		if r := recover(); r != nil {
			err = generatorPanic{r}
		}
	}()
	return gen.Sample(rng)
}

func safeProp[T any](prop func(T) bool, v T) (holds, panicked bool, value any) {
	defer func() {
		if r := recover(); r != nil {
			holds, panicked, value = false, true, r
		}
	}()
	return prop(v), false, nil
}

func exhaustedVerdict(err error, iteration int, redact bool) Verdict {
	var text string
	switch {
	case isExhausted(err):
		text = fmt.Sprintf("filter precondition rejected all candidates at iteration %d", iteration)
	case redact:
		text = fmt.Sprintf("generator error at iteration %d (%T)", iteration, err)
	default:
		text = fmt.Sprintf("generator error at iteration %d: %s", iteration, safeString(err))
	}
	return Verdict{FailureID: FailureExhausted, Message: text, CasesRun: iteration - 1, Iteration: iteration}
}

func isExhausted(err error) bool { return errors.Is(err, ErrExhausted) }

func panicVerdict(seed uint64, iteration int, value any, redact bool) Verdict {
	if redact {
		return Verdict{
			FailureID: FailurePanic,
			Message:   fmt.Sprintf("property evaluation panicked at iteration %d (%T)", iteration, value),
			CasesRun:  iteration, Iteration: iteration,
		}
	}
	s := seed
	return Verdict{
		FailureID: FailurePanic,
		Message:   fmt.Sprintf("property evaluation panicked at iteration %d: %s", iteration, safeString(value)),
		CasesRun:  iteration, Iteration: iteration, Seed: &s,
	}
}

func falsified[T any](seed uint64, iteration int, candidate T, gen Gen[T], prop func(T) bool, shrink ShrinkFunc[T], o Options, redact bool) Verdict {
	// A candidate the property panics on is not a counterexample to the claim;
	// it is a different failure, so it is never accepted as "smaller".
	falsifies := func(v T) bool {
		holds, panicked, _ := safeProp(prop, v)
		return !panicked && !holds
	}
	minimal, steps := candidate, 0
	shrinker := shrink
	if shrinker == nil {
		shrinker = gen.Shrink
	}
	func() {
		defer func() { _ = recover() }() // untrusted shrinker: fall back to the original
		shrunk, used := shrinker(candidate, falsifies, o.MaxShrinkSteps)
		// Re-verify: the witness we report must really fail the property.
		if used >= 0 && used <= o.MaxShrinkSteps && falsifies(shrunk) {
			minimal, steps = shrunk, used
		}
	}()

	if redact {
		preimage := fmt.Sprintf("iter:%d;steps:%d;shrunk:%s", iteration, steps, Canon(minimal))
		digest := fmt.Sprintf("%016x", prng.FNV1a64String(preimage))
		return Verdict{
			FailureID: FailureCounterexample,
			Message: fmt.Sprintf("property falsified after %d iterations (shrunk in %d steps, repro-fnv: %s)",
				iteration, steps, digest),
			CasesRun: iteration, Iteration: iteration, ShrinkSteps: steps, Digest: digest,
		}
	}
	s := seed
	return Verdict{
		FailureID: FailureCounterexample,
		Message: fmt.Sprintf("property falsified at iteration %d (seed=%d): initial=%s, minimal=%s after %d shrink steps",
			iteration, seed, Canon(candidate), Canon(minimal), steps),
		CasesRun: iteration, Iteration: iteration, ShrinkSteps: steps,
		Seed: &s, Counterexample: candidate, Minimal: minimal,
	}
}
