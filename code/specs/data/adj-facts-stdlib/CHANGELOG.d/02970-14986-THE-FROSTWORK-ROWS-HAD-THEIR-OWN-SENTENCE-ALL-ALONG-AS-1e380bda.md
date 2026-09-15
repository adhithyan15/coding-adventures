- **#14986: the frostwork rows had their own sentence all along — as a corroboration.**
  `earth-science/speleothem-substrate.adj` held BOTH justifying sentences already: the helictite one
  as the envelope `source`, the frostwork one as a table-level `cites`. Every row carried both, so the
  five FROSTWORK rows were warranted by a sentence about helictites, with their own evidence demoted
  to an untiered corroboration.

  That is what the atmosphere review rejected in the other direction (#15137): **the field carrying
  the tier must be the sentence that supports the row.** Nothing was fetched and no span was widened —
  both sentences already name their own speleothem. The `cites` became the frostwork rows' `source`.

  The envelope now carries the page's own DEFINITION of a speleothem. A draft used *"In general,
  however, one thing caves do have in common is where speleothems form."*; review read it in context
  and found that on the page it introduces the **water-table zone**, not the substrate — and that its
  leading connective has no antecedent inside the quote, the defect this file's own header argues
  against.

  ### The header had recorded this as a display quirk

  It read: *"Provenance is TABLE-level, so all eleven rows carry both sentences and `--explain` shows
  only the primary one — meaning the frostwork rows display under the helictite sentence there…
  the JSON output is the authoritative view."* But the JSON was not a different view of the same
  thing: there too, a frostwork row's `source` — the tiered field — was the helictite sentence. It
  was not a rendering problem, it was the warrant. (#13898, the `--explain` renderer dropping
  corroborations, stays open; this file simply no longer has corroborations to drop.)

  ### Two dead ends in pinning it, both worth recording

  1. A **fully-ground query** (`speleothem_substrate(frostwork, ledge)`) emits no `citations` at all —
     the engine ranks it as a hypothesis instead of recalling it. The first assertion failed with zero
     citation blocks, and the count was right.
  2. Binding only the speleothem returns six rows for `helictite`, so a whole-stdout `contains` is
     satisfied by any sibling's intact copy — the masking defect found in `joint-types` (#15164).

  The shape that works: bind the SUBSTRATE and use a needle that **spans from the binding into the
  citation** — `"bindings":{"S":"frostwork"},"citations":[{"source":"Frostwork can also…`. It cannot
  match an answer that bound a different speleothem even when both are in the same output, which is
  what `cave_wall` and `cave_ceiling` require: those two substrates belong to both speleothems.

  ### Pins

  **16 of 16 mutants killed, two controls counted separately.** Warranting a frostwork row with the
  helictite sentence and the reverse; restoring the old table-level `cites`; **all eleven rows broken
  one at a time**; dropping a row's source so it inherits the framing envelope; repointing the
  locator. Controls: unmutated green, fabricated envelope green — disclosed, since every row overrides
  `source`.

  **The `trust` gap, disclosed as in the sibling entries and adjudicated against this file:** changing
  the envelope to `trust consensus` reddens the suite (inheritance is real, all eleven rows inherit),
  but *deleting* the envelope's `trust authoritative` line leaves it green, because `lower.rs:2622`
  defaults an envelope carrying a `source` to `Authoritative`. So the `"trust":"authoritative"` in
  every needle cannot tell inheritance from that default.

  The harness anchored per-row mutants on the row header immediately followed by its `source` line,
  which **missed the first row of each speleothem** — those carry an explanatory comment in between.
  The assert caught it as a harness bug; anchors are now whole `row … { … }` blocks.

  A draft of this paragraph said that miss would have reported "eleven surviving mutants". Counted on
  the shipped file: **two**. Nine of the eleven row blocks open with `source` on the line after the
  header, so the naive anchor matched them; only `helictite/cave_ceiling` and `frostwork/stalactite`
  carry a comment in between.

  **Fourth anchor miss of this family**, counted in this file: `rainforest-layer` and
  `atmosphere-layers` (both produced false "surviving mutant" reports), `hormone-glands` below
  (caught by its assert), and this one. A draft named `joint-types` as the third — that entry's
  harness defect was the sibling-row MASKING bug, a different fault, and it was not caught by an
  assert.

