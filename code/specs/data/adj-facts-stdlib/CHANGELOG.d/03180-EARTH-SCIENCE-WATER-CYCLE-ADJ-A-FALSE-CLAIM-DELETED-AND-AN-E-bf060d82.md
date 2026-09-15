- `earth-science/water-cycle.adj` — **a false claim deleted, and an envelope that
  mis-warranted 4 of 5 rows replaced.** This table is *deliberately not* converted to per-row
  `source` (RS-5e, #14986), and the measurement is the reason.

  **The page does not state the step numbers.** The second column is an INTEGER, and no
  sentence on `https://water.usgs.gov/edu/watercycle-kids-beg.html` assigns a number to a
  stage. Measured 2026-09-14, and stated so it can be re-run: **the page has no ordinal
  vocabulary at all** — `first`, `second`, `third`, `fourth`, `fifth`, `step`, `order`,
  `begins`, `next`, `follow` are each **0 occurrences**, zero even as *substrings*. Digits are
  nearly as scarce, and the extraction rule matters, so: splitting the whole tag-stripped page
  on sentence punctuation and **retaining** `<script>`/`<style>` text gives **3** "sentences"
  with any digit — the `<head>` blob (CSS rule plus every inline script body), *"oceans cover
  70% of the Earth's surface"*, and the footer block, whose digits are `Page Last Modified:
  Wednesday, 02-Apr-2025 16:43:33 EDT`. **Dropping** script and style first gives **2**. Under
  either rule, none numbers a stage.

  A per-row `source` asserts "this span warrants this row". None of these spans warrants an
  integer, so attaching one would make the file look better warranted while claiming more than
  was measured — the defect #14986 exists to remove, not an instance of fixing it. The ordering
  is an **inference** from the prose handoff chain, which is the shape still undecided.

  **A false claim is deleted.** The header said the page "names these stages and describes them
  in this process order". It does not: the page's **six diagram section headings** run
  Condensation → Evaporation → Groundwater → Precipitation → Runoff → The Sun, which is
  **alphabetical** (sorting that list returns it unchanged), and is not the table's order. The
  *handoff* half of the claim is true and is kept; the *ordering* half is gone. Six, not seven:
  the page carries a seventh `<h3>`, first in source order — the sidebar's *"Versions
  available:"* — and including it makes the list unsorted, so naming which six is the
  difference between a claim that can be re-run and one that cannot.

  **What is fixed, needing no undecided machinery.** The envelope `source` was the EVAPORATION
  sentence, so `? water_cycle_stage(runoff, $N)` came back warranted by a sentence about the
  sun evaporating water — **four of the five rows were in that position**. It is now the page's
  own framing sentence, which names no stage. Each row gains a row-level `cites` holding the
  sentence that names its own stage: **`cites`, deliberately not `source`**, because the
  sentence corroborates that the stage belongs to this cycle and says nothing about its number.

  **A mutant survived, and it is the finding, not a footnote.** Renumbering `runoff` from 4 to
  9 left the whole suite green — because every assertion in it is about provenance, and *no
  citation can catch a wrong step number when no citation states one*. The remedy was neither a
  smaller mutant nor a fake citation: the five pairs are now pinned to the **shipped artifact**,
  in the `.adj` and in what the engine returns, with the test saying plainly that this is an
  artifact pin and not a warrant.

  **Security review then found the guard that mattered most was a text match.** "No row carries
  a `source`" was implemented as a count of `\n        source "` in the `.adj` — so a row
  `source` written at any indentation other than exactly eight spaces evaded it, evaded the
  four-space table-level filter too, and left the `cites` count at five, while **nothing
  anywhere asserted that a row's emitted primary `source` is still the envelope**. That is the
  entire claim of this table's shape. It is now asserted semantically on the CLI's real output,
  per row: one contiguous span binding envelope source + locator + trust to that row's own
  sentence as first corroboration. Three new mutants — a row `source` at seven spaces, at nine
  spaces, and one added *alongside* its `cites` so both counts stay correct — are killed by
  that pin and by nothing else. Review also corrected two mislabelled digit-sentences, an
  incomplete `<h3>` inventory, a dead filter conjunct, and added a path-component assert where
  a row key read from raw `.adj` bytes reaches a temp-directory name.

  12 of 12 mutants killed, green baseline before and after (local scratch harness, so that
  count is not reproducible from the repo).

  The suite also gains a pairing guard for the trap here: the groundwater sentence names
  **three** stages (*"Some precipitation and runoff soaks into the ground to become
  groundwater"*), so "the span mentions the key" would hand it to the precipitation or runoff
  row. And the old citation assertion — `contains("water.usgs.gov") &&
  contains("\"trust\":\"authoritative\"")` — was the #15139 shape, two halves satisfiable by
  different parts of the output; it is now one contiguous span.
