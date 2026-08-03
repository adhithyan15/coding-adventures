# Changelog

## 2026-08-03

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
