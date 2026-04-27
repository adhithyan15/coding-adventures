# Changelog

## [0.1.0] - 2026-04-13

### Added

- Initial implementation of `MarkovChain` (DT28).
- `MarkovChain(order, smoothing, states)` constructor with optional
  pre-registered alphabet for smoothing denominator coverage.
- `train(sequence)` — accumulates raw counts from a list of states, then
  normalises with Laplace/Lidstone smoothing over all known states.
- `train_string(text)` — convenience wrapper: treats each character as a state.
- `next_state(current)` — categorical sampling via single uniform draw;
  raises `ValueError` for unknown contexts.
- `generate(start, length)` — returns exactly `length` states starting from
  `start`; supports order-k context windows.
- `generate_string(seed, length)` — character chain convenience; returns
  exactly `length` characters starting with `seed`.
- `probability(from_state, to_state)` — returns 0.0 gracefully for unseen
  contexts or targets.
- `stationary_distribution()` — power iteration (up to 10,000 steps,
  convergence threshold 1e-10); raises `ValueError` for non-ergodic chains.
- `states()` — sorted list of all known atomic states.
- `transition_matrix()` — returns an independent copy of the probability table.
- Order-k support: context keys are k-tuples for `order > 1`; `generate` and
  `generate_string` maintain a sliding window of depth k.
- Internal `DirectedGraph(allow_self_loops=True)` for topology (DT01):
  non-zero transitions are synced as directed edges after each normalisation,
  enabling graph-algorithm queries (reachability, SCC, etc.).
- Separate `_transitions` dict for probabilities, decoupled from graph weights.
- Multi-train accumulation: raw counts persist across `train()` calls;
  re-normalisation happens at the end of each call.
- `py.typed` marker for PEP 561 compliance.
- Full literate-programming inline documentation (Knuth style).
- 95%+ test coverage across all 10 spec test cases plus edge-case tests.
