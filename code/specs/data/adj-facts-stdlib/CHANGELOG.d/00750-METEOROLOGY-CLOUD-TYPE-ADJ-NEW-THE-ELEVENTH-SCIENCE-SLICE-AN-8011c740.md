- `meteorology/cloud-type.adj` (new) -- the ELEVENTH science slice, and a genuinely new
  "observation and measurement" axis from the already-shipped `weather-instruments.adj` and
  `ocean-observing-instruments.adj` (instrument -> quantity measured): this table names a cloud
  TYPE and the weather it indicates, not an instrument at all. A new `cloud_type(cloud,
  weather_indication)` table names three cloud types (cirrus -> approaching_warm_front,
  cumulonimbus -> heavy_rain_thunderstorm, stratus -> light_rain_drizzle_or_none), quoted
  verbatim from the National Weather Service's (Louisville forecast office) "Cloud
  Classification" education page, `trust authoritative`. WebFetch-verified before writing (note:
  the related jetstream.noaa.gov domain 403s WebFetch entirely -- weather.gov was used instead,
  per this stdlib's established workaround). Honest abstention on "altocumulus" (a real cloud
  type, but not one this source classifies by weather indication). New manifest objective
  `adj.science.3to5.cloud_type` (band 3-5, `recall` competency, `ngss` coverage root). New e2e
  test `facts_cloudtype_e2e.rs` (3 tests: direct recall, reverse binding, honest abstention on
  an untabled cloud).
