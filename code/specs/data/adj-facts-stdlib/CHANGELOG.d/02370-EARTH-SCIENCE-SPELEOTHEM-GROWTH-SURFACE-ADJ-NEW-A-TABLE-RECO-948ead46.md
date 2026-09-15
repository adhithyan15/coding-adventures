- `earth-science/speleothem-growth-surface.adj` (new) — a `table` recording which cave
  surface a named dripstone speleothem grows from:
  `speleothem_growth_surface(speleothem, surface)`, `stalactite` → `cave_ceiling`, `stalagmite` →
  `cave_floor`. The FIRST cave/karst library in this stdlib — a full-tree grep for `speleothem`,
  `stalactite`, `stalagmite`, `dripstone` and `flowstone` returned nothing beforehand. Sourced from
  the U.S. National Park Service's "Speleothems" article in its Caves and Karst subject series,
  curl-fetched and read byte-for-byte; `trust authoritative`, the same publisher
  `earth-science/weathering-cause-type.adj` already cites for a different Earth process.

  "Which one hangs from the ceiling — the stalactite or the stalagmite?" is the most-confused pair
  in elementary Earth science, and exactly the kind of question that should be answered from a
  citation rather than from a mnemonic. The relation runs BACKWARD too, which is the direction the
  confusion actually runs in: `? speleothem_growth_surface($P, cave_floor)` binds `stalagmite`.

  THE ABSTENTIONS CARRY AS MUCH OF THIS FILE'S CONTENT AS THE TWO ROWS DO, and two of the five e2e
  tests are about what the table declines to say. `column` is not a row, and the source says why in
  its own words: "Columns are not stalactites nor are they stalagmites; they are both, together." A
  column forms when a stalagmite grows together with its counterpart feeder stalactite — produced by
  two speleothems JOINING rather than growing from one surface — so it has no single growth surface
  to bind; recording either surface would be false, and recording both would misrepresent the
  relation. A test asserts neither surface is ever bound for it. `helictite` is not a row either,
  although the source DOES place it: "Helictites grow on cave ceilings, walls, and less often on cave
  floors." Three surfaces, with the source's own frequency hedge on the third — tabling one would
  silently drop the others, and tabling all three as equals would flatten the "less often" the
  source deliberately states, so it abstains until a shape exists that can carry the hedge.
  `flowstone` and `draperies` are described by APPEARANCE and mode of deposition ("melted cake
  icing", "frozen waterfalls", build-up in layers or bands) rather than by a growth surface, so
  neither has a value for this relation. `soda_straw` is the hollow tube every stalactite BEGINS as
  ("All stalactites, whatever their composition, begin their growth as hollow soda straws") — a
  developmental stage rather than a separately-placed speleothem, and a candidate for its own future
  table on a different axis.

  SOURCE SELECTION NOTE: this page was CONTENT-verified, not merely status-checked — 200, no
  soft-404 markers, 118 substantive sentences, zero hub/navigation markers. That check matters
  because an earlier candidate this session returned HTTP 200 while serving a 404 body, and three
  others returned clean 200s for pages that were hubs or link directories. Four probes in the same
  round as this one failed the check (two 404s, two CloudFront "Request blocked" 403s). New
  `speleothem-growth-surface.query.adj` and `facts_speleothemgrowthsurface_e2e.rs` (5 tests: the
  classic pair settled with its citation, both grounding sentences carried, backward recall from the
  surface, and the two abstentions with negative assertions that no wrong surface is ever bound). New
  manifest objective `adj.science.3to5.speleothem_growth_surface`.

