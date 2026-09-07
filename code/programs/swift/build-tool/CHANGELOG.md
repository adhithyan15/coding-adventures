# Changelog

## Unreleased

- Align declared-source glob matching with the language-neutral portable
  grammar and exact 50,000,000-unit match-work budget. Character classes now
  implement Python-compatible negation, ranges, and literal edge characters;
  invalid classes fail before source or diff selection, and dynamic programming
  bounds globstar and segment matching without regex backtracking.
- Adopt the complete checked repository source-input boundary in production
  hashing and diff selection. A generated typed projection covers all 18
  boundaries, 21 registrations, 19 tracked paths, nine neutral cases, and the
  domain-separated digest while pinning the package-local registry it extends.
  Each batch proves applicable paths through bounded NUL-delimited stage-zero
  Git index records with regular modes `100644` or `100755`, rechecks the index
  after hashing, opens exact files through retained no-follow repository
  handles, and unions them with package-local inputs under the existing framed
  hash and byte ceilings. The same projection reverse-indexes boundary-only
  diffs to exact consumers without ambient ancestor, generated-directory,
  credential, secret, symlink, reparse, submodule, conflict-stage, or near-path
  authority.
- Replace the Swift hasher's independent source and metadata maps with a
  generated, source-embedded projection of the complete 23-language neutral
  registry. Production collection now distinguishes all seven selector roles,
  consumes the four package-local neutral fixtures, includes exact hybrid
  companions, resources, scripts, Engram WASM inputs, and root capability
  metadata, rejects unknown languages and portable-path aliases, and bounds
  candidate, result, per-file, and aggregate bytes without accepting a runtime
  registry path. Canonical package paths must contain a package name and match
  the selected language; incremental enumeration applies the candidate ceiling
  before sorting; and tests recompute the domain-separated digest while
  checking every fixture pin. Repository-relative inputs remain a distinct
  exact projection rather than being widened into package-tree authority.
- Bound retained-handle file reads to the exact snapshotted size and probe one
  byte past it without appending, so a concurrent POSIX writer cannot grow a
  source beyond the per-file ceiling before the mutation check.
- Complete portable source hashing with every established lane's primary
  sources and package metadata, the shared 26-component exact pruning registry, OCaml
  coverage, strict declared-source selection, and all 13 neutral collection
  cases. Package digests now use the local pure Swift SHA-256 package and
  hashing-v1 path/content length frames on every platform; no-follow reads
  retain and recheck ancestor/object identities, and failures use one stable,
  root-redacted CLI diagnostic with exit code `2`.
- Add a pure bounded extra-CI toolchain snapshot evaluator that consumes all 11
  language-neutral declaration fixtures. It enforces selected BUILD-front
  precedence, UTF-8 byte and LF-delimited logical-line ceilings, exact CRLF
  grammar, stable deduplication, the complete 16-key registry, and deterministic
  unsupported diagnostics without importing Foundation or gaining host
  authority.
- Make `BUILD_windows` distinguish a missing Swift toolchain from a failing
  native test run. Missing Swift still emits the stable skip diagnostic and
  exits successfully; present-toolchain failures now propagate their exact
  nonzero status.
- Exclude Dune's exact, case-sensitive `_build` generated directory from
  package discovery and source hashing while preserving `_Build` and
  `_build-example`. Shared-fixture and focused Swift tests cover the boundary.
- Add a pure orphan-crate snapshot validator consuming all four shared neutral
  fixtures. It enforces direct and ancestor BUILD coverage, independent empty
  BUILD reporting, exact artifact exclusions, portable and reasoned exemption
  records, NFC full-fold duplicate identities, stale-entry precedence,
  hostile-path redaction, pending counts, and deterministic Unicode-scalar plus
  Python-compatible ASCII JSON diagnostic ordering without filesystem,
  process, environment, credential, link, or network authority.
- Add a pure tracked-artifact snapshot validator consuming all five shared
  neutral fixtures, with hostile-path redaction, exact problem precedence,
  Unicode-scalar length and ordering, inert entry metadata, and exact
  Unicode 17 alias plus Windows-reserved-basename behavior.
- Add generated, source-embedded Unicode 17.0.0 NFC, NFKC, full-fold,
  NFKC-fold, and full-uppercase tables, the Unicode License v3 notice, and a
  required isolated Swift 6.3.3 official-vector CI check that preserves the
  driver-sensitive `swift` and `swiftc` entrypoint names and limits POSIX
  linker discovery to root-owned system tool directories.
- Exclude Cabal's exact, case-sensitive `dist-newstyle` generated directory
  from package discovery and source hashing while preserving near-names such
  as `dist-newstyle-example`. Shared-fixture and focused Swift tests cover the
  boundary.
- Classify package and program buckets with the canonical discovery language
  registry, exclude specification fixture trees, preserve program identities,
  and fail closed on duplicate qualified names with
  `DUPLICATE_PACKAGE_IDENTITY`, sorted repository-relative paths, and CLI exit
  code `2`. Shared registry and duplicate-identity fixtures now cover the API
  and real executable.
- Decode Lua `.rockspec` metadata as strict UTF-8 before dependency parsing.
  Invalid bytes now fail closed with `METADATA_INVALID_UTF8`, stable package
  and repository-relative manifest identity, CLI exit code `2`, and no
  checkout-path disclosure. Resolver and CLI coverage consume the shared
  positive and invalid-byte conformance fixtures and require the exact
  expected edge set.

## 0.1.0

- Added a full Swift port of the monorepo build tool.
- Implemented package discovery, dependency graph resolution, hashing, cache persistence, parallel execution, plan IO, and CI validation.
- Added lightweight Starlark BUILD parsing for the repo's declarative BUILD rules.
- Added focused Swift tests covering discovery, Swift dependency parsing, plan round-tripping, Starlark parsing, and CI validator behavior.
