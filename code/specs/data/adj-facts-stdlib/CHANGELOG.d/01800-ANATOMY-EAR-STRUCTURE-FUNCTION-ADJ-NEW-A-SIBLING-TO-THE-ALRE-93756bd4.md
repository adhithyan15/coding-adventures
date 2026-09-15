- `anatomy/ear-structure-function.adj` (new) — a sibling to the already-shipped `ear-parts.adj`
  (`ear_structure_region(structure, region)`, ear_canal/malleus/incus/stapes/cochlea →
  outer_ear/middle_ear/middle_ear/middle_ear/inner_ear). That table's own header already quotes,
  verbatim, two chained NIDCD sentences that together name the three middle-ear ossicles AND
  state what they do to the signal — "The bones in the middle ear amplify, or increase, the sound
  vibrations and send them to the cochlea..." — but the structure/region schema had room only for
  WHERE each structure sits, not WHAT the ossicles DO. New `ear_structure_function(structure,
  function)` table decodes that action verb as each ossicle's own row: malleus/incus/stapes → all
  amplifies_sound. Honest abstention on ear_canal, a real already-tabled ear structure whose own
  quote states no action-verb function. New e2e test file `facts_earstructurefunction_e2e.rs` (3
  tests: forward recall with citation, 3-answer backward recall, honest abstention). No manifest
  objective, matching `ear-parts.adj`'s own precedent. First slice from a fresh anatomy/ domain
  sweep — the domain's ~20 shipped tables turned out to be only partially mined (the standing
  "only lung-lobes shipped" note was stale), and a targeted Explore-agent sweep found 9 further
  STRONG sibling-table candidates across the directory.
