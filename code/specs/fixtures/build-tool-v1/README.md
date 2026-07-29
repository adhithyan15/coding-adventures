# Build-Tool Conformance Fixtures v1

This directory is the versioned, language-neutral build-tool behavior oracle
defined by `code/specs/build-tool-conformance.md`.

## Layout

```text
build-tool-v1/
  schema.json
  result.schema.json
  pure-domains.schema.json
  implementations.schema.json
  implementations.json
  CHANGELOG.md
  cases/
    *.json
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
- normalized validation snapshots and the complete toolchain registry,
  including OCaml; and
- process-free CLI exit-decision classification.

The outer envelope and build-plan payload use `schema.json`,
`result.schema.json`, and `code/specs/schemas/build-plan-v1.schema.json`.
The seven added decision domains additionally validate a closed
`{domain,outcome,input,result}` projection against
`pure-domains.schema.json`; generic JSON is never a fallback.

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

Both commands use bounded strict JSON parsing, formal Draft 2020-12 validation,
semantic path and identity checks, domain-aware result canonicalization, and
stable error codes. `validate-corpus` also performs two-phase validation and
bounded in-memory decoding of pure fixture workspaces so invalid base64, path
aliases, collisions, prefix conflicts, and aggregate size violations fail
without creating a filesystem root. Domain checks verify reference integrity,
framed hashes, cache state, inline Starlark loads, shard closure/cost,
validation diagnostics, complete toolchain maps, and CLI exit decisions.

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
- validation and toolchain detection; and
- CLI exit-decision semantics.

Execution remains the only intentionally unmodeled domain.

## Validation

```text
python -m unittest discover \
  -s code/scripts/tests \
  -p "test_build_tool_conformance_schema.py"
python -m unittest discover \
  -s code/scripts/tests \
  -p "test_build_tool_conformance_runner.py"
```

CI installs the pinned `jsonschema==4.26.0` validator before running both suites
and the corpus validator.
