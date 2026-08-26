# Changelog

All notable changes to this package will be documented in this file.

## [Unreleased]

### Added

- A pure orphan-crate snapshot validator consuming all four shared neutral
  cases, with exact artifact filtering, direct and ancestor BUILD coverage,
  empty-BUILD diagnostics, reasoned exemption validation and stale cleanup,
  active PENDING counts, hostile-path redaction, pinned Unicode 17 duplicate
  identities and reserved basenames, and canonical deterministic diagnostics.
- A pure tracked-artifact snapshot validator consuming all five shared neutral
  cases, with hostile-path redaction, scalar-bounded and scalar-ordered
  diagnostics, exact problem precedence, inert entry metadata, and pinned
  Unicode 17 alias and Windows-reserved-basename handling.
- Generated, source-embedded Unicode 17.0.0 NFC, NFKC, full-fold, NFKC-fold,
  and full-uppercase tables plus the Unicode License v3 notice and a required
  isolated GHC 9.4.8 official-vector CI check.
- Shared valid and invalid Lua rockspec fixture coverage, typed
  `METADATA_INVALID_UTF8` diagnostics, representative malformed-sequence
  checks, literal U+FFFD coverage, and front-door exit-code validation.
- An exact package-hash regression covering non-ASCII UTF-8, NUL, and malformed
  source bytes against a fixed Git blob digest.
- Shared Lua resolution fixtures for authoritative dependency tables, genuine
  cycles, selected BUILD dependency metadata, qualified program identities,
  and package/program alias precedence.
- Shared Cabal resolution coverage for inline and multiline `build-depends`
  fields plus misleading package names in metadata, options, and comments.
- Haskell consumption of the shared Python dependency diamond plus a PEP 621
  field-boundary and distribution-name normalization fixture.
- Shared Rust field-boundary coverage for top-level Cargo path dependencies,
  package renames, and representative non-authoritative tables.
- Shared Ruby field-boundary coverage for runtime dependency synonyms,
  declared gem aliases, quote forms, optional parentheses, and representative
  non-authoritative fields and calls.
- Shared Perl field-boundary coverage for top-level runtime requirements,
  module and distribution aliases, phase-block exclusions, quote forms,
  optional versions, and non-authoritative `Makefile.PL` fields.
- Shared Swift field-boundary coverage for local package path declarations,
  directory aliases, comments, external URLs, and unrelated package, product,
  and target metadata.
- Shared Go field-boundary coverage for single and block `require` directives,
  indirect requirements, comments, and replace-only local module aliases.
- Shared Elixir field-boundary coverage for direct, block, and shorthand
  dependency lists, multiline local path tuples, comments, project metadata,
  lockfiles, and non-path dependencies.
- First-class Dart discovery plus shared `pubspec.yaml` field-boundary coverage
  for root dependency maps, declared package aliases, nested source options,
  overrides, comments, and unrelated metadata.
- Shared Java and Kotlin Gradle field-boundary coverage for direct and nested
  relative composite-build paths, comments, strings, absolute paths,
  build-script coordinates, cross-lane targets, and unknown targets.
- Shared C#, F#, and cross-language .NET field-boundary coverage for exact root
  project paths, portable separators, XML decoys, MSBuild dynamic paths,
  nested projects, and unknown targets.
- Shared TypeScript field-boundary coverage for root runtime and development
  dependency objects, exact top-level package-name aliases, single-line
  tables, and representative peer, optional, script, nested, and name decoys.

### Changed

- Suppress ambiguous multi-Cabal metadata and duplicate Dart manifest-name
  aliases without dropping their discovered package nodes or Haskell directory
  aliases, matching the shared resolver contract under forced builds.
- Read rockspecs as raw bytes and decode strict UTF-8 before dependency
  resolution, with repository-relative diagnostics and no checkout-root leak.
- Force existing lazy text reads to EOF before parsing so Windows file handles
  close deterministically after discovery and validation.
- Hash source and manifest contents as raw bytes, normalize relative paths to
  portable UTF-8, and feed `git hash-object` through binary pipes instead of
  locale-sensitive text handles.
- Exercise the Haskell build tool from `BUILD_windows` whenever Cabal is
  available instead of unconditionally skipping the package.
- Resolve Elixir dependencies only from local `path:` tuples in authoritative
  `deps:` lists instead of tokenizing the complete `mix.exs` and `mix.lock`.
- Discover Dart packages and programs, hash `pubspec.yaml` and `.dart` inputs,
  register exact root `name:` aliases, and resolve only direct keys under root
  `dependencies:` and `dev_dependencies:` maps.
- Resolve Java and Kotlin edges only from comment-aware `includeBuild("...")`
  calls in root `settings.gradle.kts`, matching normalized relative targets to
  exact same-lane package roots without following or reading those paths, and
  hash Java/Kotlin source plus Gradle settings/build inputs.
- Resolve C#, F#, and shared .NET edges only from literal `Include` attributes
  on unqualified `ProjectReference` elements in root project files, matching
  lexically normalized paths to exact project aliases across the shared .NET
  scope without opening referenced files.
- Parse TypeScript `package.json` with Aeson, register only its exact root
  top-level `name`, and resolve only direct keys of root `dependencies` and
  `devDependencies` objects instead of tokenizing the complete manifest.
- Resolve Lua edges only from quoted values in the rockspec `dependencies`
  table, merge qualified `# build-tool: deps=` entries from the selected BUILD
  file, preserve program identity segments, and prefer package aliases over
  same-basename program aliases.
- Resolve Haskell package edges only from Cabal `build-depends` fields across
  stanzas, ignoring comments and every non-authoritative manifest field.
- Resolve Python edges only from PEP 621 `[project].dependencies`, with PEP 503
  case and separator normalization before internal-package lookup.
- Resolve Rust edges only from inline path dependencies in Cargo's top-level
  `[dependencies]` table, honoring `package` renames before alias lookup.
- Resolve Ruby edges only from `add_dependency` and
  `add_runtime_dependency` calls on the gem specification receiver, ignoring
  development dependencies, metadata, comments, and unrelated text.
- Resolve Perl edges only from top-level root `cpanfile` `requires`
  declarations, ignoring test and other phase blocks plus `Makefile.PL`
  dependency tables, while registering exact declared module names and
  current and legacy distribution aliases.
- Resolve Swift edges only from relative `.package(path: "...")`
  declarations, using the final directory component and ignoring line and
  nested block comments plus unrelated Swift strings and initializer fields.
- Make the Go resolver apply the same comment-aware Swift manifest boundary
  instead of accepting a `.package(path:)` example inside a block comment.
- Resolve Go edges only from single-line `require` directives and `require`
  blocks, ignoring module metadata, comments, and `replace`, `exclude`, and
  `retract` directives.
- Declare aes-modes' now-authoritative local AES prerequisite in its portable
  BUILD recipe so standalone build validation remains correct.

## [0.1.0] - 2026-04-05

### Added

- Initial package scaffolding generated by scaffold-generator
