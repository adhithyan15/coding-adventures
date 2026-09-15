- `anatomy/body-counts.adj` — all 9 rows converted to per-row provenance (RS-5e, #14986),
  and **this table's defect is sharper than its siblings': the envelope imposed a TRUST TIER
  the file's own header said was wrong.** Two defects in one paragraph:

  1. **The span.** The CHROMOSOME sentence was this table's `source`, the field that carries
     the tier, for every row, so `? body_count(pairs_of_ribs, $N)` came back proved by
     *"Humans have 22 pairs of numbered chromosomes … for a total of 46."*
  2. **The tier — declared, then contradicted.** The header says plainly that the three
     hand-bone rows *"are `consensus`-tier, one rung below the table's `authoritative`
     envelope trust"*, and the table shipped `authoritative` on them, because one envelope
     imposes one tier. **The e2e test that queried exactly those three rows asserted only
     their NUMBERS**, so nothing caught it. They now carry `trust consensus` in the row, which
     is what a query returns.

  Two facts about row/envelope provenance were **established by running them**, not by reading
  the lowering code: a row's `trust` does override the envelope's, per row; and an envelope
  `source` is REQUIRED — an envelope carrying only `locator` and `trust` is rejected with
  `Lower(TableMissingProvenance)`, so the framing-span pattern this cascade uses is not a
  stylistic choice.

  All nine spans verified against their pages before writing, read out of the shipped header
  rather than retyped, each with a negative arm (final word altered) absent from every page.
  9/9 verbatim. **One span was widened**: the lungs row quoted `the pair of spongy,
  pinkish-gray organs in your chest` — opening mid-sentence, so the quoted run never said
  whose, or that it was about lungs at all. The page's full sentence is contiguous and is now
  quoted whole (same class as the truncation triage in #15185, where this table's lungs span
  is one of the 81 candidates the probe listed).

  The envelope becomes a FRAMING span — the SEER module's opening description of the body as
  billions of smaller structures — which states no count of anything this table counts.

  `facts_anatomy_e2e.rs`: 6 tests. The citation assertion that let this conversion pass
  unchanged is recorded as the defect in test form: `out.contains("genome.gov")` was the
  envelope locator on all nine rows, so it held for any answer the table produced. All nine
  rows are now pinned individually **with their tier**, the three hand-bone rows get a
  dedicated test asserting they never return `authoritative`, and the pairing is asserted
  rather than assumed — each row's count must appear in the span that is supposed to state it,
  in numeral or word form (with one stated limit: the hand sentence carries 27, 8, 5 and 14, so
  for those three rows this check is satisfied by any of the four, and what separates them is
  the binding assertion under the one-citation gate). **That assertion found a real gap on its first run**: the
  heart-chambers span writes "four", not "4". 18 of 18 mutants killed, baseline green.
