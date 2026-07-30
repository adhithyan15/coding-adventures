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
- recognize repository-supported legacy and current package-name aliases;
- keep dependency scope within the implementation ecosystem unless metadata
  explicitly names another qualified package;
- reject self-edges, ambiguous manifests, ambiguous aliases, and duplicate
  package identities;
- emit sorted, unique internal edges; and
- report malformed metadata with stable diagnostics instead of silently
  inventing a partial graph.

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

Starlark evaluation MUST NOT read undeclared host files, environment variables,
network resources, clocks, random sources, or process APIs.

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

### 10. Validation

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
| `validation` | platform, selected checks, normalized package declarations, and dependency edges | `valid` plus sorted stable diagnostic codes |
| `toolchain_detection` | package-language records, `null`/empty/explicit package selection, and forced toolchains | the complete canonical toolchain registry as a sorted boolean map |
| `cli` | a portable action, decision condition, and whether the action would require later execution | exit code only |

These records intentionally model decisions, not host operations:

- diff selection receives `changed_paths`; it never invokes Git;
- hashing receives inline bytes; it never reads host metadata;
- Starlark receives inline source and context; it never executes a command;
- validation inspects inline repository data only;
- toolchain detection never probes installed programs; and
- CLI fixtures classify exit decisions without standardizing native argument
  grammar, invoking a front door, or launching a build.

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

Validation v1 uses the stable diagnostic registry
`BUILD_FILE_MISSING`, `BUILD_FILE_EMPTY`, `LOCAL_DEPENDENCY_UNDECLARED`,
`STANDALONE_PREREQUISITE_MISSING`, `STARLARK_SOURCE_INVALID`,
`STARLARK_DEPENDENCY_INVALID`, `IDENTITY_AMBIGUOUS`, `MANIFEST_AMBIGUOUS`,
`TOOLCHAIN_UNSUPPORTED`, and `PATH_UNSAFE`. `outcome: "ok"` requires
`valid: true` with no diagnostic codes. `outcome: "error"` requires
`valid: false`, one or more codes, and matching diagnostic-envelope codes.
Platform BUILD precedence is a discovery decision; validation does not invent
a missing-platform error when canonical `BUILD` fallback is available.
Validation input is the sole normalized repository-data snapshot for this
domain. It does not carry a second inline BUILD-file source of truth, and the
adapter MUST NOT consult the workspace or host checkout.

The closed process-free v1 record currently exposes only
`build_file_presence`, whose result is derived exactly from each package's
normalized `build_file_state`. The other diagnostic families above remain the
target contract, but they are not valid pure-domain check values until their
inputs and deterministic semantic oracles are added. A self-consistent result
is never sufficient evidence for an unmodeled validation check.

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

The process-free CLI record is a decision table only:

| Condition | Exit code |
|---|---:|
| `success` | `0` |
| `package_failure`, `validation_failure` | `1` |
| `invalid_usage`, `unsafe_input` | `2` |

Every process-free CLI case requires `requires_execution: false`; a true value
is rejected before workspace decoding. Native argument parsing and
machine-output compatibility become conformance claims only when a later
sandbox executes each language front door.

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
   computes a framed SHA-256 digest over the execution corpus, validates
   runner-owned adapter and backend identities, and returns stable
   non-passing results for unavailable backends. It imports no process API and
   never materializes a workspace.
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
- the process-owning Linux OCI backend implementation;
- the exact raw Linux backend identity document stored beside the external
  bundle.

The Linux backend identity document transitively binds the reviewed Podman and
`crun` binaries, exact OCI manifest/config identities, seccomp profile,
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
`linux-oci-backend.schema.json`. That document binds the Podman and `crun`
binaries, their fixed absolute paths, the exact OCI manifest and local config
image identities, the reviewed seccomp profile, and the in-image shim and
runner-owned invariant probe. The image `reference` contains the manifest
digest and MUST agree with `manifest_sha256`; a run addresses only the already
present `sha256:<config_sha256>` image with pull disabled. The runner verifies
both image identities, operating system, architecture, and absence of
image-declared volumes before it creates a container.

Linux capability preflight is implemented in a separate process-owning module.
Its direct CLI is authority-disabled, and the process-free authority verifier
MUST NOT import it. A later loader must establish that the exact retained
backend bytes and approved import closure are the code being invoked before it
may use the module's fixed direct argument vectors and runner-owned sanitized
environment. Capability inspection then MUST prove local non-remote rootless
operation, `crun`, cgroup v2 with delegated `cpu`, `memory`, and `pids`
controllers, seccomp, exact runtime binary hashes, and the exact local image.
Missing or mismatched capabilities produce a stable non-passing result before
any fixture is decoded or materialized.

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
   the backend's structural `preflight_prevalidated`, unavailable-error, and
   command-result declarations. It does not execute backend module code, invoke
   preflight, inspect Podman, open an execution case, construct a container
   argv, or retain a reusable worker.
6. The parent bounds the worker protocol, timeout, output, descriptor set, and
   exit status. Success is only a loadability receipt binding the authority
   digest, protected source IDs, and exact component digests. It is not
   capability, containment, or readiness evidence.

A later protected capability-command broker must retain an atomic private state
root, execute already-open verified runtime binaries or rely on an attested
immutable runner image, allow only Podman `info` and exact-config `image
inspect`, stream a combined output cap, enforce time and complete-descendant
cleanup, and return stable non-passing capability diagnostics. Until that
broker and its own authority profile land, the bare Linux CLI remains disabled
and no real preflight runs. This loader cannot mark Linux ready.

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

The execution-corpus digest is SHA-256 over all `*.json` cases sorted by their
portable path relative to `execution-cases/`. For each case, append the
unsigned 64-bit big-endian path-byte length, UTF-8 path bytes, unsigned 64-bit
big-endian raw-content length, and exact raw bytes. The empty corpus therefore
has the standard SHA-256 empty digest. Any byte, filename, addition, or removal
changes the internal corpus identity; it does not grant authority.

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
