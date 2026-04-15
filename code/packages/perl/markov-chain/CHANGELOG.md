# Changelog

All notable changes to `CodingAdventures::MarkovChain` are documented here.

## [0.1.0] - 2026-04-13

### Added

- Initial implementation of `CodingAdventures::MarkovChain` (DT28).
- Constructor `new(order, smoothing, states)` with full keyword-argument API.
- `train(\@sequence)` — sliding-window count accumulation with re-normalisation.
- `train_string($text)` — character-level convenience wrapper.
- `next_state($context)` — CDF sampling with sorted-state determinism; dies on
  unknown contexts.
- `generate($start, $length)` — produces an arrayref of exactly `$length` states.
- `generate_string($seed, $length)` — convenience wrapper returning a string.
- `probability($from, $to)` — returns T[from][to] or 0.0.
- `stationary_distribution()` — power-iteration to convergence (ε = 1e-10).
- `states()` — sorted arrayref of all known atomic states.
- `transition_matrix()` — deep-copy of internal transition table.
- Laplace / Lidstone smoothing via `smoothing` constructor parameter.
- Order-k chain support: context key = `join("\0", @window[0..k-1])`.
- 13 tests covering all 10 spec-required cases plus load, transition-matrix,
  and unseen-probability edge cases.
- `BUILD` script installs `directed-graph` dependency then runs `prove -l -v t/`.
- `BUILD_windows` stub (Perl not supported on Windows CI).
- `Makefile.PL` and `cpanfile` for CPAN-style packaging.
- `README.md` with full API table and usage examples.

### Implementation notes

- Uses `CodingAdventures::DirectedGraph->new_allow_self_loops` for topology
  (self-loops arise when a state transitions to itself, e.g., B→B).
- Raw counts are stored in `_counts` and converted to probabilities in
  `_transitions` after every `train()` call, so multi-call accumulation is
  transparent to callers.
- For order-k chains (k > 1), the context keys in `_transitions` are k-gram
  strings; the `states()` method always returns the single-step atomic states
  that were individually registered via `_register_state`.
