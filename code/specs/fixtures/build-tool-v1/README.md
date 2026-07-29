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
  execution-policy.schema.json
  execution-policy.json
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

## Runner

Validate the complete corpus and inventory:

```text
python code/scripts/build_tool_conformance.py validate-corpus
```

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

The separate `run-case` command requires both
`--allow-trusted-execution` and an exact
`--approved-corpus-sha256`. In this policy-only tranche it can only return a
stable non-passing skip. It never imports a process API, decodes executable
workspace payloads, materializes files, or launches an adapter.

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

This tranche is intentionally process-free:

- it never launches an adapter or shell;
- it never applies fixture environment data;
- it never interprets manifest content as a command;
- it rejects `execution` or `trusted_execution` intent before decoding files,
  changing permissions, or using a process API; and
- it has no flag that can enable execution.

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

Pull-request CI validates only the process-free schemas, policy, digest, and
unit tests. Real execution belongs to a protected reviewed revision with
read-only repository permissions, no repository secrets, and the approved
digest supplied out of band.

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
python code/scripts/build_tool_conformance_execution.py validate-contract
```

CI installs the pinned `jsonschema==4.26.0` validator before running the suites
and both process-free corpus validators.
