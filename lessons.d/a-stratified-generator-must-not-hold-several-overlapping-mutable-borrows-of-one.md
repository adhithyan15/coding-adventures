---
category: Rust
---

# A stratified generator must not hold several overlapping mutable borrows of one PRNG

A config generator cached several closures, each capturing `&mut SplitMix64`. Rust rejected the
closures because their borrows overlap for the full scope even though the eventual match calls only
one of them. Draw directly in the selected match branch or put each draw sequence in a helper that
borrows the PRNG for only that call.
