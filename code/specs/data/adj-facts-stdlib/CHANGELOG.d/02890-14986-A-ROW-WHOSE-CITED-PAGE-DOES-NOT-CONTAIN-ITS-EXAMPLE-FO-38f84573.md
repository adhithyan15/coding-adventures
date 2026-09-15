- **#14986: a row whose cited page does not contain its example, found while converting
  `biology/kingdoms.adj` to per-row provenance.** The conversion is the smaller half of this entry.

  ### The row

  `plantae → multicellular_algae` is **removed**. Re-measured 2026-09-13 UTC with a raw fetch, the
  cited Science Notes page's plantae line reads *"Examples: Flowers, grasses, conifers, ferns,
  mosses"* — it does not contain "multicellular algae", and that phrase occurs **zero times**
  anywhere on the page: zero in the rendered text *and* zero in the raw HTML, which no choice of
  extractor can move. ("algae" occurs twice, both Protista, where the page actively places algae.)
  A row the cited page does not state is precisely what this library exists not to ship, so the
  recall now abstains instead of asserting it.

  **Which of three causes is not determined here, and that is deliberate.** The previous header
  recorded the plantae span *with* "multicellular algae" and described the check as follows — quoted
  as a block so its own inner double quotes survive intact rather than being requoted:

  > WebFetch-verified (re-fetched the live page and confirmed every "Examples:" line word-for-word
  > against the header's original transcription)

  That trailing qualifier is load-bearing, and an earlier draft of this very entry cut it — quoting
  the sentence as if it ended at "word-for-word". In an entry whose subject is verbatim-span
  discipline, that reproduced the defect it exists to correct, and it is restored here in full. The
  qualifier says the old check compared the page against **this file's own prior transcription**,
  which admits a third cause beside the obvious two: the page changed after the check; the
  summarizing fetcher misreported the line; or the line was never independently read, and the check
  confirmed the header against itself. The evidence to hand cannot separate them, so none is
  asserted. What is settled: a raw fetch today does not contain the phrase, and a summarizing
  fetcher is not an instrument for a verbatim span.

  ### The census that found it

  2026-09-13 UTC, over the one cited page. Rows read out of the shipped `.adj`, not retyped.

  **Two counts, because they answer different questions and only one of them justifies deleting a
  row.** An earlier draft of this entry reported a single "22 of 23", which hid the distinction:

  Counted over the **23 rows as shipped before this change**; the right-hand column is where the
  22 rows shipped after it stand, so neither number is falsified by the commit carrying it:

  | test | before (23 rows) | after (22 rows) |
  | --- | --- | --- |
  | the atom is a **contiguous substring** of its kingdom's line | **21** | 21 |
  | every **token** of the atom appears in that line | **22** | 22 |

  The two rows that differ are not the same case, and reporting one number concealed that:

  - **`bacteria → gram_positive_bacteria` — KEPT.** Every token is present; the page writes
    *"Gram-positive and Gram-negative bacteria"*, distributing one head noun across two examples.
    The row rests on reading that conjunction as distributive. That is a *reading*, not a
    quotation, and the header now says so instead of implying it with an ellipsis.
  - **`plantae → multicellular_algae` — REMOVED.** Not a substring, not all tokens, and the phrase
    occurs zero times on the page under any reading.

  Deleting one while keeping the other is defensible only if the standard is stated. It now is.

  **Two instrument failures had to be fixed before that number meant anything.**

  1. The first harness scored **zero on its own positive control** — it could not find the span that
     was, at the time, this table's shipped `source`. Cause: the page writes
     `<strong>Examples</strong>:`, and an extractor that replaces *every* tag with a space yields
     `"Examples : "`. Inline tags must vanish with no separator; block tags become a break.
  2. With that fixed, four rows read as MISSES. **Two were the instrument**: the page writes
     "single-celled algae" and "Gram-negative", and normalising only underscores to spaces leaves
     the hyphens unmatched. Of the remaining two, one is the distributive-reading case above and
     one is the real defect. (An earlier draft of this entry said hyphens cleared *three* — it
     clears two, and `gram_positive_bacteria` is not a hyphen artifact at all.)

  Controls: a fabricated atom fails for every kingdom; a known-good atom passes.

  ### Also measured, and deliberately not acted on

  The fungi line reads *"Mushrooms, yeast, molds, rusts"* today — the three shipped fungi rows are
  each stated by it, and `rusts` is a fourth example this table does not carry. The page also now
  describes a sixth kingdom, Archaea. Both are under-coverage, not defects; neither is expanded here.

  ### The conversion

  The remaining 22 rows each carry their own kingdom's line as an RS-5e `source`. No row restates
  `locator` or `trust` — one page, one tier, inherited. The envelope becomes a genuine **framing**
  span, *"The traditional five kingdoms are Animalia, Plantae, Fungi, Protista, and Monera."*, plus
  a `cites` for *"The six-kingdom system separates Monera into Bacteria and Archaea."* — which is
  what makes `bacteria` a kingdom key at all. Previously the envelope held the **animalia** line, so
  asking for a fungus returned a citation about humans and sponges.

  All seven shipped spans occur exactly once in the page's rendered text; a nonsense span and a
  near-miss variant each score zero.

  ### Pins

  **8/8 mutants behaved, and two of the eight exist because earlier versions survived.**

  - Pinning the span plus a loose `contains("sciencenotes.org")` stayed **green** when the envelope's
    `locator` was deleted — the envelope's own `cites` carries that locator, so the check was
    satisfied by the corroboration, never by the inheritance it claimed to test. Now the pin is the
    whole `source`/`locator`/`trust` object.
  - Dropping **one** row's block stayed **green**, because a kingdom query returns one answer per row
    and the siblings satisfied a `contains` check — cross-row masking again, this time inside a
    single kingdom. Now the object's occurrence count is asserted exactly.

  The remaining six: swapping a kingdom's span for the animalia line, dropping a single row's block,
  truncating the fungi span, swapping plantae's span, **re-adding the removed row**, and dropping
  the shared locator all redden. Restore returns to green.

  **Two further gaps the review found, both now closed.**

  - **14 of the 22 rows had no per-row pin at all.** Only `fungi` and `plantae` were covered, so
    stripping every `animalia` or `bacteria` `{ source }` block — reverting those rows to the
    envelope, *the exact defect this entry is about* — left the whole suite green. All five
    kingdoms are pinned now.

    That closing the gap also **retired a control**: "mutate a kingdom neither test queries, expect
    green" no longer has a subject, because no kingdom is unqueried. What replaced it is stronger —
    **cross-kingdom isolation**: stripping or fabricating any one kingdom's spans reddens exactly
    that kingdom's test and no other, verified for all five.
  - **The envelope's `cites` was unpinned.** Deleting the span that makes `bacteria` a kingdom key
    at all left the suite green, because the object pin stops at `"trust"`, before
    `corroborations`. Now asserted.

  One correction to this entry's own accounting: re-adding the removed row reddens the **exact row
  count**, not the `!contains("multicellular")` absence pin, which the count assertion dominates.
  The absence pin is redundant rather than vacuous, and is kept as a statement of intent.

