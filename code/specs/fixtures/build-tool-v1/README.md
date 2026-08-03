# Build-Tool Conformance Fixtures v1

This directory is the versioned, language-neutral build-tool behavior oracle
defined by `code/specs/build-tool-conformance.md`.

## Layout

```text
build-tool-v1/
  schema.json
  result.schema.json
  pure-domains.schema.json
  execution.schema.json
  execution-authority.schema.json
  execution-preflight-loader-authority.schema.json
  execution-capability-broker-authority.schema.json
  execution-policy.schema.json
  execution-policy.json
  linux-oci-backend.schema.json
  linux-capability-preflight-broker.schema.json
  linux-capability-preflight-broker.json
  preflight-imports.json
  preflight-broker-backend-imports.json
  implementations.schema.json
  implementations.json
  CHANGELOG.md
  cases/
    *.json
  execution-cases/
    README.md
```

`implementations.json` inventories all 15 established language lanes plus the
emerging OCaml lane. It records front-door and shared-engine state but contains
no executable commands. Every adapter is currently marked missing, so a valid
inventory is not reported as conformance success.

The 38-case bootstrap corpus covers every process-free v1 domain:

- canonical package and program membership, language-registry classification,
  fixture-tree exclusion, fail-closed duplicate package identities, plus
  Windows, macOS, and Linux BUILD precedence;
- the shared Python dependency diamond, distinct package/program identities,
  legacy BUILD dependency comments, fail-closed dependency self-edges, and
  positive UTF-8 plus fail-closed invalid-UTF-8 Lua rockspec resolution;
- deterministic diamond graph levels;
- the build-plan distinction between `affected_packages: null` and `[]`; and
- fail-closed rejection of a future plan version;
- conservative diff selection and prerequisite closure;
- framed SHA-256 hashing plus hit, miss, and corrupt-cache recovery;
- inline-only Starlark module resolution, bounded evaluation requests,
  structured command extraction, and stable missing/outside errors;
- deterministic prerequisite-closed sharding and invalid input handling;
- normalized BUILD-file validation snapshots and the complete toolchain registry,
  including OCaml; and
- process-free CLI exit-decision classification.

The outer envelope and build-plan payload use `schema.json`,
`result.schema.json`, and `code/specs/schemas/build-plan-v1.schema.json`.
The seven added decision domains additionally validate a closed
`{domain,outcome,input,result}` projection against
`pure-domains.schema.json`; generic JSON is never a fallback.

Execution has a separate closed `{domain,outcome,input,result}` projection in
`execution.schema.json`. `schema.json` and `result.schema.json` enforce the
same closed command, package, dependency, resource-lock, and canonical result
records in the outer envelope. Structured commands contain only `program` and
`args`; they have no fixture-controlled cwd, shell, redirection, or per-command
environment. Legacy commands remain distinct line records.

Execution status records use one fail-stop state machine. Succeeded commands
return zero, failed commands return nonzero, and not-run commands return null.
Built packages contain only succeeded commands; failed packages contain one
failed command, preserve that exit code as the package return code, and mark
every later command not-run. `dep-skipped` and `would-build` packages never
execute commands and return null. The projection additionally ties dry-run to
an all-`would-build` successful outcome, ordinary success to all-`built`, and
errors to at least one failure plus dependency-propagated skips. The semantic
validator enforces command order, return-code equality, and graph propagation
that Draft 2020-12 cannot express.

`execution-policy.json` is runner-owned authority rather than fixture data or
implementation metadata. It records the exact execution-corpus digest, hard
ceilings, backend identities, and adapter executable digests. The checked-in
policy is disabled, has no adapters, and marks all three platform backends
unavailable. The empty `execution-cases/*.json` set therefore has the standard
empty SHA-256 digest. Execution cases are added only after an enforcing backend
is reviewed.

The process-free execution validator captures that corpus once as a typed,
immutable exact-byte snapshot. POSIX uses retained directory descriptors;
Windows requires a fixed non-remappable local volume, retains a non-reparse
directory chain, enumerates the root by handle, and matches each member's
volume and file identity while topology changes are blocked. Direct
lowercase-`.json` names must be portable, exact NFC, unique after case folding,
regular, singly linked, bounded, and identity-stable. Digesting, semantic
validation, and typed selection all consume the retained bytes instead of
reopening a pathname. A successful selection binds the canonical member name,
corpus digest, and those exact bytes, but it grants no authority and does not
make an adapter or backend ready. Capture admits at most 4096 directory
entries, 256 corpus members, 2000000 raw bytes per member, and 16777216 raw
bytes across the retained snapshot. Only the validating factory constructs
member, snapshot, and selection records; callers cannot supply a digest.

`linux-oci-backend.schema.json` closes the separate immutable identity document
for the first Linux backend tranche. It binds exact statically linked rootless
Podman, `crun`, Conmon,
OCI manifest/config, seccomp, shim, and invariant-probe identities. The
capability broker validates the non-root Linux/amd64 host, delegated cgroup-v2
controllers, kernel seccomp actions, exact runtime binary identities, and the
absence of a Podman ELF `PT_INTERP` segment. Requiring `linkage: static`
prevents an allowed dynamic loader from becoming an execution trampoline. A
mandatory Landlock execute ruleset permits pathname-backed execution only of
the retained Podman inode, so constructor hooks, `catatonit`, and other
pathname-backed helpers in the reviewed flow cannot execute. After closing
unlisted descriptors, the broker also installs an amd64 classic-seccomp filter
that denies `execveat`, anonymous executable memfds, executable mappings,
`SHM_EXEC`, descriptor receipt or acquisition syscalls, `uselib`, and
`io_uring_*`. Podman begins through pathname `execve` of its retained
`/proc/self/fd` inode. This closes new executable code
paths after the transition without removing the protected runner-image TCB. The
process-free `build_tool_conformance_linux_oci.py` backend validates only the
bounded local version and image results without decoding a fixture or creating
a container. No identity document is checked in while Linux remains unavailable.

`execution-authority.schema.json` closes the external, post-review authority
bundle for the first safe authorization profile. Its exact raw bytes are
approved out of band with a domain- and length-separated SHA-256. The bundle
binds the reviewed source commit/tree, policy and schemas, process-free
bootstrap and verifier, Linux preflight backend, and one external Linux
identity document. Scope `linux_capability_preflight_v1` binds no corpus,
adapter, launcher, or executable case and cannot authorize container creation
or trusted execution.

`execution-preflight-loader-authority.schema.json` is a separately
domain-bound ten-role profile. It adds the exact loader and closed stdlib
import manifest without broadening the earlier scope. On Linux, the verifier
traverses every component from retained directory handles, and the loader
copies its own source, the backend, manifest, and identity into sealed memfds.
A fresh `python -I -S -B` worker executes the sealed loader, rejects undeclared
or dynamic imports and executable import-time statements, compiles but never
executes the backend, and verifies its three required structural interfaces.
Its receipt reports loadability only: it does not call preflight, Podman, a
fixture, an adapter, or a container.

`execution-capability-broker-authority.schema.json` is the separately
domain-bound thirteen-role capability-preflight profile. It additionally binds
the broker-specific process-free backend import manifest, exact broker source,
and the language-neutral broker behavior manifest and schema. The checked-in
manifest closes the two permissible Podman operations, reviewed runtime and
state descriptors, environment, timeout, combined streaming output ceiling,
Landlock pathname-exec policy, classic-seccomp in-memory-exec policy, and
delegated-cgroup descendant cleanup. Its authority cannot authorize a
container, execution case, adapter, invariant probe, or Linux readiness.

## Runner

Validate the complete corpus and inventory:

```text
python code/scripts/build_tool_conformance.py validate-corpus
```

Validate an approved external bundle without importing a process API:

```text
python code/scripts/build_tool_conformance_authority.py validate-authority \
  --authority-bundle path/to/external-authority.json \
  --approved-authority-sha256 <out-of-band-sha256> \
  --source-commit <full-reviewed-commit> \
  --source-tree <full-reviewed-tree>
```

Validate a separately approved exact-loader bundle on Linux:

```text
python code/scripts/build_tool_conformance_backend_loader.py \
  --authority-bundle path/to/external-loader-authority.json \
  --approved-authority-sha256 <out-of-band-loader-sha256> \
  --source-commit <full-reviewed-commit> \
  --source-tree <full-reviewed-tree> \
  --repository-root <absolute-reviewed-checkout>
```

This command starts only one isolated Python loadability worker. The bare
`build_tool_conformance_linux_oci.py --identity ...` entry point fails closed
with `LINUX_OCI_AUTHORITY_REQUIRED`. The separate broker profile is required
for real capability inspection; it validates and FD-executes the static Podman
runtime, verifies the reviewed
`crun` and Conmon bytes, retains state roots without reopening them, streams
bounded combined output, and owns full descendant
cleanup. It still cannot report trusted-execution readiness.

Compare one externally produced adapter result with its fixture:

```text
python code/scripts/build_tool_conformance.py validate-result \
  --case code/specs/fixtures/build-tool-v1/cases/graph-diamond.json \
  --result path/to/result.json
```

Validate the process-free trusted-execution contract:

```text
python code/scripts/build_tool_conformance_execution.py validate-contract
```

The separate `run-case` command requires `--allow-trusted-execution`, the
external bundle and approved authority SHA-256, and the protected full source
commit/tree identities. Corpus-only approval is no longer accepted. Because
schema v1 is preflight-only, `run-case` returns the stable non-passing
`EXECUTION_AUTHORITY_SCOPE_UNAVAILABLE` result after validation. It never
imports a process API, decodes executable workspace payloads, materializes
files, or launches an adapter.

Both commands use bounded strict JSON parsing, formal Draft 2020-12 validation,
semantic path and identity checks, domain-aware result canonicalization, and
stable error codes. `validate-corpus` also performs two-phase validation and
bounded in-memory decoding of pure fixture workspaces so invalid base64, path
aliases, collisions, prefix conflicts, and aggregate size violations fail
without creating a filesystem root. Domain checks verify reference integrity,
framed hashes, cache state, inline Starlark loads, shard closure/cost,
BUILD-file validation diagnostics, complete toolchain maps, and CLI exit
decisions.

## Security boundary

The bootstrap and authority verifiers remain process-free:

- it never launches an adapter or shell;
- it never applies fixture environment data;
- it never interprets manifest content as a command;
- it rejects `execution` or `trusted_execution` intent before decoding files,
  changing permissions, or using a process API; and
- the preflight authority profile has no flag that can enable execution.

The exact-loader tranche has one narrow process handoff: a fresh isolated
Python worker started from the sealed approved loader bytes. It receives only
sealed descriptors, fixed digests, a scrubbed environment, and standard-library
search paths. The worker validates source closure and interfaces and exits; it
cannot make the unavailable Linux backend available.

The capability broker adds a separately authorized process boundary for two
fixed runtime-version and image-inspection commands. It executes retained
verified Podman bytes,
verifies reviewed `crun` and Conmon bytes, passes private state descriptors through
`/proc/self/fd`, applies a fixed environment and command grammar, and owns a
fresh delegated cgroup until it proves empty. The state root contains only a
prepopulated private image store before the broker creates its transient
children. The backend receives only
bounded command results and remains unable to spawn a process itself.

The bootstrap runner performs no workspace materialization. Adapter
orchestration and execution fixtures remain blocked until the separate
trusted-sandbox item implements atomic no-follow filesystem access, network
isolation, a sanitized environment, direct argument vectors, and complete
process-tree resource limits.

The pure-domain expansion keeps that boundary intact. Diff selection consumes
declared changed paths instead of Git, hashing consumes inline bytes instead of
filesystem metadata, Starlark returns structured commands without running
them, validation reads only fixture data, toolchain detection never probes the
host, and CLI cases classify exit decisions without parsing native argv or
invoking a build.

The corpus now closes all process-free v1 domains:

- discovery, resolution, graph, and plan;
- diff selection and hashing/cache;
- Starlark evaluation and structured-command extraction;
- prerequisite-closed sharding;
- BUILD-file validation and toolchain detection; and
- CLI exit-decision semantics.

Execution now has a closed data model and authority policy, but no execution
backend. This distinction is deliberate: schemas and digests are prerequisites
for a sandbox, not evidence that one exists.

The exact-byte corpus snapshot closes only a repository read boundary. Unsafe,
outside, case-alias, and normalization-alias selectors fail before lookup;
linked, reparse, multiply linked, identity-aliased, changed, or oversized
members fail during capture. Later pathname replacement cannot change selected
bytes because selection reads only the immutable snapshot. No execution case is
decoded by an authority verifier, and no snapshot digest is authorization.

The platform delivery order is explicit:

1. Linux OCI with a pinned pre-existing image, namespace isolation,
   read-only rootfs, dropped capabilities, bounded tmpfs, cgroups, streaming
   output accounting, and whole-container termination.
2. Windows AppContainer or LPAC plus private ACLs, reparse-safe root-handle
   operations, and a no-breakaway Job Object assigned before resume.
3. macOS signed-helper or isolated-VM containment. `sandbox-exec` alone is not
   accepted.
4. Closed execution-semantics fixtures and runner-owned adversarial boundary
   probes after all required platform boundaries exist.

Pull-request CI validates the process-free schemas, policy, candidate digest
algorithm, platform-neutral unit tests, and Linux kernel integration probes for
the retained-inode Landlock plus classic-seccomp transition. Real Podman
capability inspection and future container execution belong to a protected
reviewed revision with read-only repository permissions, no repository
secrets, and the approved authority-bundle digest supplied out of band.

## Validation

```text
python -m unittest discover \
  -s code/scripts/tests \
  -p "test_build_tool_conformance_schema.py"
python -m unittest discover \
  -s code/scripts/tests \
  -p "test_build_tool_conformance_runner.py"
python -m unittest discover \
  -s code/scripts/tests \
  -p "test_build_tool_conformance_execution_schema.py"
python -m unittest discover \
  -s code/scripts/tests \
  -p "test_build_tool_conformance_execution_runner.py"
python -m unittest discover \
  -s code/scripts/tests \
  -p "test_build_tool_conformance_authority.py"
python -m unittest discover \
  -s code/scripts/tests \
  -p "test_build_tool_conformance_backend_loader.py"
python -m unittest discover \
  -s code/scripts/tests \
  -p "test_build_tool_conformance_capability_broker.py"
python -m unittest discover \
  -s code/scripts/tests \
  -p "test_build_tool_conformance_linux_oci.py"
python code/scripts/build_tool_conformance_execution.py validate-contract
```

CI installs the pinned `jsonschema==4.26.0` validator before running the suites
and both process-free corpus validators.
