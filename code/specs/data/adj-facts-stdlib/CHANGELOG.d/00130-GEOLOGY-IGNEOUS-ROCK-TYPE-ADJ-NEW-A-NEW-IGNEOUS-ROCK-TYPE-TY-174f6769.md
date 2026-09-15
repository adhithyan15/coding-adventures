- `geology/igneous-rock-type.adj` (new) -- a new `igneous_rock_type(type,
  description)` table names the two broad types of igneous rock and what
  actually defines each (intrusive->solidifies_within_earth,
  extrusive->erupted_onto_the_surface_or_into_the_atmosphere), each quoted
  verbatim from its own standalone sentence in the U.S. National Park
  Service's "Igneous Rocks" geology page -- `trust authoritative` (a
  U.S. government .gov source). Distinct from the sibling
  `earth-science/rock-types.adj`, which names the three ROCK-CYCLE families
  (igneous/sedimentary/metamorphic) by formation mechanism, not this
  within-igneous split by cooling location. Picked after checking two
  not-yet-reviewed science tables this window (heat-transfer.adj,
  chemical-bonds.adj -- both closed exhaustive classifications,
  non-extendable), then researching igneous rock types as a fresh topic.
  Unlike most sibling tables, intrusive/extrusive is a genuinely
  EXHAUSTIVE two-way split (the source's own opening line states there are
  exactly two broad types), so honest abstention is instead demonstrated
  with `hypabyssal`: a real geological term for shallow-depth cooling, but
  one this source's own two-category framework does not name or pin to a
  defining sentence. New manifest objective
  `adj.science.3to5.igneous_rock_type` (164 objectives total, up from
  163). New e2e test `facts_igneousrocktype_e2e.rs` (3 tests: direct
  recall, reverse binding, honest abstention).
