### Changed - honest zero-new-atom review measurement (#12496)

- Count schema-v2 `review`, `practice`, and `practice-mix` lessons as measurable
  zero-new-atom steps only when they explicitly declare an empty introduction
  list and a non-empty practice contract.
- Keep legacy prose, missing or malformed knowledge fields, teaching lessons,
  and unclassified synthesis steps fail-closed as measurement-blind.
- Regenerate per-language gentle-ramp snapshots so retrieval work no longer
  inflates migration debt; no ramp limit or violation gate changes.
