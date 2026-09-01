# coding-adventures-build-tool (Swift)

An incremental, parallel monorepo build tool implemented in Swift.

## What it does

This port mirrors the other build-tool implementations in the repo:

1. Discovers packages by recursively walking `BUILD` files under `code/`
2. Evaluates simple Starlark-style BUILD targets used in this monorepo
3. Resolves internal dependencies across the repository's implementation
   languages
4. Detects changed packages from `git diff`
5. Hashes package sources and dependency state for cache fallback
6. Builds independent packages in parallel by dependency level
7. Emits and consumes JSON build plans for CI
8. Validates the CI full-build toolchain contract
9. Validates bounded orphan-crate coverage snapshots and exemption records
10. Evaluates bounded extra-CI toolchain declarations from inert snapshots

## Extra CI toolchain declarations

`ToolchainDetection.evaluateSnapshot` accepts caller-supplied package names,
languages, BUILD-front strings, the explicit target platform, scheduling state,
and forced toolchains. It returns a fresh complete result over the canonical
sorted 16-key registry. Platform-specific fronts win by presence, including an
empty override, and only selected packages contribute their language or exact
`# needs-toolchain: NAME` declarations.

The adapter meters every supplied front before selection: each string is
limited to 65,536 UTF-8 bytes and 4,096 LF-delimited logical lines, with a 1 MiB
aggregate ceiling. It strips only a CR that directly precedes an LF terminator,
trims only ASCII space and tab, and keeps malformed or unknown declaration
lookalikes inert. It imports no Foundation and cannot enumerate files, inspect
Git or environment state, launch processes, consult clocks or randomness, read
credentials, or access the network. The Swift Testing suite dynamically
discovers and evaluates all 11 language-neutral toolchain snapshots plus direct
boundary, precedence, alias, freshness, and error-ordering cases.

## Orphan-crate validation

`Validator.validateOrphanCrateSnapshot` consumes caller-supplied directory,
Cargo manifest, BUILD-file, and exemption metadata. It requires every source
manifest to have a direct or ancestor runnable BUILD or one active, reasoned
`EXCLUDED`/`PENDING` entry. Empty BUILDs remain visible diagnostics, and a
nearer empty BUILD never masks a runnable ancestor.

The adapter uses the exact case-sensitive artifact registry, validates and
deduplicates portable exemption paths with the pinned Unicode 17 tables,
reports stale exemptions, redacts hostile paths to `code/BUILD-EXEMPTIONS`,
and sorts diagnostics with Unicode-scalar paths plus Python-compatible ASCII
JSON details. It is deliberately process-free: snapshot validation does not
enumerate files, follow links, invoke Git, read environment state or
credentials, launch processes, or access the network. All four shared neutral
orphan-validation fixtures and focused adversarial boundary tests are required
by the Swift package test suite.

## Tracked-artifact validation

`Validator.validateTrackedArtifactSnapshot` consumes an already bounded,
inert list of tracked repository paths. It rejects non-portable paths with a
fixed redacted diagnostic and detects exact, nested, separator-normalized,
case, and Unicode-compatibility aliases of `node_modules`. Regular files,
symlinks, and reparse points remain inert metadata: this pure adapter does not
enumerate a checkout, follow a link, invoke Git, read a path or environment
variable, launch a process, or access the network.

Generated source-embedded Unicode 17.0.0 tables provide exact NFC, NFKC,
full-case-fold, and full-uppercase behavior independently of the host OS and
Swift runtime. All five language-neutral tracked-artifact fixtures plus the
official Unicode normalization, folding, and uppercase vectors are checked in
tests and required CI.

## Metadata safety

Lua `.rockspec` files are decoded as strict UTF-8 before dependency parsing.
Invalid bytes stop resolution, return CLI exit code `2`, and emit a stable
diagnostic with package and repository-relative manifest identity:

```text
METADATA_INVALID_UTF8: package=lua/pkg manifest=code/packages/lua/pkg/coding-adventures-pkg-0.1.0-1.rockspec encoding=UTF-8
```

The resolver tests consume the language-neutral `resolution/lua-utf8` and
`resolution/lua-invalid-utf8` fixtures, require the exact success edge set,
and verify that diagnostics never expose the checkout root.

## Discovery identity safety

Package discovery classifies the exact bucket immediately below `packages` or
`programs` using the canonical language registry. It covers all established
and emerging implementation lanes, the WASM target, Mosaic and Twig domain
languages, Starlark, and the shared .NET program host. Specification fixture
trees are excluded, programs retain a `/programs/` identity segment, and an
unrecognized bucket remains `unknown` instead of borrowing a language name
from a later path component.

If distinct directories still collapse to one qualified name, discovery stops
with CLI exit code `2` and a stable diagnostic containing sorted repository-
relative paths:

```text
DUPLICATE_PACKAGE_IDENTITY: package=unknown/demo paths=code/packages/alpha/demo,code/packages/beta/demo
```

The shared `discovery/language-registry` and
`discovery/duplicate-identity` fixtures cover this behavior through the Swift
API and the real executable without disclosing the checkout root.

Discovery also excludes Cabal's exact, case-sensitive `dist-newstyle` and
Dune's exact, case-sensitive `_build` generated-directory components. Near and
case-variant names such as `dist-newstyle-example`, `_build-example`, and
`_Build` remain discoverable. The shared language-registry fixture and focused
Swift coverage enforce those boundaries. Source hashing applies its complete
related registry separately because discovery-only directories are still
eligible package source.

## Portable source hashing

Source collection has its own exact generated-output boundary: `.git`, `.hg`,
`.svn`, `.venv`, `.tox`, `.mypy_cache`, `.pytest_cache`, `.ruff_cache`,
`.stack-work`, `__pycache__`, `node_modules`, `vendor`, `dist`,
`dist-newstyle`, `_build`, `build`, `target`, `.claude`, `Pods`, `.gradle`,
`.dart_tool`, `gradle-build`, `deps`, `.build`, `.cargo`, and `cover`. These
names are case-sensitive whole path components. Near names and the
discovery-only `specs` directory remain hashable source.

The production collector uses a generated, source-embedded projection of the
complete checked 23-language registry. It does not read a fixture, environment
variable, or command-line registry path at runtime. Tests decode the canonical
JSON and compare its complete raw object with the production serialization, so
missing selectors, undeclared extras, scoped-rule metadata, and ownership drift
all fail together.

Extension mode resolves recursive sources and exact metadata plus bounded
native-companion and resource scopes. Declared mode instead applies strict
portable `srcs` globs. Both modes retain the five universal BUILD fronts,
root-only `required_capabilities.json`, lane root metadata and variable
manifests, fixed relative inputs, and exact package-specific inputs. This
includes SwiftPM C-family targets below `Sources`, reviewed Rust companions,
resources and scripts, and only the three exact Engram WASM BUILD inputs for
the canonical Engram package root. Unknown languages, non-portable paths,
NFC/full-casefold aliases, oversized inputs, and selector widening fail closed.

Package collection deliberately remains below the package root. The separate
repository-relative boundary registry, tracked-file proof, and reverse diff
index have their own parity owner; they are not approximated by traversing
ancestors or unpruning generated directories here.

Package hashing sorts portable repository-relative paths by UTF-8 bytes and
feeds this byte stream to the repository-local pure Swift SHA-256 package for
every file:

```text
uint64_be(path_utf8_length) || path_utf8 ||
uint64_be(raw_file_length)  || raw_file_bytes
```

Absolute checkout paths, timestamps, ownership, locale, and directory order
never enter the digest. Every read walks down from the trusted repository root
through retained no-follow handles; symbolic links, Windows reparse points,
ancestor identity changes, and file identity/size/timestamp changes fail
closed. Missing, unreadable, or unstable inputs produce CLI exit `2` and one
quoted, root-redacted diagnostic whose control and format characters are
escaped:

```text
HASH_PACKAGE_FAILED: package="swift/example"
```

The test suite consumes both neutral source-collection fixtures and the shared
hashing-v1 missing-cache oracle, then independently covers binary bytes, CRLF,
Unicode path ordering, same-content renames, empty packages, every established
language registry entry, declared root manifests, nested links, real Windows
junctions (including an ancestor-junction plan path), and fresh executable
failure paths.

## Usage

```bash
# Auto-detect the repo root
swift run build-tool

# Dry-run only the affected packages
swift run build-tool --dry-run

# Rebuild everything
swift run build-tool --force

# Limit parallel jobs
swift run build-tool --jobs 4

# Only consider Swift packages
swift run build-tool --language swift

# Emit a CI build plan
swift run build-tool --emit-plan build-plan.json
```

## Development

The Windows build front checks for Swift before launching the package suite.
An absent toolchain emits the stable skip message and exits successfully. Once
Swift is present, `swift test` owns the front's exit status, so native failures
cannot be relabeled as toolchain absence. Package tests execute the checked-in
front with controlled success, failure, and absent-toolchain fixtures.

```bash
cd code/programs/swift/build-tool
swift test
swift run build-tool --help
```
