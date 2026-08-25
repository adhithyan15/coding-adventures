### Added - conflict-resistant gentle-ramp corpus snapshots (#12320)

- Replace the hand-maintained global gentle-ramp totals and queue head with one
  deterministic, full-fidelity snapshot per language.
- Rebuild and verify the global summary and work queue from the shards so exact
  regression coverage remains fail-closed without making unrelated language PRs
  edit the same test lines.
- Add write/check commands, CI and local verification wiring, stale-file detection,
  and a regression proving a one-language change alters exactly one output shard.

