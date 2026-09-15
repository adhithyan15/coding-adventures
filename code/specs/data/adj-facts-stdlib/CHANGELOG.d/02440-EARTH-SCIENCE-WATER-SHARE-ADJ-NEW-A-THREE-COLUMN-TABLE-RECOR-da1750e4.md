- `earth-science/water-share.adj` (new) — a THREE-column `table` recording what share of a stated
  body of water sits in each place, and what that share is a share of: `water_share(place, share,
  base)`, seven rows from the U.S. Geological Survey's "Where is Earth's Water?". Second
  USGS-sourced library; the first table in this stdlib on a percent/share axis.

  THE DENOMINATOR IS PART OF THE FACT, AND THAT IS THE WHOLE DESIGN. The page states seven shares
  against THREE different bases: `all_earths_water` (saline water over 96%, freshwater 2.5%),
  `all_freshwater` (ice and glaciers over 68%, ground 30%, surface water a little more than 1.2%),
  and `surface_freshwater` (lakes 20.9%, rivers 0.49%). A two-column `water_share(place, share)`
  would put "over 96 percent" and "over 68 percent" in one column as though they were comparable
  quantities. THEY ARE NOT: 96% is of all water on Earth, 68% is of the 2.5% of it that is fresh.
  Reading them as commensurable is the commonest error made with these statistics, and a table that
  invited it would be WORSE THAN NO TABLE, because it would carry a citation while doing it.

  With the base as a column, no query can retrieve a share without also retrieving what it is a share
  of, and the error becomes unrepresentable rather than merely warned about. `water_share(ground, $S,
  all_earths_water)` ABSTAINS: ground water is stated as 30% OF FRESHWATER, and converting bases is
  arithmetic this source never performs — a recall library that performed it would be presenting
  reasoning as citation. A bare unhedged figure also abstains, since the source says "over 96
  percent" and the hedge rides inside the atom per the `veto-override.adj` placement rule.

  NORMALISE REFERENTS, NEVER NORMALISE HEDGES — a distinction this library establishes explicitly.
  The source words one quantity several ways ("the world's total water supply" and "Earth's water";
  "the total freshwater", "all freshwater" and bare "freshwater"). Those are REFERENTS, different
  phrasings pointing at one physical quantity, so collapsing them to `all_earths_water` and
  `all_freshwater` loses nothing and keeps the reverse-by-base query from fragmenting across
  synonyms. That is deliberately different from `speleothem-substrate.adj`, which keeps frostwork's
  "less occasionally" distinct from helictite's "less often" even though they plainly mean the same
  thing — those words are the SOURCE'S OWN EPISTEMIC QUALIFICATION, and normalising them would edit a
  claim. A referent names the same thing twice; a hedge states how confident the source is.

  THE LAKES FIGURE REQUIRED A THREE-SENTENCE CITATION, because its base is a PRONOUN. It appears as
  "Most of this water is locked up in ice, and another 20.9% is found in lakes." — and "this water"
  means surface freshwater only because the preceding sentence says "The right bar shows the
  breakdown of surface freshwater." Cited alone, that 20.9% would have NO denominator at all, which
  is precisely the failure this library exists to prevent. The sentences are adjacent in the article,
  so quoting the run is still verbatim and makes the base recoverable from the citation itself.

  A LANGUAGE CONSTRAINT WORTH RECORDING: digit-initial atoms are a parse error, so the share atoms
  carry a uniform `share_` prefix (`share_over_96_percent`, `share_a_little_more_than_1_2_percent`)
  rather than a bare number. Honest abstention also covers atmospheric water, soil moisture and
  swamps, which appear in the page's HTML TABLES rather than its prose. Table cells do not survive
  tag-stripping reliably, and a figure that cannot be cited byte-faithfully is a figure this library
  does not state. The page was content-verified by raw text extraction (HTTP 200, 300 substantive
  sentences, zero soft-404 markers), and every quoted string was checked to appear verbatim in that
  extraction before shipping.

  New `water-share.query.adj` and `facts_watershare_e2e.rs` (6 tests: a share always arriving with
  its denominator, the reverse lookup scoped by base with a negative that other bases' places do not
  leak in, the cross-base abstention with a positive control, the unhedged-figure abstention with a
  positive control, the pronoun base travelling with its antecedent as a SINGLE spanning needle, and
  abstention on table-only figures with a positive control). Every negative assertion was
  mutation-tested and each fails under exactly the mutation it claims to guard: adding a cross-base
  row, baring the hedged figure, leaking a base, and splitting the pronoun citation.

  THREE ASSERTIONS WERE TIGHTENED AFTER REVIEW, and the reason generalises to every multi-column
  table in this stdlib. `out.contains("\"S\":\"x\"") && out.contains("\"B\":\"y\"")` proves only that
  each string appeared SOMEWHERE in stdout — it does NOT prove they arrived in the same answer.
  Mutation-verified: splitting the ice row into `(ice, over_68, surface_freshwater)` and
  `(ice, 2_5, all_freshwater)` satisfies both separate needles while the data is wrong, and a test
  whose message read "the share and its base are returned together" never noticed. The assertions now
  match the emitted binding object whole — `"bindings":{"S":"share_over_68_percent","B":
  "all_freshwater"}` — which the emitter writes contiguously in first-appearance variable order. The
  same fix pins place-to-share in the reverse lookup, where swapping the lakes and rivers figures had
  previously left the test green. A fourth abstention test gained the positive control it was
  missing, having stayed green when the relation was renamed so the library asserted nothing at all.
  This is the fourth instance in this series of a test passing while proving nothing; all four had the
  same root cause, which is asserting on the stdout blob rather than on the structural property. New manifest objective
  `adj.science.3to5.water_share`.

