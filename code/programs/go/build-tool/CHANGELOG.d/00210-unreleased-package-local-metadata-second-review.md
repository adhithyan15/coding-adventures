## Unreleased — package-local metadata second review

- **An empty diff is "no answer" again.** `parseNameStatus` returned a
  non-nil empty slice for an empty diff. `run()` tests the length and fell
  back to hashing, but `selectPackageRoots` tests for nil, so `-emit-plan`
  wrote "build nothing" (`[]`) into every platform override while the
  top-level set said "build everything" (`null`); `plan.go` applies the
  override, so build jobs would have built nothing. Empty output now gives
  nil, nil, and the overrides are `null` again, as on main.
- **An undecodable diff falls back as a whole.** A truncated record or an
  empty path used to return the paths read so far and drop the rest. It now
  returns nil, nil, so the caller takes the same fallback as when Git fails.
- **The metadata-execution check is described as what it is.**
  `ValidateNoExecutedMetadata` matches literal paths in fixed forms, so a
  shell variable, pipe, wrapper command, `concat!`/`env!` path,
  `cfg_attr(path)` or `build.rs` copy gets past it. The spec and the
  validator now call it a best-effort tripwire, not a boundary; the type
  restriction is what keeps scripts and sources under `evidence/` and
  `CHANGELOG.d/` propagating.
