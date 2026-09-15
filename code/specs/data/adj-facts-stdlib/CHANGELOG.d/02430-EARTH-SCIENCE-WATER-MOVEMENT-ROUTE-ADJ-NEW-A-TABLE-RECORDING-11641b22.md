- `earth-science/water-movement-route.adj` (new) — a `table` recording which way through the Earth
  system a named water cycle process moves water: `water_movement_route(process, route)`, eight rows
  from three parallel sentences. `evaporation`, `evapotranspiration` and `precipitation` →
  `between_atmosphere_and_surface`; `snowmelt`, `runoff` and `streamflow` → `across_the_surface`;
  `infiltration` and `groundwater_recharge` → `into_the_ground`. The first library sourced from the
  U.S. Geological Survey's Water Science School.

  The BACKWARD query is the one worth having: "which processes move water into the ground?" is an
  ordinary elementary question, and nothing in this stdlib could answer it before.

  NOT A DUPLICATE OF `water-cycle.adj`, and the difference is worth stating because the names are
  close. `water_cycle_stage(stage, step_number)` ORDERS five processes — evaporation 1,
  condensation 2, precipitation 3, runoff 4, groundwater 5 — and answers "what comes next?". This
  relation answers "WHICH WAY does water move?", covers eight processes, and is not an ordering at
  all. They overlap on THREE atoms (`evaporation`, `precipitation`, `runoff`) and disagree about
  nothing, because they answer different questions about them: runoff is stage 4 there AND moves
  water across the surface here, and neither claim implies the other. Five of this table's processes
  have no stage number, and `condensation`, which has one, has no route here. (`groundwater`, stage
  5, and `groundwater_recharge`, a route value here, are different atoms naming a reservoir and a
  process respectively, and are deliberately not unified.)

  An earlier draft of this entry described the sibling as a three-stage table overlapping on two
  atoms. That was wrong — it was read off a grep whose pattern happened to exclude `runoff` — and is
  recorded because the error class matters more than the fact: a filtered view was generalised into a
  claim about the whole table.

  THE ABSTENTION WORTH UNDERSTANDING IS `condensation`. It is unmistakably a water cycle process, it
  is stage 2 in the sibling table, and this very page discusses it — so a system answering from
  general knowledge would assign it a route without hesitating. None of the three route sentences
  lists it, so this relation has no value for it. BEING A FAMOUS PROCESS IS NOT EVIDENCE ABOUT THIS
  RELATION. `sublimation` abstains for the adjacent reason (named once on the page, never placed in a
  route sentence — mentioned-on-the-page is not the same as stated-by-the-sentences this relation
  draws from), and `transpiration` abstains because the sentence lists the compound
  `evapotranspiration`; splitting it would assert a decomposition the source does not make, which is
  a real temptation since the compound visibly contains both words. The three routes are not claimed
  exhaustive or exclusive: a process absent here means these sentences did not list it.

  The route atoms compress the sentences' own phrasing ("between the atmosphere and the surface",
  "across the surface", "into the ground") into single atoms. That is a naming choice rather than a
  claim — the full phrasing is recoverable verbatim from the citation attached to every row.

  AN EXTRACTION HAZARD IS RECORDED AND PINNED BY A TEST, because it nearly put a wrong string in the
  `source` envelope. Each process name on the page is wrapped in a link — `<a>evaporation</a>,
  `<a>evapotranspiration</a>` — so naive tag-stripping yields "evaporation , evapotranspiration",
  with spaces the page does not contain. The sentences were checked against the RAW HTML instead, and
  the e2e test asserts both that the real punctuation survived and that the artifact form is absent.
  A citation is only worth having if it is byte-faithful. The page was content-verified by raw text
  extraction (HTTP 200, 84 substantive sentences, zero soft-404 markers), never from a fetch summary.

  New `water-movement-route.query.adj` and `facts_watermovementroute_e2e.rs` (6 tests: the direct
  route with its verbatim sentence and citation, both reverse lookups plus a negative that other
  routes' processes are not returned, the byte-faithful punctuation guard, the condensation
  abstention, the compound not being split alongside a positive check that the compound itself is a
  value, and the sublimation abstention). Every negative assertion was mutation-tested: adding
  condensation and transpiration rows makes both abstentions bind, and injecting the
  space-before-comma artifact makes the artifact needle appear while the real-punctuation needle
  disappears.

  BOTH ABSTENTION TESTS CARRY A POSITIVE CONTROL, added after review showed they stayed green against
  a table whose atoms had all been renamed — an abstention assertion alone cannot distinguish "this
  process has no route" from "this library answers nothing". A dead needle was also REMOVED rather
  than kept: asserting that no route atom appears is strictly implied by the abstention, and would
  have gone silently vacuous if the query variable were ever renamed — the same silent-degradation
  shape as the two vacuous tests already caught in this series. New manifest objective
  `adj.science.3to5.water_movement_route`.

