- `earth-science/well-aquifer-type.adj` (new) — a `table` recording which kind of aquifer each kind
  of well is drilled into: `well_aquifer_type(well, aquifer_type)`, three rows grounded by a single
  sentence from the U.S. Geological Survey's Water Science School aquifers page. `artesian_well` and
  `flowing_artesian_well` → `confined_aquifer`; `water_table_well` → `unconfined_aquifer`. Third
  USGS-sourced library.

  The REVERSE query is the one worth having: "which wells reach a confined aquifer?" returns BOTH
  artesian kinds. The sentence names "an artesian well AND a flowing artesian well" as distinct
  things, so both are rows; collapsing them into one atom would assert that the source treats them as
  the same well.

  TWO HONESTY CALLS ARE STATED IN THE HEADER RATHER THAN LEFT IMPLICIT. First, the grounding sentence
  is a FIGURE CAPTION, not body prose — it is preceded on the page by "Media Sources/Usage: Public
  Domain. View Media Details", so the header says to look under the illustration — a citation that
  sends a reader hunting in the wrong part of a page is a worse citation. The header also records
  that the caption block closes with "Credit: Environment and Climate Change Canada", so the
  illustration is REPUBLISHED by the USGS rather than drawn by it. The quoted text appears verbatim
  on a USGS page and that is what `locator` points at, but the header deliberately does NOT claim the
  USGS authored the sentence, because the page does not support that. `trust authoritative` rests on
  the USGS being the first-party PUBLISHER of what it serves, which is the claim the page does
  support. An earlier draft of this header asserted USGS authorship, which was a claim beyond the
  evidence. Second, a tempting corroboration was DECLINED: the article also says "Groundwater in
  aquifers between layers of poorly permeable rock, such as clay or shale, may be confined under
  pressure", which explains what CONFINED means and is genuinely useful — but it grounds no row,
  because it never says which wells reach which aquifer. Attaching it as a `cites` would dress up
  background as corroboration. A corroboration should support the rows, or it should not be in the
  envelope; it is recorded in the header as context instead.

  Honest abstention on dug, drilled and driven wells — standard types a reader may well ask for, but
  not among the three this sentence names — and on `spring`, which is a groundwater feature the page
  discusses but is not a well. The relation says which aquifer a well is DRILLED INTO and nothing
  else: not depth, not yield, and not whether the well flows without pumping, which the page does
  discuss for artesian wells but which is a different relation.

  Recorded without being made a row: this page says "The upper surface of this zone of saturation is
  called the water table", which is consistent with `zone-water-table-position.adj` — built from a
  National Park Service page — placing `zone_of_saturation` below the water table. Two independent
  .gov publishers using the same vocabulary the same way is worth knowing, but it is a definition
  rather than a new axis.

  The page was content-verified by raw text extraction (HTTP 200, 285 substantive sentences, zero
  soft-404 markers) and the quoted sentence confirmed to appear verbatim in that extraction. New
  `well-aquifer-type.query.adj` and `facts_wellaquifertype_e2e.rs` (5 tests: the direct lookup with
  its verbatim caption and citation, both artesian kinds returned for the confined aquifer with a
  negative that the unconfined well does not leak in, the water table well as the only unconfined one
  with the converse negative, abstention on unnamed well types, and abstention on a spring — the last
  two each with a positive control). EVERY assertion uses the JOINT binding form
  (`"bindings":{"W":"..."}`) rather than independent substring scans, applying this series' hardest-
  won lesson from the start rather than after review. All four negative assertions were
  mutation-tested and each fires under exactly its own mutation. New manifest objective
  `adj.science.3to5.well_aquifer_type`.

