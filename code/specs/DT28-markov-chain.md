# DT28 — Markov Chain

## Overview

A Markov Chain is a mathematical model of a system that moves between a finite set of
**states** over time, where the probability of transitioning to the next state depends
**only on the current state** — not on any history of how the system got there. This
"memorylessness" property is called the **Markov property**.

```
States:   S = {s₀, s₁, …, sₙ₋₁}
Transitions: P(next = sⱼ | current = sᵢ) = T[i][j]
```

The entire chain is captured in a single **transition matrix** T where:
- `T[i][j]` is the probability of going from state `sᵢ` to state `sⱼ`.
- Each row of T sums to 1.0 (the chain must go *somewhere*).

Markov Chains appear across a surprising breadth of domains:

```
Text generation:      States = characters or words. Train on a corpus; generate
                      new text by sampling transitions.

Compression (LZMA):   States = recent byte values; transition probabilities drive
                      a range coder (the "Markov chain" in LZMA's name).

PageRank:             States = web pages. Random surfer model: "from this page,
                      follow a random link." Stationary distribution = page rank.

Game AI:              States = game positions or NPC moods. Transitions = actions
                      with given likelihoods.

Biology:              States = nucleotides (A, C, G, T). CpG island detection,
                      gene prediction.

Monte Carlo methods:  MCMC samplers (Metropolis–Hastings, Gibbs) construct a chain
                      whose stationary distribution equals the target distribution.

Queueing theory:      States = number of items in a queue. Used to model server
                      load and predict wait times.
```

## Historical Context

Andrei Andreyevich Markov (1856–1922) introduced the model in 1906 while studying the
distribution of vowels and consonants in Pushkin's *Eugene Onegin*. He wanted to show
that the law of large numbers applied to dependent (non-independent) events — a pointed
rebuttal to critics who claimed probability theory only applied to coin-flips.

His model was purely mathematical until the mid-20th century, when Claude Shannon (1948)
used it in *A Mathematical Theory of Communication* to model English text as a
statistical process. From there the model spread into every quantitative field.

## Key Concepts

### State Space and Transitions

```
Example: weather model with 3 states.

States: {Sunny, Cloudy, Rainy}

Transition matrix T:
          → Sunny  → Cloudy  → Rainy
Sunny  [    0.7      0.2      0.1  ]
Cloudy [    0.3      0.4      0.3  ]
Rainy  [    0.2      0.3      0.5  ]

Reading row "Cloudy": if it is Cloudy today, then tomorrow it is:
  Sunny  with probability 0.3
  Cloudy with probability 0.4
  Rainy  with probability 0.3
```

### Building a Chain from Data (Training)

Given a sequence of observed states, estimate transition probabilities by counting:

```
Observations: [A, B, A, C, A, B, B, A]

Count transitions:
  A → B: 2   A → C: 1   B → A: 2   B → B: 1   C → A: 1

Normalize each row:
  From A: {B: 2/3, C: 1/3}
  From B: {A: 2/3, B: 1/3}
  From C: {A: 1/1}

Transition matrix (rows must sum to 1):
      A     B     C
  A [0.00  0.67  0.33]
  B [0.67  0.33  0.00]
  C [1.00  0.00  0.00]
```

Unknown transitions (count = 0) can use **Laplace smoothing** to avoid zero
probabilities, which would make the chain get "stuck":

```
Smoothed count(i → j) = raw_count(i → j) + α    (α = 1 for Laplace)
Smoothed T[i][j] = smoothed_count(i → j) / sum_k(smoothed_count(i → k))
```

### Sampling / Generation

To generate a new sequence:

```
1. Choose an initial state (e.g., uniformly at random, or from the stationary
   distribution, or from a user-supplied seed).
2. Repeatedly sample the next state from the current state's row of T.
3. Stop after n steps, or when a terminal state is reached.
```

Sampling one step from a row is equivalent to sampling from a categorical distribution:

```
row = T[current_state]   # a vector of probabilities summing to 1
r   = random_uniform(0, 1)
cumulative = 0
for (state, probability) in row:
  cumulative += probability
  if r < cumulative: return state
```

### Stationary Distribution

For an ergodic chain (all states reachable from all others, no periodic traps), the
chain converges to a unique **stationary distribution** π such that:

```
π · T = π      (π is a left eigenvector of T with eigenvalue 1)
```

The stationary distribution answers: "In the long run, what fraction of time does the
chain spend in each state?" For the weather model above, π ≈ [0.47, 0.28, 0.25].

Computing the stationary distribution:
- **Power iteration:** multiply an arbitrary distribution by T repeatedly until convergence.
- **Exact:** solve the linear system `π · (T - I) = 0` with the constraint `∑πᵢ = 1`.
- **Eigendecomposition:** find left eigenvector for eigenvalue 1 of T.

CMP06 (and LZMA) use the stationary distribution implicitly: the chain trains on data
and then samples from it to predict the next symbol. The range coder uses the
probability estimates from the current state's row to assign shorter codes to more
likely symbols.

### Order-k Markov Chains

A standard (order-1) Markov chain's next state depends on only the last 1 observed
state. An **order-k** chain extends the memory window to k states:

```
P(next | last k states) = T[s_{n-k}, …, s_{n-1}][s_n]
```

For text generation:
- Order 1: "e" → next letter probabilities for each letter
- Order 2: "th" → next letter probabilities for each digram
- Order 3: "the" → next letter probabilities for each trigram

Higher orders produce more realistic output but require exponentially more memory
(|S|^k states instead of |S|).

Implementation note: an order-k chain over alphabet Σ has |Σ|^k states. For
characters (|Σ|=256) and k=2, that is 65536 states. Store the transition table as
a sparse map from k-grams to probability distributions rather than a dense matrix.

## Data Structure

```
MarkovChain {
  states:      List<T>                        # ordered list of unique states
  state_index: Map<T, int>                    # state → row index
  transitions: Map<T, Map<T, float>>          # state → {next_state → probability}
  order:       int                            # k (default 1)
}
```

For order > 1, replace `T` with `Tuple<T, k>` (a k-gram). The external API stays the
same; the k-gram is assembled from a sliding window of the last k observations.

## Public API

```
# Construction
MarkovChain.new(order: int = 1) → MarkovChain

# Training
chain.train(sequence: List<T>) → void
  # Slide a window of size (order+1) over sequence.
  # For each window, increment count(window[0..order-1] → window[order]).
  # After all windows, normalise each row to probabilities.
  # May be called multiple times; counts accumulate before re-normalising.

chain.train_string(text: String) → void
  # Convenience: treat each character as a state. Equivalent to train(text.chars).

# Querying
chain.next_state(current: T) → T
  # Sample one transition from T[current]. Raises if current is unknown.
  # For order-k chain, current is a k-tuple.

chain.generate(start: T, length: int) → List<T>
  # Emit a sequence of `length` states starting from `start` (inclusive).
  # For order-k: start is a k-tuple; each step advances the window by 1.

chain.generate_string(seed: String, length: int) → String
  # Convenience for character chains. Seed must be at least `order` characters.

chain.probability(from: T, to: T) → float
  # Return T[from][to], or 0.0 if the transition was never observed (and smoothing=0).

chain.stationary_distribution() → Map<T, float>
  # Power iteration until convergence (|π_{n+1} - π_n| < 1e-10).
  # Raises if the chain is not ergodic.

# Inspection
chain.states() → List<T>
chain.transition_matrix() → Map<T, Map<T, float>>
```

## Smoothing

The constructor (or a separate method) accepts a `smoothing` parameter (default 0.0):

- `0.0` — no smoothing; zero-probability transitions stay zero.
- `1.0` — Laplace smoothing; every unseen transition gets count 1.
- `α > 0` — Lidstone smoothing; every unseen transition gets count α.

Smoothing is applied at training time when rows are normalised. It ensures that
`next_state` never gets stuck in a state with no outgoing transitions.

## Algorithm: `train`

```
function train(sequence):
  for i in 0..(len(sequence) - order - 1):
    context = sequence[i .. i+order-1]   # k-gram key
    target  = sequence[i + order]
    counts[context][target] += 1

  for context in counts:
    total = sum(counts[context].values) + smoothing * |all_states|
    for target in all_states:
      raw = counts[context].get(target, 0) + smoothing
      transitions[context][target] = raw / total
```

## Algorithm: `stationary_distribution` (power iteration)

```
function stationary_distribution():
  n = |states|
  π = uniform distribution: {s: 1/n for s in states}

  loop:
    π_new = {}
    for s_j in states:
      π_new[s_j] = sum(π[s_i] * transitions[s_i][s_j] for s_i in states)
    if max(|π_new[s] - π[s]| for s in states) < 1e-10: break
    π = π_new

  return π
```

## Example: Character-Level Text Generation

```python
chain = MarkovChain(order=2, smoothing=1.0)
chain.train_string("the quick brown fox jumps over the lazy dog " * 100)

# seed must be >= order characters
generated = chain.generate_string("th", length=200)
# → "the quich brown fox jumpse lay the quick brows fox ju..."
```

Higher-order chains produce more realistic output at the cost of needing more training
data to fill the transition table without relying on smoothing:

```
Order 1: "eoevh tsoi nftuu  aeu h h  lheo e  ue tsa..."   (random-ish)
Order 2: "the he the quicke ther the quirownthe quic..."    (word fragments)
Order 3: "the quick brown fox jumps over the laze dog..."   (nearly verbatim)
```

## Use in Compression (Connection to LZMA / CMP08)

LZMA's "Markov chain" refers to using the **current byte value** (and recent match
history) as a state, and the transition probabilities as **range coder symbol
probabilities**. Instead of one fixed probability for each bit, LZMA maintains
2048+ probability tables (one per context state) and updates them dynamically as each
bit is coded. This lets the range coder adapt to local statistics in the data.

The CodingAdventures MarkovChain package (DT28) does not implement LZMA's specific
probability update rule; it is a general-purpose Markov chain for training on sequences
and sampling from them. LZMA (CMP08, if implemented) would use DT28 as a conceptual
foundation but would need its own adaptive probability tracking.

## Test Cases

Every implementation MUST pass:

1. **Construction** — `MarkovChain.new()` creates an empty chain with 0 states.
2. **Train single pair** — train on `[A, B]` (order=1). `probability(A, B) == 1.0`.
3. **Train sequence** — train on `[A, B, A, C, A, B, B, A]`.
   - `probability(A, B) ≈ 0.667`, `probability(A, C) ≈ 0.333`.
   - `probability(B, A) ≈ 0.667`, `probability(B, B) ≈ 0.333`.
4. **Laplace smoothing** — train on `[A, B]` with smoothing=1.0 and 3 total states
   (A, B, C). `probability(A, C) == 1/4` (1 smoothed count out of 4 total).
5. **Generate length** — `generate(A, 10)` returns a list of exactly 10 states.
6. **Generate string** — `generate_string("th", 50)` on a character chain trained on
   English text returns a 50-char string starting with "th".
7. **Stationary distribution sums to 1** — for any ergodic chain, `sum(stationary_distribution().values) ≈ 1.0`.
8. **Order-2 chain** — train on `"abcabcabc"` with order=2. The context `"ab"` should
   transition to `'c'` with probability 1.0. `generate_string("ab", 9) == "abcabcabc"`.
9. **Unknown state** — calling `next_state` on an unseen state raises an error.
10. **Multi-train accumulation** — calling `train` twice accumulates counts before
    re-normalising, so probabilities reflect the combined training data.

## Packages

Implement as `coding-adventures-markov-chain` in each language:

| Language   | Package name                              | Module/namespace                          |
|------------|-------------------------------------------|-------------------------------------------|
| Python     | `coding-adventures-markov-chain`          | `coding_adventures.markov_chain`          |
| Go         | `coding-adventures-markov-chain`          | `codingadventures/markovchain`            |
| Ruby       | `coding_adventures_markov_chain`          | `CodingAdventures::MarkovChain`           |
| TypeScript | `@coding-adventures/markov-chain`         | `CodingAdventures.MarkovChain`            |
| Rust       | `coding-adventures-markov-chain`          | `coding_adventures_markov_chain`          |
| Elixir     | `coding_adventures_markov_chain`          | `CodingAdventures.MarkovChain`            |
| Lua        | `coding-adventures-markov-chain`          | `CodingAdventures.MarkovChain`            |
| Perl       | `CodingAdventures::MarkovChain`           | `CodingAdventures::MarkovChain`           |
| Swift      | `CodingAdventuresMarkovChain`             | `CodingAdventures.MarkovChain`            |
