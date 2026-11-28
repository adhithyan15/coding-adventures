# Changelog

## Unreleased

- Require exact decoded lowercase keys and bounded portable integer tokens in
  the stale-only fixture adapter; preserve one-time escaped canonical keys and
  reject the reproduced case aliases before model replay.
- Add `SelectCopyAware`, ranking eligible blocks by stale-to-copy ratio with
  exact bounded cross products, excluded-block masks, and external spare
  capacity. Add model-first vector replay, exhaustive role/mask enumeration,
  seeded checks, fuzz targets, and four disposable mutation controls.
- Add deterministic whole-block victim selection over DB05a physical status,
  with geometry-first refusal, stale-page scoring and lowest-index ties.
- Add model-first shared-vector checks, exhaustive 2–6-page layouts, bounded
  fixture/status fuzzing and disposable policy mutation controls.
- Add `SelectCopyAwareWithFences` with independent victim and destination
  masks. Keep the shared-mask selector unchanged; check per-candidate external
  free capacity, exact scoring ties, validation order, 1,128,000 exhaustive
  small role/status cases, 12,000 seeded histories, bounded fuzz inputs, and
  three additional clean-controlled mutations.
