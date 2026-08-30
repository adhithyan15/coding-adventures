# Build-Tool Conformance Contract

## Status and authority

This document defines the language-neutral behavior required of every supported
implementation of the coding-adventures build tool.

The older build-system specifications describe important pieces of the target,
but they were written at different stages of the build tool's evolution. Where
those documents disagree about observable behavior, this contract is the
conformance authority:

- `12-build-system.md` describes the overall architecture;
- `build-plan-v1.md` defines the build-plan interchange format;
- `build-plan-sharding.md` defines prerequisite-closed CI shards;
- `15-os-aware-build-rules.md` defines structured, OS-aware Starlark commands;
- `B05-build-windows-executor.md` defines the planned Windows executor and
  platform-override corrections.

Implementation-specific internals are not normative. The Go implementation is
the current operational reference used by CI, but a behavior does not become
portable merely because Go implements it. Portable behavior is established by
this contract and the shared fixtures.

## Purpose

Directory presence is not parity. A conforming build tool must make the same
decisions from the same repository-shaped input:

1. discover the same packages;
2. resolve the same local dependency edges;
3. select the same changed, affected, and prerequisite package sets;
4. produce compatible hashes, plans, shards, diagnostics, and status values;
5. apply the same platform BUILD precedence and Starlark semantics;
6. execute or simulate work with the same failure-propagation rules; and
7. expose those decisions as deterministic machine-readable output.

The shared fixture corpus is the behavioral oracle. A Python runner may
orchestrate implementations, but no implementation language is the oracle.

## Implementation scope

The established package-parity denominator currently contains 15
implementation languages:

`csharp`, `dart`, `elixir`, `fsharp`, `go`, `haskell`, `java`, `kotlin`, `lua`,
`perl`, `python`, `ruby`, `rust`, `swift`, and `typescript`.

Build-tool front doors currently exist for C#, Elixir, F#, Go, Haskell, Lua,
Perl, Python, Ruby, Rust, Swift, and TypeScript. Dart, Java, and Kotlin still
need implementations. The F# front door currently delegates to the C# engine;
that is a shared-engine exception candidate, not proof of an independent F#
implementation.

Every shared-engine front door MUST expose a language-native adapter and
independently consume each required conformance fixture. Transitive coverage
from the shared engine does not count as front-door coverage for that lane.
For toolchain detection, that adapter MUST accept the caller-supplied bounded
snapshot through a language-native symbol and independently consume every
`toolchain-detection-*.json` case. Merely executing a CLI that delegates to the
shared engine is not independent lane evidence.

Independent build-tool engines MUST expose the same process-free snapshot
boundary through their native module surface before toolchain-detection parity
is claimed. The Elixir engine uses
`BuildTool.ToolchainDetection.evaluate_snapshot/5`; its package-local suite
MUST discover and evaluate every `toolchain-detection-*.json` case rather than
relying on the neutral Python oracle or the Go front door.

The Haskell engine uses
`ToolchainDetection.evaluateToolchainSnapshot`; its package-local Hspec suite
MUST discover and evaluate every `toolchain-detection-*.json` case through that
pure native boundary, including exact outcomes, all 16 canonical flags,
diagnostics, platform precedence, CRLF grammar, scheduling, forced toolchains,
and the shared byte, line, and aggregate ceilings.

The Lua engine uses
`require("build_tool.toolchain_detection").evaluate_snapshot`; its
package-local Busted suite MUST discover and evaluate every
`toolchain-detection-*.json` case through that pure native boundary. The
adapter MUST remain process-free and host-state-free, preserve byte-exact CRLF
grammar and platform-front precedence, return all 16 canonical flags, and
enforce the shared per-file and aggregate resource ceilings before evaluation.

C and C++ remain emerging implementation lanes. OCaml also begins as emerging
and must implement this contract before promotion. WASM is an execution target,
Mosaic and Twig are domain languages, and Starlark is a build language; none is
automatically required to provide a build-tool program.

Final parity requires every established implementation language to have either:

- a native build-tool engine that passes all required fixtures; or
- a narrow, reviewed shared-engine exception recorded in the parity roadmap,
  with a language-native front door and the same fixture results.

## Normative language

The terms **MUST**, **MUST NOT**, **SHOULD**, **SHOULD NOT**, and **MAY** are
normative.

- **Required** fixture domains are completion gates for every supported
  build-tool implementation.
- **Capability-gated** cases apply only when their declared capability is
  implemented, but every established implementation must eventually implement
  all capabilities marked `required_for_final_parity`.
- **Platform-gated** cases run only on listed operating systems.
- **Deferred** behavior is planned and tested by fixtures that may initially be
  expected failures. Deferred cases must not be reported as passing or omitted
  silently.

## Conformance adapter

Every implementation MUST eventually expose this test-only interface:

```text
build-tool --conformance CASE.json --workspace-root ROOT --output RESULT.json
```

The adapter MUST:

1. read one fixture conforming to
   `code/specs/fixtures/build-tool-v1/schema.json`;
2. operate only on the runner-created, already validated `ROOT`;
3. run exactly one requested domain operation;
4. write one canonical result object;
5. write no timestamps, durations, absolute host paths, random identifiers, or
   locale-dependent text into the result;
6. exit `0` only when the adapter itself completed and wrote a result;
7. represent an expected build-tool error inside the result rather than using
   the adapter process exit code; and
8. exit nonzero for malformed fixtures, unsupported schema versions, unsafe
   materialization, adapter crashes, or failure to write the result.

The runner, not the adapter, materializes the fixture into `ROOT`. It then
launches the adapter inside an outer sandbox whose only writable mount is the
runner-owned scratch tree. This keeps containment policy bound to a root chosen
before untrusted implementation code starts. The runner compares the adapter
result with the fixture's `expected` object. Human-readable stdout and stderr
are diagnostic only and are not conformance inputs.

### Bootstrap runner modes

The first runner tranche is deliberately non-executing. It provides two
language-neutral validation modes:

```text
python code/scripts/build_tool_conformance.py validate-corpus
python code/scripts/build_tool_conformance.py validate-result \
  --case CASE.json --result RESULT.json
```

`validate-corpus` strictly parses and validates the implementation manifest,
every case, every expected result, path semantics, corpus-wide identity
uniqueness, canonical ordering, and bounded in-memory decoding of non-execution
workspaces. The bootstrap runner never stages fixture files on disk.

`validate-result` strictly parses one case and one externally produced result,
validates both against their schemas, verifies the echoed case identity and
domain, applies domain-aware canonicalization, and compares the result with the
fixture oracle. This lets each language wire its adapter in its own package
tests before sandboxed process orchestration is enabled.

Neither mode launches an adapter, invokes a shell, reads an executable command
from a manifest, or runs fixture content. The bootstrap runner MUST reject any
case whose domain, operation, or capabilities request execution before it
decodes file content or uses a process API. It MUST NOT create a temporary
workspace or change filesystem permissions. There is no command-line flag that
can weaken this rule.
Trusted execution becomes a separate delivery gate only after the runner can
enforce the complete filesystem, network, environment, and process-tree
boundary in this contract.

The bootstrap implementation manifest is metadata, not an executable registry.
It records every established language, emerging lanes under active review,
front-door presence, shared-engine relationships, advertised capabilities, and
reviewed expected failures. It MUST NOT contain adapter commands, environment
assignments, or arbitrary argument vectors.

## Fixture manifest

Each fixture is a single JSON document. Inline files make a case atomic and
portable across Git checkouts and operating systems.

Required top-level fields:

| Field | Meaning |
|---|---|
| `schema_version` | Integer fixture schema version; v1 is `1`. |
| `id` | Stable lowercase identifier, unique across the corpus. |
| `domain` | One conformance domain from the registry below. |
| `summary` | Short explanation of the behavior under test. |
| `platforms` | Normalized OS names on which the case applies. |
| `capabilities` | Capabilities required to run the case. |
| `workspace.files` | Inline UTF-8 or base64 files materialized under a fresh root. |
| `input` | Domain-specific operation parameters. |
| `expected` | Canonical result expected from every conforming implementation. |
| `limits` | Requested timeout and output-size limits, always capped by runner policy. |

The schema rejects obvious absolute paths, backslashes, empty path components,
and `..` components. The runner MUST additionally perform semantic containment
checks after platform-native normalization. JSON Schema alone cannot detect
drive-relative paths, UNC/device names, Unicode or case-folding collisions,
symlink/reparse escapes, or filesystem-specific aliases. Base64 content is
strictly decoded, and every path must remain unique after platform-native case
folding and Unicode normalization. The fixture `id` is data and is never used
as a filesystem path.

## Canonical result

The `expected` object and adapter output share this shape:

```json
{
  "schema_version": 1,
  "case_id": "discovery/platform-precedence",
  "domain": "discovery",
  "outcome": "ok",
  "result": {},
  "diagnostics": []
}
```

Rules:

- adapters may emit ordinary valid JSON; the runner parses, schema-validates,
  normalizes, and serializes object keys lexicographically before comparison;
- package names, paths, dependency edges, diagnostics, language lists, and
  other set-like arrays are sorted as defined by the domain;
- command order, dependency-level order, and other sequence-semantic arrays are
  preserved; packages inside a dependency level are sorted;
- repository-relative paths use `/` on every platform;
- no path may escape the materialized repository root;
- absent optional values are omitted unless the domain assigns meaning to
  `null`;
- `null` and `[]` remain distinct;
- numeric values are signed integers in the interoperable
  `[-9007199254740991, 9007199254740991]` range; floating-point values,
  non-finite values, and negative zero are forbidden, and domain decimals use
  canonical strings;
- diagnostics use stable codes and severities; prose may be present but is not
  used as the sole assertion;
- duplicate set-like values are invalid rather than silently deduplicated;
- outcome is one of `ok`, `error`, `unsupported`, or `skipped`;
- `unsupported` and `skipped` require a stable diagnostic code and never count
  as conformance success.

The adapter reports the operation outcome. Only the outer runner decides
whether a case passes after comparing the normalized output with `expected`.
The result envelope echoes both `case_id` and `domain`; semantic validation
requires them to equal the fixture's `id` and `domain`, respectively.

## Domain registry

### 1. Discovery

Required behavior:

- walk the configured code root recursively;
- require a canonical `BUILD` file to establish package membership, match
  filenames exactly, and stop recursion at a package root;
- skip the canonical generated/dependency directories;
- infer language only from exact path components;
- distinguish packages and programs with the same basename;
- return packages sorted by qualified name;
- reject duplicate qualified names; and
- select platform files in this order:
  - Windows: `BUILD_windows`, then canonical `BUILD`;
  - macOS: `BUILD_mac`, then `BUILD_mac_and_linux`, then canonical `BUILD`;
  - Linux: `BUILD_linux`, then `BUILD_mac_and_linux`, then canonical `BUILD`.

The bucket component immediately below `packages` or `programs` is the sole
language discriminator; canonical words later in a path do not change the
language. A package root has qualified identity `<language>/<basename>`, while
a program root has qualified identity `<language>/programs/<basename>` so a
package and program with the same basename remain distinct. The canonical
discovery registry classifies all established implementation lanes (`csharp`,
`dart`, `elixir`, `fsharp`, `go`, `haskell`, `java`, `kotlin`,
`lua`, `perl`, `python`, `ruby`, `rust`, `swift`, and `typescript`), emerging
implementation lanes (`c`, `cpp`, and `ocaml`), the `wasm` execution target,
the `mosaic` and `twig` domain languages, and the `starlark` build language.
The repository also retains `dotnet` for programs hosted by the shared .NET
engine; it is not a separate package-parity denominator. An unrecognized
component is reported as `unknown` rather than borrowed from a substring.

Cabal's exact, case-sensitive `dist-newstyle` directory component is generated
build output and is therefore excluded before BUILD-file membership is tested.
A similarly named source component such as `dist-newstyle-example` is not
excluded. The shared language-registry fixture makes this exclusion normative
for every discovery implementation.

If two discovered directories still produce one qualified name, discovery
fails with `DUPLICATE_PACKAGE_IDENTITY`. The diagnostic includes the duplicate
package identity and every repository-relative package path in sorted order;
it must not disclose the checkout root.

The selected canonical `BUILD` may contain legacy shell lines or Starlark.
Conformance v1 does not recognize `BUILD.lark` as a package marker; that name in
older migration documents is aspirational. Implementations must not infer
Starlark from arbitrary executable text after a platform-specific shell
override has already won precedence. Platform variants override recipes; they
do not create a host-specific package universe.

### 2. Dependency resolution

Resolvers MUST cover every established implementation language and any emerging
language the implementation advertises. Fixtures use real minimal manifests,
not invented dependency summaries.

The canonical graph edge `[from, to]` means "`to` depends on `from`". Resolution
MUST:

- ignore external dependencies;
- read local dependency candidates only from the ecosystem's authoritative
  dependency fields or sections; package identity, descriptions, source/build
  metadata, comments, and unrelated quoted strings MUST NOT create edges;
- recognize repository-supported legacy and current package-name aliases;
- keep dependency scope within the implementation ecosystem unless metadata
  explicitly names another qualified package;
- preserve the `programs` identity segment when a package and program share a
  language and basename, so a program dependency resolves to the package
  instead of collapsing into a self-edge;
- merge qualified internal dependencies from the selected legacy BUILD file's
  `# build-tool: deps=` comment with ecosystem metadata, preserving canonical
  package/program identities and emitting sorted unique edges;
- reject a resolved self-edge as `DEPENDENCY_SELF_EDGE`, including the
  repository-relative manifest path, package identity, and dependency identity;
- reject ambiguous manifests, ambiguous aliases, and duplicate package
  identities;
- decode `.rockspec` text metadata as strict UTF-8 without replacement or
  locale fallback;
- report invalid rockspec encoding as `METADATA_INVALID_UTF8`, including the
  repository-relative manifest path and package identity;
- surface that failure from a standalone build-tool front door as unsafe or
  malformed input: exit `2`, write the stable diagnostic to standard error,
  write no success output, and never disclose the absolute checkout root;
- emit sorted, unique internal edges; and
- report malformed metadata with stable diagnostics instead of silently
  inventing a partial graph.

For Go `go.mod` manifests, dependency candidates come only from module paths in
single-line `require` directives and entries inside `require ( ... )` blocks.
The module path is matched case-insensitively against known Go module aliases;
the version and an optional `// indirect` annotation do not form part of the
identity. Resolvers ignore the current module declaration, Go/toolchain
versions, comments, `replace`, `exclude`, and `retract` directives, and every
other field or block. In particular, a local `replace` target without a
corresponding `require` entry changes module lookup but does not create a build
graph edge.

For Elixir `mix.exs` manifests, dependency candidates come only from tuples in
the list returned by the root `defp deps` function, in either block or
single-expression shorthand form, when the tuple begins with a dependency atom
and contains a quoted `path:` option. The tuple may span multiple lines, and
options after `path:` do not form part of the identity. The
dependency atom is matched case-insensitively against known Elixir application
and package aliases. Resolvers ignore the project `app:` field, application
configuration, module and function names, source text, strings outside the
dependency list, line comments, `mix.lock`, non-path Hex/Git dependencies, and
every other function or field. The quoted path identifies the declaration as
local metadata; resolvers do not follow or read the referenced path.

For Dart `pubspec.yaml` manifests, dependency candidates come only from the
direct mapping keys of the root `dependencies:` and `dev_dependencies:`
fields. The key is an unquoted Dart package identifier and is matched
case-insensitively against known directory and root `name:` aliases. A direct
entry may use a scalar constraint or a nested source map; nested `path:`,
`git:`, `url:`, `ref:`, `sdk:`, and other source-option keys do not become
dependencies. Resolvers ignore package descriptions, environment constraints,
`dependency_overrides`, Flutter and tool configuration, comments, inline prose,
lockfiles, and every other root field. A local `path:` value identifies source
metadata only; resolvers do not follow or read the referenced path.
If a root `name:` collides with another same-priority Dart package's directory
or declared alias, that alias is ambiguous and contributes no dependency edge.

For TypeScript `package.json` manifests, dependency candidates come only from
the direct property names of the root `dependencies` and `devDependencies`
objects. Each property name is matched case-insensitively against known
directory aliases and the exact string value of another package's root
top-level `name` property. Dependency values, including registry constraints,
`workspace:` ranges, and `file:` paths, do not form part of the identity and
are not followed. Resolvers ignore `peerDependencies`,
`optionalDependencies`, scripts, tool configuration, nested objects whose
property names resemble dependency tables, nested `name` properties, lock
files, descriptions, and every other root field. A dependency field whose
value is not an object contributes no candidates, and malformed JSON must not
produce a partial graph.

For Java and Kotlin Gradle composite builds, dependency candidates come only
from `includeBuild("...")` call expressions in the package root's
`settings.gradle.kts`. The call may span lines, and its sole argument is a
quoted relative path. Resolvers normalize that path lexically against the
declaring package root and create an edge only when the result exactly matches
a discovered package root in the same language scope. They do not follow or
read the referenced path. Resolvers ignore line and block comments, string
literals that merely contain example calls, `include(...)`, project and plugin
metadata, dependency coordinates in `build.gradle` or `build.gradle.kts`,
absolute paths, paths outside the current language scope, undiscovered targets,
and every other settings field or call. `settings.gradle.kts`, Gradle build
files, and Java or Kotlin sources participate in the package hash so a
dependency declaration change invalidates the cache.

For C#, F#, and shared .NET packages, dependency candidates come only from
unqualified `ProjectReference` start elements with a quoted literal `Include`
attribute in `.csproj` or `.fsproj` files directly inside the declaring package
root; project-file extensions are compared ASCII-case-insensitively. A static
include remains a conservative dependency even when the element
has a `Condition`; resolvers do not evaluate MSBuild properties or conditions.
The include must be a relative project-file path with no property expansion,
glob, query, fragment, or XML entity reference. Both `/` and `\` are portable
separators. Resolvers normalize the path lexically against the directory of the
declaring root project and create an edge only when it exactly matches a root
project file belonging to another discovered C#, F#, or shared .NET package in
the current `dotnet` dependency scope. They do not follow or read the referenced
path. Resolvers ignore XML comments, CDATA, processing instructions, escaped
text examples, namespaced elements, `PackageReference`, `ProjectReference`
elements without `Include` (including `Update` and `Remove`), project files in
nested test or tool directories, absolute paths, unknown targets, and every
other XML element or attribute. Duplicate and self references do not create
additional edges.

For Cabal manifests, a package must have exactly one `.cabal` file directly in
its root; zero manifests contribute no dependencies, and multiple manifests
are ambiguous and therefore contribute neither a manifest-declared alias nor
manifest dependencies. Directory-derived aliases still come from discovery.
Dependency candidates come only from each `build-depends:` field and its
indented comma-separated continuation lines. Resolvers match candidates
case-insensitively against a discovered Haskell package's directory name,
legacy `coding-adventures-<directory>` alias, and the sole manifest's declared
top-level `name:` field. Resolvers ignore Cabal comments, package identity and
descriptive metadata, source directories, compiler options, and every other
field. A new stanza may introduce another `build-depends:` field; reaching a
sibling field or stanza ends the current field before scanning continues.

For OCaml packages, directory aliases are the lower-case root basename, its
`coding-adventures-` opam form, and its underscore-normalized
`coding_adventures_` Dune form. Exactly one regular root `.opam` manifest may add its
filename stem and top-level `name:` string as aliases and quoted dependency
candidates from the top-level `depends: [ ... ]` field; multiple manifests are
ambiguous and contribute neither manifest aliases nor manifest dependencies.
Resolvers union those candidates with direct atoms or quoted names from
`(libraries ...)` fields in only the regular files `dune`, `src/dune`, `bin/dune`, and
`test/dune`. They ignore comments, constraints, filters, pins, build commands,
other opam fields, other Dune forms or files, nested expressions, variables,
external names, and self references. Package aliases take priority over a
same-named program, two packages claiming one declared alias make it unusable,
and duplicate candidates collapse to one edge. OCAML04 is the normative
detailed contract for this emerging lane.

For Python `pyproject.toml` manifests, dependency candidates come only from the
PEP 621 `[project]` table's `dependencies = [...]` array. Resolvers ignore
package identity and descriptive metadata, `[build-system]`,
`[project.optional-dependencies]`, tool-specific tables, comments, and every
other field. Distribution names are matched case-insensitively after PEP 503
normalization: each run of hyphens, underscores, or periods becomes one hyphen
before internal-package lookup. Extras, version specifiers, and environment
markers do not form part of the normalized distribution name.

For Rust `Cargo.toml` manifests, dependency candidates come only from inline
entries in the top-level `[dependencies]` table whose value contains a `path`
assignment. The dependency key before the first `=` is matched against known
Cargo package names unless the inline table provides a quoted `package`
override, in which case that published package name is authoritative. Resolvers
ignore `[package]`, `[lib]`, features, workspace
metadata, dev dependencies, build dependencies, target-specific dependency
tables, non-path registry dependencies, comments, and every other field or
table. Reaching any new TOML table ends the authoritative dependency table.

For Ruby `.gemspec` manifests, dependency candidates come only from the first
quoted gem-name argument of `add_dependency` and `add_runtime_dependency`
calls on the `Gem::Specification.new` block receiver. The two methods are
runtime-dependency synonyms for graph purposes. Resolvers ignore gem identity,
summary and description text, file and require-path lists, metadata, comments,
`add_development_dependency` calls, and every other field or method. Both Ruby
quote forms and optional call parentheses are accepted; the gem name is matched
case-insensitively against known Ruby package aliases.

For Swift `Package.swift` manifests, dependency candidates come only from the
quoted relative path in local `.package(path: "...")` declarations. The final
path component is matched case-insensitively against known Swift package
directory aliases. Resolvers ignore package and product identity, target and
product dependency names, external `.package(url: ...)` declarations, source
text, string literals, line and block comments, and every other initializer or
field. Absolute paths and paths whose final component is empty, `.` or `..` do
not create internal graph edges.

For Perl manifests, dependency candidates come only from top-level `requires`
statements in the root `cpanfile`. A statement is top-level only when it begins
outside every Perl block; `requires` calls inside `on ... => sub { ... }` phase
blocks are not runtime graph edges. Resolvers ignore root `Makefile.PL`
dependency tables, package identity, abstracts and other metadata, comments,
test/develop/configure/build phase requirements, and every other statement.
Both Perl quote forms are accepted and an optional version argument is ignored.
The dependency name is matched case-insensitively against the package
directory's exact, kebab-case, and snake-case aliases; the corresponding
`coding-adventures-` and `coding_adventures_` distribution aliases; and the
exact module name declared by the root `Makefile.PL` `NAME` field. The
`Makefile.PL` `NAME` field contributes an alias only and never contributes an
edge.

### 3. Graph and scheduling

Required cases cover isolated nodes, chains, diamonds, multiple components,
cycles, affected dependents, prerequisite closure, independent levels, and
failure propagation.

Topological ordering is deterministic: packages inside a level are sorted by
qualified name. A cycle is a build-plan error. If package `A` fails, every
transitive dependent of `A` is `dep-skipped`; unrelated packages may continue.

### 4. Diff selection

Git-diff fixtures provide changed repository-relative paths rather than invoking
the host Git binary. Implementations MUST map:

- package-local changes to that package;
- shared workflow/toolchain changes to the declared toolchain set;
- strict Starlark `srcs` globs only to matching packages;
- package changes to all transitive dependents; and
- changes outside known packages according to the explicit conservative
  fixture policy: select all declared packages or return an error.

The adapter must not read the caller's real checkout, Git config, hooks, or
credentials.

### 5. Hashing and cache

Required behavior:

- sort normalized relative paths before hashing;
- exclude generated, dependency, VCS, cache, and temporary directories;
- include applicable BUILD and manifest files;
- include OCaml `.ml`, `.mli`, and `.opam` sources plus exact `dune`,
  `dune-project`, and `.ocamlformat` metadata names;
- hash source and manifest contents as raw bytes without decoding them through
  the host locale, and encode normalized relative paths explicitly as UTF-8;
- when a subprocess supplies the digest primitive, write its standard input in
  binary mode so NUL, non-ASCII, and malformed text bytes are preserved;
- preserve file boundaries unambiguously;
- combine dependency hashes in deterministic graph order;
- invalidate dependents when a prerequisite hash changes; and
- recover from a missing or corrupt cache with a stable diagnostic.

Fixtures provide file bytes. Implementations MUST NOT include host metadata,
absolute paths, mtimes, ownership, locale, or directory enumeration order.

### 6. Starlark

Final parity requires:

- deterministic BUILD evaluation with repository-contained `load()` paths;
- `_ctx` propagation through nested loads using the v1 context schema;
- normalized `os`, `arch`, `cpu_count`, `ci`, and `repo_root` fields;
- filtering of platform-inapplicable `None` commands;
- structured command objects with separate `program` and `args`;
- platform-correct rendering without executing text during evaluation; and
- a declared fallback for legacy targets that do not yet expose structured
  commands.

Once a selected canonical `BUILD` is identified as Starlark, parse, load,
evaluation, target-shape, and structured-command errors are fatal. An
implementation MUST NOT reinterpret that source as legacy shell commands or
retain raw `BUILD` lines after an evaluation error. The declared legacy
fallback applies only after successful evaluation of a valid target that does
not expose structured commands.

Starlark evaluation MUST NOT read undeclared host files, environment variables,
network resources, clocks, random sources, or process APIs.

An executable front door that delegates evaluation to repository packages MUST
declare its complete repository-local runtime closure through its supported
dependency or bootstrap manifest. A clean source-tree test MUST remove ambient
language search-path injection, load the evaluator without skipping, and prove
that build execution does not need network resolution.

### 7. Build-plan v1

`build-plan-v1.md` remains the interchange schema. Fixtures require:

- stable round-trip serialization;
- forward-slash relative paths;
- preservation of `affected_packages: null` versus `[]`;
- rejection and fallback for unsupported future versions;
- tolerance of unknown optional fields;
- sorted packages and edges;
- validation that every edge and affected name references a declared package;
- rejection of paths outside the repository root; and
- atomic replacement of an existing plan destination with no partial plan or
  retained writer temporary file;
- no execution of `build_commands` during plan parsing.

Only Go, Python, Ruby, and Swift currently expose an end-to-end v1 plan
consumption path. Other front doors emit divergent plans, expose library-only
readers, parse and discard the plan, or have no plan support. Those are tracked
gaps, not alternate valid formats.

The logical package set, dependency graph, and change selection are
platform-independent. A v1 plan's concrete `build_commands` are a
producer-platform recipe. Before execution, a consumer on another platform
MUST re-resolve the selected platform override or re-evaluate the canonical
Starlark BUILD with the target context. It MUST NOT execute producer commands
blindly. A future plan version should separate the portable logical plan from
platform recipes explicitly.

Cross-language compatibility is proven only when one implementation's emitted
plan is consumed by another implementation under the fixture matrix.

### 8. Sharding

Final parity requires the behavior in `build-plan-sharding.md`:

- deterministic balancing for a fixed input and shard count;
- stable zero-based shard indexes and names;
- exactly one direct assignment for every scheduled package;
- prerequisite-closed `package_names`;
- toolchains computed from each closed shard;
- no unknown packages or cross-shard artifact assumptions; and
- explicit errors for invalid shard counts or indexes.

Duplicate prerequisites across shards are permitted.

### 9. Execution

Execution fixtures are separate from pure planning fixtures and are
`trusted_execution` capability cases. They use only commands supplied by the
repository-owned corpus and run inside an isolated temporary root.

Required behavior:

- each legacy BUILD line is a separate shell invocation in package order;
- independent packages respect the job limit;
- dependent packages never start before prerequisites finish;
- first command failure stops that package;
- failed prerequisites produce `dep-skipped` dependents;
- dry-run executes no commands;
- declared shared-resource locks serialize conflicting package operations;
- cancellation, timeout, and output limits terminate the full child process
  tree; and
- platform shell selection follows the resolved Windows-executor contract.

The current Go `cmd /C` behavior and lack of process limits are known gaps, not
portable semantics.

Execution results use one closed state machine:

- `succeeded` commands have exit code `0`;
- `failed` commands have a nonzero exit code;
- `not-run` commands have a null exit code;
- `built` packages have return code `0` and every command is `succeeded`;
- `failed` packages have exactly one `failed` command, use that command's
  nonzero exit code as the package return code, have only `succeeded` commands
  before the failure, and only `not-run` commands after it;
- `dep-skipped` and `would-build` packages have a null return code and every
  command is `not-run`;
- a `failed` or `dep-skipped` prerequisite makes each direct dependent
  `dep-skipped`; conversely, every `dep-skipped` package has at least one direct
  `failed` or `dep-skipped` prerequisite;
- dry-run cases have outcome `ok` and every package is `would-build`;
- non-dry-run outcome `ok` means every package is `built`; and
- outcome `error` is non-dry-run, contains at least one `failed` package, and
  otherwise contains only `built` or `dep-skipped` packages.

The schemas enforce every local state/return-code constraint and the
input/outcome status sets. The process-free execution contract validator also
enforces command fail-stop order, failed-command/package return-code equality,
complete package classification, and dependency-graph propagation. No adapter
or execution case may enter the corpus with a contradictory record.

### 10. Validation

Perl BUILD validation MAY admit local source references declared only inside a
root `cpanfile` test block, plus their authoritative runtime prerequisites,
without promoting those test-only references into the runtime dependency
graph. This allowlist is validation-only: undeclared references and references
from any other ignored metadata block still fail closed.

For a Lua package whose canonical `BUILD` bootstraps repository-local sibling
rocks, the canonical recipe itself MUST install the complete transitive local
rock closure in dependency order before its final self-install; listing only
direct siblings is not standalone on a clean LuaRocks tree. Validation MUST
also require a `BUILD_windows` standalone recipe. That recipe MUST preserve
every canonical sibling install and the same complete closure using Windows
path and redirect syntax. If a canonical sibling install uses
`--deps-mode=none` or
`--no-manifest`, the Windows install MUST retain equivalent hardening. Once a
recipe bootstraps sibling rocks, its final self-install MUST also disable
dependency resolution. Each BUILD line is a separate shell invocation, so a
recipe MUST NOT depend on working-directory state from an earlier line.

Validation fixtures report stable diagnostic codes for:

- missing or empty BUILD files;
- missing platform coverage;
- undeclared local dependency references;
- metadata dependencies missing from standalone BUILD prerequisites;
- invalid Starlark source/dependency declarations;
- ambiguous identities or manifests;
- unsupported languages/toolchains; and
- unsafe paths or commands in plans and fixtures.

Diagnostics are sorted by `(code, path, package, detail_key)`. Human prose is
informational.

### 11. Toolchain detection

Toolchain detection operates on the scheduled package set after affected and
shard filtering. It MUST:

- use normalized implementation-language keys;
- collapse only explicitly shared toolchains, such as C#/F# to `dotnet` and
  C/C++ to `cpp`;
- include forced CI toolchains from workflow changes;
- distinguish `null` all-package selection from an empty selection; and
- return a complete, sorted boolean map for the supported registry.

OCaml enters this registry before its build-tool implementation is promoted.
Its process-free registry key and CI output are `ocaml`; its execution setup
remains owned by the separate OCAML03 workflow until the execution-coupled
substrate is reviewed.

### 12. CLI and reporting

Required stable process semantics:

| Condition | Exit code |
|---|---:|
| successful build, validation, dry-run, or plan emission | `0` |
| invalid CLI usage or unsafe/malformed input | `2` |
| package build or validation failure | `1` |
| internal adapter failure | nonzero, with no forged result |

Machine output is JSON. Human progress output may vary and is not compared by
the conformance runner.

### 13. Closed pure-domain fixture model

The process-free bootstrap closes the remaining non-execution domains with the
following v1 input and result records. All lists named below are bounded by the
fixture schema. Package names, paths, toolchain keys, diagnostic codes, and
structured command fields use the shared definitions in the corpus schema.

| Domain | `input.options` | Successful `result` |
|---|---|---|
| `diff_selection` | packages with repository-relative roots and an explicit `package_prefix` or `strict_globs` source mode, dependency edges, forced packages, and an `all` or `error` unknown-path policy | sorted `changed_packages`, `affected_packages`, and prerequisite-only `prerequisite_packages` |
| `hashing_cache` | SHA-256 mode, package, included paths, dependency digests, dependents, and a closed missing, corrupt, or typed prior-cache record | lowercase `package_digest`, `dependencies_digest`, `combined_digest`, cache status, and sorted invalidated packages |
| `starlark` | repository-contained entrypoint, v1 `_ctx`, and declared legacy fallback policy | sorted targets containing rule metadata, structured commands, deterministic display rendering, and the per-target command source |
| `sharding` | package languages and build-command counts, dependency edges, scheduled packages, shard count, and optional shard index | stable prerequisite-closed shard records with assignments, package closure, toolchains, and estimated cost |
| `validation` | platform, selected checks, normalized package declarations, dependency edges, and optional orphan-crate/ledger snapshots | `valid` plus sorted stable diagnostic codes |
| `toolchain_detection` | target platform, package-language records with closed inline BUILD-front snapshots, `null`/empty/explicit package selection, explicit forced-full mode, and forced toolchains | the complete canonical toolchain registry as a sorted boolean map |
| `cli` | a portable action, decision condition, and whether the action would require later execution | exit code only |

These records intentionally model decisions, not host operations:

- diff selection receives `changed_paths`; it never invokes Git;
- hashing receives inline bytes; it never reads host metadata;
- Starlark receives inline source and context; it never executes a command;
- validation inspects inline repository data only;
- toolchain detection never probes installed programs or reads a checkout; and
- CLI fixtures parse a bounded, language-neutral `argv` grammar into a closed
  typed record and then classify an explicitly supplied post-parse outcome.
  They never invoke a front door or launch a build.

Hashing v1 uses SHA-256 over an unambiguous byte stream. Included files are
sorted by normalized forward-slash path. For each file, append the unsigned
64-bit big-endian path-byte length, UTF-8 path bytes, unsigned 64-bit
big-endian content length, and exact content bytes. Dependency digests are
sorted by package name and encoded the same way, using the package name as the
first byte string and the 32 decoded digest bytes as the second. The package
stream and dependency stream are hashed separately; `combined_digest` is
SHA-256 over the 32 package-digest bytes followed by the 32 dependency-digest
bytes. An empty stream therefore has the standard SHA-256 empty digest.
Prior cache records are data, not nested executable or parser input:
`missing` and `corrupt` carry no payload; `record` carries exactly a combined
digest and `success` or `failed` status. A matching successful record is a
`hit` with no invalidations. Missing, failed, or stale records are a `miss`;
corrupt records are `recovered`. Every non-hit invalidates the package and its
declared dependent closure.

Toolchain detection v1 treats extra-CI declarations as inert BUILD metadata.
Each package supplies a required generic `BUILD` string plus optional
`BUILD_windows`, `BUILD_mac`, `BUILD_linux`, and `BUILD_mac_and_linux` strings.
The target platform selects exactly one front: the platform-specific string,
then `BUILD_mac_and_linux` for Darwin or Linux, then generic `BUILD`. No
declaration in an unselected front has any effect. Each string is at most
65,536 UTF-8 bytes and 4,096 logical lines; all BUILD strings in one input are
at most 1 MiB. Exceeding a ceiling makes the case invalid before an adapter can
run.

Split BUILD content on LF bytes. For each LF-terminated logical line, remove
the immediately preceding CR byte if and only if that CR and LF form the line
terminator. A CR anywhere else is content: in particular, a final lone CR and
a CR followed only by trailing ASCII spaces or tabs are never whitespace.
After that line-ending step, remove only leading and trailing ASCII space and
tab. A declaration line has the exact form
`# needs-toolchain: NAME`. At least one ASCII space or tab separates the colon
from `NAME`; `NAME` must be one lowercase key in the canonical toolchain
registry, and nothing except trailing ASCII space or tab may follow it. Empty,
unknown, wrong-case, fused, suffixed, and otherwise malformed lookalikes are
inert comments. Valid declarations are retained in first-occurrence file order
and stably deduplicated.

Only explicitly scheduled packages contribute their inferred language and
selected-front declarations. An empty selection contributes neither; `null`
selects every supplied package. Forced toolchains union into either result.
When `force_full` is true, `scheduled_packages` must be `null` and every
canonical toolchain is enabled, matching a CI full-rebuild decision without
parsing host state. Unsupported selected package languages and unsupported
forced-toolchain values remain the stable `TOOLCHAIN_UNSUPPORTED` error;
unknown declaration values are ignored. Every successful result contains all
canonical keys, including false values, in deterministic key order.

Sharding v1 normalizes package languages to the canonical toolchain registry
before computing cost. Each package costs `1 + build_command_count` plus the
following toolchain weight:

| Toolchain | Weight |
|---|---:|
| `rust` | 6 |
| `dotnet`, `haskell`, `swift`, `typescript` | 4 |
| `java`, `kotlin` | 3 |
| `elixir`, `python`, `ruby` | 2 |
| every other registered toolchain | 0 |

Scheduled roots sort by descending package cost and then qualified package
name. Each root is assigned to the shard with the lowest current direct-root
cost, breaking ties by lowest shard index. A positive shard count larger than
the number of scheduled roots is clamped to that number. An empty selection
produces one stable empty shard. Zero or negative counts are
`SHARD_COUNT_INVALID`; an index outside the produced shard range is
`SHARD_INDEX_INVALID`. `package_names` is the transitive prerequisite closure
of each shard's direct assignments, and `estimated_cost` is the sum of every
closed package's cost. Every declared edge endpoint and scheduled name MUST
reference a declared package.

Starlark v1 injects `repo_root` into `_ctx` from the runner-owned workspace; it
is never accepted from fixture input. Repository `load()` labels are resolved
only against the immutable inline file table and cannot escape the root.
`//` labels and normalized labels without a `./` or `../` prefix are
repository-root-relative; explicit dot-prefixed labels are relative to the
loading module. Missing inline modules never fall back to the host filesystem.
Each fixture declares bounded step, recursion, load-depth, module-count,
aggregate-value, and diagnostic-output requests, all capped by stricter
runner policy. A target with structured commands reports
`command_source: "structured"`; a target that uses the declared generator
reports `"legacy_fallback"`. For process-free comparison,
`rendered_commands` is a deterministic display form, not executable authority:
tokens containing only ASCII letters, digits, `_`, `@`, `%`, `+`, `=`, `:`,
`,`, `.`, `/`, and `-` are emitted unchanged; every other token is emitted as a
JSON string; tokens are joined by one ASCII space. Trusted execution MUST use
the structured `program` and `args` through the platform executor rather than
executing this display string.

Starlark metering is deterministic behavior, not an implementation-specific
timeout. The `evaluation_limits` record has these meanings:

- `step_count` is a shared fuel budget. Entering a statement, evaluating an
  expression, attempting a function or built-in call, and producing one loop
  or comprehension iteration each consumes one step. Load evaluation consumes
  from the same budget. Exhaustion reports `STARLARK_STEP_LIMIT` before the
  next event and produces no target result.
- `recursion_depth` counts active user-function calls, including the first
  call. Crossing the limit reports `STARLARK_RECURSION_LIMIT`. Implementations
  that reject every recursive cycle earlier may report the same code; they
  MUST NOT overflow a native stack first.
- `load_depth` counts load edges from the entrypoint, whose depth is zero.
  Crossing it reports `STARLARK_LOAD_DEPTH_LIMIT`. `module_count` counts unique
  modules including the entrypoint and reports `STARLARK_MODULE_LIMIT` before
  evaluating the first module beyond the limit. A cycle in the active load
  chain reports `STARLARK_LOAD_CYCLE`, even when every member was already
  observed; completed-module caching is not permission to hide a cycle.
- `value_items` is a cumulative allocation budget. Each item materialized into
  a list, tuple, or dictionary consumes one unit; replacing a dictionary value
  does not refund or duplicate the key unit. Exhaustion reports
  `STARLARK_AGGREGATE_LIMIT` before the item becomes visible.
- optional `range_items` bounds the logical cardinality of any one `range`
  before iteration or allocation and reports `STARLARK_RANGE_LIMIT`. Optional
  `value_bytes` bounds the UTF-8 byte length of one string or the exact length
  of one bytes value and reports `STARLARK_VALUE_LIMIT`. For backward-compatible
  v1 inputs that omit either field, `value_items` supplies that ceiling.
- `output_bytes` bounds the combined UTF-8 bytes of `print` and evaluator trace
  events, including one line-feed byte per event. The implementation measures
  the complete event before emission; an event that would cross the ceiling is
  not partially emitted and reports `STARLARK_OUTPUT_LIMIT`.

All limit failures use `outcome: "error"`, an empty result, severity `error`,
and the repository-relative module path at which the next charged event was
rejected. No limit failure may fall back to legacy shell interpretation. Fuel,
allocation, and diagnostic budgets are independent: exhausting one cannot be
masked by unused capacity in another. The adversarial corpus includes one
positive case that reaches, but does not cross, every requested ceiling plus
negative cases for steps, recursion, aggregate allocation, range size, scalar
value size, load depth, module count, load cycles, and combined print/trace
output.

Validation v1 uses the stable diagnostic registry
`BUILD_FILE_MISSING`, `BUILD_FILE_EMPTY`, `LOCAL_DEPENDENCY_UNDECLARED`,
`STANDALONE_PREREQUISITE_MISSING`, `STARLARK_SOURCE_INVALID`,
`STARLARK_DEPENDENCY_INVALID`, `IDENTITY_AMBIGUOUS`, `MANIFEST_AMBIGUOUS`,
`TOOLCHAIN_UNSUPPORTED`, `PATH_UNSAFE`, `ORPHAN_CRATE_UNLISTED`,
`ORPHAN_CRATE_EMPTY_BUILD`, `ORPHAN_EXEMPTION_INVALID`, and
`ORPHAN_EXEMPTION_STALE`, `TRACKED_ARTIFACT_FORBIDDEN`, and
`TRACKED_ARTIFACT_PATH_INVALID`. The closed check registry is
`build_file_presence`, `local_dependency_declarations`,
`standalone_prerequisites`, `starlark_declarations`, `identity_uniqueness`,
`manifest_uniqueness`, `toolchain_support`, `path_safety`, and
`lua_windows_sibling_parity`, plus `orphan_crate_coverage` and
`tracked_artifact_absence`.

Every validation package carries one normalized snapshot: canonical package
identity and root, implementation language, selected BUILD state and local
references, Starlark source/dependency declarations, canonical identity-root
candidates, manifest candidates, and raw path candidates. Dependency edges are
ordered `[prerequisite, dependent]`; both endpoints MUST be declared, duplicate
edges are rejected by the closed schema, and cycles are invalid case input.
These records are data, never instructions:

- `local_dependency_declarations` reports every BUILD reference other than the
  package itself that is not in the package's transitive prerequisite closure;
- `standalone_prerequisites` applies to the fixed isolated-environment registry
  `python`, `typescript`, and `perl`, and reports transitive prerequisites that
  the selected BUILD does not reference;
- `starlark_declarations` reports source patterns that fail the portable-glob
  grammar and declared dependency names that are unknown or outside the
  package's transitive prerequisite closure;
- `identity_uniqueness` requires exactly one candidate root equal to the
  package's normalized root, while `manifest_uniqueness` permits at most one
  canonical manifest candidate;
- `toolchain_support` derives support only from the canonical v1 toolchain
  registry below; fixtures cannot assert support with a boolean; and
- `path_safety` applies the shared atomic repository-relative path validator to
  each raw candidate and reports unsafe strings in diagnostic details while
  retaining the package's safe normalized root as the diagnostic path.

Every produced diagnostic includes a stable code and severity plus the safe
package root or selected BUILD path. Package-scoped diagnostics include the
qualified package name and sorted machine-readable detail lists. The complete
diagnostic array is sorted by `(code, path, package, canonical details)`.
`outcome: "ok"` requires `valid: true` with no diagnostic codes. `outcome:
"error"` requires `valid: false`, one or more codes, and matching
diagnostic-envelope codes. A fixture's expected result is never evidence for a
check: the runner independently derives the diagnostic array from the input
snapshot and rejects a self-consistent but dishonest result.

Platform BUILD precedence is a discovery decision; validation does not invent
a missing-platform error when canonical `BUILD` fallback is available.
Validation input is the sole normalized repository-data snapshot for this
domain. It does not carry a second inline BUILD-file source of truth, and the
adapter MUST NOT consult the workspace or host checkout, Git, a process API, or
the network.

The Lua sibling-parity check additionally consumes
`canonical_lua_sibling_installs`, `windows_build_file_state`, and
`windows_lua_sibling_installs`. Every referenced sibling is a declared package
identity. A canonical sibling missing from the Windows set produces
`STANDALONE_PREREQUISITE_MISSING` at the package's `BUILD_windows` path,
including when `windows_build_file_state` is `missing`; Windows may contain
additional transitive siblings.

The process-free `orphan_crate_coverage` check consumes one closed
`orphan_snapshot`. Its sorted `directories` set is the bounded union of every
supplied Cargo-manifest directory and every existing exemption-target
directory; it is not an inventory of the full tree. Sorted `manifests` records
carry a normalized directory path and `package` or `virtual_workspace` kind.
Sorted `build_files` records carry the path and independently normalized
`runnable` or `empty` state of every relevant recognized BUILD. A BUILD path
must name one of `BUILD`, `BUILD_windows`, `BUILD_mac`, `BUILD_linux`, or
`BUILD_mac_and_linux`, and its directory must be beneath `code/`. Sorted
`exemptions` retain the bounded raw kind, path, and reason plus a unique,
strictly increasing source line so invalid policy data remains testable.

Every established front door that shares an engine MUST still expose a
language-native orphan-snapshot adapter and independently consume every
registered `orphan_crate_coverage` fixture through that adapter. Exercising the
shared engine only through a sibling language's test target is not front-door
coverage. The native adapter remains an inert type-and-result boundary: it MUST
NOT enumerate a checkout, inspect the filesystem, consult Git, launch a
process, read the environment, or access the network.

A package or virtual-workspace manifest is covered only by a runnable BUILD in
its own directory or a component-wise ancestor through `code/`. The closest
runnable ancestor wins using the fixed filename order above; a nearer empty
BUILD does not mask a runnable ancestor. When no runnable ancestor exists, the
closest empty BUILD identifies the empty-build diagnostic. Empty, blank, and
comment-only BUILD files are `empty`, never coverage. Manifest records below `.git`, `target`,
`node_modules`, `vendor`, `.venv`, `_build`, `deps`, `.build`,
`dist-newstyle`, or `.cargo` are build artifacts and are ignored. This skip
registry is exact, case-sensitive path-component matching; a similarly named
source directory is not ignored.

Each exemption is bounded data with a source line, raw kind, raw path, and
reason. `EXCLUDED` and `PENDING` are the only valid kinds and both suppress an
otherwise uncovered manifest. The reason must be non-empty. The path must be a
portable NFC repository-relative directory beneath `code/`, outside the
artifact skip registry, and occur at most once. Absolute, drive, UNC,
backslash, traversal, non-NFC, outside-scan, artifact, missing-reason,
unknown-kind, and normalized duplicate aliases produce
`ORPHAN_EXEMPTION_INVALID` at the fixed redacted ledger path
`code/BUILD-EXEMPTIONS`. Its details contain the source line and stable problem
only; a raw invalid path is never emitted. Invalid entries never suppress an
orphan.

A valid exemption is stale when its directory is absent from `directories`,
the directory no longer contains a supplied Cargo manifest, or the manifest is
now covered by a runnable BUILD. These
states produce `ORPHAN_EXEMPTION_STALE` and force ledger cleanup. An uncovered
manifest without a valid exemption produces `ORPHAN_CRATE_UNLISTED`; an empty
BUILD produces `ORPHAN_CRATE_EMPTY_BUILD`, including when the file contains
only blanks or comments. The result's required `pending_exemption_count` is the
number of structurally valid, non-stale `PENDING` entries and remains visible
even when there are no diagnostics. Diagnostics are independently derived,
retain only safe paths, and sort by the normal validation ordering. The snapshot and
expected result provide no filesystem, Git, process, environment, or network
authority.

The process-free `tracked_artifact_absence` check consumes one closed
`tracked_artifact_snapshot`. Its required `unicode_version` is exactly
`17.0.0`. NFC, NFKC, full default case folding, and locale-independent full
uppercase must all use that one reviewed Unicode data snapshot; adapters must
not inherit normalization or casing tables from the host runtime. Its `entries`
are bounded inert records with a
strictly increasing positive `ordinal`, a raw `path` of zero through 513
Unicode scalar values, and an
`entry_kind` of `regular`, `symlink`, or `reparse`. Entry kind is metadata, not
authority: every kind follows the same policy, and no path is opened or
followed. Native adapters may populate this snapshot from a retained Git index
or another reviewed source, but Git discovery, repository-root selection,
filesystem inspection, symlink resolution, and reparse-point inspection all
remain outside this oracle.

For each entry, the validator first replaces backslash separators with `/`.
The resulting path must be NFC, repository-relative, at most 512 Unicode
scalar values,
and satisfy the shared portable-path rules: no absolute, drive-qualified, UNC,
empty, dot, traversal, trailing-dot/space, control, reserved-character, or
Windows-reserved segments. Every empty component is invalid, including one
created by a trailing slash or backslash after separator normalization. Invalid
records produce
`TRACKED_ARTIFACT_PATH_INVALID` at the fixed redacted path `repository`; details
contain only `ordinal`, `entry_kind`, and one stable problem from `EMPTY`,
`TOO_LONG`, `NON_NFC`, `ABSOLUTE`, `DRIVE_QUALIFIED`, `EMPTY_SEGMENT`,
`DOT_SEGMENT`, `TRAILING_DOT_OR_SPACE`, `UNSAFE_CHARACTER`, or
`RESERVED_BASENAME`. Windows-reserved basenames are compared with the closed
ASCII reserved-name set after full Unicode uppercase mapping; this includes
the U+0131 DOTLESS I mapping needed to recognize `CONIN$`. The raw invalid path
is never emitted.

A valid normalized path is forbidden when any component has NFKC-casefolded
identity `node_modules`. This rejects the exact component, nested components,
case aliases, and Unicode compatibility aliases while allowing similarly
named components such as `node_modules-cache`. The diagnostic is
`TRACKED_ARTIFACT_FORBIDDEN` at the safe slash-normalized path with only the
entry ordinal and kind in details. Multiple diagnostics use the normal
`(code, path, package, canonical details)` ordering, where path comparison is
lexicographic by Unicode scalar value rather than UTF-8, UTF-16, or locale
order, so snapshot enumeration and runtime string representation cannot change
the result. Expected results are not evidence for either
classification, and the neutral validator has no filesystem, Git, process,
environment, or network authority.

Canonical result ordering is domain-aware:

- every package, path, toolchain, diagnostic-code, and invalidation set is
  lexicographically sorted;
- diff-selection sets are disjoint where
  `prerequisite_packages` excludes already affected packages;
- targets sort by `(rule, name)` and commands retain execution order;
- shards sort by index while their assignment, closure, and toolchain sets
  sort lexicographically; and
- toolchain maps contain every registry key even when all values are false.

The canonical v1 toolchain registry is:

`cpp`, `dart`, `dotnet`, `elixir`, `go`, `haskell`, `java`, `kotlin`, `lua`,
`ocaml`, `perl`, `python`, `ruby`, `rust`, `swift`, and `typescript`.

`c` and `cpp` packages map to `cpp`; C#, F#, and .NET map to `dotnet`; WASM
maps to `rust`. OCaml is present in this decision registry before its build-tool
implementation is promoted. Unknown package languages and unknown forced
toolchains are stable validation errors rather than new result-map keys.
`scheduled_packages: null` selects every supplied package, while `[]` selects
none. Forced toolchains are unioned into the derived set.

The process-free CLI record standardizes a canonical parser surface. Its input
contains `argv`, `dispatch_outcome`, and `requires_execution: false`. `argv`
accepts at most 64 non-empty UTF-8 tokens, each at most 256 Unicode scalar
values and at most 4,096 encoded bytes in aggregate. The fixture envelope
admits only the exact one-over boundaries of 65 tokens and 257 scalar values so
portable cases can prove their rejection without admitting unbounded input.
Only the following long-form spellings are accepted; a value flag accepts
either `--name value` or `--name=value`:

| Kind | Flags |
|---|---|
| Boolean | `--force`, `--dry-run`, `--validate-build-files`, `--no-validate-build-files`, `--detect-languages`, `--emit-shard-matrix`, `--clippy` |
| Integer | `--jobs`, `--shard-count`, `--shard-index` |
| Language | `--language` |
| Portable repository-relative path | `--root`, `--cache-file`, `--emit-plan`, `--plan-file` |
| Git ref data | `--diff-base` |

Every logical flag may occur at most once. Boolean flags do not accept values.
`jobs` and `shard-count` are in `1..256`; `shard-index` is in `0..255`.
`emit-plan` and `plan-file` are mutually exclusive. `shard-count` requires
`emit-plan`; `shard-index` requires `plan-file`; and `emit-shard-matrix`
requires both `emit-plan` and `shard-count`. The language value is `all` or a
closed discovery identifier: `c`, `cpp`, `csharp`, `dart`, `dotnet`, `elixir`,
`fsharp`, `go`, `haskell`, `java`, `kotlin`, `lua`, `mosaic`, `ocaml`, `perl`,
`python`, `ruby`, `rust`, `starlark`, `swift`, `twig`, `typescript`, or `wasm`.

The typed result always supplies deterministic normalized defaults after a
successful parse: absent `root`, `jobs`, plan paths, and shard values are
`null`; `language` is `all`; `diff_base` is `origin/main`; `cache_file` is
`.build-cache.json`; `validate_build_files` is true; and other booleans are
false. A literal `.` is accepted only for `root`; every other path is checked
lexically by the shared portable-path rules. No default may inspect the current
directory, CPU count, environment, platform, clock, or filesystem.

Argument-count, token-length, and aggregate-byte overflow is
`CLI_ARGUMENT_LIMIT`. The adapter-reserved flags `--conformance`,
`--workspace-root`, and `--output`
(including `--name=value`) are `CLI_ARGUMENT_RESERVED`. Response-file tokens
beginning with `@`, environment-assignment positionals, environment expansions,
shell metacharacters, redirection, and command substitution are
`CLI_ARGUMENT_UNSAFE`. Shell syntax includes grouping parentheses and the
Windows command escape `^`. Unsafe path values, including non-NFC and trailing-
slash forms, are `CLI_PATH_UNSAFE`. Git refs are slash-separated ASCII ref data;
no component may be empty, a dot form, begin with `.`, or end with `.` or
`.lock`, and range or trailing-slash forms are forbidden. One optional `~`
suffix must contain a canonical non-negative integer. Unknown,
duplicate, incomplete, out-of-range, positional, or inconsistent arguments are
`CLI_USAGE_INVALID`. Every parser rejection has exit code 2, omits the typed
parse record, emits exactly one stable error diagnostic, and does not echo the
rejected token. Overlapping failures use this precedence: argument limit,
reserved adapter flag, shell/environment/response-file syntax, path safety,
then ordinary usage. Once parsing succeeds, the supplied decision table
remains:

| Condition | Exit code |
|---|---:|
| `success` | `0` |
| `package_failure`, `validation_failure` | `1` |
| parser rejection | `2` |

Every process-free CLI case requires `requires_execution: false`; a true value
is rejected before workspace decoding. The reference runner independently
re-parses `argv` and rejects dishonest typed results. Native front-door
invocation and machine-output compatibility become conformance claims only
when a later sandbox executes each language front door.

## Security and trust boundary

A fixture is data supplied to a program that can execute commands. Therefore:

1. The runner MUST treat every fixture and adapter binary as untrusted by
   default. A fixture cannot
   grant itself trust with `trusted_execution`. Execution trust comes only from
   out-of-band runner policy: an explicit operator/CI flag plus one reviewed,
   domain-separated authority-bundle digest. Each closed authorization profile
   binds only the exact source revision and components it actually consumes.
   The initial capability-preflight profile binds no corpus, adapter, launcher,
   or executable case. A corpus digest alone is not authorization.
   Pull-request changes are untrusted until reviewed and approved out of band.
2. Every adapter run, including a pure domain, MUST be enclosed by the outer
   runner sandbox. Pure domains MUST NOT execute fixture commands, spawn child
   processes, use the network, or consult the host checkout.
3. Every adapter receives a fixed runner-owned sanitized environment. Fixture
   environment values remain inert JSON input and are never applied to the
   adapter process. A trusted execution adapter may pass only explicitly
   allowlisted `CONFORMANCE_*` keys to its sandboxed children. Execution
   fixtures MUST run with no inherited secrets, filesystem containment, and
   network disabled. If the
   runner cannot enforce both filesystem and network containment on the current
   platform, it MUST fail closed or report a non-passing `skipped` result; it
   MUST NOT execute the adapter or commands.
4. Materialization MUST reject absolute, parent, drive-relative, UNC, device,
   alternate-data-stream, NUL-containing, reserved-name, case-folding, Unicode
   normalization, symlink, and reparse-point escapes.
5. The runner MUST reject non-regular files and use handle-relative, no-follow
   operations (or an equivalent atomic "beneath root" primitive) for every
   materialization, read, write, copy, and output operation. A separate
   check-then-use containment check is insufficient because a link or reparse
   point can be swapped between the check and operation.
6. Fixture-controlled output paths MUST remain below a runner-owned temporary
   directory and use atomic replacement.
7. Plan `rel_path`, declared sources, load paths, and build commands are
   untrusted inputs. Parsing a plan MUST NOT execute commands.
8. Fixture limits are requests, not authority. The effective limit is the
   smaller of the fixture request and a runner-controlled hard ceiling. Hard
   ceilings cover input JSON bytes/depth, decoded file bytes/count, process
   count, CPU time, memory, wall time, combined stdout/stderr/result bytes, and
   workspace size for the adapter and its entire process tree. Output limits are
   enforced while streaming, not after capture. Limit exhaustion produces a
   stable diagnostic.
9. Results MUST redact the temporary root and MUST NOT contain environment
   values, credentials, usernames, or host-specific absolute paths.
10. Schema validation is necessary but not sufficient; semantic safety checks
    are mandatory in the runner and every adapter.
11. The runner resolves each trusted adapter executable and launches it with
    the fixed sanitized runner environment and a direct argument vector, never
    through a fixture-controlled shell command. The executable, case,
    workspace-root, and result paths are separate arguments even when they
    contain shell metacharacters.
12. Domain/capability consistency is semantic: only execution-domain cases may
    request `trusted_execution`, every execution case must request it, and the
    capability request never confers trust by itself. The runner also requires
    exact equality between top-level `domain` and `input.operation`.
13. Security invariants are runner-enforced and non-oracular. Fixture-controlled
    `expected`, `platforms`, `capabilities`, and `limits` values can never
    authorize host or network access, weaken hard ceilings, or turn a sandbox
    escape into a passing result.
14. The runner requires exact equality between fixture `id` and
    `expected.case_id`, and between fixture `domain` and `expected.domain`.
15. `input.arguments` remains inert JSON data. It MUST NOT be appended to the
    adapter invocation or alter the runner-owned case, workspace-root, or output
    arguments. Per-domain adapters interpret only registered options, and every
    path-valued option receives the same atomic beneath-root validation.
16. Before schema validation, the runner performs bounded strict UTF-8 RFC 8259
    parsing. It rejects a BOM, duplicate object keys at any depth, non-finite
    numbers, floating-point values, integers outside the interoperable range,
    invalid or unpaired Unicode, excessive nesting, and oversized raw input.
    Different language runtimes MUST NOT apply first-key/last-key or permissive
    numeric behavior to security decisions.

### 14. Trusted-execution delivery profile

Trusted execution is delivered in independently reviewable layers. A policy or
schema layer MUST NOT be treated as an execution sandbox:

1. The process-free policy layer closes the execution input and result records,
   captures one immutable exact-byte execution-corpus snapshot, computes its
   framed SHA-256 digest, validates runner-owned adapter and backend identities,
   and returns stable non-passing results for unavailable backends. It imports
   no process API and never materializes a workspace.
2. The Linux layer may execute only through a pinned, already-present OCI image
   identity. The runner MUST NOT pull, build, tag, or resolve a mutable image
   name during a conformance run. The container uses private mount, user, PID,
   and network namespaces; a read-only root filesystem; no new privileges; no
   capabilities; no host devices; a non-root identity; only explicit
   read-only inputs; a size-bounded writable tmpfs; cgroup-backed aggregate
   limits; streaming output accounting; and whole-container termination.
   The supported v1 runtime is local, rootless Podman at the fixed absolute
   path `/usr/bin/podman`, using local `crun`, cgroup v2, delegated `cpu`,
   `memory`, and `pids` controllers, and seccomp. Remote Podman, rootful
   Podman, Docker, mutable image references, PATH lookup, and best-effort
   fallback are not conforming backends.
3. The Windows layer requires a capability-less AppContainer or LPAC and
   private filesystem ACLs. The child is created suspended, assigned to a Job
   Object with kill-on-close, no-breakaway, process, CPU, and memory limits,
   and only then resumed. Every filesystem operation is root-handle-relative
   and rejects reparse points.
4. The macOS layer remains unavailable until a signed helper or isolated VM
   backend can prove filesystem and network containment, aggregate resource
   ceilings, bounded writable storage, and full descendant termination.
   `sandbox-exec`, a copied working directory, process groups, or `rlimit`
   alone do not satisfy this contract.
5. Execution semantics enter the reviewed corpus only after an enforcing
   backend exists. Command ordering, fail-stop, dependency skips, dry-run,
   jobs, resource locks, legacy shell behavior, and structured direct argv are
   fixture oracles. Filesystem escape, network denial, environment leakage,
   link races, cancellation, descendant termination, and every hard ceiling
   remain runner-owned invariant probes; fixtures cannot define them away.

The execution policy is separate from `implementations.json`. It contains an
exact conformance revision, exact execution-corpus SHA-256, runner-controlled
hard ceilings, backend identities, and adapter executable digests. A backend
or adapter marked ready requires an immutable SHA-256 identity. The policy does
not contain operator authorization: `run-case` additionally requires an
explicit `--allow-trusted-execution` invocation flag, an exact reviewed source
revision, and the corresponding out-of-band approved authority-bundle SHA-256.
The old corpus-only approval is deliberately insufficient.

The process-free authority bundle is an external, post-review artifact
described by `execution-authority.schema.json`. It is generated only after the
reviewed commit and required backend artifacts exist; a production bundle is
not checked into the commit whose identity it carries. The v1 preflight
profile's closed record binds:

- one exact authorization scope, repository, Linux platform/architecture, and
  full Git commit and tree object identities;
- the policy, authority, and Linux OCI identity schemas;
- the checked-in execution policy;
- the process-free bootstrap runner;
- the process-free authority verifier;
- the process-free Linux OCI result validator and the separately authorized
  process-owning capability broker;
- the exact raw Linux backend identity document stored beside the external
  bundle.

The Linux backend identity document transitively binds the reviewed Podman,
`crun`, and Conmon binaries, exact OCI manifest/config identities, seccomp profile,
in-image shim, and invariant probe. The bundle binds the exact raw identity
document; the verifier validates its typed fields and cross-field identities.
A future seccomp artifact or host execution launcher becomes authority only
after its exact raw bytes are a separate role. The current preflight
implementation MUST NOT be mislabeled as an enforcing execution launcher.

Bundle components are a closed object with eight fixed roles. Seven paths are
fixed repository-relative constants; the Linux identity is the sole
bundle-relative artifact and has a fixed filename. Missing, unknown, extra, or
role-swapped components fail schema validation. The verifier rejects a linked,
reparse, non-regular, or multiply linked final file, unsafe paths, exact
byte-length/digest mismatches, and source-identity mismatches. Component
identity is the retained approved bytes, not pathname topology. The protected
orchestrator proves the checkout tree; the later process handoff additionally
requires atomic beneath-root traversal and an exact-byte loader. These reads
remain process-free and never enumerate or decode an execution case.

Authorization scopes are cryptographically and semantically distinct. Schema
v1 admits only `linux_capability_preflight_v1`: it approves the exact
components intended for a future host/image capability inspection and binds an
empty execution corpus and no adapters. Its process-free verifier cannot
import a process backend, create a container, or decode a case. The exact-byte
loader, invariant-probe profile, and trusted-execution profile use distinct
domain-separated scopes and closed contracts after their corresponding
components exist; a digest for one scope is therefore structurally unusable
for another.

The authority SHA-256 uses this exact framing:

1. append the ASCII domain separator
   `coding-adventures/build-tool-authority/linux-capability-preflight/v1`
   followed by one NUL;
2. append the unsigned 64-bit big-endian exact raw-bundle byte length; and
3. append the exact bounded raw UTF-8 JSON bytes.

The bundle MUST NOT contain its own digest. Exact-byte identity is deliberate:
whitespace, key order, or a trailing newline changes the approval. The runner
checks the syntactically valid out-of-band digest with a constant-time
comparison before following any component path named by the bundle. It then
strictly parses the already-approved bytes, rejects duplicate keys and invalid
Unicode, validates the closed schema, and proves every component and semantic
cross-equality.

The protected orchestrator supplies the expected full commit and tree
identities from immutable runner metadata and proves the checkout is exactly
that revision before invoking repository code. The approved authority digest
is the sole out-of-band content approval. Recomputing a candidate digest inside
an untrusted pull request does not approve it.

For `linux_oci`, `identity_sha256` is the SHA-256 of the exact raw bytes of a
closed Linux OCI backend identity document validated by
`linux-oci-backend.schema.json`. That document binds the statically linked
Podman runtime, `crun`, and Conmon binaries, their fixed absolute paths, the
exact OCI manifest and local config image identities, the reviewed seccomp
profile, and the in-image shim and runner-owned invariant probe. The runtime
identity MUST declare `linkage` as `static`. The image `reference` contains the manifest
digest and MUST agree with `manifest_sha256`; a run addresses only the already
present `sha256:<config_sha256>` image with pull disabled. The runner verifies
both image identities, operating system, architecture, and absence of
image-declared volumes before it creates a container.

Linux capability validation is implemented in a separate backend module. Its
direct CLI is authority-disabled, and the process-free authority verifier MUST
NOT import it. The backend is a pure consumer of brokered command results: it
has no process import, pathname binary reopen, state-root construction, or
default command/digest implementation. The authority-gated broker proves a
non-root Linux amd64 process, a real cgroup-v2 delegated root with enabled
`cpu`, `memory`, and `pids` controllers, required kernel seccomp actions, exact
runtime binary hashes, and the exact prepopulated local image. It forces local
Podman mode and rejects a malformed, non-amd64, or dynamically linked Podman
ELF before command rendering. Before Podman starts, the broker closes every
descriptor outside its exact command allowlist, installs a Landlock ABI-v1
execute ruleset whose sole allow-rule is the already-retained Podman inode, and
installs an amd64 classic-seccomp filter. Podman starts through
`/proc/self/fd/<retained-fd>` pathname `execve`, while pre-exec hooks,
`catatonit`, every other pathname-backed helper, `execveat`, anonymous
executable memfds, executable mappings, and descriptor receipt or acquisition
through `recvmsg`, `recvmmsg`, `pidfd_getfd`, or `open_by_handle_at` are denied
by the inherited kernel policy regardless of `PATH`. This closes
post-transition executable creation and descriptor replenishment in
broker-command descendants; it does not remove the already-loaded
protected Python, standard-library, native-extension, libc, or loader
runner-image TCB.
The broker uses the closed `version --format json` request instead of
`podman info`, whose host inspection executes configured helpers and package
managers outside this authority. The process-free backend validates only the
bounded local version and image results. Missing or mismatched capabilities
produce a stable non-passing result before any fixture is decoded or
materialized.

The exact loader is a separate loadability-only prerequisite, not yet the
capability-command handoff. Its authority record is described by
`execution-preflight-loader-authority.schema.json`, uses authorization scope
`linux_capability_preflight_loader_v1`, and is approved with this distinct
framing:

1. append the ASCII domain separator
   `coding-adventures/build-tool-authority/linux-capability-preflight-loader/v1`
   followed by one NUL;
2. append the unsigned 64-bit big-endian exact raw-bundle byte length; and
3. append the exact bounded raw UTF-8 JSON bytes.

The loader profile binds exactly ten roles: its own authority schema, the
execution policy and schema, the Linux identity schema, the process-free
bootstrap and authority verifier, the exact loader, the stdlib-only Linux
preflight backend, the closed backend import manifest, and the external Linux
identity. The protected source commit/tree remains mandatory. The older
eight-role `linux_capability_preflight_v1` bundle cannot be upgraded or reused.

The loader profile and implementation have these requirements:

1. Authority validation opens repository and bundle roots once, traverses
   every fixed component path one segment at a time relative to retained
   directory handles, and applies no-follow, directory, close-on-exec,
   bounded-read, stable-file, and singly-linked regular-file checks. Absolute,
   empty, dot, linked, and separator-containing path segments fail closed.
2. The stage copies the exact retained loader, backend, import-manifest, and
   identity bytes into anonymous Linux memory files and applies write, grow,
   shrink, and seal seals before a worker sees them. If handle-relative
   traversal, anonymous files, or sealing are unavailable, it fails closed.
3. One fresh worker starts with the protected interpreter using `-I -S -B`, a
   fixed scrubbed environment, no checkout directory on `sys.path`, no bytecode
   cache, no caller modules, and only the sealed component descriptors. The
   interpreter, standard library, native extensions, libc, and dynamic loader
   are explicitly part of the protected runner-image TCB.
4. The worker strictly parses the closed import manifest, rejects invalid
   UTF-8, a BOM, NUL, relative or wildcard imports, and any import not listed
   exactly. It rejects executable module/class statements, decorators,
   defaults, comprehensions, or dynamic imports outside the closed static
   profile, then compiles the sealed backend bytes without executing them.
   Every declared import root must be reviewed standard library. The backend
   has no repository or third-party dependency.
5. The worker verifies the exact loader/backend/manifest/identity digests and
   the backend's structural `preflight_brokered`, unavailable-error, and
   command-result declarations. It does not execute
   backend module code, invoke
   preflight, inspect Podman, open an execution case, construct a container
   argv, or retain a reusable worker.
6. The parent bounds the worker protocol, timeout, output, descriptor set, and
   exit status. Success is only a loadability receipt binding the authority
   digest, protected source IDs, and exact component digests. It is not
   capability, containment, or readiness evidence.

The protected capability-command broker has a third, non-reusable authority
record described by
`execution-capability-broker-authority.schema.json`. It uses authorization
scope `linux_capability_preflight_broker_v1` and this distinct exact-byte
framing:

1. append the ASCII domain separator
   `coding-adventures/build-tool-authority/linux-capability-preflight-broker/v1`
   followed by one NUL;
2. append the unsigned 64-bit big-endian exact raw-bundle byte length; and
3. append the exact bounded raw UTF-8 JSON bytes.

The broker profile binds exactly thirteen roles: its authority schema; the
execution policy and schema; the Linux identity schema; the process-free
bootstrap and authority verifier; the exact loader; the process-free Linux
preflight backend; the broker-specific closed backend import manifest; the
capability broker; the language-neutral broker behavior manifest and its exact
schema; and the external Linux identity. The protected source commit/tree
remains mandatory. The eight-role preflight and ten-role loader authorities
cannot be upgraded or reused.

The broker behavior is closed by
`linux-capability-preflight-broker.schema.json` and
`linux-capability-preflight-broker.json`:

1. The protected parent verifies the bundle first, retains the repository and
   bundle roots, and passes only sealed approved source/configuration
   descriptors to a fresh isolated worker. The protected interpreter, standard
   library, native extensions, libc, and dynamic loader remain the immutable
   runner-image TCB.
2. The worker opens `/usr/bin/podman`, `/usr/bin/crun`, and `/usr/bin/conmon`
   once with no-follow
   and close-on-exec controls, then requires each to be a root-owned,
   group/world-non-writable, executable, regular file without set-user-ID or
   set-group-ID bits. It hashes retained bytes and compares stable pre/post
   `fstat` identities including ctime. It also parses Podman's bounded ELF64
   program-header table and rejects any `PT_INTERP`, malformed layout, or
   architecture other than amd64. This static-linkage requirement keeps the
   dynamic loader from becoming an allowed execution trampoline. Podman is
   executed from the retained descriptor. The exact command supplies the reviewed absolute
   `/usr/bin/crun` and `/usr/bin/conmon` paths, whose bytes are independently
   retained and verified. This preflight does not claim that Podman reports
   those paths or executes either binary. Later container execution still
   requires the immutable runner-image TCB receipt to bind their absolute
   pathnames and all remaining ambient dependencies.
3. The protected orchestrator supplies an already-open private state-root
   descriptor containing only a private `storage` child prepopulated with the
   reviewed image. The broker retains that child and creates exactly `config`,
   `home`, `runtime`, and `runroot` relative to the root, with mode `0700`,
   no-follow semantics, same-owner checks, and no path-based reopen. Command
   arguments and environment locate retained state children only through
   `/proc/self/fd/<fd>`. The broker never pulls or imports an image.
4. The broker renders exactly two direct Podman requests in order:
   `--remote=false version --format json` and `--remote=false image inspect
   --format json sha256:<approved-config>`. Only the image request carries the
   exact retained root, runroot, reviewed `crun` and Conmon paths, and `vfs`
   global options from the manifest. `PATH=/nonexistent` makes unexpected
   ambient lookup fail closed. Before the final sandbox transition, the child
   closes every descriptor outside the manifest's allowlist. A Landlock
   execute-only filesystem ruleset permits pathname-backed execution only of
   the retained static Podman inode, and Podman starts via pathname `execve` of
   `/proc/self/fd/<retained-fd>`. An amd64 classic-seccomp filter kills
   architecture mismatch and the x32 syscall space; denies `execveat`,
   `memfd_create`, `memfd_secret`, `recvmsg`, `recvmmsg`, `pidfd_getfd`,
   `open_by_handle_at`, `uselib`, and `io_uring_*`; and returns `EPERM` for
   `mmap(PROT_EXEC)`, `mprotect(PROT_EXEC)`, `pkey_mprotect(PROT_EXEC)`, and
   `shmat(SHM_EXEC)`. Both restrictions are inherited by every fork, namespace
   transition, and re-exec. The broker refuses to run if either transition
   cannot be installed. This prevents broker-command descendants from creating
   new anonymous executable mappings or executing unapproved pathname- or
   FD-backed code after the transition; it does not claim total host-code
   provenance or remove the protected interpreter/stdlib/libc/loader from the
   runner-image TCB. No fixture, caller argument, shell, host environment,
   network request, image pull, container operation, adapter, or invariant
   probe can alter this grammar.
5. Each command has null stdin, a retained `home` cwd, an exact scrubbed
   environment, and the manifest's closed descriptor set. Standard output and
   standard error are read concurrently in nonblocking mode under one
   262144-byte aggregate streaming ceiling. The command deadline is 15000
   milliseconds. Partial output after a limit, timeout, or cleanup failure is
   rejected.
6. Before releasing the child to execute, the broker puts it in a fresh
   delegated cgroup-v2 child, creates a private session, installs parent-death
   termination, and acts as a subreaper. Absence of delegation or `cgroup.kill`
   fails before spawn. On every failure and after normal exit, the broker uses
   `cgroup.kill`, verifies `cgroup.events` reports `populated 0`, supplements
   with process-group termination, reaps adopted descendants, and completes
   cleanup within 5000 milliseconds.
7. The internal worker protocol contains only the two bounded command results
   or one closed error. It accepts no loader/backend source and carries no
   authority digest, source identity, conformance status, or readiness claim.
   The authority-gated parent alone loads the exact approved process-free
   backend, strictly validates the internal protocol, and adds authority/source
   bindings to both passing and non-passing receipts.
8. Successful command results are handed to the process-free backend's
   `preflight_brokered` interface. JSON depth exhaustion and every malformed
   runtime response produce the stable
   `LINUX_OCI_RUNTIME_RESPONSE_INVALID` diagnostic. A successful result proves
   only capability preflight; it creates no execution case and does not mark
   Linux containment or trusted execution ready.

The broker parent CLI is authority-gated. Worker mode is an internal,
non-authoritative process boundary intrinsically unable to accept executable
Python components or emit an authority/readiness receipt. The bare Linux backend
CLI remains disabled. Pull-request CI exercises schemas plus Linux retained-FD,
pathname/FD-exec, executable-mapping, stream, limit, timeout, and
descendant-cleanup helpers. Real Podman
inspection belongs to a protected no-secrets, read-only reviewed-revision
workflow supplied with the approved broker-authority digest and immutable
runner-image TCB.

The first Linux delivery tranche contains identity validation, capability
preflight logic, and construction of the runner-owned invariant-probe
container argv. It does not invoke that argv and MUST NOT decode or execute
fixture commands. The candidate argv specifies an exact config image identity
and `--pull=never`, a private unmapped rootless user namespace, private
PID/IPC/UTS/cgroup namespaces, network disabled, a read-only root with
implicit writable tmpfs mounts disabled, all capabilities dropped,
`no_new_privileges`, no devices or bind mounts, a fixed non-root identity and
environment, and one bounded runner-owned tmpfs. A zero-byte workspace is
rejected rather than expressed as `tmpfs size=0`, which can mean unlimited.
Swap is disabled with the cgroup v2 `memory.swap.max=0` control;
`--memory-swap=<memory>` is not used. CPU quota limits rate only, so later
execution MUST also meter aggregate `cpu.stat usage_usec` and kill the entire
container cgroup at the effective CPU-time ceiling.

The checked-in execution policy remains disabled and Linux remains
`unavailable` until the exact identity document and image have passed every
runner-owned filesystem, network, environment, namespace, resource, output,
cancellation, and descendant-termination probe in a protected Linux
workflow. Preflight success alone never marks the backend ready.

The execution-corpus snapshot is a closed, process-free exact-byte boundary:

1. Capture opens the absolute `execution-cases/` directory without following a
   link or reparse point and retains that root while it enumerates and reads
   members. POSIX capture traverses and opens members relative to retained
   directory descriptors. Windows capture requires a fixed local drive whose
   DOS target is one non-remappable hard-disk volume, retains a non-reparse
   directory chain, enumerates the final root by handle, matches each member's
   volume serial and file identity to that enumeration, and prevents pathname
   mutation while each no-follow member handle is opened. An implementation
   that cannot provide these guarantees fails closed.
2. The corpus contains only direct, lowercase-`.json` members. Names are
   portable relative paths in exact NFC form, contain no separator, and are
   unique under NFC plus Unicode case folding. Non-regular, linked, reparse,
   multiply linked, oversized, or identity-aliased members fail closed.
   Enumeration stops after 4096 directory entries, the corpus contains at most
   256 cases, each case is at most 2000000 raw bytes, and the complete retained
   snapshot is at most 16777216 raw bytes.
3. Each member is opened once, read under the runner byte ceiling, and checked
   for a stable device/file identity, link count, size, and modification/change
   identity before and after the read. The directory membership is checked
   again before capture completes. The snapshot retains the sorted names and
   immutable raw `bytes`; later validation or selection does not reopen a
   pathname.
4. A typed selector accepts exactly one canonical direct member name. An unsafe
   name, a case- or normalization-alias of a retained member, and a missing
   member are distinct stable failures. A successful selection binds the
   requested name, the snapshot's corpus digest, and the already-retained exact
   bytes. Renaming, replacing, linking, or mutating the pathname after capture
   cannot change the selected bytes. Member, snapshot, and selection records
   have factory-only constructors; the snapshot factory revalidates every
   member and computes the digest rather than accepting one from a caller.
5. Snapshot capture, digesting, and selection do not parse a case, import a
   process API, confer authority, mark an adapter or backend ready, or execute
   fixture content.

The execution-corpus digest is SHA-256 over the snapshot's members sorted by
their direct portable name relative to `execution-cases/`. For each case,
append the unsigned 64-bit big-endian path-byte length, UTF-8 path bytes,
unsigned 64-bit big-endian raw-content length, and exact retained bytes. The
empty corpus therefore has the standard SHA-256 empty digest. Any byte,
filename, addition, or removal changes the internal corpus identity; it does
not grant authority.

The process-free `validate-corpus` and `validate-result` bootstrap commands
remain unchanged and MUST reject execution intent. A separate execution
contract validator may validate schemas, semantic invariants, policy, and
digests without decoding executable file payloads, setting permissions,
creating a workspace, probing a toolchain, importing a process API, or
launching anything.

Pull-request workflows may run only these process-free checks and fake-backend
unit tests. They MAY print an unapproved candidate bundle for review. Real
execution requires a protected reviewed revision, read-only repository
permissions, no repository secrets, and the approved authority-bundle digest
supplied out of band. `pull_request_target` MUST NOT execute changed fixtures
or runner code.

## Capability registry

V1 capabilities are:

| Capability | Final parity |
|---|---:|
| `discovery` | required |
| `resolution` | required |
| `graph` | required |
| `diff_selection` | required |
| `hashing_cache` | required |
| `starlark` | required |
| `plan_v1_read` | required |
| `plan_v1_write` | required |
| `sharding` | required |
| `execution` | required |
| `validation` | required |
| `toolchain_detection` | required |
| `cli` | required |
| `trusted_execution` | required, platform-gated |

An implementation manifest added by the fixture-runner work records supported
capabilities and reviewed temporary expected failures. A missing capability
never becomes an implicit pass.

## Delivery and completion gates

The conformance program proceeds in dependency order:

1. land this contract, the v1 fixture schema, and schema self-tests;
2. add the cross-implementation runner and representative fixtures;
3. make the Go operational reference pass, including structured Starlark and
   B05 behavior;
4. remediate existing implementations in fixture-failure order;
5. add Java, Kotlin, and Dart build tools;
6. add the OCaml build tool after the OCaml lane substrate is stable;
7. decide the C/C++ graduation and native build-tool requirement; and
8. run every supported implementation against the same corpus in CI.

The project is not at build-tool parity until:

- every required domain has positive and adversarial cases;
- all established implementations pass all applicable required cases or have a
  reviewed shared-engine exception;
- Go-emitted plans are consumed by every implementation and every
  implementation emits plans consumed by Go;
- Linux, macOS, and Windows execution cases pass where applicable;
- zero unreviewed expected failures remain; and
- the parity roadmap records the conformance revision for every front door.
