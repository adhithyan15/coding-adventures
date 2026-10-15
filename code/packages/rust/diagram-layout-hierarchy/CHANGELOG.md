# Changelog

## Unreleased

- Global flowchart curve is applied only when an edge has no explicit curve. Rank propagation, optimizer increments/constraints and rank count use checked integer additions and return RankArithmeticOverflow through try_layout_swimlane before geometric arithmetic; the legacy wrapper explicitly refuses that error. Normal output, reference bits, work counters and parser limits remain unchanged.
- Miri **DEFERRED/PENDING**; no whole or hosted acceptance inferred.

- Add checked spacing/derived geometry boundary and explicit legacy-wrapper refusal. Keep large-padding lane dimensions nonnegative. Repair coincident-center NaN endpoints with finite anchors, preserving all endpoint bits/reference/work assertions and adding independent exact anchor controls. The later checked-rank slice now refuses rank arithmetic overflow.
- Final-source Miri **DEFERRED/PENDING**; native/hosted and whole-PR gates remain separately attributed.

- Evaluate adjacent automatic lane swaps from checked lane balances and a
  borrowed pair-weight index, preserving nine starts, seeded shuffles, strict
  improvement and source-distance ties. Canonical unique-lane graphs visit each
  weighted pair at most27 times across all nine initializations.
- Retain the original full-cost oracle for729 four-lane graphs/all17496 initial
  permutations, public layout/metadata and complete authored paint comparisons.
  Malformed non-bijective direct orders retain the full-cost fallback; checked
  score arithmetic refuses overflow before comparison.

- Reuse the existing minimum-link index in cross-lane rank construction after
  upstream integration; verify exact enabled-rank spacing and 512 visits/14
  queries in all four directions without relaxing the legacy gap controls.

- Preserve alternating Linear/Basis curves in the duplicate-gap resource fixture
  and assert their exact layout propagation when integrating edge curve IR.

- Index Swimlane minimum-link constraints once by borrowed unordered endpoint
  pair, preserving duplicate/reversed edge rendering and maximum/default gaps
  while removing per-gap full-edge scans (#17334).
- Accumulate geometric gap units without usize sum overflow for caller-owned IR.
- Add actual-layout operation-count, four-direction spacing/edge-preservation
  and extreme caller-owned-length regressions. Existing production grammar,
  safety controls and dependency policy remain intact.

## 0.1.0

- Add deterministic alternating partition layout for Mermaid treemap IR.
