---
category: Rust
---

# A guard written as debug_assert is no contract: a release build accepts the garbage, and the gates that still rejected it did so by luck

**What went wrong.** `logic-gates` modelled a wire as a `u8` that must be 0 or 1, and guarded every operation with `validate_bit`, a `debug_assert!`. Tests run in debug, so everything passed. In a release build the guard vanished: `tri_state(2, 1)` returned `Some(2)`, `priority_encoder` read a 2 as an inactive input, and `encoder` counted active inputs in a `u8`, so 257 of them wrapped to 1 and passed the one-hot check. The gates, `mux` and `decoder` still panicked in release, but only because the transistor model underneath happened to reject the value: three gates had no guard of their own. Nothing in the debug-only suite could see any of it.

**Fix.** `assert!` instead of `debug_assert!` for a guard that defines the input domain, one check per input; count in a type that cannot wrap (`usize`); every gate validates its own inputs. Add a test that runs under `cargo test --release` and fails there before the fix (`tests/bit_domain.rs`), and run the generated Warrant harness in release too (BUILD).

**Do differently.** `debug_assert!` is for invariants the code itself establishes, never for a precondition the caller can violate. If a function states a domain, find out what it does outside it in *both* profiles (a rejection that exists only because a callee happens to reject is not a contract), and put a release-profile run in BUILD so the two cannot drift. A sum of 0/1 values in a narrow integer is a wrap-around waiting for a long enough slice.
