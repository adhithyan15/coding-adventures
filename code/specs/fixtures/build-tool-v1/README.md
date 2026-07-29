# Build-Tool Conformance Fixtures v1

This directory contains the versioned, language-neutral fixture format defined
by `code/specs/build-tool-conformance.md`.

## Layout

```text
build-tool-v1/
  schema.json
  README.md
  examples/
    discovery-windows-override.json
```

The contract PR includes the schema and a non-executing example. The next
fixture-runner work item will add the case corpus, adapter manifests, and runner.

Every case is one JSON document with:

- an inline repository-shaped workspace;
- one conformance domain operation;
- one canonical expected result; and
- requested resource limits that can only be tightened by runner policy.

UTF-8 files use `content_utf8`. Binary files use strict padded
`content_base64`. Fixtures cannot declare symlinks, directories, or special
files.

## Validation

The repository self-test performs structural, formal Draft 2020-12, and
adversarial checks. CI installs the pinned `jsonschema` validator before
running it:

```text
python -m unittest discover \
  -s code/scripts/tests \
  -p "test_build_tool_conformance_schema.py"
```

Schema validation does not make a fixture safe to execute. The future runner
must enforce the sandbox, trust, containment, normalization, and hard-limit
requirements in the conformance contract. In particular, a fixture's
`trusted_execution` capability is a request and never authorizes execution.
