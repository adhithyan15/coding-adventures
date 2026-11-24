package pbt

import (
	"errors"
	"fmt"
	"math/big"
	"slices"
)

// Bounds that keep every loop and allocation finite (CWE-400).  A caller or a
// fixture can ask for more only by being refused.
const (
	// MaxCases is the most cases CheckProperty will run.
	MaxCases = 100_000
	// MaxShrinkSteps is the largest shrink budget.
	MaxShrinkSteps = 10_000
	// MaxInputBytes is the longest Bytes generator.
	MaxInputBytes = 1 << 20
	// MaxListLen is the longest List generator.
	MaxListLen = 100_000
	// MaxGenBytesTotal bounds count x maxLen for GenBytes.
	MaxGenBytesTotal = 1 << 20
	// MaxFrequencyTotal is the largest total weight of Frequency: it draws with
	// a bounded 64-bit draw, and the VER05 limit is 2^32.
	MaxFrequencyTotal = 1 << 32
)

var (
	// ErrNotFalsifying is returned by the shrinkers when the start value
	// satisfies the property.  Shrinking a value that does not fail is a caller
	// bug; Go reports it as an error instead of panicking (VER05d section 9).
	ErrNotFalsifying = errors.New("pbt: initial value does not falsify the property")
	// ErrInvalidArgument wraps every rejected argument.
	ErrInvalidArgument = errors.New("pbt: invalid argument")
	// ErrExhausted means a generator could not produce a value, for example a
	// Filter that rejected every candidate.  The runner reports it as
	// property-exhausted.
	ErrExhausted = errors.New("pbt: generator exhausted")
)

func argErr(format string, args ...any) error {
	return fmt.Errorf("%w: %s", ErrInvalidArgument, fmt.Sprintf(format, args...))
}

// ShrinkFunc shrinks a failing value.  falsifies(v) is true while v still makes
// the property fail; the function returns a value that still falsifies (or the
// original) and the number of accepted moves.
type ShrinkFunc[T any] func(value T, falsifies func(T) bool, maxSteps int) (T, int)

// Sabotage hooks.  These are the only production indirections added for the
// negative controls of VER05d section 6: a test swaps one, runs the corpus, and
// the runner must report problems.  Production code never reassigns them.
var (
	// bisectCandidate is the bisection move: current - floor(diff/2).
	bisectCandidate = func(current, diff *big.Int) *big.Int {
		half := new(big.Int).Div(diff, big.NewInt(2)) // Euclidean = floor for divisor 2
		return half.Sub(current, half)
	}
	// unitCandidate is the unit move: one step toward the target.
	unitCandidate = func(current, diff *big.Int) *big.Int {
		return new(big.Int).Sub(current, big.NewInt(int64(diff.Sign())))
	}
	// skipDeletionPhase disables the single-deletion phase of sequence shrinking.
	skipDeletionPhase = false
)

var (
	bigOne = big.NewInt(1)
	bigTwo = big.NewInt(2)
)

func inBounds(v, lo, hi *big.Int) bool {
	return (lo == nil || lo.Cmp(v) <= 0) && (hi == nil || v.Cmp(hi) <= 0)
}

// intWalk bisects then unit-steps current toward target.
//
// Each turn tries, in order,
//
//  1. the bisection candidate current - floor(diff/2), diff = current - target,
//     when |diff| > 1;
//  2. the unit candidate current -/+ 1 toward the target.
//
// A candidate is accepted only if it is inside [lo, hi] (nil means unbounded),
// differs from current and still falsifies.  Only accepted moves count as
// steps.  No acceptable candidate ends the walk.
//
// Worked example (corpus pbt/shrink_int/54321_le_100, property x <= 100):
//
//	54321 -> 27161 -> 13581 -> ... -> 101        15 accepted moves
//
// 101 is the smallest value that still fails; 100 would satisfy the property,
// so neither the bisection nor the unit candidate from 101 is accepted.
func intWalk(current, target, lo, hi *big.Int, falsifies func(*big.Int) bool, budget int) (*big.Int, int) {
	steps := 0
	for steps < budget && current.Cmp(target) != 0 {
		diff := new(big.Int).Sub(current, target)
		moved := false
		if diff.CmpAbs(bigOne) > 0 {
			cand := bisectCandidate(current, diff)
			if cand.Cmp(current) != 0 && inBounds(cand, lo, hi) && falsifies(cand) {
				current, steps, moved = cand, steps+1, true
			}
		}
		if !moved {
			cand := unitCandidate(current, diff)
			if cand.Cmp(current) != 0 && inBounds(cand, lo, hi) && falsifies(cand) {
				current, steps, moved = cand, steps+1, true
			}
		}
		if !moved {
			break
		}
	}
	return current, steps
}

// shrinkIntCore is the target probe followed by the walk (VER05 section 4.2).
func shrinkIntCore(initial, target, lo, hi *big.Int, falsifies func(*big.Int) bool, maxSteps int) (*big.Int, int) {
	if maxSteps == 0 {
		return initial, 0 // no budget, no shrinking
	}
	if inBounds(target, lo, hi) && target.Cmp(initial) != 0 && falsifies(target) {
		return new(big.Int).Set(target), 1 // the target itself fails: cannot do better
	}
	return intWalk(initial, target, lo, hi, falsifies, maxSteps)
}

func byteWalk(value byte, falsifies func(byte) bool, budget int) (byte, int) {
	got, steps := intWalk(big.NewInt(int64(value)), big.NewInt(0), big.NewInt(0), big.NewInt(255),
		func(b *big.Int) bool { return falsifies(byte(b.Uint64())) }, budget)
	return byte(got.Uint64()), steps
}

func withElement[E any](current []E, index int, falsifies func([]E) bool) func(E) bool {
	return func(candidate E) bool {
		probe := slices.Clone(current)
		probe[index] = candidate
		return falsifies(probe)
	}
}

// shrinkSequence shrinks a falsifying sequence (VER05 section 4.3, corpus
// phases).  falsifies always receives a fresh slice it may keep or mutate.
//
//	Phase 0  probe.  minLen == 0: does the empty list fail?  Then it is the
//	         answer, one step.  (minLen > 0: probe the length-minLen prefix and
//	         carry on from it.)
//	Phase 1  bisection: keep the left half, else the right half, while one of
//	         them still fails.
//	Phase 2  single deletion, right to left.
//	Phase 3  element-wise, left to right, via shrinkElem (nil means none).
//
// Worked example (corpus list_sum_le_20, initial [10 5 8 4]): the empty list
// satisfies the property; both halves ([10 5], [8 4]) satisfy it too; deleting
// index 3 leaves [10 5 8] (sum 23, still failing); element-wise 10 -> 8 gives
// the 1-minimal [8 5 8].
func shrinkSequence[E any](initial []E, falsifies func([]E) bool, maxSteps, minLen int, shrinkElem ShrinkFunc[E]) ([]E, int) {
	current := slices.Clone(initial)
	steps := 0
	if maxSteps == 0 {
		return current, 0
	}
	try := func(cand []E) bool { return falsifies(slices.Clone(cand)) }

	if len(current) > minLen {
		probe := slices.Clone(current[:minLen])
		if try(probe) {
			if minLen == 0 {
				return probe, 1
			}
			current, steps = probe, 1
		}
	}
	floor := max(1, minLen)

	// Phase 1: bisection.
	for len(current) > floor && steps < maxSteps {
		mid := len(current) / 2
		left := slices.Clone(current[:mid])
		if len(left) >= minLen && try(left) {
			current, steps = left, steps+1
			continue
		}
		right := slices.Clone(current[mid:])
		if len(right) >= minLen && try(right) {
			current, steps = right, steps+1
			continue
		}
		break
	}

	// Phase 2: single-element deletion, right to left.
	if !skipDeletionPhase {
		for i := len(current) - 1; i >= 0 && len(current) > floor && steps < maxSteps; {
			cand := slices.Delete(slices.Clone(current), i, i+1)
			if try(cand) {
				current, steps = cand, steps+1
				i = min(i, len(current)-1)
			} else {
				i--
			}
		}
	}

	// Phase 3: element-wise, left to right.
	if shrinkElem != nil {
		for index := range current {
			remaining := maxSteps - steps
			if remaining <= 0 {
				break
			}
			value, used := shrinkElem(current[index], withElement(current, index, falsifies), remaining)
			if used > 0 {
				current = slices.Clone(current)
				current[index] = value
				steps += used
			}
		}
	}
	return current, steps
}

func checkBudget(maxSteps int) error {
	if maxSteps < 0 || maxSteps > MaxShrinkSteps {
		return argErr("maxSteps %d outside [0, %d]", maxSteps, MaxShrinkSteps)
	}
	return nil
}

// ShrinkInt shrinks a falsifying integer toward target inside [lo, hi]
// (the corpus calls these min and max).
//
// prop(x) is true when the property holds.  It returns (minimal, steps, nil);
// steps counts accepted moves only and the target probe is one move.  It
// returns ErrNotFalsifying when initial satisfies prop, and an error wrapping
// ErrInvalidArgument for a nil argument, lo > hi, an initial outside the
// bounds, or a budget outside [0, MaxShrinkSteps].  The result is never outside
// [lo, hi], and a value that does not fail is never returned.  big.Int cannot
// overflow, so extreme starts need no saturating subtraction.
func ShrinkInt(initial, target, lo, hi *big.Int, prop func(*big.Int) bool, maxSteps int) (*big.Int, int, error) {
	if initial == nil || target == nil || lo == nil || hi == nil || prop == nil {
		return nil, 0, argErr("nil argument")
	}
	if err := checkBudget(maxSteps); err != nil {
		return nil, 0, err
	}
	if lo.Cmp(hi) > 0 {
		return nil, 0, argErr("min exceeds max")
	}
	if !inBounds(initial, lo, hi) {
		return nil, 0, argErr("initial value must lie within [min, max]")
	}
	start := new(big.Int).Set(initial)
	if prop(new(big.Int).Set(start)) {
		return nil, 0, ErrNotFalsifying
	}
	falsifies := func(v *big.Int) bool { return !prop(new(big.Int).Set(v)) }
	got, steps := shrinkIntCore(start, target, lo, hi, falsifies, maxSteps)
	return got, steps, nil
}

func cloneBigs(in []*big.Int) []*big.Int {
	out := make([]*big.Int, len(in))
	for i, v := range in {
		out[i] = new(big.Int).Set(v)
	}
	return out
}

// ShrinkList shrinks a falsifying list of integers: empty probe, halves,
// single deletion, then each element toward elemTarget (unbounded).  prop is
// true when the list satisfies the property.
func ShrinkList(initial []*big.Int, elemTarget *big.Int, prop func([]*big.Int) bool, maxSteps int) ([]*big.Int, int, error) {
	if elemTarget == nil || prop == nil {
		return nil, 0, argErr("nil argument")
	}
	if err := checkBudget(maxSteps); err != nil {
		return nil, 0, err
	}
	items := make([]*big.Int, len(initial))
	for i, v := range initial {
		if v == nil {
			return nil, 0, argErr("nil element at index %d", i)
		}
		items[i] = new(big.Int).Set(v)
	}
	// The property gets deep copies, so one that scribbles on its argument
	// cannot corrupt the shrinker's state.
	holds := func(c []*big.Int) bool { return prop(cloneBigs(c)) }
	if holds(items) {
		return nil, 0, ErrNotFalsifying
	}
	walk := func(value *big.Int, falsifies func(*big.Int) bool, budget int) (*big.Int, int) {
		return intWalk(value, elemTarget, nil, nil, falsifies, budget)
	}
	got, steps := shrinkSequence(items, func(c []*big.Int) bool { return !holds(c) }, maxSteps, 0, walk)
	return got, steps, nil
}

// ShrinkBytes shrinks falsifying bytes: sequence shrinking over byte values
// toward 0x00.  prop is true when the bytes satisfy the property.
func ShrinkBytes(initial []byte, prop func([]byte) bool, maxSteps int) ([]byte, int, error) {
	if prop == nil {
		return nil, 0, argErr("nil argument")
	}
	if err := checkBudget(maxSteps); err != nil {
		return nil, 0, err
	}
	if prop(slices.Clone(initial)) {
		return nil, 0, ErrNotFalsifying
	}
	got, steps := shrinkSequence(slices.Clone(initial), func(c []byte) bool { return !prop(c) }, maxSteps, 0, byteWalk)
	return got, steps, nil
}
