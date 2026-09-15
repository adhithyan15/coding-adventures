- `anatomy/valve-kind.adj` (new) — a sibling to the already-shipped `heart-valves.adj`
  (`valve_separates(valve, boundary)`, tricuspid/mitral/pulmonary/aortic → their own two-chamber-
  or-vessel boundary). That table's own header already quotes, verbatim, the NCI SEER sentences
  that name each valve AND classify it as one of two physiological kinds — "atrioventricular" (the
  two valves between an atrium and its ventricle) or "semilunar" (the two valves at the base of a
  great vessel leaving a ventricle) — a fact the valve/boundary schema had no room for. New
  `valve_kind(valve, kind)` table decodes that classifying adjective as each valve's own row:
  tricuspid/mitral → atrioventricular, pulmonary/aortic → semilunar. Confirmed via discipline #30
  that no existing table already maps valve→kind under a different name (only `heart-valves.adj`'s
  own header prose mentions these classification terms at all). Honest abstention on `eustachian`,
  a real cardiac valve name but not one of the four valves this table covers. New e2e test file
  `facts_valvekind_e2e.rs` (3 tests: forward recall with citation, 2-answer backward recall,
  honest abstention). No manifest objective, matching `heart-valves.adj`'s own precedent. Fifth
  slice from the anatomy/ domain sweep — steady at 176 objectives. 100th content slice overall.
