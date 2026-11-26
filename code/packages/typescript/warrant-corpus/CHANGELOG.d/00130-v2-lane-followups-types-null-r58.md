## Unreleased - v2 runner: `types: null` refused, R58 reserved built-in names (VER04r 2.2, 16.16 R58)

- **`types: null` in `v2-value`.** `runCase` read `c["types"] ?? {}`, so a null
  `types` was taken for an absent one. Only an absent `types` means `{}` now; a
  null, `[]`, string, number, `false` or a list holding a row is refused as
  `type`. The other contexts already refused them (`ill-typed` in `v2-ir` and
  `v2-fault`, `descriptor` in `v2-domain`, `corpus` in `v2-chunk` and the corpus
  reader); tests now pin every value in every context.
- **R58.** `readTypeTable` refuses a type row (record, variant or error type)
  or a constructor named for a reserved built-in, with the caller's code: the
  grammar checker's `V0_BUILTINS` (`len`, `at`, `concat`, `fixed`, `div`,
  `Some`, `None`, `Ok`, `Err` and the type names) plus every primitive type name,
  the v1 widths included. `len`, `at`, `concat`, `fixed`, `div`, `Some`, `None`,
  `Ok` and `Err` were accepted before; the primitive names and `Bytes`, `List`,
  `Option`, `Result` as type rows were already refused. Field names are not
  covered by R58 and stay free.
- No committed kit case changes outcome: 782 run, 176 skipped, 0 failed.
  Runner source changed, so `ct-compare`'s `test-manifest.json` and
  `domain-labels.json` were regenerated.
