## [0.1.0] - UNRELEASED

### Added

- The traced legacy keys (KILN04 stage 1b-ii: sections 3, 4.2, 5.2 and 11), in
  `internal/key/traced.go` and `internal/legacy/traced.go`:
  - `key`: the four `path:` state digests (absent, directory, file, link), the
    missing-package digest, the listing digest, `T(q)`, `CheckEntryName` (the
    `pkg:`, `path:`, `list:` allowlist; `weak:` and anything else refused) and
    `StrongConsumed`.
  - `legacy`: `Traced`, with `WeakKey` (semantics version 2: `platform`, `host`,
    `action` = `rel_path` and `test`, the BUILD recipe, empty `inputs` and
    `reads`, `consumed` = `tool:build-tool`, `policy:env.json`,
    `policy:host-roots.json` and `policy:git`, `tools` of the static closure, and
    the scrubbed `env` plus `kiln:root`, `kiln:recorder` and
    `kiln:executor-args`), `StrongKey`, and the memoised digest provider
    `CurrentDigests`, which reads only the tracked tree through `Source`.
  - `ScrubbedEnv` and `EnvironOf`: the single source of the traced run's
    environment, so what the recorder runs with is exactly what is keyed.
  - A package that cannot be traced gets a `Refusal` (`host-unknown`,
    `unknown-toolchain:<name>`, `no-build-file:<os>`, `symlink:<file>`).
- Golden vectors: a weak key, a strong key, each state digest, the missing-package
  digest, listing digests, `T(q)` with and without nested packages, the strong
  key's consumed section, and refused manifest entries (the `weak:` prefix among
  them). The vector file gains `digests`, `strong_consumed` and `refused_entries`
  arrays; `kiln verify-vectors` checks them and reports their counts.

### Changed

- `legacy.buildRecipe` is the one place the selected BUILD becomes a recipe; the
  static key and the weak key both use it. No static key changes.
- KILN04 gains section 11.1 (key decisions) and KILN02 section 6 describes the
  new vector kinds.
