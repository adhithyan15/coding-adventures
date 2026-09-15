- `biology/muscle-striation.adj` (new) — a sibling to the already-shipped `muscle-types.adj`
  (`muscle_trait(muscle, trait)`, ONE distinctive characteristic per muscle type — skeletal →
  voluntary, smooth → involuntary, cardiac → intercalated_disks). That table's own header already
  quotes, verbatim, each muscle type's defining NCI SEER sentence, and buried in every one of those
  three spans, alongside the trait the parent table already captures, is a second, cleanly binary
  fact the voluntary/trait schema had no room for: whether the tissue is striated. New
  `muscle_striated(muscle, striated)` table covers the full three-type domain with no abstention:
  skeletal → yes, smooth → no, cardiac → yes. New e2e test file `facts_musclestriation_e2e.rs` (3
  tests: forward recall with citation, backward recall of the one non-striated type, full-domain
  coverage with no abstention). No manifest objective, matching `muscle-types.adj`'s own precedent
  of not having one. Second slice from the fresh biology/ sweep tranche, queued after
  `vertebrate-thermoregulation.adj`.
