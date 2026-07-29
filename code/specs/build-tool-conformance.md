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
- changes outside known packages according to explicit fixture policy.

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

## Security and trust boundary

A fixture is data supplied to a program that can execute commands. Therefore:

1. The runner MUST treat every fixture and adapter binary as untrusted by
   default. A fixture cannot
   grant itself trust with `trusted_execution`. Execution trust comes only from
   out-of-band runner policy: an explicit operator/CI flag plus a reviewed,
   immutable corpus digest. Pull-request changes are untrusted until reviewed
   and merged into that policy.
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
