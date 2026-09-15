- **#14070 installment 2: `chemistry/mixture-types` — a joined five-clause `source` becomes one
  `source` plus four `cites`.** Interior-ellipsis count over shipped values falls 5 → 4.

  **The old value was a documented workaround, not carelessness.** The header said so outright:
  *"An ADJ `table` carries ONE provenance envelope, so the `source` below holds the five verbatim
  example sentences (joined with ' … ')."* That was accurate when written. `cites` now carries
  per-row corroborations on the same envelope, so the join is unnecessary — and a joined value is a
  span no page displays, which is what made it a #14070 defect. See #14099: **82 library headers
  still carry that obsolete rationale**, and it produces both this issue's constructed spans and
  #13934's evidence-left-in-comments.

  Every row now has **its own** verbatim span naming both its type and its example — strictly better
  than the joined value, which covered all five in one string. My first attempt at this library
  (withdrawn in installment 1) replaced the join with a single block grounding **none** of the rows;
  a per-row coverage check now guards that, and aggregate coverage would not have caught it.

  ON `suspension`. The page's **first** block mentioning salad dressing calls it a *"liquid
  mixture"* and never says "suspension"; the sentence that does is four blocks later. Encoding the
  first match would have grounded `suspension → salad_dressing` on a sentence naming only its
  category — the error that removed `tooth-types` from an earlier batch. **Reading every occurrence
  rather than the first match is what separated them**, and a mutation now swaps the correct
  sentence for that decoy: it is real verbatim page text, and the pin must still reject it.

  THE PRE-EXISTING PIN IS REPOINTED, NOT DELETED. It asserted the whole joined value, anchored on the
  JSON key and closed on the terminating quote — correct methodology, and it caught the withdrawn
  repair. What it was faithfully defending was a constructed span. The test comment now records that,
  because "an anchored pin defends whatever it is pointed at" is the part worth keeping.

  The header is corrected in two places: the obsolete joined-source rationale, and the truth-table cue
  column, whose quotes elided mid-sentence (*"The salt water ... is homogeneous"*) — the same
  constructed-span shape as the data defect, sitting in a comment. Leaving it would repeat the
  header/data desync caught in installment 1.

  Four mutations pass: restoring the join reddens the envelope; truncating `suspension`'s cite reddens
  its own pin (it sits at corroboration index 2, so a prefix pin would have missed it); a pure reorder
  of two earlier cites reddens the whole-list pin; and the decoy swap reddens too.

  *Harness note:* the first mutation run reported M2 and M4 GREEN. A plain `text.replace(…, 1)` hit
  the sentence's **first** occurrence — in the header's provenance block — so the mutation edited a
  comment and never touched the data. That is the identical failure the batch-4 harness had, which is
  why a line-targeted helper exists; I reached for a plain replace anyway. The lesson only holds if
  the tool encoding it actually gets used.

  533 test binaries / 1618 tests green, clippy `-D warnings` clean; the `.query.adj` parses, runs and
  abstains correctly.

