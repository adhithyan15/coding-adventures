- `astronomy/sun-layer.adj` (new) -- a new `sun_layer(layer, description)` table names two
  layers of the Sun and what each actually is
  (photosphere->the_visible_surface_of_the_sun, corona->the_suns_outer_atmosphere), quoted
  verbatim from NASA's "Layers of the Sun" blog post (The Sun Spot) -- `trust authoritative`,
  the same tier the sibling `celestial-objects.adj`/`comet-part.adj`/`space-rock-stage.adj`/
  `lunar-eclipse-type.adj` already use for their NASA citations. Picked using the mandatory
  full-tree-grep-before-scoping discipline -- zero hits for `sun_layer` before writing (a
  stray `corona` hit in `anatomy/heart-valves.adj` was confirmed to be "coronary sinus", an
  unrelated anatomical structure, not a false conflict). WebFetch-verified twice -- the
  second pass pulled the full surrounding paragraph for every layer named on the page,
  confirming `photosphere` and `corona` are the only two whose own sentence states a single
  clean fact. Only two rows are shipped, an intentionally smaller table than most siblings
  in this directory, since core/radiative-zone/convection-zone/chromosphere each bundle
  location together with process or temperature details in one grammatically unified
  sentence rather than stating one clean fact -- the honest-abstention discipline applies to
  table SIZE as much as to individual queries, reinforcing the "reject bundled-fact
  sentences" lesson from earlier this session. Honest abstention on `chromosphere`: a real
  solar layer the same page names, but its sentence bundles position together with a
  temperature range as one relative clause rather than one clean fact. New manifest
  objective `adj.science.3to5.sun_layer` (band 3-5, `recall` competency, `ngss` coverage
  root). New e2e test `facts_sunlayer_e2e.rs` (3 tests: direct recall, reverse binding,
  honest abstention on a real-but-bundled layer).
