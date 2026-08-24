# Changelog

## 2026-08-24

- Added one process-free tracked-artifact Unicode boundary case. It makes empty
  and 513-scalar invalid paths schema-reachable, proves the 512-scalar valid
  boundary with astral input, pins diagnostic path ordering to Unicode scalar
  values, and exercises full-uppercase Windows reserved-basename matching with
  U+0131 DOTLESS I.
- Added four process-free tracked-artifact validation cases for clean source
  paths, exact and nested `node_modules`, separator normalization, case and
  Unicode compatibility aliases, inert symlink/reparse metadata, sorted
  diagnostics, and fixed-path redaction of unsafe index records. The neutral
  oracle consumes a bounded closed snapshot and performs no Git, filesystem,
  process, environment, or network operation.
- Added four process-free orphan-crate validation cases for direct, ancestor,
  and platform BUILD coverage; exact artifact exclusion; unlisted and empty
  crates; reasoned EXCLUDED and countable PENDING entries; invalid redacted
  ledger records; and stale covered, missing-directory, or removed-manifest
  entries. The runner derives coverage from a bounded normalized snapshot and
  rejects invalid exemptions without filesystem, Git, process, environment,
  or network authority.
- Replaced CLI exit-decision-only input with a bounded inert `argv` grammar,
  deterministic typed parse results, and an independent parser oracle.
  Twenty-seven new positive and adversarial cases cover separated/equals
  values, Unicode-scalar and UTF-8-byte limits, reserved adapter flags, unsafe
  paths and Git-ref components, shell and environment syntax, response files,
  duplicate options, numeric bounds, conflicting modes, and missing values
  without host reads or dispatch.
- Closed validation v1 with six language-neutral positive and adversarial
  cases for local dependency declarations, standalone prerequisites, Starlark
  sources and dependencies, package-root and manifest uniqueness, toolchain
  support, and path safety. The process-free runner now derives all nine
  validation checks from bounded normalized snapshots and rejects dishonest
  result assertions, unknown endpoints, duplicate identities, and cycles.
- Added one bounded positive Starlark metering case plus stable adversarial
  oracles for step fuel, recursion, aggregate allocations, range cardinality,
  scalar bytes, load depth, module count, load cycles, and combined print/trace
  output. The closed v1 limit record now accepts optional per-range and
  per-value ceilings while older records inherit `value_items`.

## 2026-08-13

- Added a language-neutral repeated plan-write case that requires atomic
  replacement of an existing destination and cleanup of the writer temporary
  file on Windows and POSIX. The runner requires write capability and validates
  both the existing and replacement plans before adapter execution.

## 2026-08-12

- Expanded the canonical discovery registry with a Dart program identity and
  a generated `.dart_tool` decoy so consumers must preserve the `programs`
  segment while excluding generated Dart package trees.
- Strengthened Dart resolution with a declared-name versus canonical-directory
  collision that must fail closed rather than redirecting a dependency edge.
- Expanded the canonical discovery registry with paired C# and F# package and
  program identities so every consumer must preserve the `programs` segment
  before exposing the established .NET lanes as filters.

## 2026-08-11

- Expanded the canonical discovery registry with colliding package/program
  basenames for Haskell, Java, and Kotlin so every consumer must retain the
  `programs` identity segment for the newly exposed Python filter lanes.
- Added a Cabal `dist-newstyle` decoy to the discovery registry so generated
  Haskell build output can never become a package or break a repository scan.
- Strengthened the Haskell field-aware case with plain directory and declared
  Cabal aliases plus fail-closed ambiguous root manifests.
- Strengthened the Java and Kotlin Gradle cases with duplicate declarations,
  nested block comments, interpolated unknown paths, and multiline real calls.

## 2026-08-10

- Added an adversarial ecosystem-scoped alias-resolution case. Same-spelled
  Lua, Perl, Python, and Haskell packages must resolve locally, while one exact
  qualified BUILD dependency preserves an intentional cross-language edge.

## 2026-08-08

- Added the process-free `lua_windows_sibling_parity` validation check and an
  absent-`BUILD_windows` fixture that requires the complete canonical Lua
  sibling-install set with `STANDALONE_PREREQUISITE_MISSING` diagnostics.

## 2026-08-03

- Expanded the canonical language-registry case with representative Elixir,
  Go, and Rust programs whose identities retain the `programs` segment.
- Added a language-neutral legacy BUILD dependency-comment case that preserves
  the `lua/conduit` to `lua/programs/conduit-hello` edge.
- Expanded the structured Starlark command/context case to use the canonical
  multiline target return shape, including trailing commas, so adapters must
  parse the same form used by repository rule helpers.

## 2026-08-02

- Added discovery cases for every canonical language bucket missing from the
  Rust registry, canonical fixture-tree exclusion, and fail-closed duplicate
  qualified identities with the stable `DUPLICATE_PACKAGE_IDENTITY`
  diagnostic.
- Added Elixir resolution cases that preserve distinct package/program
  identities and reject genuine dependency self-edges with the stable
  `DEPENDENCY_SELF_EDGE` diagnostic.
- Declared strict UTF-8 as the portable metadata text contract for dependency
  resolution and reserved `METADATA_INVALID_UTF8` for deterministic failures.
- Added positive Unicode and adversarial invalid-byte Lua rockspec cases so
  every build-tool implementation has the same resolution oracle.

## 2026-07-31

- Replaced pathname `lstat`/glob/reopen execution-corpus reads with one
  retained-root, bounded, exact-byte snapshot on POSIX and Windows.
- Added frozen typed member, snapshot, and selection records so the selected
  case bytes are exactly the bytes covered by the framed corpus digest.
- Made those typed records factory-only and added fixed enumeration, member,
  per-case, and aggregate-byte ceilings so callers cannot forge a digest-bound
  record or grow an unbounded in-memory snapshot.
- Required Windows snapshots to come from a fixed non-remappable local volume,
  bound member volume/file identities to the retained root enumeration, and
  made CLI error JSON host-path-independent.
- Added stable rejection of unsafe or aliased selectors, case/Unicode filename
  collisions, links and reparses, multiply linked or identity-aliased members,
  unstable reads, changed directory membership, and post-digest pathname
  substitution without granting execution authority.
- Replaced the duplicated `dependency-skipped` execution status with the
  normative `dep-skipped` vocabulary.
- Closed command status/exit-code and package status/return-code combinations
  in both execution result schemas.
- Added dry-run, overall-outcome, fail-stop command ordering, failed-command
  return-code equality, and dependency-propagation invariants before execution
  cases may enter the corpus.

## 2026-07-30

- Added the separately domain-bound thirteen-role capability-broker authority
  schema, broker-specific process-free backend import manifest, and exact
  language-neutral behavior manifest/schema.
- Added retained verified Podman, `crun`, and Conmon descriptors, a
  handle-relative prepopulated image store plus private transient state, a
  fixed two-command grammar and environment, combined nonblocking output
  accounting, hard timeouts, and delegated cgroup-v2 descendant cleanup.
- Added mandatory Landlock ABI-v1 execute confinement whose sole allow-rule is
  the retained Podman inode, plus a closed `linkage: static` identity field and
  bounded ELF64 program-header validation that rejects `PT_INTERP`. Podman may
  re-exec itself without admitting a dynamic-loader execution trampoline, while constructor
  hooks, `catatonit`, and every other pathname-backed helper in the reviewed
  flow fail closed.
- Closed post-transition anonymous and FD-backed execution by moving the
  retained Podman handoff to pathname `execve`, closing unlisted descriptors,
  and installing an inherited amd64 classic-seccomp filter that rejects
  `execveat`, `memfd_create`, `memfd_secret`, descriptor receipt or
  acquisition, executable mappings, `SHM_EXEC`, `uselib`, `io_uring_*`, x32
  syscalls, and architecture mismatch.
- Kept the isolated worker protocol non-authoritative: it accepts no loader or
  backend source and returns only bounded command results or one closed error;
  the authority-gated parent alone loads the approved backend and binds source
  and authority identities to passing and non-passing receipts.
- Removed process and pathname-reopen authority from the Linux preflight
  backend. Its sole brokered interface now consumes only bounded command
  results and treats recursive/depth-exhausting runtime JSON as a stable
  invalid response.
- Kept the broker scope capability-only: no fixture, adapter, container,
  invariant probe, or Linux trusted-execution readiness is authorized.
- Added a separately domain-bound exact-loader authority schema with ten fixed
  component roles and a closed standard-library import manifest.
- Added retained-directory-handle component reads, sealed anonymous component
  copies, and a fresh `python -I -S -B` loadability worker that runs the exact
  approved loader bytes with a scrubbed environment while compiling but never
  executing the approved backend module.
- Made the Linux backend import closure standard-library-only and split out the
  formally prevalidated interface. Loader validation checks the interface but
  never calls it, Podman, a fixture, an adapter, or a container.
- Kept capability inspection disabled and recorded its protected command broker
  as a required dependency before invariant-probe authority.
- Added the closed external authority-bundle schema and exact
  domain/length/raw-byte approval digest for Linux capability preflight.
- Added a process-free authority verifier that binds the protected source
  commit/tree, eight fixed component roles, disabled empty execution policy,
  and external Linux backend identity before any capability command.
- Replaced corpus-only `run-case` approval with fail-closed external-authority
  inputs. The preflight-only scope remains structurally unable to authorize a
  case, adapter, invariant-probe container, or trusted execution.
- Disabled the bare identity-only Linux CLI entry point. This tranche exposes
  no process handoff; an exact-byte backend loader remains a required
  dependency before protected capability inspection.

## 2026-07-29

- Added the process-free bootstrap conformance runner.
- Added standalone result, implementation-inventory, and build-plan schemas.
- Added the 16-lane implementation inventory with 12 present front doors,
  three missing established implementations, and emerging OCaml.
- Added seven representative discovery, resolution, graph, and plan cases.
- Added bounded parsing, two-phase in-memory workspace preflight,
  domain-aware canonical comparison, and fail-closed execution rejection.
- Expanded the corpus from seven cases and four domains to 30 cases covering
  all 11 process-free v1 domains.
- Added a closed pure-domain schema, conservative unknown-path handling,
  framed hash/cache oracles, bounded inline-only Starlark records,
  prerequisite-closed shard verification, normalized BUILD-file validation
  snapshots, the complete toolchain registry including OCaml, and CLI exit
  decisions.
- Added semantic reference, path, hash, cache, Starlark load, shard,
  diagnostic, and toolchain checks while preserving the zero-process,
  zero-materialization bootstrap boundary.
- Added the closed execution input/result projection and runner-owned execution
  policy schema.
- Added framed execution-corpus digests, immutable backend/adapter identity
  records, explicit hard ceilings, and a disabled-by-default checked-in policy.
- Added a separate process-free execution contract validator. Its `run-case`
  entry point checks explicit operator authorization and the exact reviewed
  digest, then returns a stable non-passing result because no enforcing backend
  exists in this tranche.
- Split Linux OCI, Windows AppContainer, macOS isolation, and the later
  execution-semantics/security-probe corpus into explicit dependent backlog
  items. The bootstrap runner remains execution-disabled.
- Added the closed Linux OCI backend identity schema and specified the
  fail-closed rootless-Podman capability/preflight tranche. It binds exact
  runtime, image, seccomp, shim, and invariant-probe identities while keeping
  the checked-in policy disabled and the execution corpus empty.
