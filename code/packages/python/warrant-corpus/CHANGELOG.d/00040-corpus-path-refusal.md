## Unreleased

- A corpus outside the repository root (or on another Windows drive) made
  `Path.relative_to` raise an uncaught `ValueError`, a traceback instead of the
  runner's exit-2 refusal. `session.corpus_relative` now turns it into a
  `BindingError("corpus-path")`, used by `prepare` and `validate-manifest`. The
  containment itself was never bypassable (`relative_to` raises rather than
  returning an absolute path). Tests added; `runner.src_sha256` changed, so the
  `ct-compare` manifest was regenerated.
