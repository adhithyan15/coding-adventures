# Changelog

All notable changes to `coding_adventures_markov_chain` (Elixir) will be documented
in this file.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

---

## [0.1.0] — 2026-04-13

### Added

- Initial release of the Elixir Markov Chain package (DT28).

- `CodingAdventures.MarkovChain` struct with fields:
  - `order` — context depth (k-gram length).
  - `smoothing` — Laplace/Lidstone α parameter.
  - `graph` — `CodingAdventures.DirectedGraph.Graph` holding chain topology.
  - `counts` — raw transition count map (accumulated across multiple `train/2` calls).
  - `transitions` — normalised probability map (recomputed after each `train/2`).
  - `all_states` — flat list of known single-element states (the alphabet).

- `new/0-3` — constructor with optional `order`, `smoothing`, and pre-registered
  `states`.

- `train/2` — trains on a list of states. Slides a window of size `order + 1`;
  accumulates counts; re-normalises with Laplace/Lidstone smoothing. May be called
  multiple times; counts accumulate across calls.

- `train_string/2` — convenience wrapper that splits a string into graphemes and
  delegates to `train/2`.

- `next_state/2` — samples one transition from the current context using the
  categorical distribution. Returns `{:ok, state}` or `{:error, reason}` for unknown
  contexts.

- `generate/3` — emits a sequence of `length` states starting from `start`. Supports
  both order-1 and order-k chains. Returns `{:ok, list}` or `{:error, reason}`.

- `generate_string/3` — convenience wrapper for character chains. Takes a seed string
  and returns `{:ok, string}` of exactly `length` characters.

- `probability/3` — returns `T[from][to]` (0.0 if unseen and smoothing = 0).

- `stationary_distribution/1` — power iteration (up to 10,000 steps, convergence
  threshold 1e-10). Returns `{:ok, map}` or `{:error, reason}`.

- `states/1` — returns the flat alphabet list.

- `transition_matrix/1` — returns the full nested probability map.

- Full ExUnit test suite covering all 10 spec test cases (DT28 §Test Cases) plus
  additional edge-case and property tests for >80% coverage.

- Literate source code with `@moduledoc` and `@doc` on every public function.
  Includes diagrams, algorithm descriptions, and worked examples inline.

- `mix.exs` with `excoveralls` for coverage reporting and `coding_adventures_directed_graph`
  as a local path dependency.

- `BUILD` / `BUILD_windows` scripts that first compile the directed_graph dependency
  then run `mix test --cover`.

### Implementation notes

- Order-1 context keys are stored as bare terms (e.g., `"A"`), not as single-element
  lists `["A"]`, to keep the transition map ergonomic and match user expectations.
- Order-k context keys are stored as lists of length `k` (e.g., `["a", "b"]`).
- The `graph` field stores topology for inspection (e.g., checking which states are
  reachable), but the primary probability data lives in `transitions`.
- The `normalise/3` private function applies smoothing and produces a **sparse** map —
  zero-probability entries are omitted to keep the representation compact for large
  alphabets with α = 0.
- Power iteration normalises `π` at each step to correct floating-point drift.
