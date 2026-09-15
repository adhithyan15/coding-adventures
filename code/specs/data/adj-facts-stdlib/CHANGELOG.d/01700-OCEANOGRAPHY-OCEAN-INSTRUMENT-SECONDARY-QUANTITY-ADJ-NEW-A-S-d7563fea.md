- `oceanography/ocean-instrument-secondary-quantity.adj` (new) — a sibling to the already-shipped
  `ocean-observing-instruments.adj` (`ocean_instrument(instrument, quantity)`, ONE quantity per
  instrument — sonar → distance_to_object). That table's own header already quotes, verbatim, a NOAA
  sentence stating sonar determines "the range and orientation of the object" — a second, distinct
  quantity ("orientation") the single `quantity` atom had no room for. New
  `ocean_instrument_secondary_quantity(instrument, secondary_quantity)` table decodes that clause as its
  own row: sonar → orientation_of_object. Honest abstention on tide_gauge, whose cited span names no
  second quantity. New e2e test file `facts_oceaninstrumentsecondaryquantity_e2e.rs` (3 tests: forward
  recall with citation, backward recall, honest abstention on tide_gauge). No manifest objective, matching
  `ocean-observing-instruments.adj`'s own precedent. Second slice from the oceanography/ domain sweep.
