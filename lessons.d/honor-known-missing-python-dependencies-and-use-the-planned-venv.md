---
category: Python
---

# Honor known missing Python dependencies and use the planned venv

The slice audit already recorded that system Python lacks `jsonschema` and
specified an isolated venv with `jsonschema==4.26.0`, but the validation command
nevertheless retried the system interpreter and failed with
`ModuleNotFoundError`. Treat verified environment notes as execution inputs:
create the planned temporary environment first instead of re-probing a known
missing dependency during the gate sequence.
