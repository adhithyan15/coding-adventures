---
category: Security boundaries
---

# A rejection an upper crate inherits from a lower crate's guard is not owned by it, so pin it in the release profile in the upper crate's own BUILD

**What went wrong (nearly).** `arithmetic` (adders, ALU) states a domain: every bit is 0 or 1. It checks nothing itself: each bit goes through a `logic-gates` gate, and the gates reject a non-bit. That was true only after the gates' own finding SEC-007 (a `debug_assert!` that compiled away in release). The adders' source never changed, yet their behaviour on `ripple_carry_adder(&[2], &[0])` depended on a one-word edit in another crate. A suite that runs only in the debug profile, and that never names the rule, would have kept passing the day that guard regressed again, and the adder's contract ("a non-bit panics, in every build profile") would have silently become false.

**Fix.** The upper crate states the rule in its own contract (AR01 "Domain": the rejection is inherited) and pins it itself: `tests/bit_domain.rs` hits every operation at every position of every operand and the carry-in, and BUILD runs that file under `cargo test --release` as well as the generated Warrant harness. A regression in the lower guard now fails the upper crate's build, with the upper crate's operation named in the failure.

**Do differently.** When a crate's contract depends on a callee's check, the callee's tests are not the evidence: write the rule into the caller's contract, test it through the caller's public API, and run it in the profile where the callee's check could vanish. "It panics because the gate underneath panics" is a fact about a layer you do not own; record it as inherited, and let the upper layer's build own the pin. The same holds for any layered chain (here gates, then adders, then a CPU step): each layer's evidence names what it inherits.
