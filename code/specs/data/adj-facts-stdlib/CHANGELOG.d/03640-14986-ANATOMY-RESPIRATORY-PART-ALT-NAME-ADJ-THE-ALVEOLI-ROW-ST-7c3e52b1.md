- **#14986: `anatomy/respiratory-part-alt-name.adj` -- the alveoli row stops being warranted by the trachea sentence, and stops being addressed to the wrong page.**
  The envelope was the `trachea` span, so asking which part is called the air sacs returned `alveoli` evidenced by
  *"The trachea, commonly called the windpipe, is the main airway to the lungs."* -- a sentence about a different
  part. **This table had a second defect the others in this batch did not:** its single `locator` was
  `passages/larynx.html`, while the alveoli sentence is on `passages/bronchi.html`. That answer was warranted by
  the wrong sentence AND served under a page its own sentence is not on.

  ### What each row carries now

  Measured 2026-09-16 UTC against the two NCI SEER pages at the row `locator`s, tags removed **without inserting a
  space**, HTML entities unescaped, only ASCII whitespace collapsed. A text measurement, not a byte-provenance
  check -- see the ceiling note.

  - Each span occurs **exactly once** on its own page, **zero** times on the other, and **zero** times against a
    **real 404 control** on the same host (19,393 bytes against 41-46 KB real pages). This host genuinely 404s,
    so absence here is a real absence -- not the content-based-only kind that `oceans` and `plant-parts` are
    limited to, whose hosts answer 200 on a nonsense path.
  - All three shipped spans are **pure ASCII**.

  **The recipe decides two of the three, so it is part of the claim.** Two recipes that both collapse whitespace
  and differ in one thing only -- what an inline tag becomes: send it to the **empty string** and all three spans
  score x1; send it to a **space** and the trachea and alveoli spans score **x0**. The envelope scores x1 under
  both. Those zeros are the *instrument*, not the pages. Shards 03600 and 03630 found the same split — SEER for
  03600, `nps.gov` for 03630 — so it is **two hosts across three tables**, and the naive recipe is evidently not
  naive-looking enough to stay caught once.

  *A draft of this entry said "two other hosts; that is three hosts now". Both halves were wrong. 03600 is
  `muscle-nuclei-count.adj` on **this file's own host**, so it is not an "other" host; and the distinct-host count
  is two, not three. The error came from re-reading 03630's careful sentence — "shard 03600 found the same split
  on a different host (SEER)", which means different **from NPS**, said from 03630's vantage point — as though it
  had been said from this file's. Measured from each table's own shipped `locator` lines: 03600
  `training.seer.cancer.gov`, 03630 `www.nps.gov`, 03640 `training.seer.cancer.gov`.*

  ### Two pages for two rows, so both rows restate their locator

  ADJ-TABLES §4: restate a row locator only when its page **differs** from the envelope's. The envelope is now the
  module index and neither row is on it, so both restate. The parent `respiratory-parts.adj` is the precedent and
  the contrast: its `lungs` row **inherits** precisely because its sentence *is* on the index. No row here is in
  that position.

  ### The query example, reported as it runs

  3 queries, 2 answers, 1 abstention (`larynx`); **2** citations arrays, **4** empty-corroboration occurrences and
  **0 non-empty**, envelope wording in **zero** answers.

  |  | before | after |
  |---|---|---|
  | trachea span | x4 | **x2** |
  | alveoli span | **x0** | **x2** |
  | `larynx.html` | x4 | **x2** |
  | `bronchi.html` | **x0** | **x2** |
  | alveoli answer bound to the trachea span | **x1** | **x0** |

  The corroborations arrays were empty before and remain so. **Nothing here is a corroboration claim.**

  ### Pins

  - **Inverted:** `contains("training.seer.cancer.gov") && contains("\"trust\":\"authoritative\"")` -- satisfied by
    the host and the tier appearing anywhere in the output, in two unrelated places, binding neither to a source.
    Before this change it was satisfied just as well when the alveoli answer carried the trachea sentence under
    larynx.html. The parent's own PR records this identical pin letting a whole conversion pass with zero test
    changes. Each answer is now pinned to its **whole serialised citation object**, closing on `"corroborations":[]`.
  - **The backward test had no citation arm at all**, which is why this defect sat here undetected: it pinned only
    the `term`, so the alveoli row's provenance was entirely unpinned.
  - **Added, each observed failing** against the pre-conversion bytes, each in its own `#[test]` and each panicking
    on its own line: the alveoli answer carries the **bronchi** sentence under bronchi.html; and the **trachea
    sentence does not warrant the alveoli answer**.
  - **Added as a forward guard:** the envelope reaches no answer. This one **passes on both trees** and is declared
    as such rather than counted among the discriminating arms.

  ### Three test-design defects I had to be shown, not reasoned, out of

  The conversion was mechanical. The tests were not, and all three defects survived a careful reading:

  1. **I predicted the envelope arm would fail pre-conversion. It passed.** The prediction confused the envelope's
     *role* with the `ENVELOPE` *string*: the old envelope was the trachea sentence, and the respiration framing
     sentence occurred nowhere in that file, so `!out.contains(ENVELOPE)` was trivially true. Caught because the
     observation script carried a **written prediction the run could contradict** -- had it merely said "verify the
     arms fail", a 1-of-3 result would have read as success.
  2. **The negative arm was masked.** It lived inside the positive arm's test, so `assert!` panicked on the first
     failure and it never executed. I then reported the test as "failing on both arms" without checking which line
     panicked. Unexecuted is neither passing nor failing -- it is unproven. It now has its own `#[test]`.
  3. **The envelope screen was half-blind, and a mutation harness proved it.** Under RS-5e every row overrides
     `source` and `locator`, so the envelope can never *reach* an answer -- the assertion can only fire on a text
     collision with whichever row is queried. Querying only the trachea row, an envelope swapped to the **alveoli**
     sentence was **accepted**: a row-warranting envelope drifted in with every assertion green, which is exactly
     what that arm's own comment claimed could not happen. It now queries **both** rows. Re-run: both
     row-warranting mutants rejected **on the screen's own message**, and the negative control -- real page prose
     naming no row -- still accepted, so the screen is not merely rejecting everything.

  Mutating the envelope means changing **both** the `.adj` and the test's `ENVELOPE` constant together; changing
  the `.adj` alone makes the test fail for an unrelated reason, as shard 03630 records.

  ### What this table claims, and its ceiling

  Nothing carries a `quote ... at <byte_offset> snapshot "<sha256>"` annotation -- the repo's byte-provenance
  mechanism. The spans are labelled with the page they were read from and nothing more, so **this table claims no
  more than `source_labeled`**.

  `gaps.missing_byte_pin` and `gaps.missing_pin_syntax` list this file at **584 and 584 on the pre-conversion and
  post-conversion trees alike** -- measured on both, not quoted from a briefing. Wave 1 state, pre-existing,
  **unchanged by this PR**, neither fixed nor worsened.

  ### The header's retired claim

  It said the spans reproduced the parent "byte-for-byte ... **no new WebFetch**". Retired: they are now measured
  against the pages themselves, which is a stronger statement than the one it was avoiding making. The retired
  wording is quoted inside the paragraph that withdraws it. The header also said each row lowers to a relation
  *"carrying the citation"* -- the #15336 spelling, now gone from this file.
