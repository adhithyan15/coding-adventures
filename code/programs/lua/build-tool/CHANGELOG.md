# Changelog

## Unreleased

### Added

- A process-free bounded graph and diff-selection core now dynamically
  consumes all eight graph and twelve diff-selection neutral fixtures. It
  provides canonical levels and closures, stable all-or-error diagnostics,
  Unicode-scalar strict-glob matching with a 50,000,000-unit preflight, exact
  BUILD-front handling, and domain-separated repository-boundary reverse
  selection including shared-input multi-consumer fanout.
- Process-free dependency and combined SHA-256 digests plus inert cache-hit,
  miss, and corrupt-recovery decisions now consume all 11 neutral
  `hashing-cache-*` fixtures, including dependency-only changes and shared
  input consumers; native persistence and CLI activation remain separate.
- A process-free portable source selector and package-digest API consumes the
  complete checked 23-language, seven-role source-input registry from generated
  package data. The native Busted suite exercises all seven package-local
  neutral cases, SHA-256 v1 path/content framing, generated and link pruning,
  invalid glob rejection, path aliases, and raw-byte/rename sensitivity.
- Native discovery tests now project the shared language-registry fixture's
  OCaml package and exact Dune `_build` decoy, with direct case-variant and
  near-name coverage.
- A pure bounded toolchain-detection adapter now independently consumes all 11
  language-neutral snapshots, including exact extra-CI declarations,
  platform BUILD precedence, affected and forced scheduling, complete
  canonical flags, stable unsupported diagnostics, and shared resource
  ceilings without host or execution authority.
- A pure orphan-crate snapshot validator now independently consumes all four
  language-neutral fixtures, with artifact filtering, fixed ancestor BUILD
  precedence, redacted portable-path and ledger failures, Unicode full-fold
  duplicate identities, stale exemptions, active pending counts, and stable
  ASCII-canonical diagnostic ordering.
- A pure tracked-artifact snapshot validator now independently consumes the
  five language-neutral fixtures, including redacted portable-path failures,
  Unicode-scalar boundaries and ordering, inert link metadata, and exact,
  case, nested, and Unicode-compatible `node_modules` aliases.
- Generated source-embedded Unicode 17.0.0 normalization, full default-fold,
  and full-uppercase tables keep validation independent of host Lua tables;
  the generator verifies the emitted Lua module against every official vector.

### Fixed

- Diff selection now charges the exact UTF-8 bytes of first-seen unique glob
  declarations against a 4 MiB ceiling before normalization, even when no
  paths changed. Package-root classification uses one reusable changed-path
  index and preflights all slash-prefix materialization against an explicit
  50,000,000-unit ceiling. Boundary digest diagnostics retain precedence over
  that request-dependent preflight, while relevant-input, exact-root,
  descendant-root, and exclusion indexes avoid package-by-rule scans and
  eager input-consumer expansion.
- Strict diff globs now treat the package root as a zero-component relative
  path, so `*` and `?` cannot select it while a whole `**` segment can.
- Glob character-class compilation and folded package-root validation are
  linear in their bounded inputs, and inert snapshots no longer turn package
  limits or malformed edges into generic input errors or default a missing
  `source_globs` field.
- Glob validation no longer retains compiled token graphs for declarations
  that never reach matching; compilation is lazy, one-pattern-at-a-time, and
  reused across a package's candidate paths. Folded-root validation now uses a
  compact separator-aware sorted list instead of per-segment trie tables, and
  ignored record fields no longer trigger unbounded pre-snapshot enumeration.
- Complete glob validation and scalar match-work factors are memoized by
  immutable pattern string across packages, while duplicate detection remains
  package-local and an empty changed-path set skips match preflight entirely.
- Package discovery now classifies the emerging `ocaml` bucket while continuing
  to prune only exact case-sensitive `_build` output; `_Build` and
  `_build-example` remain discoverable source paths.
- The emitted-Lua Unicode verifier now requires an explicit pinned Lua 5.4.7
  executable, ignores Lua initialization and module-path environment state,
  bounds retained child output, and terminates the isolated process tree after
  every exit. Windows starts the verifier suspended inside a kill-on-close Job
  Object; POSIX uses an isolated process group. Timeout, early-root-exit, and
  cleanup-error regressions cover descendant containment, and a Job Object
  setup failure now reaps the still-suspended verifier before propagating.
- Lua `.rockspec` dependency metadata now follows the shared strict UTF-8
  contract. Invalid bytes fail closed with the stable
  `METADATA_INVALID_UTF8` diagnostic and CLI exit code 2 without leaking host
  checkout paths.

## 0.1.0 — 2026-03-23

### Added
- Initial implementation of the Lua build tool.
- Package discovery via recursive BUILD file walk.
- Dependency resolution for all 7 supported languages:
  Python (pyproject.toml), Ruby (.gemspec), Go (go.mod),
  TypeScript (package.json), Rust (Cargo.toml), Elixir (mix.exs),
  Lua (.rockspec).
- Directed graph with Kahn's algorithm for topological sort.
- Sequential build execution with pass/fail tracking.
- Build report with summary statistics.
- Platform-specific BUILD file support (BUILD_mac, BUILD_linux, BUILD_windows).
- CLI with --root, --dry-run, --language, --force options.
