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
  execution-policy.schema.json
  execution-policy.json
  linux-oci-backend.schema.json
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

The 30-case bootstrap corpus covers every process-free v1 domain:

- canonical membership plus Windows, macOS, and Linux BUILD precedence;
- the shared Python dependency diamond;
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

`execution-policy.json` is runner-owned authority rather than fixture data or
implementation metadata. It records the exact execution-corpus digest, hard
ceilings, backend identities, and adapter executable digests. The checked-in
policy is disabled, has no adapters, and marks all three platform backends
unavailable. The empty `execution-cases/*.json` set therefore has the standard
empty SHA-256 digest. Execution cases are added only after an enforcing backend
is reviewed.

`linux-oci-backend.schema.json` closes the separate immutable identity document
for the first Linux backend tranche. It binds exact rootless Podman, `crun`,
OCI manifest/config, seccomp, shim, and invariant-probe identities. The
process-owning `build_tool_conformance_linux_oci.py` preflight validates those
identities and host capabilities without decoding a fixture or creating a
container. No identity document is checked in while Linux remains unavailable.

`execution-authority.schema.json` closes the external, post-review authority
bundle for the first safe authorization profile. Its exact raw bytes are
approved out of band with a domain- and length-separated SHA-256. The bundle
binds the reviewed source commit/tree, policy and schemas, process-free
bootstrap and verifier, Linux preflight backend, and one external Linux
identity document. Scope `linux_capability_preflight_v1` binds no corpus,
adapter, launcher, or executable case and cannot authorize container creation
or trusted execution.

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

This tranche exposes no process-owning `preflight` subcommand. The bare
`build_tool_conformance_linux_oci.py --identity ...` entry point fails closed
with `LINUX_OCI_AUTHORITY_REQUIRED`. A follow-on exact-byte loader must execute
the retained approved backend and its bound import closure without a
name-based Python import before protected capability inspection can run.

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

Authority validation and pull-request CI are intentionally process-free:

- it never launches an adapter or shell;
- it never applies fixture environment data;
- it never interprets manifest content as a command;
- it rejects `execution` or `trusted_execution` intent before decoding files,
  changing permissions, or using a process API; and
- the preflight authority profile has no flag that can enable execution.

There is intentionally no process handoff in this tranche. Ordinary
name-based importing after validation would not prove that the approved
backend bytes are the code being executed. A later loader must consume the
exact retained artifact and its bound import closure through an atomic
runner-owned boundary before capability inspection can run.

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

Pull-request CI validates only the process-free schemas, policy, candidate
digest algorithm, and fake unit tests. Real capability inspection and future
execution belong to a protected reviewed revision with read-only repository
permissions, no repository secrets, and the approved authority-bundle digest
supplied out of band.

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
  -p "test_build_tool_conformance_linux_oci.py"
python code/scripts/build_tool_conformance_execution.py validate-contract
```

CI installs the pinned `jsonschema==4.26.0` validator before running the suites
and both process-free corpus validators.
