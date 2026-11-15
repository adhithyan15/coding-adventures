## Unreleased - source-input boundary sync (python packages)

- The graph-diff conformance test pins the repository source-input boundary digest; it moved from
  `f338935e...` to `3f8aff75...` because `ca-capability-analyzer` and `test-bootstrap` were added
  to the Python uv workspace manifest (#19424). Test pin only; no tool behaviour change.
