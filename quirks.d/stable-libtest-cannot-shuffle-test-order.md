---
package: *
lane: rust
rung: E1
status: mitigated
---

# Stable libtest cannot shuffle test order

Rust's built-in test harness has no stable option to run tests in a random
order with a recorded seed, so hidden ordering dependencies between tests stay
hidden. VER00 section 5 requires shuffled order.

**Mitigated:** the pilot's bootstrap harness (VER26 L0) is a custom harness
(`harness = false`) that sorts tests by stable id, shuffles with a seeded
generator and prints the seed.
