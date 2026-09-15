- **#14986: `physics/band-emitter.adj` — the infrared and visible emitters stop being warranted by the ultraviolet sentence, and the visible row drops its detector sentence.**
  The envelope was the ultraviolet row's own sentence, *"Ultraviolet radiation is emitted by the Sun
  and are the reason skin tans and burns."*, so it was the primary source of all three answers. The
  infrared and visible emitters were read across two table-level `cites`. The header said so on
  purpose, "An ADJ `table` carries ONE provenance envelope".

  ### What each row carries now

  Measured 2026-09-15 on NASA Goddard's "Imagine the Universe!" electromagnetic-spectrum page (HTTP
  200; a nonsense path on the same host returns 404), with inline tags removed without a space and only
  ASCII whitespace collapsed. The nonsense page holds none of the sentences below.
  - **Ultraviolet and infrared** each take their own sentence as a `source`. Each names the band and
    its emitter.
  - **Visible** takes only *"Fireflies, light bulbs, and stars all emit visible light."*, which names
    the band and the emitter. The old `cites` also carried "Our eyes detect visible light.", which names
    a detector, not an emitter.
  - **The envelope** is now the page's *"The electromagnetic (EM) spectrum is the range of all types
    of EM radiation."* It names no band and no emitter.
  - Each of the three sentences and the envelope occurs **exactly once**, inside a `<p>`, both with
    script and style data set aside and over the whole file.
  - Every row's page is the envelope's, so no row restates a locator line or trust (`ADJ-TABLES.md`
    §4).

  ### Header corrections

  - **The provenance note** said the spans were reproduced "byte-for-byte" from `em-spectrum.adj`'s
    header with "no new WebFetch". That header quotes "Our eyes detect visible light." and only notes
    that the page goes on to the fireflies sentence. The note now names the page and points to the
    measurement.
  - **The one-envelope wording** is replaced by the per-row explanation.
  - **The rows' trailing `% NASA: "..."` comments** are gone; each row's `source` now carries its
    sentence.

  ### Pins

  The test keeps every behaviour it shipped with: the forward recall of ultraviolet, the backward recall
  from sun (and that infrared is not recalled for it), and the abstention on radio. Its
  `contains("imagine.gsfc.nasa.gov") && contains(trust)` check (#15209's shape) becomes one answer
  whose citations array holds exactly the ultraviolet sentence. Added:
  - every emitter's answer is one answer whose citations array holds exactly its own sentence, with no
    other band's sentence and not the envelope;
  - the visible row carries the fireflies sentence, and the detector sentence is not in the table;
  - a table-shape test.

  **10 of 10 mutants killed, two controls:**
  - the visible row warranted by the detector sentence;
  - the visible row carrying both sentences again;
  - the infrared row warranted by the ultraviolet sentence;
  - the infrared `source` demoted to `cites`;
  - the ultraviolet row reverted to a bare row that inherits the envelope;
  - the old envelope restored;
  - a table-level `cites` added;
  - a row restating a different trust tier;
  - an emitter atom rebound;
  - a band named in the envelope.

  The controls are the unmutated suite, green before the first mutant and after the last, with the
  file byte-identical afterwards. It's a local scratch harness, so that count can't be reproduced
  from the repo.

  **The envelope's wording reaches no answer, and is pinned only in the file.** Every row overrides
  it, so the framing sentence occurs zero times in the query example's output.
