- `anatomy/valve-alternate-name.adj` (new) — a sibling to the already-shipped `heart-valves.adj`
  (`valve_separates`) and `valve-kind.adj` (`valve_kind`). Both of those tables' own already-quoted
  NCI SEER span also names an everyday alternate name for the mitral valve -- a fact neither the
  boundary nor the kind schema had room for: "The left atrioventricular valve is the bicuspid, or
  mitral, valve." New `valve_alternate_name(valve, alt_name)` table decodes that span as its own
  row: mitral → bicuspid. No new WebFetch -- reuses the same already-cited NCI SEER sentence.
  Honest abstention on `tricuspid`, `pulmonary`, and `aortic`, whose own quotes never supply a
  second name. New e2e test file `facts_valvealternatename_e2e.rs` (3 tests: forward recall with
  citation, backward recall, honest abstention). No manifest objective, matching sibling tables'
  own precedent. Second of the 3 MODERATE anatomy/ fallback candidates. 107th content slice
  overall.
