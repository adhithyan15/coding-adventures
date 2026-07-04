# fsrs — zero-dependency FSRS-6 scheduler (forward-only)

A from-scratch, **zero-dependency** Rust implementation of the *scheduling* half
of [FSRS-6](https://github.com/open-spaced-repetition/fsrs-rs) — the algorithm
modern Anki uses to decide when you should next review a flashcard.

## Why it exists

The Engram flashcard stack needs FSRS scheduling, but the upstream `fsrs` crate
pulls in the `burn` tensor framework (and dozens of transitive crates) because it
*also* implements parameter **training** via gradient descent. Engram never
trains parameters — it only *schedules* — and the entire scheduling path in the
upstream crate is plain scalar `f32` arithmetic. This crate reimplements exactly
that path with **no third-party dependencies**, so the Engram stack honours the
repository's zero-dependency policy.

## Where it sits in the stack

```
engram-core (scheduler.rs, search.rs)
        │  use fsrs::{FSRS, MemoryState, ItemState, current_retrievability, …}
        ▼
   fsrs (this crate)     ← zero third-party deps
```

`engram-core` is the only consumer. It calls `FSRS::next_states` on every review
to advance a card's `(stability, difficulty)` and compute the next interval, and
`current_retrievability` to rank how "due" cards are for search filters.

## What it does

- `FSRS::new(params)` — build a scheduler from a 21-weight parameter set (or an
  empty slice for the defaults; legacy 17/19-weight sets are upgraded).
- `FSRS::next_states(current, desired_retention, days_elapsed)` — the four
  possible next states, one per answer button (Again/Hard/Good/Easy).
- `FSRS::memory_state_from_sm2(ease, interval, retention)` — back-solve a memory
  state for cards migrated from the older SM-2 algorithm.
- `current_retrievability(state, days_elapsed, decay)` — current recall
  probability for a stored memory state.

## What it does *not* do

Training, the optimizer, tensor/batch code, evaluation metrics, and the
simulation harness — the parts that require `burn`. This is the inference half
only.

## Fidelity

The formulas, constants, clamps, and *operation order* are transcribed faithfully
from upstream `fsrs` 6.6.1. A cross-check test (run against the live upstream
crate before the dependency was dropped) confirmed **5,900+ comparisons** across a
grid of parameters, memory states, elapsed days, and retention targets all match
upstream within a `1e-4` relative tolerance — in practice bit-for-bit, because
the arithmetic is identical. Those ground-truth values are frozen as unit-test
snapshots in `src/lib.rs` so the behaviour stays locked without needing the
third-party crate.

## Usage

```rust
use fsrs::{FSRS, DEFAULT_PARAMETERS};

let scheduler = FSRS::new(&DEFAULT_PARAMETERS).unwrap();

// A brand-new card: no prior memory state.
let states = scheduler.next_states(None, 0.90, 0).unwrap();
println!("If you press Good, next interval = {} days", states.good.interval);

// An existing card reviewed 3 days after the last review.
use fsrs::MemoryState;
let current = MemoryState { stability: 7.0, difficulty: 5.0 };
let states = scheduler.next_states(Some(current), 0.90, 3).unwrap();
println!("New stability after Good = {}", states.good.memory.stability);
```

## Testing

```
cargo test -p fsrs
```
