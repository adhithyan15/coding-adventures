- `oceanography/ocean-observing-instruments.adj` (new) -- a THIRD "observation and measurement"
  axis for the science domain, after `chemistry/measuring-tools.adj` (lab tools) and
  `meteorology/weather-instruments.adj` (weather-observing instruments) -- this one covers
  OCEAN-observing instruments. A new `ocean_instrument(instrument, quantity)` table names which
  ONE quantity each of three instruments measures or detects (tide_gauge -> sea_level,
  hydrophone -> underwater_sound, sonar -> distance_to_object), quoted verbatim from three
  DIFFERENT NOAA oceanservice.noaa.gov "facts" pages, WebFetch-verified before writing.
  `trust authoritative` -- a primary NOAA (.gov) source, the same tier `weather-instruments.adj`'s
  source earned. UNLIKE `weather-instruments.adj` (six rows sharing ONE source page), this
  table's three rows each cite a DIFFERENT page -- since the ADJ table grammar carries only one
  table-level `source`/`locator`/`trust` block (confirmed by reading `weather-instruments.adj`
  and `word-families.adj` before writing), the table's own citation is the primary/first-listed
  source (tide-gauge.html) and each other row's own distinct citation is documented in the
  file's header prose, the same discipline `word-families.adj`'s multi-family extensions
  established. Deliberately excludes a CTD (which measures MULTIPLE quantities at once --
  conductivity, temperature, and depth -- not one, so it does not fit this table's
  one-instrument-one-quantity shape) and a buoy/ocean glider (both 404'd on
  oceanservice.noaa.gov this session, no citable page found). New manifest objective
  `adj.science.3to5.ocean_instruments` (band 3-5, `recall` competency, matching
  measuring-tools.adj's and weather-instruments.adj's band). New e2e test
  `facts_oceaninstruments_e2e.rs` (3 tests: direct recall, reverse binding, honest abstention on
  a CTD).
