# coding_adventures_markov_chain

A general-purpose **Markov Chain** library for Ruby — part of the
[coding-adventures](https://github.com/adhithyan15/coding-adventures)
computing stack.

Specification: `code/specs/DT28-markov-chain.md`

## What is a Markov Chain?

A Markov Chain models a system that moves between a finite set of **states**
over time, where the probability of the next state depends *only* on the
current state — not on the history of how the system got there.  This
"memorylessness" is called the **Markov property**.

The chain is fully described by a **transition matrix** T where `T[i][j]` is
the probability of going from state `sᵢ` to state `sⱼ`.  Each row sums to 1.

## Where It Fits in the Stack

```
DT00  Graph (undirected)
DT01  DirectedGraph           ← this package wraps this
DT28  MarkovChain             ← you are here
```

`CodingAdventures::MarkovChain` internally holds a
`CodingAdventures::DirectedGraph::Graph` (with `allow_self_loops: true`) for
topology management, plus a separate `@transitions` hash for probabilities.

## Installation

Add to your Gemfile:

```ruby
gem "coding_adventures_markov_chain"
```

## Quick Start

```ruby
require "coding_adventures_markov_chain"

# ── Order-1 character chain ──────────────────────────────────────────────
chain = CodingAdventures::MarkovChain.new
chain.train_string("the quick brown fox jumps over the lazy dog " * 10)
puts chain.generate_string("t", 50)
# => "the quich lazydog the over the ..."   (varies)

# ── Order-2 chain (bigram context) ───────────────────────────────────────
chain2 = CodingAdventures::MarkovChain.new(order: 2)
chain2.train_string("abcabcabc")
puts chain2.generate_string("ab", 9)
# => "abcabcabc"   (deterministic for this input)

# ── Training on arbitrary state sequences ────────────────────────────────
weather = CodingAdventures::MarkovChain.new(smoothing: 0.1)
weather.train(%w[Sunny Sunny Cloudy Rainy Sunny Cloudy Cloudy Rainy Sunny])
puts weather.probability(["Sunny"].freeze, "Cloudy")

# ── Stationary distribution ──────────────────────────────────────────────
dist = weather.stationary_distribution
puts dist.sort.map { |s, p| "#{s}: #{p.round(3)}" }.join(", ")
```

## API

### `MarkovChain.new(order: 1, smoothing: 0.0, states: nil)`

Create a new chain.

- `order` — context window size (default 1).
- `smoothing` — Laplace/Lidstone smoothing α (default 0.0).
- `states` — optional Array of all known states; pre-registers the alphabet
  so smoothing denominators are correct even before training.

### `chain.train(sequence)`

Train on an Array of states.  Sliding a window of size `order+1` generates
context → target pairs whose counts accumulate across multiple `train` calls.

### `chain.train_string(text)`

Convenience wrapper: treats each character of `text` as a state.

### `chain.next_state(current)`

Sample one transition from `current`.  Raises `KeyError` if `current` is not
a known context.  For order-k chains, `current` is a frozen Array of k states.

### `chain.generate(start, length)`

Return an Array of exactly `length` states starting with `start`.

### `chain.generate_string(seed, length)`

Return a String of exactly `length` characters.  For order-k chains, `seed`
must be at least `order` characters long.

### `chain.probability(from, to)`

Return `T[from][to]`, or `0.0` if the transition is unknown.  For order-k,
`from` is a frozen Array of k states.

### `chain.stationary_distribution`

Compute the stationary distribution via power iteration.  Returns a Hash
`{ state => Float }` whose values sum to ≈ 1.0.  Raises `RuntimeError` if the
chain is not ergodic after 10,000 iterations.

### `chain.states`

Sorted Array of all known states in the alphabet.

### `chain.transition_matrix`

A copy of the full probability table: `{ context_key => { target => Float } }`.

## Smoothing

| α value | Name     | Effect |
|---------|----------|--------|
| 0.0     | None     | Zero-probability transitions stay zero |
| 1.0     | Laplace  | Every unseen transition gets count 1 |
| 0 < α < 1 | Lidstone | Fractional pseudo-count per unseen pair |

Smoothing is applied at normalisation time.  With smoothing, `next_state`
will never get stuck in a dead-end state.

## Order-k Chains

Higher-order chains use a k-gram context key (frozen Array) instead of a
single state.  The context window advances one step at a time:

```
Order-2, sequence: [a, b, c, a, b, c]

Windows:
  [a, b] → c
  [b, c] → a
  [c, a] → b
  [a, b] → c   (second occurrence — count accumulates)
```

Higher orders produce more realistic generated output at the cost of needing
more training data to populate the exponentially larger state space.

## License

MIT
