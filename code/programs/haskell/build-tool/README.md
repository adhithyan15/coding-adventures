# build-tool

Haskell implementation of the monorepo build tool.

## What it does

This version discovers packages by walking `code/`, resolves internal
dependencies from package manifests, hashes package inputs for incremental
builds, uses git diff information to narrow the build set, and executes
`BUILD` scripts in dependency order.

Tracked-artifact validation is a pure adapter over caller-supplied snapshots.
It rejects unsafe repository paths and every exact, case, or Unicode
compatibility alias of a `node_modules` component. Invalid paths are redacted
to `repository`; safe forbidden paths retain slash-normalized spelling, and
diagnostics use Unicode-scalar ordering. The implementation embeds the exact
Unicode 17.0.0 NFC, NFKC, full-fold, NFKC-fold, and full-uppercase tables, so
results do not depend on GHC's host Unicode version. All five shared neutral
fixtures and every official Unicode normalization and casing vector are
exercised with an isolated, explicitly pinned GHC 9.4.8/runghc pair.

Orphan-crate validation is a second pure adapter over one caller-supplied
Cargo/BUILD/exemption snapshot. It filters the exact artifact-component
registry, finds direct or component-wise ancestor coverage using the fixed
BUILD filename rank, reports the closest empty BUILD only when no runnable
ancestor exists, and validates reasoned `EXCLUDED` and countable `PENDING`
ledger entries. Portable paths use embedded Unicode 17 NFC, full folding for
duplicate identities, and full uppercase for Windows reserved basenames.
Unsafe exemptions are replaced by a fixed `code/BUILD-EXEMPTIONS` diagnostic;
no hostile path is retained. All four shared neutral fixtures plus adversarial
scalar, reason, precedence, and canonical-order cases run without filesystem,
Git, process, environment, network, credential, or link authority.

Lua rockspec metadata is read as raw bytes and decoded as strict UTF-8 before
dependency resolution. Malformed bytes fail closed with package and
repository-relative manifest identity:

```text
METADATA_INVALID_UTF8: package=lua/pkg manifest=code/packages/lua/pkg/coding-adventures-pkg-0.1.0-1.rockspec encoding=UTF-8
```

The standalone front door writes that diagnostic to stderr and exits `2`.
The package tests consume the shared `resolution/lua-utf8` and
`resolution/lua-invalid-utf8` fixtures, cover representative malformed UTF-8
classes, and distinguish a valid literal U+FFFD from a decoder replacement.

Lua dependency resolution reads quoted package requirements only from the
authoritative `dependencies = { ... }` table. Package declarations,
descriptions, source metadata, comments, and unrelated strings cannot invent
graph edges. Selected `# build-tool: deps=` metadata is merged separately,
program identities retain the `language/programs/name` segment, and a package
alias wins over a same-basename program alias. Shared fixtures cover the
field-aware boundary, genuine cycles, legacy BUILD dependencies, and
package/program alias collisions.

Go dependency resolution reads module paths only from single-line `require`
directives and `require ( ... )` blocks. Module declarations, Go and toolchain
versions, comments, and `replace`, `exclude`, and `retract` directives cannot
invent graph edges; in particular, a local replacement without a corresponding
requirement is not a dependency. Versions and `// indirect` annotations are
ignored after the authoritative module path. The shared field-boundary fixture
covers those exclusions, and the complete 302-package lane matches the Go
resolver exactly at 936 edges.

Elixir dependency resolution reads local dependency tuples only from direct
project `deps:` lists and lists returned by block or shorthand `defp deps`
functions. Multiline tuples are accepted when they contain a quoted `path:`
option. Project and application metadata, source prose, comments, `mix.lock`,
and non-path Hex or Git dependencies cannot invent graph edges. The shared
field-boundary fixture covers direct, block, shorthand, multiline, comment,
metadata, and external-dependency cases; the complete 282-package lane matches
the Go resolver exactly at 472 edges.

Dart is a first-class discovery lane for both packages and programs.
Dependency resolution reads only direct package keys under root
`dependencies:` and `dev_dependencies:` maps in `pubspec.yaml`. Scalar
constraints and nested source maps are accepted as direct entries, while
nested source options, dependency overrides, package metadata, comments, and
unrelated YAML fields cannot invent edges. Root `name:` fields and
directory-derived snake-case forms supply aliases, and hashing includes
`pubspec.yaml` plus `.dart` source. The shared field-boundary fixture and the
complete 82-package lane match the Go resolver exactly at 67 edges.

TypeScript dependency resolution parses the root `package.json` and reads only
direct property names from `dependencies` and `devDependencies`. Exact
top-level `name` strings and directory-derived scoped or unscoped names supply
aliases. Dependency values are not followed, and peer or optional dependency
tables, scripts, nested tool configuration, nested names, and malformed JSON
cannot invent partial graph edges. The shared field-boundary fixture covers
single-line objects and adversarial decoys; the complete 470-package lane
matches the Go resolver edge-for-edge at 1,076 edges.

Java and Kotlin dependency resolution reads only `includeBuild("...")` calls
from root `settings.gradle.kts` files. Calls may span lines; their quoted
relative paths are normalized lexically and must exactly match a discovered
package root in the same language lane. Comments, unrelated strings,
`include(...)`, build-script coordinates, absolute paths, cross-lane paths,
and unknown targets cannot invent graph edges. Referenced targets are never
opened. Java/Kotlin source and Gradle settings/build files participate in the
package hash. The two shared field-boundary fixtures cover direct and nested
paths plus adversarial decoys; both complete lanes match the Go resolver
exactly at 186 Java edges and 166 Kotlin edges.

C#, F#, and shared .NET dependency resolution reads only literal `Include`
attributes on unqualified `ProjectReference` start elements in root `.csproj`
and `.fsproj` files. Relative paths use portable separators, normalize
lexically, and must exactly match another discovered root project file in the
shared .NET scope. Comments, CDATA, processing instructions, escaped examples,
namespaced elements, package references, MSBuild properties, globs, absolute
paths, nested test projects, and unknown targets cannot invent graph edges;
referenced files are never opened. Three shared fixtures cover both language
lanes and cross-language references. The complete 198-package C# and
197-package F# lanes match the Go resolver edge-for-edge at 238 and 239 edges.

Haskell dependency resolution reads package requirements only from Cabal
`build-depends` fields, including indented comma continuations and repeated
fields across stanzas. Synopsis and description text, source directories,
compiler options, comments, and other fields cannot invent graph edges. The
shared `resolution-haskell-field-aware` fixture covers both inline and
multiline dependencies plus representative non-authoritative collisions. The
complete 205-package Haskell lane matches the Go oracle edge-for-edge.

Python dependency resolution reads only the PEP 621 `[project]` table's
`dependencies` array. Build-system requirements, optional dependency groups,
tool tables, package metadata, and comments cannot invent graph edges. Python
distribution names are lowercased and normalize every run of `-`, `_`, or `.`
to one hyphen before lookup, so repository packages resolve consistently with
PEP 503 naming. Shared fixtures cover the canonical diamond, field boundaries,
version and environment markers, and mixed-separator aliases. The complete
488-package Python lane matches the Go oracle at 1,118 edges.

Rust dependency resolution reads only inline, path-based entries in the
top-level Cargo `[dependencies]` table. Package metadata, features, workspace
fields, dev/build/target dependency tables, comments, and registry-only
dependencies cannot invent graph edges. Renamed dependencies use the inline
`package = "..."` value rather than the local source alias. The shared
field-boundary fixture and complete 948-package lane match the canonical Go
resolver exactly at 2,373 edges.

Ruby dependency resolution reads only `add_dependency` and
`add_runtime_dependency` calls on the `Gem::Specification.new` block receiver.
Development dependencies, package metadata, file lists, comments, and unrelated
text cannot invent graph edges. Both quote forms and optional call parentheses
are accepted, and declared gem names provide authoritative aliases when a
directory-derived name differs. The shared field-boundary fixture and complete
301-package lane match the canonical Go resolver exactly at 454 edges.

Perl dependency resolution reads only top-level runtime `requires`
declarations from each root `cpanfile`. Requirements inside test or other
phase blocks, `Makefile.PL` dependency tables, metadata, and comments cannot
invent graph edges. Exact `Makefile.PL` `NAME` values and current and legacy
distribution spellings are aliases only. The shared field-boundary fixture and
complete 256-package lane match the canonical Go resolver exactly at 217 total
edges: 216 from authoritative manifests and one qualified BUILD dependency.

Swift dependency resolution reads only relative paths from local
`.package(path: "...")` declarations. Package and product names, target
dependency strings, external URLs, source text, and line or nested block
comments cannot invent graph edges. The final path component is matched
case-insensitively against Swift directory aliases. The shared field-boundary
fixture covers those exclusions, and the complete 164-package lane matches the
Go resolver exactly at 179 edges.

Package hashing reads included files as raw bytes. Repository-relative paths
are normalized to `/`, encoded explicitly as UTF-8, and combined with those
bytes using the existing boundary framing before `git hash-object` receives
the payload through binary standard input. Source bytes therefore do not pass
through the host locale: valid Unicode, NUL, and malformed text bytes hash
deterministically on Windows, macOS, and Linux.

## Development

```bash
# Run tests and build the executable
bash BUILD

# Windows: run the command recorded in BUILD_windows
cabal test all
```

## Usage

```bash
# Build whatever changed from origin/main
cabal run build-tool -- --language haskell

# Dry-run the repo-wide plan
cabal run build-tool -- --dry-run --emit-plan
```
