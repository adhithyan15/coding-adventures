# Changelog

All notable changes to `coding-adventures-markov-chain` (Rust) will be
documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.1.0] — 2026-04-13

### Added

- Initial implementation of `MarkovChain` for DT28.
- `MarkovChain::new(order, smoothing, states)` constructor.
  - `order ≥ 1`; panics on `order = 0`.
  - `smoothing = 0.0` for no smoothing; `1.0` for Laplace; `α > 0` for
    Lidstone.
  - `states` pre-registers an alphabet; pass `vec![]` to grow it from data.
- `train(&[&str])` — slides a window of size `order + 1` over the sequence,
  accumulates raw counts, then normalises with smoothing.  Multiple calls
  accumulate counts across calls before re-normalising.
- `train_string(&str)` — convenience wrapper; splits text into individual
  characters before calling `train`.
- `next_state(&str)` — roulette-wheel (inverse CDF) sampling using
  `rand::thread_rng`.  Returns `MarkovError::UnknownState` for unseen contexts.
- `generate(&str, usize)` — generates a sequence of exactly `length` tokens
  starting from a seed k-gram context.
- `generate_string(&str, usize)` — character-level convenience that assembles
  the first k-gram from the seed string and returns a concatenated string.
- `probability(&str, &str)` — returns the transition probability (0.0 for
  unknown pairs).
- `stationary_distribution()` — power iteration until convergence (`< 1e-10`),
  up to 10 000 iterations.  Returns `MarkovError::NotErgodic` on failure.
- `states()` — returns the registered alphabet in insertion order.
- `transition_matrix()` — returns the full `HashMap<String, HashMap<String, f64>>`.
- `MarkovError` enum with three variants: `UnknownState`, `NotErgodic`,
  `SeedTooShort`.
- Internally wraps `directed_graph::Graph::new_allow_self_loops()` for
  topology tracking; probabilities stored in a separate `HashMap`.
- Order-k context keys formed by joining tokens with `\x00` separator.
- 13 inline `#[cfg(test)]` tests covering all 10 spec test cases plus 3
  additional edge cases.
