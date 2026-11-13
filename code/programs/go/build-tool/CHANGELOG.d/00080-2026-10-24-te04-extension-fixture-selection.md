## 2026-10-24 — TE04 extension fixture selection

- Route the exact portable GFM extension corpus to the TypeScript parser and
  its dependent closure on every platform without forcing a full build.
- Retain the older fenced-only route while rejecting case, separator, backup,
  nested, schema, and documentation lookalikes for both corpus paths.
- Extend missing-reader, language-filter, source-union, toolchain, dependency,
  and atomic-plan regressions to the portable extension corpus.
