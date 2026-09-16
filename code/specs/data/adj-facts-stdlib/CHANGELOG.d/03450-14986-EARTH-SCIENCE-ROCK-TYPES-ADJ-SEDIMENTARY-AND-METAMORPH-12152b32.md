- **#14986: `earth-science/rock-types.adj` — sedimentary and metamorphic rock stop being warranted by the igneous sentence.**
  The envelope was the IGNEOUS sentence, so every answer's primary source was a sentence about
  once-molten rock: ask how sedimentary rock forms and the proof offered was "Igneous Rocks: form when
  hot, liquid rock, or magma, cools." The other two sentences were quoted in the header and cited
  nowhere, so no answer carried them.

  ### What each row carries now

  Measured 2026-09-15 on the NPS teacher page "Olympic Geology pt 2: Rock Sorting" (HTTP 200; a
  nonsense path on the same host returns 404 and holds none of these spans), with inline tags removed
  without a space and only ASCII whitespace collapsed.
  - **Each row takes the sentence that names its own type.** Each of the three spans occurs **exactly
    once**, inside one innermost `<p>`, scripts set aside and over the whole file, with no `<head>`
    copy.
  - **The metamorphic row takes TWO sentences.** "This normally happens deep underground where heat,
    pressure, and chemical activity can actually alter the minerals inside rocks." names no rock type
    on its own. The sentence before it does — "Metamorphic Rocks: are created through the
    metamorphosis, or change, of other types of rocks." — and the two are contiguous in one paragraph,
    so the row carries them as one span rather than as a corroboration.
  - **The envelope** is now the page's *"Geologists: scientists who study rocks and recognize three
    major groups of rocks."* It names no rock type and no formation. It sits in the same paragraph as
    the metamorphic pair.
  - The envelope was **extracted from the fetched page**, not retyped. That is now the rule for every
    conversion: the sibling `metamorphism-cause` envelope turned out to carry U+00A0 where a typed
    copy had plain spaces, which would have shipped a `source` occurring zero times on its own page.
    This page's envelope has no such divergence — checked byte by byte rather than assumed.
  - Every row's page is the envelope's, so no row restates a locator line or trust (`ADJ-TABLES.md`
    §4). No row needs a corroboration.

  ### Pins

  - **Kept:** both formation bindings and the `magma` abstention.
  - **Inverted:** `contains("nps.gov") && contains("\"trust\":\"authoritative\"")`. Any NPS page's
    citation satisfies that, and so does a truncated sentence, because it constrains no source text.
    Each answer is now pinned to its own whole citations array, closing on both the corroborations `]`
    and the citations `]` (#14735).
  - **Added:** a per-type check (each answer holds exactly its own span, with negative arms against
    the other two types and the envelope, and an assertion that the span carried by each type actually
    names that type — the property the defect violated); a check that the metamorphic row carries the
    sentence its span points back to and that no row is warranted by the dangling sentence alone; a
    table-shape test.

  **10 of 10 mutants killed, two controls:**
  - the igneous row reverted to a bare row that inherits the envelope;
  - the sedimentary row reverted to the igneous sentence (the exact defect being fixed);
  - the metamorphic row cut to the dangling sentence alone;
  - the metamorphic row cut to its naming sentence alone, dropping the heat-and-pressure clause its
    value comes from;
  - the old envelope restored;
  - a table-level `cites` re-added;
  - a row restating a different trust tier;
  - the metamorphic formation atom rebound;
  - the envelope naming a rock type;
  - the igneous span truncated mid-sentence.

  The controls are the unmutated suite, green before the first mutant and after the last, with the
  file byte-identical afterwards. It's a local scratch harness, so that count can't be reproduced
  from the repo.

  **The envelope's wording reaches no answer, and is pinned only in the file.** The shipped query
  example returns 3 answers and 1 abstention, and the framing sentence occurs zero times in its
  output.

  The query example said the engine returns the formation plus "the NPS source/locator/trust". It now
  describes the row's own sentence, with the locator and trust inherited — the third conversion
  running to carry that stale line, so it is now part of the per-conversion checklist rather than
  something a reviewer catches.

  ### Three things the pre-push security review caught

  - **The stdlib README's own rule contradicted this change.** §"When one sentence does not cover
    every row" reads *"If a row's own supporting sentence is not the `source`, it must be a
    `cites`"* — and this file's new shape-test asserts the table has no `cites` at all. The section
    now names RS-5e per-row `source` as the preferred remedy when the row's sentence lives on the
    envelope's page, reserving `cites` for evidence on a different page. The rule's force is
    unchanged: header-only evidence is still not provenance; RS-5e changes where it goes.
  - **A retained truth-table row attributed two words to NPS that the page never writes.** The third
    column was headed "how it is made (NPS)" and its igneous cell read "molten magma cools and
    hardens"; the page contains "molten" and "harden" zero times. The column is now marked as a
    gloss, and the cell uses the page's own words.
  - **Four added comment lines ended in trailing whitespace**, which would have made this the only
    `.adj` in the stdlib with that pattern and broken `git diff --check`. Fixed, and the converter
    that produced them now fails its own check on a trailing space, so it cannot recur.
