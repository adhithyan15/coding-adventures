- `oceanography/ocean-current-surface-position.adj` (new) — a sibling to the already-shipped
  `ocean-current-drivers.adj` (`ocean_current_driver(current_type, driver)`, ONE physical driver per
  current type — wind_driven_currents → wind). That table's own header already quotes, verbatim, a NOAA
  sentence stating "Winds drive currents that are at or near the ocean's surface." — a positional fact the
  single `driver` atom had no room for. New `ocean_current_surface_position(current_type, position)` table
  decodes that clause as its own row: wind_driven_currents → at_or_near_the_oceans_surface. Honest
  abstention on thermohaline_circulation, whose cited span names no position. New e2e test file
  `facts_oceancurrentsurfaceposition_e2e.rs` (3 tests: forward recall with citation, backward recall,
  honest abstention on thermohaline_circulation). No manifest objective, matching
  `ocean-current-drivers.adj`'s own precedent. Third slice from the oceanography/ domain sweep.
