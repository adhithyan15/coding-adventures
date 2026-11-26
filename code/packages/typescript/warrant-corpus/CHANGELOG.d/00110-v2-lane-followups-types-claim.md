## Unreleased - v2 runner: three leniencies closed (CV2-S7a follow-up)

A dual-lane review found inputs this lane accepted that VER04r says every
consumer refuses. Each is now refused, with a test per case.

- **R1.** `runDomain` (`src/v2/kit.ts`) now reads the case's optional `types`
  field (R44). A present table is validated by `readTypeTable` and the binder
  type's names are checked against it; an `errs` list that differs from the
  declared cases, order included (R72), or an invalid table is `descriptor`.
  Absent still means `{}`.
- **R2.** `readTypeTable` (`src/v2/types.ts`) enforces section 2.2 rules 3 and
  4: a record field is a `[name, type]` pair with a name in the name grammar, a
  constructor is `[name, [types]]` with a name in the grammar and at most 8
  fields. The refusal carries the caller's code: `type` (v2-value), `ill-typed`
  (v2-ir), `descriptor` (v2-domain), `corpus` (v2-chunk, a corpus).
- **R5.** The Windows device-name check on a claim (section 5.5) moves to
  `checkClaimDevice` in `src/v2/chunks.ts`, called by `loadSuite` as well as by
  `chunkFileName`, so a suite row loaded outside the corpus reader is `corpus`.
- Runner source changed, so `runner.src_sha256` moves (see `ct-compare`).
