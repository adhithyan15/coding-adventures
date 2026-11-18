---
category: Testing & coverage
---

# A Warrant Result error constructor must be qualified with its declared error type

A new `v1` model used `Err(ZeroFixedTimestep)`. The repository-wide Warrant compatibility test
rejected the document with `W0202 UnboundName`, because error variants are qualified constructors,
not bare names. The syntax accepted by current examples is `Err(ClockError.ZeroFixedTimestep)`.

When adding a pure model that returns `Result<T, E>`, qualify every error variant with its declared
error type and run `cargo test -p coding_adventures_warrant_grammar --test v0_compat
every_warrant_file_in_the_repository_still_checks_and_the_v0_ones_stay_v0` before proceeding.
