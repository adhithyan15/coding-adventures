- **#14986: `agriculture/farm-animals.adj` -- chicken, duck, rabbit and goat stop being warranted by the sheep sentence.**
  The envelope was the `sheep` span, so asking what a chicken gives returned `eggs` evidenced by
  *"Sheep are low maintenance and versatile – depending on the breed, they can produce wool, meat, and milk"* --
  a sentence about a different animal. The header already listed each row's correct CFSPH span in a comment
  (#13934's evidence-left-in-comments shape), ten lines above the defect those spans would have fixed.

  ### The defect, measured through the built CLI rather than asserted

  Four non-sheep binding queries, same binary, before and after:

  | needle | before | after |
  |---|---|---|
  | the sheep sentence | **x8** | **x0** |
  | chicken's own span | x0 | **x2** |
  | duck's own span | x0 | **x2** |
  | rabbit's own span | x0 | **x2** |
  | goat's own span | x0 | **x2** |
  | distinct citation sources across the four answers | **1** | **4** |
  | the new envelope reaching any answer | -- | **x0** |

  Each answer renders its citation on two surfaces (`citations`, and the steps array's `"kind":"fact"` entry),
  which is why a bound row reads x2. Controls, both re-run after the conversion: the `sheep` answer still
  carries its sentence (**x2**), and `tiger` still abstains (`"abstained":true`, zero products bound).

  ### What each row carries now

  Measured 2026-09-16 UTC against the CFSPH page at the `locator`, tags removed under **both** recipes
  (tag→empty and tag→space), HTML entities unescaped, ASCII whitespace collapsed. A text measurement, not a
  byte-provenance check -- see the ceiling note.

  - Each of the five spans occurs **exactly once** on the page under **both** recipes -- this host shows none
    of the tag→empty/tag→space split that SEER and nps.gov produced on shards 03600, 03630 and 03640.
  - Each scores **zero** against a **real 404 control** on the same host (status 404, 39,237 raw bytes, 240
    characters stripped). This host genuinely 404s, so absence here is a real absence -- not the
    content-based-only kind that `plant-parts` and `oceans` are limited to, whose hosts answer 200 on a
    nonsense path.

    *A review measured that stripped length as 234 and flagged 240 as wrong. Re-measured across three different
    nonsense paths, including the reviewer's own: all three return status 404, 39,237 raw bytes and **240**
    stripped characters. The length does not vary with the probe path, so it is a property of the host's error
    page rather than of my probe, and 240 stands. Recorded because a correct number nearly went out of the file:
    the cheapest way to tell a stale figure from a sound one is to re-derive it, not to defer to whoever raised
    it.*
  - A negative control fires: the chicken span with one word altered (`flock`→`flocks`) scores **x0**.
  - All five spans were **already attested in merged sibling tables** before this change -- chicken and duck as
    row-level `cites` in `farm-animal-secondary-product.adj`, rabbit in `-product-texture` and `-product-use`,
    goat in `-product-processing`, sheep in this file and two siblings. That cross-check is stronger than the
    page alone and no earlier conversion in this batch had it.

  ### The sheep span gains the terminal period the file was dropping

  The shipped envelope ended `...wool, meat, and milk` with no period. The page reads `...wool, meat, and milk.`
  followed by `Cons: parasites are a major concern.` -- measured by printing the character immediately after the
  shipped string, which is `.`. The two forms are **nested**, so equal occurrence counts are arithmetic rather
  than two confirmations; the next character is the discriminator.

  This file is now the first of **three** carrying that sentence to have the page's actual punctuation:
  `farm-animal-maintenance-level.adj` and `farm-animal-secondary-product.adj` still ship the period-less form as
  their own envelope. Contributed to **#15185**, not fixed here -- those are separate tables and separate
  conversions.

  ### No row restates `locator` or `trust`

  ADJ-TABLES §4: a row restates `locator` only when its page **differs** from the envelope's. All five spans are
  on the one page, and the spec records that a row locator equal to the envelope's is a **no-op** producing
  byte-identical output -- a line no test can distinguish from its own absence. Measured precedent, **counted
  after this conversion and therefore including it**: **76** converted tables ship exactly one locator with rows
  inheriting it, **16** of them five-row tables. Before this change the same predicate gave **75** and **15**.
  **#15197** is the open question on this convention; this table takes the inherited form the spec's own example
  uses.

  *Stating which side of the change a precedent count was taken from matters here: a draft of this entry cited
  the pre-change 75/15 as evidence FOR making the change, which reads as independent support while actually
  being the population this table was about to join.*

  ### The envelope, and the candidate a key-based screen would have accepted

  The new envelope is the page's framing sentence, *"Deciding to bring animals onto your small farm can be
  exciting. It can also be overwhelming. Here are some things to think about while deciding what animals you
  will raise."* -- x1 on the page under both recipes, x0 on the 404, and it names no animal and no product.

  The frame search forbade the row keys and products and their near-synonyms but **not the subject**: a page
  titled "Animals for Beginning Small Farmers" cannot frame itself without saying animals or farmers, and
  forbidding the subject is how a "no frame exists" finding gets manufactured (shard 03630 records exactly that
  on `plate-boundaries`). Controls: the **current** envelope had to be rejected (it names sheep, wool, milk) and
  subject-only prose accepted; both fired.

  **17 candidates survived the filter and the choice was made by reading, because the filter cannot make it.**
  Candidate 10 -- *"Cons: parasites are a major concern. Shearing can be an added cost."* -- passes the same
  key-based test while being the **sheep row's own fact in other words**. Measured: it names no row key or
  product (`False`), so a key screen accepts it. That is the `plate-boundaries` hole on a different page, and
  the script asserts nothing about its candidates for this reason.

  ### The existing test could not see any of this

  The suite shipped **one** test whose only citation assertion was
  `contains("cfsph.iastate.edu") && contains("\"trust\":\"authoritative\"")` -- the inverted shape of #14735,
  satisfied by the host and the tier appearing anywhere in the output, binding neither to a source. **It passed
  against the pre-conversion file and passed again, unchanged, against the converted one**, while the output it
  inspects moved by 8→0 and 0→2 on five different needles. A whole conversion was invisible to it.

  Ten tests now, each arm in **its own `#[test]`** so no failure is masked, each `recall` caller passing a
  distinct scratch tag (14 tags, 14 distinct -- verified) so no two tests can delete each other's files.

  ### Two predictions the run contradicted

  The observation harness carried a **written prediction the run could falsify**, and falsified two lines of it:

  1. **`the_sheep_sentence_warrants_only_the_sheep_row` was predicted to FAIL pre-conversion. It PASSED.**
     Pre-conversion the envelope was the sheep sentence **without** a terminal period, while the test constant
     carries the period the page has -- so `!out.contains(...)` was trivially true on the old tree. Measured:
     the with-period form occurs **0** times in the pre-conversion file and **2** in the converted one. The arm
     passed for a punctuation reason, not because the defect was absent. It is a **forward guard**, and is
     declared as one rather than counted among the discriminating arms. Dropping the period would not rescue it
     either: the period-less stem is a substring of both forms and matches on both trees.
  2. **`the_sheep_row_still_carries_the_sheep_sentence` was not in the prediction at all** -- it was added to
     the suite after the harness was written. It failed pre-conversion, which is correct and which I should have
     predicted. An unpredicted arm scoring anything is bookkeeping, not evidence.

  What **was** observed failing pre-conversion, each panicking on its own line: **six** arms — the four per-row
  citation arms, `the_table_shape_matches_the_measured_rows`, and `the_sheep_row_still_carries_the_sheep_sentence`.
  Five of those six were predicted; the sixth is the unpredicted arm described above. *An earlier draft of this
  paragraph named "those five" while the paragraph above it reported the sixth failing too — a clause that reads
  as an exhaustive list of observed failures and was not one.*

  ### The query example, reported as it runs

  **5** queries, **4** citations arrays, **1** abstention (`tiger`), **8** empty-corroboration occurrences and
  **0 non-empty**, envelope wording in **zero** answers. Span occurrences: **chicken x2, sheep x2, goat x4, duck
  x0, rabbit x0**.

  **The goat reads four and the four is correct.** It is the only row bound twice -- once forward, once by the
  reverse bind on `milk`, the only product naming exactly one animal -- and each answer renders on two surfaces.
  Duck and rabbit read zero because the companion does not query them; their spans are exercised by the suite.
  *An earlier draft of the companion header implied all five spans would appear; the run corrected it.*

  ### What this table claims, and its ceiling

  Nothing here carries a `quote … at <byte_offset> snapshot "<sha256>"` annotation -- the repo's byte-provenance
  mechanism. The spans are labelled with the page they were read from and nothing more, so **this table claims
  no more than `source_labeled`**.

  ### The retired justification

  The header asserted that ADJ tables carry ONE shared provenance envelope and that per-row provenance was *"a
  documented future extension (see ADJ-TABLES §6)"*. RS-5e shipped; the sentence was false, and it was the
  justification for the defect. Rewritten to state what the file now does, quoting the retired claim inside the
  retraction so the change is legible rather than silent.

  Measured across the 362 fact `.adj` files, **both sides of this change**:

  | phrasing | before | after |
  |---|---|---|
  | `ONE shared provenance envelope` | 2 | **3** |
  | `per-row provenance is a documented future extension` | 4 | **3** |
  | `ADJ-TABLES §6` | 8 | **7** |
  | **union** | **8** | **8** |

  The union is 8 on both sides. **7 of the 8 are unconverted**, and the eighth is **this file** -- which matches
  only because its retraction **quotes** the retired wording. *An earlier note of mine said six files and said
  the justification was stale in already-converted ones; both halves were wrong.*

  **The per-phrasing counts move without any file changing meaning, and that is worth naming.** This file
  matched phrasings **B and C** before and matches **A only** after. Nothing about what it says changed in that
  direction -- only where its lines break. Before, the header read `...carry ONE shared` / `% provenance
  envelope (...`, so A was split across a line break and could not match while B and C sat on single lines and
  did; after, A is unbroken, B is reworded inside the retraction, and the section reference is now §4. A needle
  that spans a line break matches one version and not the other. *A draft of this entry published the **before**
  sub-counts under an "after this conversion" label -- the same wrong-side-of-the-change error this shard flags
  about itself in the 75/15 note, committed forty lines later and caught by review.*

  **That makes the census predicate itself a defect from here on.** A plain text match cannot separate a file
  that ASSERTS the retired justification from one that QUOTES it while retracting it, and this file is now the
  first of the second kind. Any future pass over that population needs a use-versus-mention predicate -- a hit
  is a mention when retracting language ("used to say", "was false", "retired") sits within a window around it
  -- or it will count this conversion as though it had not happened. Verified here with both arms: **the one
  post-change match in this file** classifies as MENTION, and a synthetic asserting probe classifies as
  ASSERTION. *Before the change this file carried **two** matches and both were genuine assertions.*
