- `anatomy/lung-lobe-names.adj` (new) — a new sibling to the already-shipped `lung-lobes.adj`
  (which recalls only lobe COUNT per lung): a new `lung_lobe_name(lobe, lung)` table names each
  of the five individual lobes and which lung it belongs to (right_upper_lobe/right_middle_lobe/
  right_lower_lobe -> right_lung; left_upper_lobe/left_lower_lobe -> left_lung). Discovered via a
  header-revisit: `lung-lobes.adj`'s own header already quotes the corroborating StatPearls
  sentence naming all five lobes, but that table's `lung_lobe_count(lung, count)` schema has no
  room for individual lobe names -- a genuinely new predicate/table, the same "sibling table,
  different schema" shape already used for `comet-part.adj`/`comet-tail-type.adj`, rather than an
  extend of the existing table. WebFetch re-verified the sentence live, THREE separate passes, all
  byte-identical to what `lung-lobes.adj`'s header already quoted. The reverse-binding query on
  `right_lung` is a genuine one-to-many recall (all three right lobes). New e2e test
  `facts_lunglobenames_e2e.rs`. No manifest objective added, matching `lung-lobes.adj`'s own
  precedent -- this anatomy sub-family (also including `kidney-parts.adj`/`long-bone-parts.adj`)
  is not wired into the loop's manifest coverage tracking.
