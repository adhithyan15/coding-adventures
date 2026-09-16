- **#14986: `earth-science/metamorphism-cause.adj` — the sentence saying what metamorphism DOES stops being header-only prose.**
  The envelope was the cause-naming sentence, and the effect sentence — the one stating that the rock
  becomes denser and more compact — was quoted in the header and cited **nowhere**, so no answer
  carried it at all. Each row now takes the cause sentence as its own `source` and `cites` the effect
  sentence; the envelope is a framing sentence naming neither.

  ### What each row carries now

  Measured 2026-09-15 on USGS's "What are metamorphic rocks?" FAQ (HTTP 200; a nonsense path on the
  same host returns 404 and holds none of these spans), with inline tags removed without a space and
  only ASCII whitespace collapsed.
  - **All three rows share the cause sentence**, which names all three causes, and each `cites` the
    effect sentence. They are a source and a corroboration rather than one span because they are **not
    contiguous**: the cause sentence sits in the opening paragraph, the effect sentence under the
    run-in heading "Process of Metamorphism:", and the joined string occurs 0 times.
  - **The envelope** is the page's *"Metamorphic rocks started out as some other type of rock, but have
    been substantially changed from their original igneous, sedimentary, or earlier metamorphic
    form."* It names no cause and no effect.
  - **The envelope is quoted in the page's own characters.** The body copy writes **U+00A0** before
    "igneous," and before "sedimentary,". Typed with plain spaces the sentence occurs **zero** times in
    the page's text — it survives only in the `<head>` metadata copies, which do use plain spaces. The
    converter extracts the span from the fetched page rather than retyping it, and a mutant that
    "cleans up" the two non-breaking spaces is killed.
  - **Counts:** the envelope, the cause sentence and the effect sentence each occur **exactly once**,
    each inside one innermost `<p>`, with script and style data set aside and over the whole file.
  - Every row's page is the envelope's, so no row restates a locator line or trust (`ADJ-TABLES.md`
    §4).

  ### Pins

  - **Kept:** the direct recall, the reverse enumeration of all three causes, and the `sunlight`
    abstention.
  - **Inverted:** `contains("usgs.gov") && contains("\"trust\":\"authoritative\"")`. Any USGS page's
    citation satisfies that, and so does a truncated `source`, because it constrains no sentence text
    at all. The needle is now the whole citations array as the serialiser emits it, closing on both the
    corroborations `]` and the citations `]`, so a fabricated `cites` cannot be appended (#14735).
  - **Added:** a per-cause check (each answer's citations array holds the cause sentence corroborated
    by the effect sentence, and never the envelope); a table-shape test that also pins the envelope's
    two non-breaking spaces.

  **10 of 10 mutants killed, two controls:**
  - the heat row losing its corroboration;
  - the pressure row losing its corroboration;
  - the fluids row reverted to a bare row that inherits the envelope;
  - the old envelope restored;
  - the envelope's U+00A0 characters replaced with plain spaces;
  - the effect re-added as a table-level `cites`;
  - a row restating a different trust tier;
  - a row's effect atom rebound;
  - the envelope naming a cause;
  - the cause span truncated before its fluids clause.

  The controls are the unmutated suite, green before the first mutant and after the last, with the
  file byte-identical afterwards. It's a local scratch harness, so that count can't be reproduced
  from the repo.

  **The envelope's wording reaches no answer, and is pinned only in the file.** The shipped query
  example returns 4 answers and 1 abstention; every answer carries a corroboration, and the framing
  sentence occurs zero times in the output.

  The query example said the engine returns the effect plus "the table's own source/locator/trust".
  It now describes the row's own sentence and its corroboration, with the locator and trust inherited.
