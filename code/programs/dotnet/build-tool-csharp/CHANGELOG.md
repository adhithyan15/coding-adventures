# Changelog

## Unreleased - source-input boundary sync (rust/arithmetic)

- `code/packages/rust/arithmetic` is no longer a consumer of `starlark-rust-library-rule-consumers`: #19311 (Warrant adoption) rewrote its BUILD from a Starlark `rust_library` target into plain shell commands, so nothing loads `rust_library.star` there any more. The checked repository source-input boundary dropped that root, the projection was regenerated with the repository's generator, and the pinned boundary digest moved from `4569b531...` to `f338935e...`. The boundary has 491 scopes and 505 authorizations (were 492 and 506). No behaviour change in the tool.

## Unreleased - BUILD-001 registry update

- Source-input registry and repository boundary updated for BUILD-001 (generated wasm artifacts are built, not committed): `code/packages/rust/engram-wasm/pkg/engram_engine.wasm` is no longer an exact package input of `rust-engram-wasm-build-inputs`, and `visicalc-html/vendor/spreadsheet-engine-wasm.js` is no longer a cross-package input of the Deno VisiCalc boundary. Registry digest is now `8f3d9ee3...`, boundary digest `ec516870... (combined with the #19050 TypeScript boundary sync)`.

## Unreleased

- Cargo link-table security fixes. Inline values continuing onto later lines
  are joined in linear time: a `TomlBracketScanner` carries the bracket
  depth, the open string's delimiter and a pending escape from line to line
  (so a string left open at a line end still continues after the joining
  space), and the pieces are joined once, instead of rescanning the growing
  value with `+=` for every appended line, which was quadratic. Quoted field
  and sub-table keys (`a = { "path" = "../a" }`, `'package' = "..."`) are
  read as the bare key, compared verbatim and case-sensitively, per the
  amended contract. A Cargo path entry that resolves to the crate itself now
  throws `DependencySelfEdgeException` (`DEPENDENCY_SELF_EDGE`,
  repository-relative manifest; `RunAsync` writes the diagnostic to stderr
  and exits 2) instead of a silently drawn self-loop. The `[package]` `name`
  reader uses `[ \t]*` instead of a multi-line `\s*`, which backtracked
  quadratically over a run of blank lines. Tests: the 11 new one-shape cases
  (57), a 20,000-line continuation, 100,000 blank lines before an
  unterminated and a real name, self-edges by alias, by name and from a
  target table, and a front-door exit-2 test.
- Rust resolution now reads path dependencies from `[build-dependencies]`,
  `[build_dependencies]` and every `[target.<cfg>.…]` form of the three link
  tables, not just the top-level `[dependencies]`, and reads the
  `[dependencies.<name>]` sub-table form. A change to a crate used only by a
  build script or on one target now rebuilds its dependents. This implements
  the Rust line procedure of `code/specs/build-tool-conformance.md`
  (`Resolver.CargoLinkDependencyNames`): headers are split into dotted-key
  segments outside strings (quoted cfgs stay one segment, table names compare
  case-sensitively, `[[...]]` and malformed headers end the table); comments
  are stripped outside strings; inline values whose arrays continue onto
  later lines are joined; `path` and `package` field keys must be exact and
  hold one non-empty string; names are ASCII-lowercased and deduplicated.
  Dev-dependencies (every form), `[patch]`, `[replace]`, `[workspace]` and
  registry entries add no edge.
- The old reader counted any `[dependencies]` line mentioning `path` and
  looked up the entry key, so `x = { version = "1", features = ["path"] }`
  drew an edge and a `package` rename was ignored
  (`os-kernel = { package = "os-kernel-simulator", ... }` pointed at
  `rust/os-kernel`). Both are fixed. On the real repository the forced plan
  gains 121 edges and loses that one; its edges into Rust and WASM packages
  then equal the Go tool's (bar one TypeScript edge from Go's cross-language
  resolution).
- Tests: the 46 one-shape cases of the Go reference
  `TestCargoLinkDependencyNames` as a theory, duplicate collapse, and
  build/target/renamed sub-table edges against dev and patch tables. The
  reader returns the same dependency names as the Go reader for all 1,650
  tracked `Cargo.toml` files.

- Added a process-free typed CI-gate registry evaluator with deterministic
  all-gate verdicts, nullable-snapshot fail-open behavior, portable glob
  selection, shared-pattern Unicode-scalar 50-million-unit preflight, and
  no-partial-output errors. The C# suite replays all eleven neutral cases.
- Registered the C# and F# native fixture readers with the Go build-plan
  selector so fixture-only changes schedule both .NET package tests.

- Expanded dynamic source-selection and package-hash fixture replay from 16/4
  to 20/8 cases, covering before/after shared-input digest evidence for two
  Swift consumers. The production engine and host authority are unchanged.
- Rejected caller-owned diff package lists over 4,096 entries before
  enumeration or allocation, preserving the graph-limit diagnostic.
- Matched the reference's granular graph/diff diagnostics and validation
  precedence, rejected diff edge cycles and nested package roots before
  selection, and counted only consumer roots against the boundary scope limit.
- Added a process-free typed graph and diff-selection core with deterministic
  prerequisite-first levels, no-partial cycle and diff failures, Unicode 17
  path identity, exact repository-boundary digest/projection, strict portable
  globs, and the 50,000,000 Unicode-scalar match-work preflight. The C# suite
  pins and evaluates all eight graph and twelve diff fixtures plus negative
  structural, precedence, and glob cases; this does not claim a neutral CLI
  adapter or broader build-tool parity.

- Added the `forme-plugin-installer-core`, `forme-plugin-runner-conformance`, and `forme-plugin-runner-ts`
  TypeScript roots to the repository source-input boundary projection. Digest updated to
  `3370c811b51962c87757b01e98ad3db5206cad2c75e74c6853bf5209548bd18e`.

- Regenerated both source-input projections for exact lowercase `blib`, the
  current Lua and Forme boundary consumers, and the new canonical registry and
  boundary digests shared by the C# engine and F# facade.

- Retained a hard bound on Git-index snapshot capture while raising its timeout
  from 15 to 60 seconds, preventing valid large-checkout enumeration from
  failing on contended macOS CI runners.

- Regenerated the repository source-input boundary projection against the
  neutral fixture: removed the ten Haskell roots that now carry their own
  `cabal.project`, added the `lua/der_asn1` and `lua/x509_extension` lint
  roots, and picked up the
  `forme-deploy-runner-fs-adapter`, `forme-deploy-runner-github-pages-adapter`,
  and `forme-plugin-host` TypeScript roots. Digest updated to
  `7983f42a84dc9905f50729798a5d7d4000a4356016eb2b9cfd42217b15070b59`.

- Extended the generated source-input projection and xUnit coverage with the
  two exact TypeScript site roots and their 13 authored inputs. Discovery keeps
  legacy `unknown/*` graph identities while live hashing resolves those exact
  TypeScript profiles and rejects unregistered or same-shaped foreign roots.

- Adopted the complete typed language and repository-boundary source-input
  registries, including deterministic generation, canonical digest checks,
  direct Starlark declared sources, exact pruning, and reverse boundary-diff
  selection.
- Replaced ambiguous source concatenation with repository-relative Hashing-v1
  path/content frames and exact raw-byte SHA-256 package digests while leaving
  dependency hashing to its separately owned contract.
- Added bounded native no-follow source reads, hardlink/non-regular rejection,
  constant-descriptor file and directory identity rechecks, scrubbed and
  bounded Git-index capture, complete mode/OID/stage/path stability comparison,
  redacted failures, all 13 source fixtures, all three package-digest fixtures,
  and live security tests. Removed unused public discovery helpers that bypassed
  the secure reader and made .NET changes select the native Windows CI leg.
- Consumed the complete shared discovery-language registry, restricted language
  inference to the exact bucket immediately below `packages` or `programs`,
  classified Mosaic and Twig, ignored BUILD roots outside those containers,
  and pruned exact case-sensitive `_build` and `dist-newstyle` components.
- Added direct shared-fixture and exact/case/near-name discovery regressions.
- Consumed the language-neutral extra-CI-toolchain declaration contract with a
  bounded process-free snapshot evaluator, exact selected-front parsing,
  canonical `cpp` and `ocaml` keys, C/C++ language normalization, stable
  unsupported-language diagnostics, and production affected-package scheduling.
- Added direct xUnit coverage for all ten neutral toolchain fixtures, real
  platform-front discovery, canonical language mappings, and per-file plus
  aggregate resource ceilings, including strict CRLF acceptance and lone-CR
  rejection.
- Kept forced-full neutral snapshots strict for unsupported selected languages
  while production forced-full scheduling provisions the complete registry
  without misclassifying repository-only special/fixture buckets; affected
  Starlark BUILD packages use the existing Go bootstrap.
- Added process-free orphan-crate snapshot validation with exact artifact
  exclusion, direct and ancestor BUILD coverage, invalid and stale exemption
  diagnostics, pending-debt accounting, hostile-path redaction, and direct
  coverage of all four shared language-neutral fixtures.
- Pinned tracked-artifact NFC, NFKC, full default folding, and root full
  uppercase to generated source-embedded Unicode 17.0.0 tables, with explicit
  snapshot-version validation and version-delta fixture coverage. Build,
  publish, and package outputs include the Unicode License v3 notice and
  declare the mixed MIT and Unicode-3.0 licensing.
- Added the first C# implementation of the monorepo build tool.
- Implemented package discovery, dependency resolution, git-diff change
  detection, cache fallback, plan emission, CI toolchain detection, and
  parallel execution.
- Added xUnit coverage for discovery, resolver, hashing, cache, executor, and
  plan behavior.
- Added pure tracked-artifact snapshot validation with portable-path safety,
  redacted invalid-path diagnostics, Unicode-aware `node_modules` alias
  detection, Unicode-scalar length and ordering, full-uppercase Windows
  reserved-basename matching, and direct coverage of all five shared
  language-neutral conformance fixtures.
- Fixed trailing slash and backslash paths to report redacted `EMPTY_SEGMENT`
  diagnostics after separator normalization.
