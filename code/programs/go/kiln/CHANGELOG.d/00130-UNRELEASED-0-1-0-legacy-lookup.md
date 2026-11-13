## [0.1.0] - UNRELEASED

### Added (KILN04 stage 5a: lookup in `kiln legacy-plan`, shadow only)

- `kiln legacy-plan --ledger <dir> --scope shared|local [--executor-arg <flag>]...`
  adds a per-package lookup result to the plan and a histogram of results:
  `hit` (manifest id and pass record), `miss`, `no-manifest`,
  `ineligible:<reason>` and `untraced-os`. It reports and skips nothing; without
  `--ledger` the plan is unchanged. Schema stays `kiln-legacy-plan/1` with
  additive fields.
- Gating comes first: the never list, KILN03 hard reasons and preconditions, and
  `probe-mismatch`, `probe-timeout` and `probe-refused` make a package
  `ineligible` with no lookup, even when a hit is waiting. A probe refusal is
  gated but not added to a record's `reasons`.
- A miss lists the manifests tried, the strong key looked up and each entry's
  state in this tree. It does not name the first differing entry: manifests hold
  names, not digests, so that is not computable (KILN04 9.1).
- `outside_static_closure`: traced `pkg:` entries not in the package's static
  closure, with a histogram count.
- `kiln explain` prints the lookup result, weak key, manifests tried, hit pass
  record and out-of-closure entries.
- `--ledger` is refused with `--source folder`; the library refuses any source
  that is not a `source.Cleaner`.
- Store failures give `ineligible:manifest-store-unusable` or
  `ineligible:pass-ledger-unusable` and are listed in `lookup.errors`.

### Changed

- `ManifestStore.Load(now, wanted)` takes an optional set of weak keys; other
  records are ignored while parsing (`ManifestReport.Ignored`), so memory follows
  the plan, not the store. `nil` keeps the old behaviour (merge is unchanged).
- The retainer computes each manifest id once and keeps it (`ManifestSet.Retained`
  returns manifests with their ids) instead of re-hashing per comparison.
- KILN04 gains section 9.1 (lookup decisions) and its section 9 wording for a
  miss is corrected to match.
