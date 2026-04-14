# coding-adventures-markov-chain (Rust)

A general-purpose, order-k Markov Chain data structure — **DT28** in the
CodingAdventures stack.

## What is a Markov Chain?

A Markov Chain is a mathematical model of a system that moves between a
finite set of **states** over time.  The key property — the **Markov
property** — is that the probability of the next state depends *only* on the
current state, not on any earlier history.

This sounds like a limitation, but it is enormously powerful.  Text
generation, compression (LZMA), PageRank, game AI, and MCMC sampling all
build on this simple idea.

## Where it fits in the stack

```
DT01 directed-graph   ← DT28 markov-chain uses this for topology
                              ↑
                        CMP06 brotli (conceptual connection only)
```

`markov-chain` wraps a `directed_graph::Graph` (with self-loops allowed) to
track which transitions *exist*, and maintains a separate `HashMap` for the
*probabilities*.

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
coding-adventures-markov-chain = { path = "../markov-chain" }
```

## Usage

### Order-1 chain — training and sampling

```rust
use coding_adventures_markov_chain::MarkovChain;

let mut chain = MarkovChain::new(1, 0.0, vec![]);
chain.train(&["A", "B", "A", "C", "A", "B", "B", "A"]).unwrap();

// Query probabilities
assert!((chain.probability("A", "B") - 2.0/3.0).abs() < 1e-9);

// Generate a sequence of 10 states
let seq = chain.generate("A", 10).unwrap();
assert_eq!(seq.len(), 10);
```

### Laplace smoothing

```rust
use coding_adventures_markov_chain::MarkovChain;

// Pre-register the alphabet; unseen transitions get count α = 1.
let mut chain = MarkovChain::new(1, 1.0, vec!["A".into(), "B".into(), "C".into()]);
chain.train(&["A", "B"]).unwrap();

// Without smoothing, P(A→C) = 0.0.  With Laplace smoothing it is 0.25.
assert!((chain.probability("A", "C") - 0.25).abs() < 1e-9);
```

### Order-2 character chain

```rust
use coding_adventures_markov_chain::MarkovChain;

let mut chain = MarkovChain::new(2, 0.0, vec![]);
chain.train_string("abcabcabc").unwrap();

// Every transition is deterministic → perfect reconstruction.
let output = chain.generate_string("ab", 9).unwrap();
assert_eq!(output, "abcabcabc");
```

### Stationary distribution

```rust
use coding_adventures_markov_chain::MarkovChain;

let mut chain = MarkovChain::new(1, 0.0, vec![]);
chain.train(&["A", "B", "A", "C", "A", "B", "B", "A"]).unwrap();

let dist = chain.stationary_distribution().unwrap();
let sum: f64 = dist.values().sum();
assert!((sum - 1.0).abs() < 1e-6);
```

## API Reference

| Method | Description |
|--------|-------------|
| `new(order, smoothing, states)` | Create a chain; `states=vec![]` grows alphabet from data |
| `train(&[&str])` | Train on a sequence of state labels |
| `train_string(&str)` | Train treating each character as a state |
| `next_state(&str)` | Sample one transition (RNG-based) |
| `generate(&str, usize)` | Generate a sequence of N states |
| `generate_string(&str, usize)` | Generate N characters (character chains) |
| `probability(&str, &str)` | Query transition probability (0.0 if unknown) |
| `stationary_distribution()` | Compute stationary distribution via power iteration |
| `states()` | Return the registered alphabet |
| `transition_matrix()` | Return the full probability table |

## Order-k context keys

For order > 1, context keys are formed by joining the k preceding tokens with
the null-byte separator `\x00`.  For example, the order-2 context `["a","b"]`
becomes the key `"a\x00b"`.

`generate_string` and `generate` handle this automatically.  You only need to
know the format if you call `probability` directly on a higher-order chain.

## Running tests

```bash
cargo test -p coding-adventures-markov-chain
```
