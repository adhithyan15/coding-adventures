# Build-Tool Conformance Fixtures v1

This directory is the versioned, language-neutral build-tool behavior oracle
defined by `code/specs/build-tool-conformance.md`.

## Layout

```text
build-tool-v1/
  schema.json
  result.schema.json
  implementations.schema.json
  implementations.json
  CHANGELOG.md
  cases/
    discovery-simple.json
    discovery-windows-override.json
    graph-diamond.json
    plan-affected-empty.json
    plan-affected-null.json
    plan-future-version.json
    resolution-python-diamond.json
```

`implementations.json` inventories all 15 established language lanes plus the
emerging OCaml lane. It records front-door and shared-engine state but contains
no executable commands. Every adapter is currently marked missing, so a valid
inventory is not reported as conformance success.

The bootstrap corpus covers:

- canonical and Windows-override discovery;
- the shared Python dependency diamond;
- deterministic diamond graph levels;
- the build-plan distinction between `affected_packages: null` and `[]`; and
- fail-closed rejection of a future plan version.

The build-plan payload is validated against
`code/specs/schemas/build-plan-v1.schema.json`.

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
temporary materialization of pure fixture workspaces so invalid base64, path
aliases, collisions, prefix conflicts, and aggregate size violations fail
before any root is created.

## Security boundary

This tranche is intentionally process-free:

- it never launches an adapter or shell;
- it never applies fixture environment data;
- it never interprets manifest content as a command;
- it rejects `execution` or `trusted_execution` intent before decoding files,
  creating a temporary directory, changing permissions, or using a process
  API; and
- it has no flag that can enable execution.

The temporary materializer is only a data-validation stage in a private
runner-owned directory. Its output must not be handed to untrusted code.
Adapter orchestration and execution fixtures remain blocked until the separate
trusted-sandbox item implements atomic no-follow filesystem access, network
isolation, a sanitized environment, direct argument vectors, and complete
process-tree resource limits.

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
