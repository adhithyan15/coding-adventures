# Changelog

## Unreleased

- Index Swimlane minimum-link constraints once by borrowed unordered endpoint
  pair, preserving duplicate/reversed edge rendering and maximum/default gaps
  while removing per-gap full-edge scans (#17334).
- Accumulate geometric gap units without usize sum overflow for caller-owned IR.
- Add actual-layout operation-count, four-direction spacing/edge-preservation
  and extreme caller-owned-length regressions. Existing production grammar,
  safety controls and dependency policy remain intact.

## 0.1.0

- Add deterministic alternating partition layout for Mermaid treemap IR.
