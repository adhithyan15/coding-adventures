# coding-adventures-markov-chain (DT28)

A general-purpose **Markov Chain** data structure built on top of the
`directed-graph` package (DT01). Supports order-k chains, Laplace/Lidstone
smoothing, sequence generation, and stationary distribution computation.

## What is a Markov Chain?

A Markov Chain models a system that moves between a finite set of **states**
over time, where the probability of the next state depends **only on the current
state** — not on any history of how the system arrived there. This
"memorylessness" property is the **Markov property**.

The chain is captured in a **transition matrix** T where `T[i][j]` is the
probability of going from state `sᵢ` to state `sⱼ`. Each row sums to 1.0.

### Where Markov Chains appear

- **Text generation** — train on characters/words; sample to generate text
- **Compression (LZMA)** — context-sensitive symbol probability tables
- **PageRank** — random surfer model; stationary distribution = page rank
- **Game AI** — states = positions or moods; transitions = stochastic actions
- **Biology** — nucleotide sequences (A, C, G, T) for CpG island detection

## How it fits the stack

```
MarkovChain (DT28)
    └─ DirectedGraph (DT01)   [topology: states as nodes, transitions as edges]
           └─ Graph (DT00)    [base adjacency list]
```

The `DirectedGraph` (with `allow_self_loops=True`) tracks **which state-to-state
transitions exist** (topology). Transition probabilities live in a separate
`_transitions` dict — the graph enables graph-algorithm queries (reachability,
SCCs) while the dict drives sampling.

## Installation

```bash
pip install coding-adventures-markov-chain
```

## API

```python
from coding_adventures_markov_chain import MarkovChain

# Construction
chain = MarkovChain(order=1, smoothing=0.0, states=None)
```

| Method | Description |
|--------|-------------|
| `train(sequence)` | Accumulate transition counts from a list of states, then normalise. |
| `train_string(text)` | Convenience: treat each character as a state. |
| `next_state(current)` | Sample one transition. Raises `ValueError` if `current` is unknown. |
| `generate(start, length)` | Return exactly `length` states starting from `start`. |
| `generate_string(seed, length)` | Return exactly `length` characters starting with `seed`. |
| `probability(from_state, to_state)` | Transition probability; 0.0 if unknown. |
| `stationary_distribution()` | Power iteration; raises if chain is not ergodic. |
| `states()` | All known atomic states (sorted). |
| `transition_matrix()` | Full probability table as a dict-of-dicts. |

### Parameters

| Parameter | Default | Description |
|-----------|---------|-------------|
| `order` | `1` | Context window depth k. order=1 uses last 1 state; order=2 uses last 2, etc. |
| `smoothing` | `0.0` | Laplace/Lidstone pseudo-count α. 0 = no smoothing; 1 = Laplace; any α > 0 = Lidstone. |
| `states` | `None` | Pre-register an alphabet. Ensures smoothing covers unseen states. |

## Usage examples

### Basic sequence training

```python
from coding_adventures_markov_chain import MarkovChain

chain = MarkovChain()
chain.train(["A", "B", "A", "C", "A", "B", "B", "A"])

chain.probability("A", "B")   # ≈ 0.667
chain.probability("A", "C")   # ≈ 0.333
chain.probability("B", "A")   # ≈ 0.667
chain.probability("B", "B")   # ≈ 0.333
```

### Character-level text generation

```python
chain = MarkovChain(order=2, smoothing=0.5)
chain.train_string(
    "the quick brown fox jumps over the lazy dog " * 50
)

# Generate 200 characters seeded with "th"
text = chain.generate_string("th", 200)
print(text)
# → "the quick brow fox jumpse lay the quiche brows fox…"
```

### Stationary distribution

```python
chain = MarkovChain()
chain.train([
    "Sunny", "Cloudy", "Rainy", "Sunny", "Sunny",
    "Cloudy", "Rainy", "Sunny", "Cloudy", "Rainy"
] * 20)

dist = chain.stationary_distribution()
print(dist)
# → {"Cloudy": 0.28, "Rainy": 0.25, "Sunny": 0.47}  (approx)

print(sum(dist.values()))   # 1.0
```

### Order-2 deterministic chain

```python
chain = MarkovChain(order=2)
chain.train_string("abcabcabc")

# Each 2-gram context maps to exactly one next character.
chain.probability(("a", "b"), "c")   # 1.0
chain.generate_string("ab", 9)       # "abcabcabc"
```

### Laplace smoothing with pre-registered alphabet

```python
# Register all 3 states upfront so smoothing covers unseen transitions.
chain = MarkovChain(order=1, smoothing=1.0, states=["A", "B", "C"])
chain.train(["A", "B"])

chain.probability("A", "A")   # 0.25   (1 pseudo-count / 4 total)
chain.probability("A", "B")   # 0.50   (2 counts / 4 total)
chain.probability("A", "C")   # 0.25   (1 pseudo-count / 4 total)
```

### Multi-train accumulation

```python
chain = MarkovChain()
chain.train(["A", "B"])   # A→B: 1 count
chain.train(["A", "C"])   # A→C: 1 count, A→B still 1

chain.probability("A", "B")   # 0.5
chain.probability("A", "C")   # 0.5
```

## Smoothing

| α | Name | Effect |
|---|------|--------|
| 0.0 | None / MLE | Zero-probability transitions stay zero. Chain can get stuck. |
| 1.0 | Laplace | Every unseen transition gets 1 pseudo-count. Standard choice. |
| 0 < α < 1 | Lidstone | Fractional pseudo-counts. Less aggressive than Laplace. |

Always provide the `states` list when using smoothing so the denominator
covers the full alphabet, not just states seen in training.

## Order-k chains

Higher orders produce more realistic output at the cost of needing more
training data:

| Order | Context | Memory |
|-------|---------|--------|
| 1 | last char: `'e'` | |Σ| states |
| 2 | last 2 chars: `('t','h')` | |Σ|² states |
| 3 | last 3 chars: `('t','h','e')` | |Σ|³ states |

For `order > 1`:
- `next_state(current)` expects a k-tuple as `current`.
- `generate(start, n)` expects a k-tuple as `start`.
- `generate_string(seed, n)` accepts a string; uses the last `order` characters as the context window.

## Running the tests

```bash
uv pip install -e ".[dev]"
uv run pytest
```

## Connection to CMP06 (Brotli)

The DT28 Markov Chain is the statistical foundation for adaptive context
modelling in compression algorithms. LZMA's "Markov chain" maintains per-state
probability tables and updates them as each symbol is coded — a specialised
variant of the general training loop implemented here.
