# Changelog

All notable changes to the `coding-adventures-markov-chain` Go package are
documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.1.0] - 2026-04-13

### Added

- Initial implementation of the `markovchain` Go package (DT28).
- `New(order int, smoothing float64, states []string) *MarkovChain` constructor
  with optional pre-registration of the state alphabet for correct Laplace
  smoothing denominators.
- `Train(sequence []string) error` — sliding window k-gram training with
  Lidstone/Laplace smoothing. Counts accumulate across multiple calls before
  re-normalizing, enabling incremental learning.
- `TrainString(text string) error` — convenience wrapper that treats each
  Unicode rune as a state (character-level chains).
- `NextState(current string) (string, error)` — samples one transition from
  the current state's probability row; returns error for unknown states.
- `Generate(start string, length int) ([]string, error)` — produces a sequence
  of exactly `length` states, advancing a sliding window for order-k chains.
- `GenerateString(seed string, length int) (string, error)` — character-level
  sequence generation; seed must be at least `order` characters long.
- `Probability(from, to string) float64` — direct probability table lookup;
  returns 0.0 for unknown contexts without panicking.
- `StationaryDistribution() (map[string]float64, error)` — power iteration
  with 10,000 max iterations and 1e-10 convergence threshold.
- `States() []string` — sorted list of all known atomic states.
- `TransitionMatrix() map[string]map[string]float64` — deep-copy of the
  probability table.
- Internal use of `directed-graph` for state topology tracking.
- Order-k support using null-byte (`"\x00"`) separated k-gram context keys.
- Comprehensive test suite with 20 test functions covering all 10 spec cases
  plus edge cases (empty sequences, Unicode, Lidstone smoothing, unknown
  states, multi-train accumulation).
