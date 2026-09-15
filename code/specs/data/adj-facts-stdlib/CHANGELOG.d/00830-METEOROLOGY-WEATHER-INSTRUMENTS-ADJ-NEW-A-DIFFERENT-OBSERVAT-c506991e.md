- `meteorology/weather-instruments.adj` (new) -- a DIFFERENT "observation and measurement" axis
  from the already-shipped `chemistry/measuring-tools.adj` (lab tools) -- this one covers
  weather-OBSERVING instruments. A new `weather_instrument(instrument, quantity)` table names
  which ONE quantity each of six instruments measures (anemometer->wind_speed,
  weather_vane->wind_direction, barometer->atmospheric_pressure,
  thermometer->air_temperature, hygrometer->humidity, rain_gauge->rainfall), quoted verbatim
  from NOAA's "Build Your Own Weather Station" education page, whose six section headings each
  name one instrument and the quantity it measures. WebFetch-verified TWICE for consistency
  before writing. `trust authoritative` -- a primary NOAA (.gov) source, matching the sibling
  `precipitation-types.adj`/`wind-scale.adj` NOAA sources' tier. Continues diversifying the
  science lane after a fresh survey of biology food-chain-roles.adj/animal-diets.adj and
  meteorology precipitation-types.adj/wind-scale.adj again found no clean, uninvented causal
  pairing (the food-chain-role/diet-category vocabularies don't share a key without asserting an
  unstated "herbivore IS a consumer" link). Grounds the NGSS science-practice observation/
  measurement gap. New manifest objective `adj.science.3to5.weather_instruments` (band 3-5,
  `recall` competency). New e2e test `facts_weatherinstruments_e2e.rs` (3 tests: direct recall,
  reverse binding, honest abstention on a non-weather instrument).
