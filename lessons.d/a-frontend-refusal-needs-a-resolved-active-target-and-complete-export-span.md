---
category: Security boundaries
---

# A frontend refusal needs a resolved active target and complete export span

The source-equivalence checker initially hashed the Rust tree, then independently
searched `lib.rs` for `pub fn` tokens and treated every resulting parser/lowering
error as a valid refusal. That could select a cfg-disabled item, tokens inside
macro input, a custom Cargo library path, or a truncated function signature; a
hash of the same source tree did not prove the probe had selected the active
export.

The producer and checker now pass the exact bytes from one bounded source-tree
frame to a shared resolver. It parses the capped Cargo manifest, accepts only
the default library target and a closed function-signature grammar, and returns
typed identity failure, frontend acceptance, or actual frontend refusal. Only
the last outcome may be recorded as refusal-only evidence. Add adversarial
tests for target overrides, cfg and macro syntax, nested or const-generic types,
and ensure unsupported identity errors abort manifest generation instead of
being converted into `not-live`.
