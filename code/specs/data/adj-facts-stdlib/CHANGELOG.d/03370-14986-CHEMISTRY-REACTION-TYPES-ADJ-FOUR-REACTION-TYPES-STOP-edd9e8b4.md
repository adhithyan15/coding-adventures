- **#14986: `chemistry/reaction-types.adj` — four reaction types stop being warranted by the combination sentence, and the single-replacement sentence gets the page's no-break space.**
  The envelope was the combination row's own sentence, *"A combination reaction is a reaction in which
  two or more substances combine to form a single new substance."*, so it was the primary source of all
  five answers. The header said so on purpose, "An ADJ `table` carries ONE provenance envelope", and
  only listed the other rows' spans.

  ### What each row carries now

  Measured 2026-09-15 on LibreTexts' "Classifying Chemical Reactions" page (HTTP 200; a nonsense path on
  the same host returns 404), with inline tags removed without a space and only ASCII whitespace
  collapsed. The nonsense page holds none of the sentences below.
  - **Every row takes the sentence its header already quoted** as a `source`.
  - **The single-replacement sentence** carries the page's U+00A0 between "fourth" and "type". The
    header quoted it with a plain space, and in that form it occurs zero times. The header quote stays,
    since a comment would not show the character, and a note records it.
  - **The envelope** is now the page's *"The key to success is to find useful ways to categorize
    reactions."* It names no reaction type.
  - Each of the five sentences and the envelope occurs **exactly once**, inside a `<p>`, both with
    script and style data set aside and over the whole file.
  - Every row's page is the envelope's, so no row restates a locator line or trust (`ADJ-TABLES.md`
    §4). The trust tier stays `consensus`.

  ### Header corrections

  - **A paraphrase in quotation marks.** The opening summary ("A combination reaction combines two or
    more substances into one; ...") sat in quotation marks. The page never writes it, and it occurs zero
    times. The marks are removed and a note says it's a paraphrase.
  - **The table's citation.** "Carrying the table's citation" becomes "carrying its row's citation".
  - **The one-envelope paragraph** is replaced by the measurement.

  ### Pins

  The test keeps every behaviour it shipped with: the forward recalls of combination, combustion and
  double replacement, the reverse recall to decomposition, and the abstention on neutralization. Its
  `contains("chem.libretexts.org") && contains(trust)` check (#15209's shape) becomes four answers, each
  whose citations array holds exactly its own row's sentence. Added:
  - every defining token's answer is one answer whose citations array holds exactly its own sentence,
    with no other type's sentence and not the envelope;
  - the single-replacement row carries one U+00A0, and no plain-space form is in the table;
  - a table-shape test.

  **9 of 9 mutants killed, two controls:**
  - the single-replacement sentence losing its no-break space;
  - the decomposition row warranted by the combination sentence;
  - the double-replacement `source` demoted to `cites`;
  - the combustion row reverted to a bare row that inherits the envelope;
  - the old envelope restored;
  - a table-level `cites` added;
  - a row restating a different trust tier;
  - a defining atom rebound;
  - a type named in the envelope.

  The controls are the unmutated suite, green before the first mutant and after the last, with the
  file byte-identical afterwards. It's a local scratch harness, so that count can't be reproduced
  from the repo.

  **The envelope's wording reaches no answer, and is pinned only in the file.** Every row overrides
  it, so the framing sentence occurs zero times in the query example's output.
