### Forme specification map reconciliation

- Restored the reserved FM05 Interactivity IR, FM06 AOT compiler, and FM07
  CLI/development-server locations, and moved the pending deploy runner to FM08
  without changing its contract.
- Added an evidence-based implementation ledger to every Forme specification,
  repaired stale cross-links and command examples, and published the complete
  canonical map from the repository README and completion roadmap.
- Added an always-on metadata test that rejects missing or duplicate numbered
  specs, absent ledgers, incomplete roadmap links, and broken local Markdown
  targets before the map can drift again.
