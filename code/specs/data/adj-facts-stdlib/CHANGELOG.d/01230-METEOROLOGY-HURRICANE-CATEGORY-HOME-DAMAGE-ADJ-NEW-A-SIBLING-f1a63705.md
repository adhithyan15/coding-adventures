- `meteorology/hurricane-category-home-damage.adj` (new) — a sibling to the already-shipped
  `hurricane-categories.adj` (`damage_level(category, descriptor)`, ONE generic damage word per
  Saffir-Simpson category): a new `hurricane_home_damage(category, home_damage_effect)` table
  names the SPECIFIC well-built-home structural effect the SAME NHC page describes at each
  category, decoded from spans already sitting unused inside `hurricane-categories.adj`'s own
  provenance block — no new WebFetch. Five rows, all distinct (unlike the parent table's
  descriptor column, which honestly duplicates `catastrophic_damage` for categories 4 and 5).
  New e2e test file `facts_hurricanehomedamage_e2e.rs` (3 tests: forward recall with citation,
  backward recall from a bound effect, honest abstention on category 6). No manifest objective,
  matching `hurricane-categories.adj`'s own precedent of not having one.
