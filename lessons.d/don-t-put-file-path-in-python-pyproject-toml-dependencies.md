---
category: Workspace & package metadata
---

# Don't put `@ file:../path` in Python `pyproject.toml` dependencies

Hatchling rejects them, and even with `allow-direct-references = true`, uv resolves the relative path from a temp build dir. Use bare names + BUILD pre-installation + `[tool.uv.sources]` for local-path redirection.
