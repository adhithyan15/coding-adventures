// Package markovchain implements a general-purpose Markov Chain data structure
// for training on sequences and sampling new sequences from the learned model.
//
// # What is a Markov Chain?
//
// A Markov Chain models a system that moves between a finite set of STATES over
// time, where the probability of the NEXT state depends ONLY on the CURRENT state
// — not on any history of how you arrived there. This "memorylessness" property
// is the celebrated Markov property.
//
// Imagine a simple weather model with three states: Sunny, Cloudy, Rainy.
//
//	Transition matrix T:
//	          → Sunny  → Cloudy  → Rainy
//	Sunny  [   0.7       0.2      0.1   ]
//	Cloudy [   0.3       0.4      0.3   ]
//	Rainy  [   0.2       0.3      0.5   ]
//
// Reading row "Cloudy": if today is Cloudy, tomorrow is Sunny with probability
// 0.3, Cloudy with probability 0.4, and Rainy with probability 0.3. Each row
// sums to 1.0 because the chain must transition somewhere.
//
// # Historical Context
//
// Andrei Andreyevich Markov (1856–1922) introduced the model in 1906 to study
// the distribution of vowels and consonants in Pushkin's "Eugene Onegin". Claude
// Shannon (1948) repurposed it in "A Mathematical Theory of Communication" to
// model English text as a statistical process — the foundation of modern
// information theory.
//
// # How This Package Fits the Stack
//
// This package is used by CMP06 (Brotli compression) as a conceptual model for
// context-adaptive probability tables. LZMA-style compressors maintain one
// probability table per "state" (a recent byte pattern), using transition
// probabilities to feed a range coder. This implementation is the general-purpose
// training and sampling layer; compression-specific probability update rules
// would sit on top.
//
// # Implementation Strategy
//
// Internally, a Graph (from the directed-graph package) stores the topology of
// known states and their transitions. A separate transitions map holds the
// floating-point probabilities. Raw training counts are kept until normalization,
// which happens at the end of every Train call.
//
// For order-k chains (k > 1), the "context key" is k consecutive states joined
// by a null byte separator ("\x00"), forming a single string. This lets us reuse
// the same Graph and map types without needing tuple keys.
package markovchain

import (
	"errors"
	"fmt"
	"math"
	"math/rand"
	"sort"
	"strings"

	directedgraph "github.com/adhithyan15/coding-adventures/code/packages/go/directed-graph"
)

// ─────────────────────────────────────────────────────────────────────────────
// Core data structure
// ─────────────────────────────────────────────────────────────────────────────

// MarkovChain is a Markov Chain of arbitrary order over string states.
//
// Fields are unexported; callers interact through the public API.
//
//	graph:       directed graph tracking which (context, target) pairs have
//	             ever been observed — used for topology queries.
//	transitions: the actual probability table:
//	               transitions[context][target] = P(target | context)
//	counts:      raw training counts accumulated before normalization:
//	               counts[context][target] = number of times target
//	               followed context in training data.
//	knownStates: the complete vocabulary of atomic states seen during
//	             training or pre-registered at construction time.
//	order:       the Markov order k. Order 1 = standard Markov chain
//	             (current state → next state). Order 2 means the context
//	             is the last TWO states, joined by "\x00".
//	smoothing:   Lidstone/Laplace smoothing parameter α. α=0 means no
//	             smoothing. α=1 is classic Laplace smoothing.
type MarkovChain struct {
	graph       *directedgraph.Graph
	transitions map[string]map[string]float64
	counts      map[string]map[string]float64
	knownStates map[string]bool
	order       int
	smoothing   float64
}

// New creates a new Markov Chain.
//
// Parameters:
//   - order:     the Markov order k (use 1 for a standard chain).
//   - smoothing: the Laplace/Lidstone smoothing parameter α.
//                0.0 = no smoothing; 1.0 = classic Laplace; any α ≥ 0 is valid.
//   - states:    optional pre-registration of the full state alphabet.
//                Pass nil to let the alphabet grow from training data alone.
//                Pre-registering states is important when smoothing > 0 and
//                you want unseen transitions to be distributed over ALL
//                possible states, not just those observed in training.
//
// # Why pre-register states?
//
// With Laplace smoothing (α=1) and states [A, B, C], the smoothed probability
// of any unseen transition is α / (count_sum + α * |all_states|). If C is
// pre-registered but never appears in training, P(A→C) = 1 / (1 + 3) = 0.25.
// Without pre-registration, C would not be in the denominator and P(A→C) = 0.
func New(order int, smoothing float64, states []string) *MarkovChain {
	// A Markov chain can loop to the same state (e.g., "A→A" is valid when
	// state A transitions to itself). We therefore allow self-loops in the
	// underlying graph.
	g := directedgraph.NewAllowSelfLoops()

	m := &MarkovChain{
		graph:       g,
		transitions: make(map[string]map[string]float64),
		counts:      make(map[string]map[string]float64),
		knownStates: make(map[string]bool),
		order:       order,
		smoothing:   smoothing,
	}

	// Pre-register any states provided by the caller. These will be included
	// in the smoothing denominator even if they never appear in training data.
	for _, s := range states {
		m.knownStates[s] = true
		g.AddNode(s)
	}

	return m
}

// ─────────────────────────────────────────────────────────────────────────────
// Training
// ─────────────────────────────────────────────────────────────────────────────

// Train learns transition probabilities from a sequence of states.
//
// The training algorithm slides a window of size (order+1) over the sequence:
//
//	for i in 0 .. len(sequence)-order-1:
//	  context = sequence[i .. i+order-1]  joined by "\x00"
//	  target  = sequence[i+order]
//	  counts[context][target]++
//
// After processing all windows, every context row is normalized to probabilities
// using Lidstone smoothing:
//
//	total = Σ counts[context] + α * |all_known_states|
//	P(context → target) = (counts[context][target] + α) / total
//
// Train may be called multiple times. Counts accumulate across calls, so the
// model improves with more training data.
//
// Returns an error if the sequence is too short to produce any transitions
// (len(sequence) < order+1). A single-element sequence is not an error — it
// simply registers the state in the vocabulary without creating transitions.
func (m *MarkovChain) Train(sequence []string) error {
	if len(sequence) == 0 {
		return nil
	}

	// Register every atomic state in the vocabulary. For order-k chains, we
	// store atomic states (individual tokens) in knownStates and add k-gram
	// context keys to the graph as separate nodes.
	for _, s := range sequence {
		m.knownStates[s] = true
		m.graph.AddNode(s)
	}

	if len(sequence) <= m.order {
		// Sequence too short to form any context → target pair. States are
		// registered but no transitions are learned.
		return nil
	}

	// Slide the window and accumulate raw counts.
	for i := 0; i <= len(sequence)-m.order-1; i++ {
		// context is the k-gram starting at position i.
		context := strings.Join(sequence[i:i+m.order], "\x00")
		target := sequence[i+m.order]

		// Ensure the context and target nodes exist in the graph.
		m.graph.AddNode(context)
		m.graph.AddNode(target)

		// Record an edge from context to target in the topology graph.
		m.graph.AddEdge(context, target)

		// Accumulate raw count.
		if m.counts[context] == nil {
			m.counts[context] = make(map[string]float64)
		}
		m.counts[context][target]++
	}

	// Normalize all count rows into probability distributions.
	m.normalize()
	return nil
}

// TrainString is a convenience wrapper that treats each character of text as a
// state. It splits the string into individual single-character tokens and
// calls Train.
//
// Example:
//
//	chain.TrainString("abcabcabc")
//	// equivalent to: chain.Train([]string{"a","b","c","a","b","c","a","b","c"})
func (m *MarkovChain) TrainString(text string) error {
	chars := make([]string, len([]rune(text)))
	for i, r := range []rune(text) {
		chars[i] = string(r)
	}
	return m.Train(chars)
}

// normalize converts all accumulated counts into probabilities, applying
// Lidstone smoothing.
//
// # The smoothing formula
//
// Without smoothing, the probability estimate for context → target is simply:
//
//	P(target | context) = count(context → target) / Σ_j count(context → j)
//
// The problem: if count(context → target) == 0, the chain can get permanently
// stuck (probability zero means "never"), which is undesirable for generation.
//
// Lidstone (1920) proposed adding a small constant α to every count:
//
//	P(target | context) = (count(context → target) + α)
//	                    / (Σ_j count(context → j) + α * |vocabulary|)
//
// Here |vocabulary| is the number of ALL known ATOMIC states (not k-grams).
// This ensures that even completely unseen transitions receive a small
// non-zero probability proportional to α.
//
// With α=0 (no smoothing), the formula reduces to the plain frequency estimate.
// With α=1 (Laplace), it is as if we observed each transition once before
// training began — a "prior" of one pseudo-count per symbol.
func (m *MarkovChain) normalize() {
	vocabSize := len(m.knownStates)

	for context, targetCounts := range m.counts {
		// Sum of raw observed counts for this context.
		countSum := 0.0
		for _, c := range targetCounts {
			countSum += c
		}

		// Denominator: raw counts + smoothing pseudo-counts over the full vocab.
		//
		//   total = Σ counts + α * |S|
		//
		// where |S| is the number of atomic states (knownStates), not k-grams.
		total := countSum + m.smoothing*float64(vocabSize)

		if total == 0 {
			continue
		}

		if m.transitions[context] == nil {
			m.transitions[context] = make(map[string]float64)
		}

		// Assign a probability to every known atomic state, even if unseen.
		// This keeps the transition matrix "full" (no zero-probability dead ends)
		// when smoothing > 0.
		for state := range m.knownStates {
			raw := targetCounts[state] // 0.0 if never seen
			m.transitions[context][state] = (raw + m.smoothing) / total
		}
	}
}

// ─────────────────────────────────────────────────────────────────────────────
// Sampling
// ─────────────────────────────────────────────────────────────────────────────

// NextState samples a single transition from the given context.
//
// For an order-1 chain, current is a single state name (e.g., "Sunny").
// For an order-k chain, current is a k-gram key with "\x00" separators
// (e.g., "a\x00b" for the bigram "ab").
//
// The sampling algorithm draws a uniform random number r ∈ [0,1) and walks
// the cumulative probability distribution:
//
//	row = transitions[current]
//	r   = rand.Float64()
//	sum = 0
//	for (state, prob) in sorted(row):
//	  sum += prob
//	  if r < sum: return state
//
// Returns an error if current is not a known context (i.e., was never seen
// during training, or is not a pre-registered state that appears as a source).
func (m *MarkovChain) NextState(current string) (string, error) {
	row, ok := m.transitions[current]
	if !ok || len(row) == 0 {
		return "", fmt.Errorf("markov chain: unknown state or context %q", current)
	}

	return sampleFromRow(row), nil
}

// sampleFromRow draws one state from a probability distribution stored as a
// map[string]float64.
//
// To ensure deterministic test behavior when all probabilities are equal, states
// are iterated in sorted order before the cumulative walk. In production use,
// the randomness comes entirely from rand.Float64().
func sampleFromRow(row map[string]float64) string {
	// Collect and sort keys for deterministic ordering.
	keys := make([]string, 0, len(row))
	for k := range row {
		keys = append(keys, k)
	}
	sort.Strings(keys)

	r := rand.Float64()
	cumulative := 0.0
	for _, k := range keys {
		cumulative += row[k]
		if r < cumulative {
			return k
		}
	}
	// Floating-point rounding may leave cumulative just below 1.0.
	// Return the last key as a safe fallback.
	return keys[len(keys)-1]
}

// Generate produces a sequence of `length` states starting from `start`.
//
// For order-1 chains, start is an atomic state name ("A", "Sunny", etc.).
// For order-k chains, start is the initial k-gram context key (k tokens joined
// by "\x00"). The returned slice contains only the atomic states produced —
// for order-1 this is length items beginning with the start state itself;
// for order-k the initial context is "unrolled" into its constituent states,
// then subsequent states are appended.
//
// # How the sliding window works for order-k
//
//	start = "a\x00b"   (order=2, first context is the bigram "ab")
//	step 1: context="a\x00b" → sample target="c" → window becomes "b\x00c"
//	step 2: context="b\x00c" → sample target="a" → window becomes "c\x00a"
//	...
//
// The output sequence for order-2 starting at "a\x00b" requesting length=5:
//
//	["a", "b", "c", "a", "b"]
//	  ↑    ↑    ↑──────────── appended by sampling
//	  └─ initial context, unrolled
//
// Returns an error if start is unknown or sampling encounters an unknown state.
func (m *MarkovChain) Generate(start string, length int) ([]string, error) {
	if length <= 0 {
		return []string{}, nil
	}

	// For order-1, the context is just the start state. For order-k, start is
	// already the k-gram key; split it into constituent tokens.
	var result []string
	var contextTokens []string

	if m.order == 1 {
		// Validate that the start state is known.
		if _, ok := m.transitions[start]; !ok {
			return nil, fmt.Errorf("markov chain: unknown start state %q", start)
		}
		result = append(result, start)
		contextTokens = []string{start}
	} else {
		// Split the k-gram key back into individual tokens.
		parts := strings.Split(start, "\x00")
		if len(parts) != m.order {
			return nil, fmt.Errorf("markov chain: start key %q has %d parts, want %d (order=%d)",
				start, len(parts), m.order, m.order)
		}
		result = append(result, parts...)
		contextTokens = parts

		// Validate the start context exists.
		if _, ok := m.transitions[start]; !ok {
			return nil, fmt.Errorf("markov chain: unknown start context %q", start)
		}
	}

	// Sample until we have `length` atomic states in the result.
	for len(result) < length {
		contextKey := strings.Join(contextTokens, "\x00")
		next, err := m.NextState(contextKey)
		if err != nil {
			return nil, err
		}
		result = append(result, next)

		// Advance the sliding window: drop the oldest token, append the newest.
		contextTokens = append(contextTokens[1:], next)
	}

	return result[:length], nil
}

// GenerateString produces a string of `length` characters using character-level
// generation. This is a convenience wrapper for chains trained with TrainString.
//
// The seed must be at least `order` characters long (one character per order
// level). The last `order` characters of seed form the initial context window.
//
// Example for order=2:
//
//	chain.TrainString("abcabcabc")
//	s, _ := chain.GenerateString("ab", 9)
//	// s == "abcabcabc"
//
// The output string is exactly `length` characters and starts with the seed's
// last `order` characters (the context window).
func (m *MarkovChain) GenerateString(seed string, length int) (string, error) {
	runes := []rune(seed)
	if len(runes) < m.order {
		return "", fmt.Errorf("markov chain: seed %q has %d chars, need at least %d (order=%d)",
			seed, len(runes), m.order, m.order)
	}

	// Take the last `order` characters of the seed as the initial context window.
	windowRunes := runes[len(runes)-m.order:]
	window := make([]string, m.order)
	for i, r := range windowRunes {
		window[i] = string(r)
	}

	// Build the start key by joining the window tokens with "\x00".
	startKey := strings.Join(window, "\x00")
	if m.order == 1 {
		startKey = window[0]
	}

	// Generate `length` states (including the initial context window tokens).
	seq, err := m.Generate(startKey, length)
	if err != nil {
		return "", err
	}

	return strings.Join(seq, ""), nil
}

// ─────────────────────────────────────────────────────────────────────────────
// Probability queries
// ─────────────────────────────────────────────────────────────────────────────

// Probability returns the probability of transitioning from context `from` to
// state `to`.
//
// For an order-1 chain, from and to are atomic state names.
// For an order-k chain, from is a k-gram key (tokens joined by "\x00") and
// to is an atomic state name.
//
// Returns 0.0 if `from` was never trained (unknown context) or if `to` was
// never seen as a target from `from` and smoothing is 0.
func (m *MarkovChain) Probability(from, to string) float64 {
	row, ok := m.transitions[from]
	if !ok {
		return 0.0
	}
	return row[to]
}

// ─────────────────────────────────────────────────────────────────────────────
// Stationary distribution
// ─────────────────────────────────────────────────────────────────────────────

// StationaryDistribution computes the long-run fraction of time spent in each
// state, using power iteration.
//
// # Mathematical background
//
// For an ergodic Markov chain (all states reachable from all others, no
// periodic traps), there exists a unique distribution π such that:
//
//	π · T = π      (π is unchanged after one step of the chain)
//
// This is the LEFT eigenvector of the transition matrix T corresponding to
// eigenvalue 1. It tells us: "In the long run, what fraction of time does
// the chain spend in each state?"
//
// For the weather example, π ≈ [Sunny: 0.47, Cloudy: 0.28, Rainy: 0.25].
//
// # Power iteration algorithm
//
// Power iteration multiplies an arbitrary initial distribution by T repeatedly
// until convergence:
//
//	π⁰ = uniform distribution over all states
//	π^(n+1)[j] = Σ_i π^n[i] * T[i][j]
//	repeat until max|π^(n+1)[j] - π^n[j]| < 1e-10
//
// This converges geometrically if the chain is ergodic (the second-largest
// eigenvalue is < 1 in absolute value).
//
// Returns an error if:
//   - the chain is untrained (no states)
//   - power iteration does not converge in 10,000 iterations (non-ergodic chain)
func (m *MarkovChain) StationaryDistribution() (map[string]float64, error) {
	// Collect the set of atomic states that appear as rows in the transition
	// table. For order-1 chains these are the same as knownStates. For order-k
	// they are the k-gram context keys; we operate over k-gram rows because
	// that is what the transition table is indexed by.
	states := m.contextKeys()
	n := len(states)
	if n == 0 {
		return nil, errors.New("markov chain: cannot compute stationary distribution — no states trained")
	}

	// Initialize π as a uniform distribution.
	pi := make(map[string]float64, n)
	for _, s := range states {
		pi[s] = 1.0 / float64(n)
	}

	const maxIter = 10000
	const threshold = 1e-10

	for iter := 0; iter < maxIter; iter++ {
		piNew := make(map[string]float64, n)

		// One power iteration step: π_new[j] = Σ_i π[i] * T[i][j]
		for _, s := range states {
			row := m.transitions[s]
			for target, prob := range row {
				piNew[target] += pi[s] * prob
			}
		}

		// Re-normalize to guard against floating-point drift.
		total := 0.0
		for _, v := range piNew {
			total += v
		}
		if total > 0 {
			for k := range piNew {
				piNew[k] /= total
			}
		}

		// Check convergence: has the maximum change across all states dropped
		// below the threshold?
		maxDelta := 0.0
		for _, s := range states {
			d := math.Abs(piNew[s] - pi[s])
			if d > maxDelta {
				maxDelta = d
			}
		}

		pi = piNew

		if maxDelta < threshold {
			return pi, nil
		}
	}

	return nil, fmt.Errorf("markov chain: stationary distribution did not converge in %d iterations (chain may not be ergodic)", maxIter)
}

// contextKeys returns all context keys (rows in the transition table) in sorted
// order. For order-1 chains this is the set of individual state names. For
// order-k chains these are the k-gram context strings.
func (m *MarkovChain) contextKeys() []string {
	keys := make([]string, 0, len(m.transitions))
	for k := range m.transitions {
		keys = append(keys, k)
	}
	sort.Strings(keys)
	return keys
}

// ─────────────────────────────────────────────────────────────────────────────
// Inspection
// ─────────────────────────────────────────────────────────────────────────────

// States returns all known atomic states in sorted order.
//
// This includes both states pre-registered at construction time and states
// encountered during training.
func (m *MarkovChain) States() []string {
	result := make([]string, 0, len(m.knownStates))
	for s := range m.knownStates {
		result = append(result, s)
	}
	sort.Strings(result)
	return result
}

// TransitionMatrix returns a copy of the complete probability table.
//
// Keys in the outer map are context strings (for order-1: state names; for
// order-k: k-gram keys with "\x00" separators). Keys in the inner map are
// atomic target state names. Values are probabilities in [0, 1].
//
// The returned map is a shallow copy of the outer level and deep copies of
// the inner level — callers may freely read it without modifying the chain.
func (m *MarkovChain) TransitionMatrix() map[string]map[string]float64 {
	result := make(map[string]map[string]float64, len(m.transitions))
	for ctx, row := range m.transitions {
		rowCopy := make(map[string]float64, len(row))
		for k, v := range row {
			rowCopy[k] = v
		}
		result[ctx] = rowCopy
	}
	return result
}
