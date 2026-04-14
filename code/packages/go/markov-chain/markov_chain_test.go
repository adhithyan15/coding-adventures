// Tests for the markovchain package.
//
// Each test case corresponds directly to the DT28 specification's list of
// required test cases. The test numbers in comments match the spec numbering
// so they are easy to cross-reference.
package markovchain

import (
	"math"
	"strings"
	"testing"
)

// ─────────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────────

// approxEqual returns true if a and b are within epsilon of each other.
// Floating-point comparisons in probability calculations require a tolerance
// because IEEE 754 arithmetic is not exact.
func approxEqual(a, b, epsilon float64) bool {
	return math.Abs(a-b) < epsilon
}

const eps = 1e-9 // tolerance for probability comparisons

// ─────────────────────────────────────────────────────────────────────────────
// Test 1: Construction — empty chain has 0 states
// ─────────────────────────────────────────────────────────────────────────────

// TestNew verifies that New() creates a chain with zero states and an empty
// transition table. This is the baseline invariant: a freshly constructed
// chain knows nothing.
func TestNew(t *testing.T) {
	m := New(1, 0.0, nil)
	if len(m.States()) != 0 {
		t.Errorf("expected 0 states, got %d", len(m.States()))
	}
	if len(m.TransitionMatrix()) != 0 {
		t.Errorf("expected empty transition matrix, got %d entries", len(m.TransitionMatrix()))
	}
}

// TestNewWithPreregisteredStates verifies that states passed to New() appear
// in the alphabet before any training occurs.
func TestNewWithPreregisteredStates(t *testing.T) {
	m := New(1, 0.0, []string{"A", "B", "C"})
	states := m.States()
	if len(states) != 3 {
		t.Errorf("expected 3 pre-registered states, got %d", len(states))
	}
}

// ─────────────────────────────────────────────────────────────────────────────
// Test 2: Train single pair — P(A→B) == 1.0
// ─────────────────────────────────────────────────────────────────────────────

// TestTrainSinglePair verifies that training on [A, B] (two states, one
// transition) gives P(A→B) = 1.0. With no smoothing there is no mass to
// distribute elsewhere.
func TestTrainSinglePair(t *testing.T) {
	m := New(1, 0.0, nil)
	if err := m.Train([]string{"A", "B"}); err != nil {
		t.Fatalf("Train failed: %v", err)
	}

	p := m.Probability("A", "B")
	if !approxEqual(p, 1.0, eps) {
		t.Errorf("P(A→B) = %f, want 1.0", p)
	}
}

// ─────────────────────────────────────────────────────────────────────────────
// Test 3: Train sequence — frequency-based probabilities
// ─────────────────────────────────────────────────────────────────────────────

// TestTrainSequence verifies that training on [A, B, A, C, A, B, B, A] (a
// classic example from the spec) produces the correct transition frequencies.
//
// Observed transitions:
//
//	A→B: 2  A→C: 1  B→A: 2  B→B: 1  C→A: 1
//
// Expected probabilities (no smoothing):
//
//	P(A→B) = 2/3 ≈ 0.6667   P(A→C) = 1/3 ≈ 0.3333
//	P(B→A) = 2/3 ≈ 0.6667   P(B→B) = 1/3 ≈ 0.3333
//	P(C→A) = 1.0
func TestTrainSequence(t *testing.T) {
	m := New(1, 0.0, nil)
	seq := []string{"A", "B", "A", "C", "A", "B", "B", "A"}
	if err := m.Train(seq); err != nil {
		t.Fatalf("Train failed: %v", err)
	}

	cases := []struct {
		from, to string
		want     float64
	}{
		{"A", "B", 2.0 / 3.0},
		{"A", "C", 1.0 / 3.0},
		{"B", "A", 2.0 / 3.0},
		{"B", "B", 1.0 / 3.0},
		{"C", "A", 1.0},
	}

	for _, tc := range cases {
		got := m.Probability(tc.from, tc.to)
		if !approxEqual(got, tc.want, 1e-6) {
			t.Errorf("P(%s→%s) = %f, want %f", tc.from, tc.to, got, tc.want)
		}
	}
}

// ─────────────────────────────────────────────────────────────────────────────
// Test 4: Laplace smoothing — P(A→C) == 1/4
// ─────────────────────────────────────────────────────────────────────────────

// TestLaplaceSmoothing verifies the smoothing formula with three pre-registered
// states [A, B, C] and training data [A, B] (smoothing=1.0, Laplace).
//
// After training:
//
//	counts[A][B] = 1  (only observed transition)
//	total = 1 + 1.0 * 3 = 4          (1 raw count + α * |states|)
//
//	P(A→A) = (0 + 1) / 4 = 0.25
//	P(A→B) = (1 + 1) / 4 = 0.50
//	P(A→C) = (0 + 1) / 4 = 0.25
func TestLaplaceSmoothing(t *testing.T) {
	m := New(1, 1.0, []string{"A", "B", "C"})
	if err := m.Train([]string{"A", "B"}); err != nil {
		t.Fatalf("Train failed: %v", err)
	}

	got := m.Probability("A", "C")
	want := 0.25 // 1 / (1 + 3)
	if !approxEqual(got, want, eps) {
		t.Errorf("P(A→C) with Laplace smoothing = %f, want %f", got, want)
	}

	// Verify P(A→B) as well: (1+1)/4 = 0.5
	gotAB := m.Probability("A", "B")
	if !approxEqual(gotAB, 0.5, eps) {
		t.Errorf("P(A→B) with Laplace smoothing = %f, want 0.5", gotAB)
	}

	// Verify that the row sums to 1.0.
	sum := m.Probability("A", "A") + m.Probability("A", "B") + m.Probability("A", "C")
	if !approxEqual(sum, 1.0, 1e-9) {
		t.Errorf("row A does not sum to 1.0 (got %f)", sum)
	}
}

// ─────────────────────────────────────────────────────────────────────────────
// Test 5: Generate length — output has exactly the requested number of states
// ─────────────────────────────────────────────────────────────────────────────

// TestGenerateLength verifies that Generate(start, 10) returns exactly 10
// states. The content is random but the length must be exact.
func TestGenerateLength(t *testing.T) {
	m := New(1, 1.0, nil) // smoothing ensures no dead ends
	if err := m.Train([]string{"A", "B", "A", "C", "A", "B"}); err != nil {
		t.Fatalf("Train failed: %v", err)
	}

	seq, err := m.Generate("A", 10)
	if err != nil {
		t.Fatalf("Generate failed: %v", err)
	}
	if len(seq) != 10 {
		t.Errorf("Generate(A, 10) returned %d states, want 10", len(seq))
	}
}

// TestGenerateLengthZero verifies that Generate with length=0 returns an empty
// slice without error.
func TestGenerateLengthZero(t *testing.T) {
	m := New(1, 0.0, nil)
	if err := m.Train([]string{"A", "B"}); err != nil {
		t.Fatalf("Train failed: %v", err)
	}
	seq, err := m.Generate("A", 0)
	if err != nil {
		t.Fatalf("Generate(A, 0) returned error: %v", err)
	}
	if len(seq) != 0 {
		t.Errorf("Generate(A, 0) should return empty slice, got len=%d", len(seq))
	}
}

// ─────────────────────────────────────────────────────────────────────────────
// Test 6: GenerateString — returns correct length string
// ─────────────────────────────────────────────────────────────────────────────

// TestGenerateStringLength verifies that GenerateString("th", 50) returns
// a string of exactly 50 characters. The chain is trained on a small English
// text snippet so "th" is a valid context.
func TestGenerateStringLength(t *testing.T) {
	m := New(1, 1.0, nil)
	sample := "the quick brown fox jumps over the lazy dog "
	if err := m.TrainString(strings.Repeat(sample, 5)); err != nil {
		t.Fatalf("TrainString failed: %v", err)
	}

	result, err := m.GenerateString("t", 50)
	if err != nil {
		t.Fatalf("GenerateString failed: %v", err)
	}
	if len([]rune(result)) != 50 {
		t.Errorf("GenerateString returned %d chars, want 50", len([]rune(result)))
	}
}

// ─────────────────────────────────────────────────────────────────────────────
// Test 7: Stationary distribution sums to 1.0
// ─────────────────────────────────────────────────────────────────────────────

// TestStationaryDistributionSumsToOne trains a simple ergodic chain and
// verifies that the stationary distribution sums to 1.0 within floating-point
// tolerance.
//
// The weather model used here is ergodic (every state is reachable from every
// other state via some path), so convergence is guaranteed.
func TestStationaryDistributionSumsToOne(t *testing.T) {
	// Build a simple weather chain: Sunny → Cloudy → Rainy → Sunny (a cycle
	// augmented with return edges to make it ergodic).
	m := New(1, 0.0, nil)
	seq := []string{
		"Sunny", "Cloudy", "Rainy", "Sunny", "Rainy", "Cloudy",
		"Sunny", "Cloudy", "Sunny", "Rainy", "Cloudy", "Rainy",
	}
	if err := m.Train(seq); err != nil {
		t.Fatalf("Train failed: %v", err)
	}

	dist, err := m.StationaryDistribution()
	if err != nil {
		t.Fatalf("StationaryDistribution failed: %v", err)
	}

	sum := 0.0
	for _, v := range dist {
		sum += v
	}
	if !approxEqual(sum, 1.0, 1e-6) {
		t.Errorf("stationary distribution sums to %f, want 1.0", sum)
	}
}

// TestStationaryDistributionAllPositive verifies all entries are non-negative.
func TestStationaryDistributionAllPositive(t *testing.T) {
	m := New(1, 1.0, []string{"A", "B", "C"})
	if err := m.Train([]string{"A", "B", "C", "A", "B", "C"}); err != nil {
		t.Fatalf("Train failed: %v", err)
	}

	dist, err := m.StationaryDistribution()
	if err != nil {
		t.Fatalf("StationaryDistribution failed: %v", err)
	}

	for s, p := range dist {
		if p < 0 {
			t.Errorf("stationary[%s] = %f (negative)", s, p)
		}
	}
}

// ─────────────────────────────────────────────────────────────────────────────
// Test 8: Order-2 chain — generates deterministic "abcabcabc"
// ─────────────────────────────────────────────────────────────────────────────

// TestOrderTwo verifies that an order-2 chain trained on "abcabcabc" learns
// the exact pattern and reproduces it faithfully.
//
// Training data (as characters): a b c a b c a b c
// Observed bigram transitions (no smoothing):
//
//	"a\x00b" → c  (3 times, P = 1.0)
//	"b\x00c" → a  (2 times, P = 1.0)
//	"c\x00a" → b  (2 times, P = 1.0)
//
// Starting context "ab" should always produce "abcabcabc".
func TestOrderTwo(t *testing.T) {
	m := New(2, 0.0, nil)
	if err := m.TrainString("abcabcabc"); err != nil {
		t.Fatalf("TrainString failed: %v", err)
	}

	// Verify the key transition probability: context "a\x00b" → "c" should be 1.0.
	ctxAB := "a\x00b"
	p := m.Probability(ctxAB, "c")
	if !approxEqual(p, 1.0, eps) {
		t.Errorf("P(ab→c) = %f, want 1.0", p)
	}

	// Generate a 9-character string starting from seed "ab".
	result, err := m.GenerateString("ab", 9)
	if err != nil {
		t.Fatalf("GenerateString failed: %v", err)
	}
	want := "abcabcabc"
	if result != want {
		t.Errorf("GenerateString(\"ab\", 9) = %q, want %q", result, want)
	}
}

// ─────────────────────────────────────────────────────────────────────────────
// Test 9: Unknown state returns error
// ─────────────────────────────────────────────────────────────────────────────

// TestNextStateUnknown verifies that calling NextState on a state that was
// never trained returns an error rather than panicking or returning empty.
//
// This is important for safety: callers should know when they have provided an
// invalid state rather than silently receiving garbage output.
func TestNextStateUnknown(t *testing.T) {
	m := New(1, 0.0, nil)
	if err := m.Train([]string{"A", "B"}); err != nil {
		t.Fatalf("Train failed: %v", err)
	}

	_, err := m.NextState("UNKNOWN")
	if err == nil {
		t.Error("NextState(\"UNKNOWN\") should return error, got nil")
	}
}

// TestGenerateUnknownStart verifies that Generate also returns an error for
// unknown start states.
func TestGenerateUnknownStart(t *testing.T) {
	m := New(1, 0.0, nil)
	if err := m.Train([]string{"A", "B"}); err != nil {
		t.Fatalf("Train failed: %v", err)
	}

	_, err := m.Generate("UNKNOWN", 5)
	if err == nil {
		t.Error("Generate(\"UNKNOWN\", 5) should return error, got nil")
	}
}

// ─────────────────────────────────────────────────────────────────────────────
// Test 10: Multi-train accumulation
// ─────────────────────────────────────────────────────────────────────────────

// TestMultiTrainAccumulation verifies that calling Train twice accumulates
// counts before re-normalizing.
//
// First training:  [A, B]                → counts added: A→B: 1
// Second training: [A, B, A, B, A, C]   → counts added: A→B: 2, A→C: 1
//
// Transition windows in the second sequence:
//
//	i=0: A→B  (A at index 0, B at index 1)
//	i=2: A→B  (A at index 2, B at index 3)
//	i=4: A→C  (A at index 4, C at index 5)
//
// Combined totals: A→B: 1+2=3, A→C: 0+1=1  (total = 4)
//
// Expected:
//
//	P(A→B) = 3/4 = 0.75
//	P(A→C) = 1/4 = 0.25
func TestMultiTrainAccumulation(t *testing.T) {
	m := New(1, 0.0, nil)

	if err := m.Train([]string{"A", "B"}); err != nil {
		t.Fatalf("first Train failed: %v", err)
	}
	if err := m.Train([]string{"A", "B", "A", "B", "A", "C"}); err != nil {
		t.Fatalf("second Train failed: %v", err)
	}

	// Combined: A→B appears 1+2=3 times; A→C appears 0+1=1 time.
	gotAB := m.Probability("A", "B")
	gotAC := m.Probability("A", "C")

	if !approxEqual(gotAB, 0.75, 1e-9) {
		t.Errorf("after two trains, P(A→B) = %f, want 0.75", gotAB)
	}
	if !approxEqual(gotAC, 0.25, 1e-9) {
		t.Errorf("after two trains, P(A→C) = %f, want 0.25", gotAC)
	}

	// Row must still sum to 1.0.
	sum := gotAB + gotAC
	if !approxEqual(sum, 1.0, 1e-9) {
		t.Errorf("row A sums to %f after multi-train, want 1.0", sum)
	}
}

// ─────────────────────────────────────────────────────────────────────────────
// Additional coverage tests
// ─────────────────────────────────────────────────────────────────────────────

// TestProbabilityUnknownContext verifies that Probability returns 0.0 for an
// unknown context rather than panicking.
func TestProbabilityUnknownContext(t *testing.T) {
	m := New(1, 0.0, nil)
	if err := m.Train([]string{"A", "B"}); err != nil {
		t.Fatalf("Train failed: %v", err)
	}

	p := m.Probability("UNKNOWN", "A")
	if p != 0.0 {
		t.Errorf("Probability(UNKNOWN, A) = %f, want 0.0", p)
	}
}

// TestTransitionMatrixCopy verifies that TransitionMatrix returns a copy —
// modifying the returned map does not affect the chain's internal state.
func TestTransitionMatrixCopy(t *testing.T) {
	m := New(1, 0.0, nil)
	if err := m.Train([]string{"A", "B"}); err != nil {
		t.Fatalf("Train failed: %v", err)
	}

	matrix := m.TransitionMatrix()
	matrix["A"]["B"] = 999.0 // mutate the copy

	// The chain's own probability should be unaffected.
	p := m.Probability("A", "B")
	if p == 999.0 {
		t.Error("TransitionMatrix returned a reference, not a copy")
	}
}

// TestStatesAfterTraining verifies that States() returns all atomic states seen
// during training in sorted order.
func TestStatesAfterTraining(t *testing.T) {
	m := New(1, 0.0, nil)
	if err := m.Train([]string{"C", "A", "B"}); err != nil {
		t.Fatalf("Train failed: %v", err)
	}

	states := m.States()
	want := []string{"A", "B", "C"} // sorted
	if len(states) != len(want) {
		t.Fatalf("States() len = %d, want %d", len(states), len(want))
	}
	for i, s := range states {
		if s != want[i] {
			t.Errorf("States()[%d] = %q, want %q", i, s, want[i])
		}
	}
}

// TestTrainEmptySequence verifies that training on an empty slice is a no-op.
func TestTrainEmptySequence(t *testing.T) {
	m := New(1, 0.0, nil)
	if err := m.Train([]string{}); err != nil {
		t.Fatalf("Train(empty) returned unexpected error: %v", err)
	}
	if len(m.States()) != 0 {
		t.Errorf("expected 0 states after empty training, got %d", len(m.States()))
	}
}

// TestTrainShortSequence verifies that a sequence of exactly `order` elements
// (too short to form any context→target pair) adds states but no transitions.
func TestTrainShortSequence(t *testing.T) {
	m := New(1, 0.0, nil)
	if err := m.Train([]string{"A"}); err != nil {
		t.Fatalf("Train([A]) returned error: %v", err)
	}
	// "A" is in the vocabulary but has no outgoing transitions.
	if len(m.TransitionMatrix()) != 0 {
		t.Errorf("expected empty transition table for single-element sequence")
	}
	if len(m.States()) != 1 {
		t.Errorf("expected 1 state, got %d", len(m.States()))
	}
}

// TestTrainStringRune verifies that TrainString handles multi-byte Unicode
// correctly by using runes, not bytes.
func TestTrainStringRune(t *testing.T) {
	m := New(1, 0.0, nil)
	// "αβ" is a 2-rune (4-byte) string.
	if err := m.TrainString("αβαβαβ"); err != nil {
		t.Fatalf("TrainString failed: %v", err)
	}
	// P(α → β) should be 1.0 (the only observed transition from α).
	p := m.Probability("α", "β")
	if !approxEqual(p, 1.0, eps) {
		t.Errorf("P(α→β) = %f, want 1.0", p)
	}
}

// TestGenerateStringTooShortSeed verifies that GenerateString returns an error
// when the seed is shorter than `order` characters.
func TestGenerateStringTooShortSeed(t *testing.T) {
	m := New(2, 0.0, nil)
	if err := m.TrainString("abcabc"); err != nil {
		t.Fatalf("TrainString failed: %v", err)
	}
	_, err := m.GenerateString("a", 5) // seed length 1, but order=2 requires 2
	if err == nil {
		t.Error("GenerateString with short seed should return error")
	}
}

// TestStationaryDistributionUntrainedReturnsError verifies the error path when
// no training has been done.
func TestStationaryDistributionUntrainedReturnsError(t *testing.T) {
	m := New(1, 0.0, nil)
	_, err := m.StationaryDistribution()
	if err == nil {
		t.Error("StationaryDistribution on untrained chain should return error")
	}
}

// TestLidstoneSmoothing verifies the smoothing formula with α = 0.5.
// With states [A, B] and training [A, B]:
//
//	counts[A][B] = 1
//	total = 1 + 0.5 * 2 = 2.0
//	P(A→B) = (1 + 0.5) / 2.0 = 0.75
//	P(A→A) = (0 + 0.5) / 2.0 = 0.25
func TestLidstoneSmoothing(t *testing.T) {
	m := New(1, 0.5, []string{"A", "B"})
	if err := m.Train([]string{"A", "B"}); err != nil {
		t.Fatalf("Train failed: %v", err)
	}

	wantAB := 1.5 / 2.0 // 0.75
	wantAA := 0.5 / 2.0 // 0.25

	gotAB := m.Probability("A", "B")
	gotAA := m.Probability("A", "A")

	if !approxEqual(gotAB, wantAB, eps) {
		t.Errorf("P(A→B) = %f, want %f", gotAB, wantAB)
	}
	if !approxEqual(gotAA, wantAA, eps) {
		t.Errorf("P(A→A) = %f, want %f", gotAA, wantAA)
	}
}

// TestOrderTwoProbabilityContext verifies order-2 context probability lookup
// using the raw "\x00"-separated key.
func TestOrderTwoProbabilityContext(t *testing.T) {
	m := New(2, 0.0, nil)
	if err := m.Train([]string{"A", "B", "C", "A", "B", "C"}); err != nil {
		t.Fatalf("Train failed: %v", err)
	}

	// Context "A\x00B" should transition to "C" with probability 1.0.
	p := m.Probability("A\x00B", "C")
	if !approxEqual(p, 1.0, eps) {
		t.Errorf("P(A,B → C) = %f, want 1.0", p)
	}
}
