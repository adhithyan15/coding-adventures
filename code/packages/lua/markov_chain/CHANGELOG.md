# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-04-13

### Added

- **MarkovChain** — general-purpose order-k Markov Chain data structure
  - `MarkovChain.new(order, smoothing, states)` constructor
    - `order` defaults to 1 (standard first-order chain)
    - `smoothing` defaults to 0.0 (no smoothing); supports Laplace (1.0) and Lidstone (any α > 0)
    - Optional `states` list for pre-registering the full alphabet before training
  - Training:
    - `train(sequence)` — sliding window over an array of state strings; accumulates counts across multiple calls
    - `train_string(text)` — convenience wrapper splitting a string into character states
  - Querying:
    - `next_state(current)` — sample one transition via inverse-CDF (categorical) sampling; raises `error()` for unknown contexts
    - `generate(start, length)` — generate an array of exactly `length` states
    - `generate_string(seed, length)` — generate a string of exactly `length` characters
    - `probability(from, to)` — return transition probability; 0.0 for unknown contexts (no error)
    - `stationary_distribution()` — power iteration until |Δ| < 1e-10, up to 10000 iterations
  - Inspection:
    - `states()` — returns a copy of the registered state list in registration order
    - `transition_matrix()` — returns a deep copy of the probability table
  - Order-k support: context keys are null-byte-separated k-grams (`table.concat(k_gram, "\0")`)
  - Topology tracked via `coding-adventures-directed-graph` (self-loops allowed for states that can transition to themselves)
  - `_normalise()` internal helper rebuilds probabilities from raw counts after every `train()` call
  - `_sample(row)` iterates states in registration order for reproducible sampling given a fixed seed
- Comprehensive busted test suite in `tests/test_markov_chain.lua`
  - All 10 spec-required test cases (DT28)
  - Additional edge cases: copy semantics, zero-smoothing, multi-train state growth
- `coding-adventures-markov-chain-0.1.0-1.rockspec` with `coding-adventures-directed-graph` dependency
- `BUILD` script: installs directed_graph dep, installs this package, runs busted tests
- `BUILD_windows` script for Windows CI
- `README.md` with usage examples, API reference, and architecture overview
- Literate programming style: all code commented with explanations, analogies, and worked examples

### Implementation notes

- `generate_string` for order-k chains handles the sliding window entirely within the string domain, avoiding the need for callers to manage `"\0"`-joined context keys
- `stationary_distribution` operates only over order-1 states; for order-k chains the distribution is over the base alphabet (not k-grams), which is consistent with how the public `states()` API is defined
- `math.randomseed(os.time())` is called once at module load; tests that require determinism should call `math.randomseed(42)` before generating
