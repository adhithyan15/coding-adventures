# The number I optimised was not the number that mattered

I measured base64 media encoding at 1.34x peak and raised the wasm media
ceiling 4x on that basis. Security review measured the same change with the
same technique and got a different answer, because I had measured the wrong
quantity: **1.34x is incremental allocation during serialisation.** It excludes
the media retained in state, the reducer's whole-state clone on every command,
and the output buffer's doubling growth. Total live heap at 128 MiB of media is
429–939 MiB depending on shape, not the ~172 MiB my arithmetic implied.

Worse, the guard I described as bounding the damage does not.
`media_budget = min(archive_len * 50, CEILING)` reads like a ratio limit, but
zip padding is free — a 2.56 MiB archive unlocks the full ceiling, so above
that size the ceiling is the *only* bound, and I had just multiplied it by
four. On `wasm32` linear memory never shrinks and `panic = "abort"` means the
`catch_unwind` never runs, so the spike takes the user's unsaved collection.

Reverted. The lesson is not "measure" — I did measure. It is:

- **A ratio is not a bound until you check what the denominator costs the
  attacker.** `* 50` sounds protective; padding bytes are free, so it protects
  only inputs too small to matter.
- **State the quantity, not just the number.** "1.34x peak" was true and
  useless. "1.34x incremental serialisation allocation, excluding retained
  state and the reducer's clone" would have been visibly insufficient for a
  ceiling decision.
- **A memory figure justifying a limit must be total live heap**, because that
  is what the allocator has to satisfy.
