# @coding-adventures/markov-chain

**DT28** — General-purpose Markov Chain data structure for TypeScript.

Built on `@coding-adventures/directed-graph` (DT01).

---

## What is a Markov Chain?

A Markov Chain models a system that moves between a finite set of **states** over
time. The key property is **memorylessness**: the probability of the next state
depends *only* on the current state, not on any history.

Think of it like a game-show spinner nailed to each square of a board:
land on square A and the spinner says "70% → B, 30% → C". No matter how many
times you've visited A, the spinner always reads the same numbers.

Applications include:

- **Text generation** — states are characters or words; train on a corpus, then
  sample to produce new text.
- **Compression (LZMA)** — the Markov chain predicts the next byte's probability
  distribution, which drives a range coder.
- **PageRank** — states are web pages; the stationary distribution gives each
  page its rank.
- **Game AI** — states are NPC moods or positions; transitions are likelihoods of
  taking certain actions.

---

## Where it fits

```
code/packages/typescript/
├── graph/                     ← DT00: undirected graph
├── directed-graph/            ← DT01: directed graph (dependency of this package)
└── markov-chain/              ← DT28: this package
```

The `MarkovChain` class uses `Graph` internally to store state-transition
topology. Non-zero probability edges are edges in the graph; the actual
probability values live in a separate `Map<string, Map<string, number>>`.

---

## Installation

```bash
npm install @coding-adventures/markov-chain
```

---

## Usage

### Order-1 character chain

```typescript
import { MarkovChain } from "@coding-adventures/markov-chain";

const chain = new MarkovChain(1, 0.1);
chain.trainString("the quick brown fox jumps over the lazy dog ".repeat(50));

// Generate 100 characters starting from "t"
console.log(chain.generateString("t", 100));
// → "the quich brox jum the lay dohe quicn foxe..."
```

### Order-2 character chain (more realistic)

```typescript
const chain = new MarkovChain(2, 0.1);
chain.trainString("abcabcabc".repeat(10));
console.log(chain.generateString("ab", 9));
// → "abcabcabc"
```

### Word-level chain

```typescript
const chain = new MarkovChain(1, 0.5);
chain.train(["the", "quick", "brown", "fox", "the", "lazy", "dog"]);
console.log(chain.generate("the", 5));
// → ["the", "quick", ...] or ["the", "lazy", ...]
```

### Stationary distribution

```typescript
const chain = new MarkovChain(1, 1.0);
chain.train(["A", "B", "C", "A", "B", "C"]);
const dist = chain.stationaryDistribution();
console.log(dist);
// Map { "A" => 0.333, "B" => 0.333, "C" => 0.333 } (symmetric chain)
```

---

## API Reference

```typescript
class MarkovChain {
  // Create a chain with given order (default 1), smoothing (default 0.0),
  // and optional pre-registered alphabet.
  constructor(order?: number, smoothing?: number, states?: string[])

  // Train on a sequence of state tokens. May be called multiple times;
  // counts accumulate before re-normalising.
  train(sequence: string[]): void

  // Convenience: treat each character of text as a state token.
  trainString(text: string): void

  // Sample the next state. Throws if current is unknown.
  nextState(current: string): string

  // Generate exactly `length` states. For order=1 start is the first state;
  // for order>1 start is the context key (k states joined by "\x00").
  generate(start: string, length: number): string[]

  // Generate exactly `length` characters. Seed must be >= order chars long.
  generateString(seed: string, length: number): string

  // Probability of transitioning from → to. Returns 0.0 if unknown.
  probability(from: string, to: string): number

  // Compute stationary distribution via power iteration.
  stationaryDistribution(): Map<string, number>

  // List of all known individual states (the alphabet).
  states(): string[]

  // Full probability table (deep copy).
  transitionMatrix(): Map<string, Map<string, number>>
}
```

---

## Order-k chains

For order > 1, context keys are k individual state tokens joined by the
null-byte separator `\x00`. You can construct or query these keys yourself:

```typescript
// Order-2 chain, query the "ab" context
chain.probability("a\x00b", "c");

// Generate starting from the "ab" context
chain.generate("a\x00b", 9);

// generateString automatically constructs the context key from the seed
chain.generateString("ab", 9);  // uses last 2 chars of "ab" as context
```

---

## Smoothing

By default (`smoothing = 0.0`) unseen transitions have probability 0, which
can cause generation to get stuck. Use Laplace smoothing (`α = 1.0`) or
Lidstone smoothing (`0 < α < 1`) to assign small pseudo-probabilities to all
transitions:

```
smoothed_prob(i → j) = (count(i → j) + α) / (total_count(i) + α × |Σ|)
```

Pre-registering the alphabet via the constructor ensures the smoothing
denominator includes ALL possible states, even those not yet seen in training:

```typescript
const chain = new MarkovChain(1, 1.0, ["A", "B", "C"]);
chain.train(["A", "B"]);
chain.probability("A", "C");  // 0.25, not 0 — because C was pre-registered
```

---

## Development

```bash
npm ci
npm run test:coverage
```

Tests use Vitest and cover all 10 spec cases from DT28-markov-chain.md.
