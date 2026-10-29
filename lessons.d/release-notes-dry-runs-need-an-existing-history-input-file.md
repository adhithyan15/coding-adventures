---
category: CI & GitHub Actions
---

# Release notes dry runs need an existing history input file

The TaskApp release-note dry run failed before rendering because its `--history`
argument named a temporary JSON file that had not been created. Materialize the
input first; an explicit empty JSON array is sufficient when validating only
authoritative changelog extraction. Treat the resulting notes as a local preview,
not evidence that artifacts were built or a release was published.
