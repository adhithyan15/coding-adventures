- New meta-primitives:
  - `For (each: <expr|slot>, as: <name>, index: <name>?)` lowers to a
    `<!-- mosaic-for each="..." as="..." index="..." -->` … `<!-- /mosaic-for -->`
    comment wrapper. The host's template engine resolves the loop at
    render time.
  - `If (when: <expr|slot>)` lowers to `<!-- mosaic-if when="..." -->` …
    `<!-- /mosaic-if -->`. Literal `when: true` / `when: false` is folded
    at compile time (no markers, just the chosen branch).
  - `Else` is recognised as a sibling of `If` and consumed by the
    sibling walker; an orphan `Else` emits a diagnostic comment marker.
