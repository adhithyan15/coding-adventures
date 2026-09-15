- `astronomy/celestial-object-alt-name.adj` (new) — a sibling to the already-shipped
  `celestial-objects.adj` (`celestial_property(object, property)`, ONE defining property per
  object): a new `celestial_object_alt_name(object, alt_name)` table names an ALTERNATE NAME the
  source uses for an object, decoded from spans already sitting unused inside
  `celestial-objects.adj`'s own provenance block — no new WebFetch. Two rows: moon →
  planetary_satellites, asteroid → minor_planets, both read off the SAME already-quoted NASA
  spans that table's single-property schema had no room for. Deliberately narrow: only these two
  of the table's five basic types have an alternate name in the already-cited spans. New e2e test
  file `facts_celestialobjectaltname_e2e.rs` (2 tests: both-term recall with citation, honest
  abstention on an object with no cited alternate name). No manifest objective, matching
  `celestial-objects.adj`'s own precedent of not having one.
