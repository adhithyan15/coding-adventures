# coding-adventures-build-tool

An incremental, parallel monorepo build tool with hash-based caching.

## What it does

This CLI program:
1. Discovers all packages via recursive `BUILD` file walking
2. Resolves dependencies from each supported ecosystem's package metadata
3. Builds a directed graph of dependencies
4. Hashes all source files in each package
5. Compares hashes against a committed cache file (.build-cache.json)
6. Only runs BUILD commands for packages whose hash (or dependency hash) changed
7. Runs independent packages in parallel using concurrent.futures

## Usage

```bash
# Auto-detect root, build changed packages
build-tool

# Specify root explicitly
build-tool --root /path/to/repo

# Rebuild everything
build-tool --force

# Show what would build without building
build-tool --dry-run

# Limit parallel workers
build-tool --jobs 4

# Only build Python packages
build-tool --language python

# Safely plan the field-aware Haskell and JVM lanes
build-tool --language haskell --dry-run
build-tool --language java --dry-run
build-tool --language kotlin --dry-run

# Safely plan the field-aware .NET lanes
build-tool --language csharp --dry-run
build-tool --language fsharp --dry-run

# Safely plan the field-aware Dart lane
build-tool --language dart --dry-run

# Re-emitting to the same path atomically replaces the prior complete plan
build-tool --emit-plan build-plan.json
build-tool --emit-plan build-plan.json
```

## How it fits in the stack

This is a standalone program (not a library) that orchestrates builds across
the entire coding-adventures monorepo. It understands the recursive `BUILD`
discovery convention used throughout the repository and orchestrates every
language listed by `build-tool --help`.

Discovery infers a language only from an exact `packages/<language>` or
`programs/<language>` bucket. Package identities use `<language>/<name>`;
program identities preserve their role as `<language>/programs/<name>`, so a
package and program with the same basename never collide. Haskell, Java,
Kotlin, C#, F#, and Dart are available as explicit filters because their
manifest resolvers are field-aware. C# and F# both request the shared `dotnet`
CI toolchain; Dart requests the `dart` toolchain.

Lua rockspec metadata is decoded as strict UTF-8. Invalid text fails closed with
the stable `METADATA_INVALID_UTF8` diagnostic rather than using a host locale or
silently replacing bytes.

Ordinary dependency aliases are resolved only inside the package's ecosystem.
For example, the same `coding-adventures-shared` name in Lua, Perl, Python, and
Haskell maps to four distinct package identities. A legacy BUILD file may name
an intentional cross-language dependency only with its exact qualified identity:

```text
# build-tool: deps=lua/shared
```

Unknown, unqualified, and self-referential comment entries are ignored. Library
packages retain priority over same-named programs within one ecosystem.

Haskell resolution accepts exactly one root Cabal manifest and reads only its
`build-depends` fields. Plain directory names, the legacy
`coding-adventures-` form, and the declared Cabal package name are aliases in
the Haskell scope. Java and Kotlin resolution scans real multiline
`includeBuild("...")` calls outside nested comments and example strings, then
normalizes their relative paths lexically against already discovered roots in
the same language. Referenced targets are never opened or followed.

C# and F# resolution scans only `.csproj` and `.fsproj` files directly inside
each discovered root. It accepts literal quoted `Include` attributes from
unqualified `ProjectReference` start elements, normalizes `/` and `\` paths
lexically, and matches exact project files already discovered in the shared
C#/F#/dotnet scope. It does not evaluate XML entities, MSBuild properties,
conditions, wildcards, package references, or nested test projects, and it
never opens or follows a referenced target.

Dart resolution reads only the root `pubspec.yaml` in each discovered package
or program. It accepts direct mapping keys under the root `dependencies` and
`dev_dependencies` sections, including dependencies whose scalar value is a
nested source map, but never treats nested `path`, `git`, `url`, `ref`, or
`sdk` keys as dependencies. Directory snake-case names, the legacy
`coding_adventures_` prefix, and an exact unquoted root `name` field are aliases
inside the Dart scope. A declared name that collides with another
same-priority Dart package makes that alias ambiguous and therefore inert.
Other root fields, comments, lockfiles, unknown names, and self references are
ignored, and referenced paths are never opened or followed.

Plan emission writes a complete JSON document to an exclusively created,
writer-owned sibling temporary file and publishes it with the platform's
replace-if-present primitive. Reusing an existing `--emit-plan` destination is
therefore supported on Windows, macOS, and Linux without following a
predictable staging path. If staging or replacement fails, the previously
published plan remains intact and the writer makes a best-effort cleanup of its
temporary file.

The process-free tracked-artifact validator consumes the shared closed snapshot
as inert ordinal, path, and entry-kind records. It normalizes separators,
rejects unsafe paths at the fixed redacted `repository` location, and rejects
exact, nested, case, and Unicode compatibility aliases of a `node_modules`
component. Regular, symlink, and reparse metadata is classified identically;
path limits and diagnostic ordering use Unicode scalar values, and Windows
reserved basenames use full Unicode uppercase mapping before comparison;
the validator never enumerates Git, opens or follows a path, launches a
process, reads the environment, or accesses the network.

## Installation

```bash
cd code/programs/python/build-tool
uv pip install -e ".[dev]"
```

## Development

```bash
uv run pytest tests/ -v
```
