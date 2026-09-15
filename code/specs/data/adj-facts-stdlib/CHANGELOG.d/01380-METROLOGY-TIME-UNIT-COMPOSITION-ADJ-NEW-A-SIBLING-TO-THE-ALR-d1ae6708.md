- `metrology/time-unit-composition.adj` (new) — a sibling to the already-shipped `time-units.adj`
  (`time_unit_seconds(unit, seconds)`, ONE seconds-length per time unit: minute 60, hour 3600, day
  86400). That table's own `source` field already quotes, verbatim, a unit-to-unit relation for
  two of the three units — how many of the next smaller unit each one is composed of — that the
  seconds-only schema had no room for: "1 h = 60 min = 3600 s" and "1 d = 24 h = 86 400 s". New
  `time_unit_composition(unit, sub_unit, count)` table: hour → (minute, 60), day → (hour, 24).
  Honest abstention on minute, whose own cited span ("1 min = 60 s") states only its seconds-length
  with no unit-to-unit relation to a smaller unit on the cited page. New e2e test file
  `facts_timeunitcomposition_e2e.rs` (3 tests: forward recall with citation, backward recall from a
  bound sub-unit and count, honest abstention on minute). No manifest objective, matching
  `time-units.adj`'s own precedent of not having one. First slice from the metrology/ sweep tranche
  (1 strong + 1 moderate candidate found; the moderate one — a bare aggregate count of SI derived
  units — was deliberately not shipped, see loop-state notes).
