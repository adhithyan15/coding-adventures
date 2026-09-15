- `earth-science/soil-texture-class.adj` (new) -- a new `soil_texture_class(class,
  description)` table names the three soil particle-size separates and the diameter range
  that actually defines each (clay->less_than_two_thousandths_of_a_millimeter_in_diameter,
  silt->between_two_thousandths_and_five_hundredths_of_a_millimeter,
  sand->larger_than_five_hundredths_of_a_millimeter_in_diameter), quoted verbatim from
  Wikipedia's "Soil texture" article -- `trust consensus`, the same tier this stdlib
  already reserves for other Wikipedia citations. Picked using the mandatory
  full-tree-grep-before-scoping discipline -- zero hits for `soil_texture_class`/
  `sand`/`silt`/`clay` before writing; distinct from the sibling `soil-horizons.adj`
  (a completely different axis -- vertical layers a soil pit exposes, not the
  particle-size classes that make up any one of those layers). Description atoms spell
  the decimal millimeter figures out in words rather than embedding a literal decimal
  point -- confirmed empirically that ADJ atoms cannot contain a `.` (a
  `less_than_0.002_millimeters` atom fails to parse) -- while the exact verbatim figures
  stay independently checkable in each row's quoted `source` span. Honest abstention on
  `loam`: a real, extremely common soil-texture term, but a composite class made of
  sand/silt/clay mixed together rather than one of the three particle-size separates
  itself. New manifest objective `adj.science.3to5.soil_texture_class` (band 3-5,
  `recall` competency, `ngss` coverage root). New e2e test
  `facts_soiltextureclass_e2e.rs` (3 tests: direct recall, reverse binding, honest
  abstention on a real-but-different-axis term).
