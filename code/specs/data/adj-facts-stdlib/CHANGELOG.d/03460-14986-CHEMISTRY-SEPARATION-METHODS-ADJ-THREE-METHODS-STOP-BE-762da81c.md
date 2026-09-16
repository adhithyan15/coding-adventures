- **#14986: `chemistry/separation-methods.adj` — three methods stop being warranted by the filtration span.**
  The envelope was the filtration span, so a distillation, evaporation or chromatography answer's
  primary source was a sentence about filtration, and those three sentences were header prose that
  reached no answer at all.

  ### What each row carries now

  Measured 2026-09-15 on the Chemistry LibreTexts page "1.16: Methods for Separating Mixtures"
  (HTTP 200; a nonsense path on the same host returns a real 404 holding none of these spans), with
  inline tags removed without a space and only ASCII whitespace collapsed.
  - **Each row takes the span that names its own method.** Each occurs exactly once, scripts set
    aside and over the whole file, and zero times on the nonsense page.
  - **The evaporation row takes TWO sentences.** Its shipped span was *"The method drives off the
    liquid components from the solid components."* — which names no technique at all. The sentence
    before it does (*"Evaporation is a technique used to separate out homogeneous mixtures that
    contain one or more dissolved salts."*), and the two are contiguous in one `<p>`, so the row
    carries them as one span. That is the remedy `README.md` prescribes under "A citation must name
    its own subject": widen the quote until it is self-contained.
  - **The envelope** is now the first bullet of the page's own **Summary** list — *"Mixtures can be
    separated using a variety of techniques."* It names no method and no basis, and occurs exactly
    once, inside one `<li>`, with no `<head>` copy.
  - **The other Summary bullets are deliberately not used as row spans.** They do name a method and
    a property ("Distillation takes advantage of differences in boiling points."), but the shipped
    `basis` atoms are grounded in the body prose — `by_volatility` comes from "the most volatile
    component vaporizes at the lowest temperature", which is not what "boiling points" says.
  - Every row's page is the envelope's, so no row restates a locator line or trust.

  ### The first `consensus`-tier conversion

  Every RS-5e conversion before this one was `trust authoritative`. This table is `consensus`
  (LibreTexts is a curated teaching resource, not a standards body), so the per-row needle pins
  `"trust":"consensus"`. A needle copied from an authoritative sibling would have asserted the wrong
  tier while still matching the sentence text — there is a mutant for exactly that, and it is killed.

  ### Pins

  - **Kept:** all three forward binds, the reverse bind, and the `centrifugation` abstention.
  - **Inverted:** `contains("chem.libretexts.org") && contains("\"trust\":\"consensus\"")` — satisfied
    by any LibreTexts citation and by a truncated span, since it constrains no source text. Each
    answer is now pinned to its own whole citations array, closing on both the corroborations `]` and
    the citations `]` (#14735).
  - **Added:** a per-method check with negative arms and an assertion that each span *names* its own
    method; a check that the evaporation row carries the sentence its span points back to and that no
    row is warranted by the dangling sentence alone; a table-shape test that also pins the tier.

  **10 of 10 mutants killed, two controls:**
  - the filtration row reverted to a bare row that inherits the envelope;
  - the distillation row reverted to the filtration span (the exact defect being fixed);
  - the evaporation row cut to the dangling sentence alone;
  - the evaporation row cut to its naming sentence alone;
  - the old envelope restored;
  - **the trust tier swapped to `authoritative`**;
  - a table-level `cites` re-added;
  - a basis atom rebound;
  - the envelope naming a method;
  - the chromatography span truncated mid-sentence.

  The controls are the unmutated suite, green before the first mutant and after the last, with the
  file byte-identical afterwards. It's a local scratch harness, so that count can't be reproduced
  from the repo.

  **The envelope's wording reaches no answer.** The shipped query example returns 4 answers and 1
  abstention, every corroboration array is empty, and the framing sentence occurs zero times in the
  output.

  The query example said the engine returns the basis plus "the table's source/locator/trust". It now
  describes the row's own span, with the locator and trust inherited — the fourth conversion running
  to carry that stale line.
