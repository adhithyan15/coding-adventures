# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-04-13

### Added

- `MarkovChain` struct — general-purpose order-k Markov Chain over `String`
  states, implementing all 10 DT28 spec test cases.
- Constructor `init(order:smoothing:states:)` — supports pre-declaring a
  state vocabulary for correct Laplace/Lidstone smoothing denominators.
- `train(_:)` — sliding-window count accumulation across multiple calls;
  counts persist and re-normalise after each call so sequential training
  produces the same result as a single combined training pass.
- `trainString(_:)` — convenience wrapper that treats each character in a
  `String` as an individual state.
- `nextState(_:)` — CDF (inverse-transform) sampling of the next state from
  a trained context; throws `MarkovError.unknownState` for unseen states.
- `generate(start:length:)` — generates a sequence of exactly `length`
  states, sliding the order-k context window forward one step at a time.
- `generateString(seed:length:)` — character-level sequence generation;
  requires a seed of at least `order` characters; throws
  `MarkovError.seedTooShort` otherwise.
- `probability(from:to:)` — returns `T[from][to]`, or 0.0 for unseen
  transitions (when smoothing = 0).
- `stationaryDistribution()` — power-iteration solver; converges to π·T = π
  within 10000 iterations or throws `MarkovError.notErgodic`.
- `states()` — sorted list of all known base states.
- `transitionMatrix()` — full normalised probability table.
- `MarkovError` enum with cases `unknownState`, `notErgodic`, `seedTooShort`.
- Dependency on `DirectedGraph` (product from `code/packages/swift/directed-graph`)
  for state-transition topology tracking.
- XCTest suite covering all 10 spec cases plus:
  - probability rows summing to 1.0
  - `seedTooShort` error path
  - pre-declared states appearing in `states()` before training
  - zero-length `generate` returning an empty array
- `BUILD` — xcrun/swift test with code-coverage flag.
- `BUILD_windows` — graceful no-op on non-Swift runners.
- `README.md` with installation, usage examples for all major features.
