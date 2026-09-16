- **#14986: `earth-science/cloud-types.adj` — four clouds stop being warranted by a sentence that names a different cloud.**
  The page states one "main types" sentence per deck. The HIGH-deck sentence was the envelope and the
  other two were table-level `cites`, so all seven answers carried all three sentences — and for
  `altostratus`, `altocumulus`, `stratus` and `cumulus` the PRIMARY source named a cloud that was not
  the one asked about. The shipped header said so in prose: "the primary attribution alone does not
  support every row and a reader should not have to discover that."

  ### What each row carries now

  Measured 2026-09-15 on NOAA/NWS Louisville's "Cloud Classification and Characteristics" page
  (HTTP 200; a nonsense path on the same host returns a real 404 that holds none of these sentences),
  with inline tags removed without a space and only ASCII whitespace collapsed.
  - **Each row takes the one deck sentence that names its own cloud**: three rows the high-deck
    sentence, two the mid-level sentence, two the low-deck sentence. No row needs a corroboration,
    because each deck sentence names every cloud in its deck.
  - **The envelope** is now the page's *"Clouds are classified according to their height above and
    appearance (texture) from the ground."* It names no cloud and no level.
  - **Counts:** each of the three deck sentences occurs **exactly once**, inside one innermost `<p>`,
    with script and style data set aside and over the whole file, and with no `<head>` copy. The
    envelope likewise occurs exactly once, inside one `<p>`.
  - The page's own singular slip — "The two main **type** of mid-level clouds" — is still reproduced,
    not corrected, and a mutant that "fixes" it is killed.
  - Every row's page is the envelope's, so no row restates a locator line or trust (`ADJ-TABLES.md`
    §4).

  ### Pins

  - **Kept:** both level bindings, the newly-added-rows recall, and the `fog` abstention.
  - **Inverted:** the whole-citation needle. It pinned ONE citation object shared by all seven rows,
    two thirds of whose text named clouds other than the one asked about — it passed precisely because
    every answer carried everything. Each answer is now pinned to its own deck sentence, and the needle
    still closes on the corroborations `]`, so a fabricated `cites` cannot be appended (#14735).
  - **Corrected:** the newly-added-rows test carried a comment that had already been wrong twice — it
    said the two answers' citations were byte-identical, with `altocumulus` named only by a
    corroboration. Per-row provenance is what makes "different citations" true, so the test now
    asserts it instead of describing it.
  - **Added:** a per-cloud check (each answer's citations array holds exactly its own deck sentence,
    with negative arms against the other two decks and the envelope); a deck-distribution check
    (3 high, 2 middle, 2 low, and no table-level `cites`); a table-shape test.

  **10 of 10 mutants killed, two controls:**
  - the cirrus row reverted to a bare row that inherits the envelope;
  - altostratus reverted to the high-deck sentence (the exact defect being fixed);
  - stratus reverted to the mid-level sentence;
  - the old envelope restored;
  - the mid-level deck re-added as a table-level `cites`;
  - a row restating a different trust tier;
  - cumulus rebound to the middle deck;
  - the envelope naming a cloud;
  - the page's singular slip "corrected" to "types";
  - the low-deck span truncated after its stratus clause.

  The controls are the unmutated suite, green before the first mutant and after the last, with the
  file byte-identical afterwards. It's a local scratch harness, so that count can't be reproduced
  from the repo.

  **The envelope's wording reaches no answer, and is pinned only in the file.** The shipped query
  example returns 4 answers and 1 abstention, and the framing sentence occurs zero times in its
  output.

  ### Three stale descriptions of the old shape

  - The stdlib README's row said the 7 rows were extended "using this table's OWN already-quoted source
    sentence, which had already named all seven clouds" — no single sentence on the page names all
    seven. It now describes the per-row shape.
  - `cloud-types.query.adj` said the engine returns the level plus "the table's" source/locator/trust.
    Each answer now returns the row's own deck sentence, with the table's locator and trust inherited.
  - The header's retained "1443 and 1446 characters apart" figures did not reproduce under the
    normalization this entry states. Collapsing **only ASCII whitespace** those gaps are **1704 and
    1483**; 1443/1446 reproduce only with U+00A0 folded to a space. Both are now stated with the rule
    each was measured under. Nothing about the sentences changed — pinning down the rule is what made
    the older pair read as wrong.

  The last two were raised by the pre-push security review, which re-fetched the page and reproduced
  every count above; the gap figures were then re-measured here rather than taken from the review.
