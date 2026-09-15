- `physics/em-spectrum.adj` — all 7 rows converted to per-row provenance (RS-5e, #14986). The
  RADIO sentence was this table's `source`, the field that carries the tier, for every row, so
  `? band_use(x_ray, $A)` came back proved by *"Your radio captures radio waves emitted by radio
  stations, bringing your favorite tunes."* — **a sentence about radio stations, warranting a
  claim about dental X-rays.** Six of the seven rows were in that position; each now carries the
  sentence that names its own band and its own use.

  **No cross-row overlap:** each of the seven spans names its own band and no other, measured
  against the page's own wording for each key. `anatomy/eye-parts.adj` has three sentences that
  name more than one part, so there is no pairing trap here and a single-row truncation cannot
  hide behind a sibling's copy of the same span.

  Two corrections from review, both to claims this entry made about itself. **It said the
  property was "unique in this cascade so far". It is not** — eleven other converted files
  already have all-distinct spans (including `biology/vitamin-deficiency-symptom.adj`, the entry
  directly below this one), and `biology/plant-tropisms.adj`, `physics/energy-forms.adj` and
  `science/scientific-method-step.adj` also have zero cross-naming. The claim was made without
  looking. **And it said the property was "asserted, not just described" when the test asserted
  something weaker:** `spans.dedup()` is string dedup — "no two rows carry the *identical* span"
  — which says nothing about a span that *mentions* a sibling. The real check now runs, in the
  page's wording, with an own-band arm so it cannot pass vacuously on spans that name no band at
  all; two new mutants (a span reworded to name another band, and a span naming none) are killed
  by it and by nothing else. `earth-science/water-cycle.adj` is also described correctly now: it
  is an **unconverted** table blocked because its value is an integer no span states, not a
  converted-but-overlapping one.

  **The envelope becomes the page's own introduction to precisely this list** — *"The image below
  shows where you might encounter each portion of the EM spectrum in your day-to-day life."* — so
  it frames exactly what the table records while naming no band. All seven rows override it, so
  the framing sentence reaches zero answers, asserted as a count with a positive control.

  **The envelope check uses the page's wording, not the atoms**, and that is load-bearing: the
  page writes `x_ray` as "X-rays" and `gamma_ray` as "gamma-ray", so a framing sentence naming
  X-rays would sail past a scan for the atom `x_ray` while plainly naming a band.

  Review found that check was **reading a test constant rather than the shipped file** — it
  harvested the row keys from the `.adj` and then tested the hardcoded `ENVELOPE`, so the mutant
  this entry credited it with killing was in fact killed by a different test's
  `assert_eq!(envelope, ENVELOPE)`, and this one caught only a *coordinated* edit to both. It
  now reads the envelope from the file, and the mutant is killed by the test named after it.

  The old citation assertion — `contains("imagine.gsfc.nasa.gov") &&
  contains("\"trust\":\"authoritative\"")` — was the #15139 shape, two halves satisfiable by
  different parts of the output; it is now one contiguous span, and the source it pins is radio's
  own sentence rather than the table's.

  Source verified by **raw extraction** (2026-09-14, HTTP 200): all eight spans verbatim and
  occurring exactly once, each wholly inside one `<p>`, with a fabricated control sentence
  reported absent by the same comparison. **"Verbatim" is conditional on one stripping rule for
  three of the eight**, and review was right that it needed saying: the envelope, the microwave
  span and the visible span each straddle an inline `<a class="glossaryDef">` anchor. The anchors
  wrap plain text, so stripping tags with no separator reproduces the page's characters exactly;
  the other five are contiguous in the raw bytes.

  The file header was also rewritten. It still described the **pre-conversion** design — "An ADJ
  `table` carries ONE provenance envelope … the NASA statement that fixes the first row" — so an
  auditor reading top-down met a false account of the file before reaching the table. It also
  quoted the visible span with a second sentence the row does not ship. 14 of
  14 mutants killed, green baseline before and after — including the three shapes that SURVIVED
  their sibling suites before the guards were widened (a single-row truncation, a misindented row
  field, and a fabricated envelope). Local scratch harness, so that count is not reproducible from
  the repo.
