## 2026-10-22 — TE04 fenced fixture selection

- Route the exact GFM fenced-block fixture to the TypeScript parser and its
  dependent closure without forcing a full build, across all platform plans.
- Preserve source unions, language filters, and deleted/renamed paths; reject
  missing or wrong-language readers before creating or replacing plan output.
- Add native selector, toolchain, dependency-order, and atomic-plan regressions.
