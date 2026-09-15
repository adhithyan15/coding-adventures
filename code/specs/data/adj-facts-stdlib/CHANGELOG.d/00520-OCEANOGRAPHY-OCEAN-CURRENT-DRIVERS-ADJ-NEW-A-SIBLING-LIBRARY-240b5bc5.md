- `oceanography/ocean-current-drivers.adj` (new) -- a sibling library to `ocean-zones.adj`, a
  DIFFERENT oceanography axis (what moves the water, not how deep light reaches). A new
  `ocean_current_driver(current_type, driver)` table names three ocean-current categories and the
  physical driver that creates each (tidal_currents->tides, wind_driven_currents->wind,
  thermohaline_circulation->density_differences_from_temperature_and_salinity), quoted verbatim
  from NOAA National Ocean Service's "What is a current?" page, which numbers exactly these three
  driving mechanisms as its own answer to that question -- `trust authoritative` (NOAA is a
  primary .gov source, the same tier the sibling `ocean-observing-instruments.adj` -- the only
  other library in this same directory -- already uses, distinct from the `consensus` tier
  `ocean-zones.adj` uses for its non-.gov WHOI citation). A MULTI-SOURCE-STYLE table (see
  `animal-survival-adaptation.adj`): each row's quote comes from a different paragraph of the same
  page, with the table's own `source` field carrying the first row's (tidal_currents) quote and
  the other two rows' quotes documented in the file's header prose. Grounds NGSS 3-5
  ocean-systems standards. Picked using the mandatory full-tree-grep-before-scoping discipline --
  `grep -rilE "\btidal.current\b|\bthermohaline\b|\bocean.current.driver\b|wind.driven.current|
  \bgulf.stream\b" code/specs/data/adj-facts-stdlib/` found ZERO hits, confirming a completely
  fresh topic before this file was written. WebFetch-verified before writing (twice, across two
  cycles of this loop). Honest abstention on "gulf_stream" (a real, specific named current the
  same page mentions, but not one of these three driver CATEGORIES tabled here). New manifest
  objective `adj.science.3to5.ocean_current_driver` (band 3-5, `recall` competency, `ngss`
  coverage root). New e2e test `facts_oceancurrentdrivers_e2e.rs` (3 tests: direct recall, reverse
  binding, honest abstention on an untabled current name).
