- `biology/plant-tropisms.adj` (extended) -- extended the already-shipped
  `tropism_stimulus(tropism, stimulus)` table from 5 to 12 rows. The SAME
  already-cited Wikipedia "Tropism" article's own "Types of tropism" list
  names seven MORE tropisms beyond the original five, each with its own
  clean single-fact definition sentence: aerotropism->wind,
  electrotropism->electric_field, heliotropism->sun_direction,
  magnetotropism->magnetic_fields, selenotropism->moon_direction,
  thermotropism->temperature, and traumatotropism->wounding.
  WebFetch-verified twice (a targeted second pass re-fetched all seven new
  terms' raw definition text directly) -- zero new source page needed.
  Honest abstention updated to `inotropism`: a real term on the SAME
  Wikipedia page, but naming a MUSCLE's contraction response to drugs, not
  a plant's growth response to an environmental stimulus -- the wrong
  domain entirely. (The page's lead paragraph also uses "anemotropism" as
  its own naming-convention example for a wind-response, a different name
  for the same wind-response the page's own types list separately calls
  "Aerotropism" -- this table uses the types-list's own canonical name,
  mirroring how it already uses `gravitropism` rather than the page's
  noted synonym `geotropism`.) Extended e2e test `facts_planttropisms_e2e.rs`
  to 2 tests. No new manifest objective (same library, same objective, 167
  total unchanged). This is the THIRD successful extend-pattern win this
  window (after animal-classes.adj +10 rows and idiom-meaning.adj +20
  rows) -- checking whether a table's own already-cited source names more
  items than tabled continues to be the strongest, safest move for both
  lanes.
