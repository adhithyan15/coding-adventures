- **#14986: `anatomy/heart-chamber-vessel.adj` — the right-ventricle, left-atrium and left-ventricle vessels stop being warranted by the right-atrium sentence.**
  The envelope was the right-atrium row's own sentence, *"The right atrium receives deoxygenated blood
  from the entire body except for the lungs (the systemic circulation) via the superior and inferior
  vena cavae."*, so it was the primary source of all six answers. The other chambers' vessels were
  read across three table-level `cites`, and the test pinned the right-atrium sentence on a
  left-ventricle answer.

  ### What each row carries now

  Measured 2026-09-15 on the NCBI StatPearls "Anatomy, Thorax, Heart" page (HTTP 200; a nonsense path
  on the same host returns 404), with inline tags removed without a space and only ASCII whitespace
  collapsed. The nonsense page holds none of the spans below.
  - **Both right-atrium rows** take the right-atrium sentence, which names both vena cavae.
  - **Both right-ventricle rows** take the right-ventricle sentence, which names the pulmonic valve and
    the pulmonary artery.
  - **The left-atrium row** takes two contiguous sentences: *"This oxygenated blood is collected by the
    four pulmonary veins, two from each lung. All four of these veins open into the left atrium that
    acts as a collection chamber for oxygenated blood."*
    - The old `cites` carried only the second, which says "these veins" and never names the pulmonary
      veins. No single sentence on the page names both the left atrium and the pulmonary veins.
    - This follows the two-sentence smooth-muscle span in `biology/muscle-types.adj`.
  - **The left-ventricle row** takes the left-ventricle sentence.
  - **The envelope** is now the page's *"The heart is a muscular organ situated in the center of the
    chest behind the sternum."* It names no chamber and no vessel.
  - **Counts:**
    - Each of the four spans and the envelope occurs **exactly once**, inside a `<p>`, both with script
      and style data set aside and over the whole file.
    - The header's four ellipsis fragments each occur once.
  - Every row's page is the envelope's, so no row restates a locator line or trust (`ADJ-TABLES.md`
    §4).

  The header's "reproduces, byte-for-byte ... no new WebFetch" provenance note is replaced by the
  measurement, and its count of "four StatPearls sentences" becomes five, since the left atrium's span
  is two.

  ### Pins

  The test keeps every behaviour it shipped with: the forward recall to the aortic valve, the reverse
  recalls of both vena cavae, and the abstention on septum. Two pins change:
  - **The forward recall** pinned the **right-atrium** sentence as the citation of a left-ventricle
    answer, which was true only because the envelope was primary for every row. It now pins one answer
    whose citations array holds exactly the left-ventricle sentence, and asserts the right-atrium
    sentence is absent.
  - **The host-plus-trust needle** (#15209's shape) is gone. The reverse recall now pins two answers,
    each whose citations array holds exactly the right-atrium sentence.

  Added:
  - every vessel's answer is one answer whose citations array holds exactly its own span's citation,
    with no other chamber's span and not the envelope;
  - the left-atrium row carries the two-sentence span, and no row carries the "these veins" sentence
    alone;
  - a table-shape test.

  **9 of 9 mutants killed, two controls:**
  - the left-atrium row cut to the "these veins" sentence alone;
  - the left-ventricle row warranted by the right-atrium sentence;
  - the pulmonic-valve `source` demoted to `cites`;
  - the superior-vena-cava row reverted to a bare row that inherits the envelope;
  - the old envelope restored;
  - a table-level `cites` re-added;
  - a row restating a different trust tier;
  - a vessel atom rebound;
  - a chamber named in the envelope.

  The controls are the unmutated suite, green before the first mutant and after the last, with the
  file byte-identical afterwards. It's a local scratch harness, so that count can't be reproduced
  from the repo.

  **The envelope's wording reaches no answer, and is pinned only in the file.** Every row overrides
  it, so the framing sentence occurs zero times in the query example's output.
