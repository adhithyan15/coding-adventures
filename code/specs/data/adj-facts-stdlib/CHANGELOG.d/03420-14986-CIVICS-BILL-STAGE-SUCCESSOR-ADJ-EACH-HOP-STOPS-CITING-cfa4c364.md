- **#14986: `civics/bill-stage-successor.adj` — each hop stops citing the whole legislative process.**
  The envelope was the first transition sentence, and the other six were table-level `cites`, so every
  answer carried all seven sentences. The answer to "what follows introduction?" cited "The president
  then considers the bill." A shipped test asserted exactly that.

  ### What each row carries now

  Measured 2026-09-15 on USA.gov's "How laws are made" page (HTTP 200; a nonsense path on the same host
  returns 404), with inline tags removed without a space and only ASCII whitespace collapsed. The
  nonsense page holds none of the sentences below.
  - **Three hops take their own sentence**, because it names both stages: `introduced`,
    `first_chamber_vote` and `second_chamber_process`.
  - **Two sentences point back to the one before them.** "Then both chambers vote ..." and "If it
    passes, they present it ..." each depend on the preceding sentence in the same list item. So
    `reconcile_differences` and `vote_on_same_version` each take their sentence together with the one
    before it, as one contiguous span.
  - **Two more point back across a list item.** "The bill is then put before that chamber ..." and "The
    president then considers the bill." point to the sentence that ends the previous list item. So
    `committee_review` and `presented_to_president` each take their sentence as `source` and `cites`
    that earlier sentence as a corroboration. The trust tier is unchanged.
  - **The envelope** is now the page's *"Congress is the lawmaking branch of the federal government."*
    It names no stage.
  - **Counts:** each of the seven transition sentences occurs **exactly once**, inside an `<li>`, both
    with script and style data set aside and over the whole file. Each two-sentence span is contiguous in
    one `<li>`. The envelope occurs exactly once, inside a `<p>`, with no `<head>` copy.
  - Every row's page is the envelope's, so no row restates a locator line or trust (`ADJ-TABLES.md`
    §4).

  ### Pins

  - **Kept:**
    - the whole-chain walk;
    - the reverse recall to `vote_on_same_version`;
    - both abstentions: the branch point and the idea origin.
  - **Inverted:** the test that asserted the *introduced* answer carries all seven sentences. It now
    asserts that answer holds exactly its own sentence and no other hop's. It is renamed to say so.
  - **Removed:** its host-plus-trust needle, which a row citing any other page would still satisfy.
  - **Added:**
    - every hop's answer is one answer whose citations array holds exactly its own span, plus its
      corroboration where it has one, and no sentence from another hop or the envelope;
    - the backward-pointing hops carry the sentence they point back to, and no row carries "Then both
      chambers vote" or "If it passes" alone;
    - a table-shape test.

  ### Two stale descriptions of the old shape

  Both were raised by the pre-push security review, which re-fetched the page and independently
  confirmed every quoted span and every count above.
  - `README.md`'s row for this library still ended "All seven transition sentences carried as one
    `source` plus six `cites`" — the shape this entry removes. It now describes the per-row shape.
  - This file's own header table said each row decodes **exactly one** transition sentence, and showed
    `reconcile_differences` and `vote_on_same_version` quoting one sentence each. Those two rows carry
    two. The table now shows both spans in full, and a note names the two rows that add a corroboration.

  **10 of 10 mutants killed, two controls:**
  - the committee hop losing its corroboration;
  - the presented hop losing its corroboration;
  - the reconcile hop cut to "Then both chambers vote" alone;
  - the same-version hop cut to "If it passes" alone;
  - the introduced row reverted to a bare row that inherits the envelope;
  - the old envelope restored;
  - a table-level `cites` re-added;
  - a row restating a different trust tier;
  - a next-stage atom rebound;
  - a stage named in the envelope.

  The controls are the unmutated suite, green before the first mutant and after the last, with the
  file byte-identical afterwards. It's a local scratch harness, so that count can't be reproduced
  from the repo.

  **The envelope's wording reaches no answer, and is pinned only in the file.** Every row overrides
  it, so the framing sentence occurs zero times in the query example's output.
