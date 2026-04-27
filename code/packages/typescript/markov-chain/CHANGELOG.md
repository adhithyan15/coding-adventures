# Changelog — @coding-adventures/markov-chain

All notable changes to this package are documented here.

This project adheres to [Semantic Versioning](https://semver.org/).

---

## [0.1.0] — 2026-04-13

### Added

- Initial implementation of `MarkovChain` class (DT28).
- Constructor: `new MarkovChain(order?, smoothing?, states?)`.
  - `order` (default 1): context window size. Order=1 is a standard first-order
    Markov chain; order=k uses the last k states as context.
  - `smoothing` (default 0.0): Laplace/Lidstone smoothing parameter α.
    Set to 1.0 for add-one smoothing.
  - `states`: optional pre-registered alphabet so smoothing covers all states
    even before training data is seen.
- `train(sequence)`: slides a window of size `order + 1` over the sequence,
  accumulates raw counts, then normalises to probabilities. Multiple calls
  accumulate correctly.
- `trainString(text)`: convenience wrapper — treats each character as a state.
- `nextState(current)`: weighted random sampling via inverse-CDF (roulette-wheel)
  from the current state's transition row. Throws a descriptive error if the
  state is unknown.
- `generate(start, length)`: returns exactly `length` states. For order=1,
  start is the first state; for order>1, start is the joined context key.
- `generateString(seed, length)`: returns exactly `length` characters for
  character-level chains. Seed must be at least `order` characters long.
- `probability(from, to)`: O(1) lookup into the transition table. Returns 0.0
  for unknown states or unseen transitions.
- `stationaryDistribution()`: power iteration until convergence (tolerance 1e-10,
  max 10,000 iterations). Returns a `Map<string, number>` over the alphabet.
- `states()`: returns the list of all known individual states (the alphabet).
- `transitionMatrix()`: returns a deep copy of the full probability table.

### Architecture

- `MarkovChain` internally holds a `Graph` (from `@coding-adventures/directed-graph`)
  for topology. Nodes = state keys; edges = non-zero probability transitions.
  `Graph.newAllowSelfLoops()` semantics are achieved via `new Graph({ allowSelfLoops: true })`.
- Probability values are stored separately in `_transitions: Map<string, Map<string, number>>`.
- Raw counts are kept in `_counts: Map<string, Map<string, number>>` and
  re-normalised after every `train()` call so multiple training calls accumulate.
- Order-k context keys use `\x00` (null-byte) as separator, making keys
  unambiguous even when states themselves contain other punctuation.

### Tests

- 10 spec-compliant test cases from DT28-markov-chain.md, plus additional edge-case
  and coverage tests.
- Vitest with `@vitest/coverage-v8`, coverage threshold set to 80% lines.
