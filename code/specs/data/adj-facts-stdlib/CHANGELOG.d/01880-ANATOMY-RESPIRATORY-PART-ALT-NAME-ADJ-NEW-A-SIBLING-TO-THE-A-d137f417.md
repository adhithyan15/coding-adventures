- `anatomy/respiratory-part-alt-name.adj` (new) — a sibling to the already-shipped
  `respiratory-parts.adj` (`part_function(part, function)`, trachea → main_airway, alveoli →
  gas_exchange, etc.). Two of that table's own per-row quoted NCI SEER spans name not only the
  part's stated function but also an everyday alternate name for the part itself — a fact the
  part/function schema had no room for: "The trachea, commonly called the windpipe, is the main
  airway to the lungs." and "...tiny air sacs called alveoli." New
  `respiratory_part_alt_name(part, alt_name)` table decodes those two spans as their own rows:
  trachea → windpipe, alveoli → air_sacs. Confirmed via discipline #30 that no other table already
  covers a respiratory part's alternate name. Honest abstention on `larynx`, a real, already-tabled
  part whose own quote states only its function, never an everyday alternate name — likewise nose,
  pharynx, lungs, and diaphragm are deliberately left unrowed. New e2e test file
  `facts_respiratorypartaltname_e2e.rs` (3 tests: forward recall with citation, backward recall,
  honest abstention). No manifest objective, matching `respiratory-parts.adj`'s own precedent.
  Ninth slice from the anatomy/ domain sweep — steady at 176 objectives. 104th content slice
  overall.
