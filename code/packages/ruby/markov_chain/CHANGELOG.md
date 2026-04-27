# Changelog

All notable changes to `coding_adventures_markov_chain` are documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).
This project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] — 2026-04-13

### Added

- Initial implementation of `CodingAdventures::MarkovChain` (DT28 spec).
- `MarkovChain.new(order:, smoothing:, states:)` constructor with optional
  alphabet pre-registration.
- `train(sequence)` — sliding-window count accumulation; multiple calls
  accumulate before re-normalising.
- `train_string(text)` — convenience wrapper treating each character as a state.
- `next_state(current)` — categorical sampling from the current row;
  raises `KeyError` for unknown contexts.
- `generate(start, length)` — returns Array of exactly `length` states.
- `generate_string(seed, length)` — returns String of exactly `length` chars.
- `probability(from, to)` — O(1) probability lookup; returns 0.0 for unknown
  transitions.
- `stationary_distribution` — power iteration converging to 1e-10 tolerance;
  raises `RuntimeError` after 10,000 iterations for non-ergodic chains.
- `states` — sorted Array of all states in the alphabet.
- `transition_matrix` — defensive copy of the full probability table.
- Order-k chains: context key is `sequence[i, order].freeze` (frozen Array).
- Laplace / Lidstone smoothing: smoothing denominator includes ALL known
  states (pre-registered + seen in training).
- Depends on `coding_adventures_directed_graph` for topology storage.
- Full Minitest test suite covering all 10 mandatory spec cases plus
  additional edge-case and coverage tests.
- `BUILD` and `BUILD_windows` files for the polyglot build system.
- `required_capabilities.json` declaring no OS capabilities needed.
