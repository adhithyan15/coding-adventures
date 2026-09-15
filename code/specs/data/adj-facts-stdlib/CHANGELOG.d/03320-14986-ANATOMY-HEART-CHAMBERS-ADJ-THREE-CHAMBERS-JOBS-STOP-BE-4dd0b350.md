- **#14986: `anatomy/heart-chambers.adj` — three chambers' jobs stop being warranted by the right-atrium sentence, and the left ventricle's "body" cites the page's gloss.**
  The envelope was the right-atrium row's own sentence, *"The right atrium receives deoxygenated blood
  from the entire body except for the lungs (the systemic circulation) via the superior and inferior
  vena cavae."*, so it was the primary source of all four answers. The header said so on purpose: "An
  ADJ `table` carries ONE provenance envelope", with every other row's span only listed in the header.
  Row blocks (RS-5e) made that no longer true.

  ### What each row carries now

  Measured 2026-09-15 on the NCBI StatPearls "Anatomy, Thorax, Heart" page (HTTP 200; a nonsense path
  on the same host returns 404), with inline tags removed without a space and only ASCII whitespace
  collapsed. The nonsense page holds none of the spans below.
  - **Each row takes the span its header already quoted for it** as a `source`, and each header quote
    matches the page character for character.
  - **The left atrium's span is two sentences.** "All four of these veins open into the left atrium ..."
    alone never names the lungs; the sentence before it does.
  - **The left ventricle also `cites` the right-atrium sentence.**
    - Its own sentence says "sending freshly oxygenated blood to the systemic circulation", not "body".
    - "systemic circulation" occurs twice on the page: in that sentence, and in the right-atrium
      sentence, which glosses it as "the entire body except for the lungs (the systemic circulation)".
    - So the row's `pumps_blood_to_body` is read across that gloss. The row keeps its own sentence as
      `source`, and the gloss is its one corroboration.
  - **The envelope** is now the page's *"The heart is a muscular organ situated in the center of the
    chest behind the sternum."*, the same framing sentence `heart-chamber-vessel.adj` uses. It names no
    chamber and no job.
  - Each of the four spans and the envelope occurs **exactly once**, inside a `<p>`, both with script
    and style data set aside and over the whole file.
  - Every row's page is the envelope's, so no row restates a locator line or trust (`ADJ-TABLES.md`
    §4). The corroboration's page is also the envelope's.
  - **Stale wording fixed.**
    - The header said each row carries "the table's citation", and the query example's comment said
      each answer returns "the table's source". Both now say the row's own.
    - The summary table said the left ventricle pumps "out to the whole body", stronger than the
      page's gloss ("the entire body except for the lungs"). It now reads "out to the body
      (systemic)".

  ### Pins

  The test keeps every behaviour it shipped with: the forward recalls of the right atrium and right
  ventricle, the reverse recall to the left ventricle, and the abstention on the aorta. Two pins
  change:
  - **Its "whole citation" pin** checked that the right-atrium sentence appeared somewhere in the output
    of all three answers. That was true only because the envelope was primary for every row. It becomes
    three answers, each whose citations array holds exactly its own chamber's citation; the left
    ventricle's includes the right-atrium corroboration.
  - **Its host-plus-trust needle** (#15209's shape) is gone.

  Added:
  - every job's answer is one answer whose citations array holds exactly its own span (and, for the
    left ventricle, its corroboration), with no other chamber's span and not the envelope;
  - a table-shape test, including the one row-level `cites`.

  **10 of 10 mutants killed, two controls:**
  - the left-atrium row cut to the "these veins" sentence alone;
  - the right-ventricle row warranted by the right-atrium sentence;
  - the left ventricle's corroboration dropped;
  - the left ventricle's `source` and corroboration swapped;
  - the right-atrium row reverted to a bare row that inherits the envelope;
  - the old envelope restored;
  - a table-level `cites` added;
  - a row restating a different trust tier;
  - a job atom rebound;
  - a chamber named in the envelope.

  The controls are the unmutated suite, green before the first mutant and after the last, with the
  file byte-identical afterwards. It's a local scratch harness, so that count can't be reproduced
  from the repo.

  **The envelope's wording reaches no answer, and is pinned only in the file.** Every row overrides
  it, so the framing sentence occurs zero times in the query example's output. In that output one
  answer, the left ventricle's, carries a corroboration; it appears twice because the answer's proof
  step repeats it.
