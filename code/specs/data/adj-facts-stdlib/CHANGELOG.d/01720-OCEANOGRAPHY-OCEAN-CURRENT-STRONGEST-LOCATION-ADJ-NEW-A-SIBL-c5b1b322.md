- `oceanography/ocean-current-strongest-location.adj` (new) — a sibling to the already-shipped
  `ocean-current-drivers.adj` and `ocean-current-surface-position.adj`. That table's own header already
  quotes, verbatim, a NOAA sentence stating tidal currents "are strongest near the shore, and in bays and
  estuaries along the coast" — a location fact the single `driver` atom had no room for. Both prepositional
  phrases are simultaneously true (the source's "and" joins two coexisting facts, not two competing
  readings), so they fold into ONE compound atom, matching this stdlib's own established convention. New
  `ocean_current_strongest_location(current_type, location)` table decodes that clause as its own row:
  tidal_currents → near_the_shore_and_in_bays_and_estuaries_along_the_coast. Honest abstention on
  thermohaline_circulation, whose cited span names no location. New e2e test file
  `facts_oceancurrentstrongestlocation_e2e.rs` (3 tests: forward recall with citation, backward recall,
  honest abstention). No manifest objective, matching `ocean-current-drivers.adj`'s own precedent. Fourth
  slice from the oceanography/ domain sweep. (The remaining candidate, `named_current_climate_effect` off
  the same file's gulf_stream aside — "brings milder winter weather to Bergen, Norway, than to New York" —
  was re-inspected and confirmed a dead end: the claim is inherently a comparison between two named places,
  which cannot honestly fit a binary (current, effect) shape without arbitrary compression of a relation
  the schema was never built to hold. This closes out the oceanography/ domain sweep.)
