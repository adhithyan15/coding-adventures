---
category: Python
---

# Schema validation commands should use the schema subdirectory

The first manual schema-validation command looked for the schema at `code/specs/required-capabilities.schema.json`; the repository stores it under `code/specs/schemas/`. Resolve the exact path with `rg --files code | rg 'required.capabilities.*schema'` before validating metadata, then rerun the command against that checked-in schema.
