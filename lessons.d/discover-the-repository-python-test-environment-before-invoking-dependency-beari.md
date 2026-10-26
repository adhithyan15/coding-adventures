---
category: Repo policy / workflow reminders
---

# Discover the repository Python test environment before invoking dependency-bearing scripts

Invoking `test_capability_taxonomy.py` with the host `python3` failed because
that interpreter did not contain `jsonschema`. The repository's conformance
tooling pins `jsonschema==4.26.0`; run the test through an explicit ephemeral
environment carrying that pin instead of assuming host packages or installing
into the global interpreter.
