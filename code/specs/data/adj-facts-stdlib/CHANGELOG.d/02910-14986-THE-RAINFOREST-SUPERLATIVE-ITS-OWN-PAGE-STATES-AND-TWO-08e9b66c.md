- **#14986: the rainforest superlative its own page states, and two atoms it does not.**
  `biology/rainforest-layer.adj` had one envelope — the EMERGENT row's sentence — so a recall of the
  forest floor was warranted by a sentence about the treetops. Each row now carries its own.

  ### The superlative was never the problem

  The bounding-language audit flagged `forest_floor → darkest_layer_hard_for_plants_to_grow` as a
  row asserting a bound whose provenance states none. Measured 2026-09-13 UTC with a raw fetch, the
  page says **"The forest floor is the darkest of all rainforest layers, making it extremely
  difficult for plants to grow."** — one occurrence. The claim was always supported; the evidence
  simply sat outside the machine-readable envelope, which is exactly what RS-5e moves.

  The two audits were pointing at one defect here. **Not for the first time** — an earlier draft of
  this entry claimed it was, and #14758's tropic rows, four entries down, are the same convergence:
  a row asserting a bound, warranted by an envelope that states none. That is the second time in
  three entries I have written a false "first in the series" claim, and the second time review
  caught it rather than me. The claim added nothing; only the observation does.

  ### Two atoms restate rather than quote

  | row | tier | why |
  | --- | --- | --- |
  | `forest_floor` | `consensus` | the superlative is on the page, verbatim |
  | `understory` | `consensus` | darker, more humid, below the canopy — stated; **"layer" is not**, see below |
  | `emergent` | **`inferred`** | **"tallest" occurs ZERO times on the page** |
  | `canopy` | **`inferred`** | **"treetop" occurs ZERO times on the page** |

  The page says *"The top layer of the rainforest is the emergent layer"* and *"trees as tall as 60
  meters"*; `tallest` restates that. It says the canopy is *"a deep layer of vegetation"* whose
  leaves form a *"roof"*; `treetop` restates that. Both are readings — not fabrications like the
  kingdoms row removed above, and not quotations either.

  **`understory` is the weakest of the two kept at `consensus`, and the table above understated it.**
  Its atom is `dark_humid_layer_below_canopy`; its span supplies *below the canopy*, *darker* and
  *more humid*, but says "environment", not "layer". The tier is kept because `layer` is this
  table's own column vocabulary — `rainforest_layer(layer, description)` — and the framing span
  names understory as one of four layers. But the envelope reaches no answer, so a consumer of the
  citation never sees that warrant. Named rather than smoothed over.

  **The atoms are left alone.** Renaming a key changes what a recall returns, which is a larger
  decision than a provenance conversion and does not belong in the same change.

  ### A cost worth naming

  Marking those two rows `inferred` **drops the `consensus` signal** — that this is a teaching
  resource, not a primary source. The five tiers are one dimension; both facts are true of those
  rows and only one fits. Recorded rather than resolved.

  ### Instrument failures, three of them, all mine

  1. The atom-vs-span checker with a 4-character stem let **"tallest" match "tall as 60 meters"** and
     reported every token present. Whole-word matching surfaced it — and is in turn too strict for
     inflection (`darker`/`dark`). Neither setting adjudicates; the phrase counts above do.
  2. A near-miss control replaced the word "layer", which the understory span does not contain, so
     the "near-miss" was **identical to the real span** and scored 1. A control that cannot differ
     from its subject proves nothing. Now the final word is altered, with an assert that it differs.
  3. The envelope replacement matched as a **substring** of the emergent row's more-indented line,
     so the row answered with the framing span while the envelope kept the old sentence. Line
     anchored now.

  ### Pins

  **6 mutants, plus a restore-to-green control.** (The si-base-units entry below retired "7/7" for
  conflating a control with a kill; an earlier draft of this entry reinstated it. Same error, three
  entries apart.) Swapping the forest-floor span for the old emergent envelope, dropping its
  superlative clause, dropping the understory row's source, flipping either reasoned row's tier to
  `consensus`, and dropping the shared locator all redden.

  **Green by design, and disclosed here as well as in the test:** reverting the envelope `source` to
  the old emergent sentence leaves every test green. Once all four rows override `source`, no row
  can reach the envelope, so its wording is invisible to any output-based test — the same
  unreachability recorded for `si-base-units` and `planets`. The headline of this entry IS that
  envelope swap, so saying it is covered would be wrong; what IS pinned is that the framing span
  reaches no answer.

  One of those started as a surviving mutant and was **a harness bug, not a coverage gap**: a bare
  phrase replace hit a header comment quoting the same sentence and left the row untouched. Anchored
  to the row line, it reddens.

  A pre-existing test asserted `emergent` answers at `consensus`. That tier **changed on purpose**,
  and the test now says so rather than being quietly made green.

