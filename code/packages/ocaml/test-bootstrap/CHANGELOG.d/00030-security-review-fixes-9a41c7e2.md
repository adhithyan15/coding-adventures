## 0.1.0 (security review)

- Budget ambient scope: a scope restores the previous ambient value only if it is still current,
  and `Budget.current ()` skips closed (spent) nodes, so interleaved scopes in one domain (systhreads,
  effect fibres) can no longer poison later top-level checks. Documented as per domain; pass `?parent`
  for concurrent checks in one domain. Regression test interleaves two fibre scopes.
- `Gen.frequency`: each weight is capped before summing, so `[max_int; max_int; 3]` no longer wraps a
  63-bit int past the 2^32 cap.
- `Tfv`: duplicate field and map-key detection uses a hash table (was quadratic).
- `Safe.recoverable`: `Fun.Finally_raised` wrapping `Sys.Break` or `Out_of_memory` is unwrapped and
  re-raised, never a panic verdict.
- Capability manifest declares `thread:spawn` (tests call `Domain.spawn`); tests no longer print.
- Documented: generation is bounded per level, not in total; `repro-fnv` is a non-secret fingerprint.
