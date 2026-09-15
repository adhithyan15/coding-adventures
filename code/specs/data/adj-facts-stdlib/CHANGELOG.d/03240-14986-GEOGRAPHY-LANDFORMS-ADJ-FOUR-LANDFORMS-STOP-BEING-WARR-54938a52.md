- **#14986: `geography/landforms.adj` — four landforms stop being warranted by mountain's definition.**
  The envelope was mountain's own definition, *"Landmasses that project conspicuously above their
  surroundings."*, so it was the primary source of the valley, plateau, plain and canyon answers
  too. Each row now carries a row-level `cites` holding its own landform's definition. The envelope
  is now the thesaurus's own description, *"Types of named geographic features."*, which names no
  landform. The table is deliberately **not** converted to per-row `source`.

  This is the sibling of `geography/landform-secondary-feature.adj`, which cites the same USGS-hosted
  Feature Type Thesaurus page and has the same structure. On that page each definition is a
  `<span class="scope">` under a separate term link and never names its landform, so the decision
  is the same: #13934's held question, answered with a corroboration rather than a warrant.

  Measured 2026-09-15 (HTTP 200; a nonsense path on the same host returns 404). Mountain's definition
  occurs once in the page's visible text, under the `mountains` term link. The page follows it with
  a `[USGS Circ 1048]` reference, and the shipped span stops before that reference. The other four
  definitions and the envelope were measured for the sibling table: each occurs once, and the valley
  apostrophe is the `&#039;` entity, which renders as the straight one shipped.

  ### The header

  The header said *"An ADJ `table` carries ONE provenance envelope"* and, on that basis, held one row's
  span as the envelope while listing the others in comments. That is the false premise #13934 is
  about, and the paragraph is replaced. Its twin, *"Each pair, with the verbatim span that states
  it"*, now reads *"the verbatim definition it cites"*.

  ### Pins

  The shipped test's `contains("apps.usgs.gov") && contains(trust)` check (#15209's shape) is replaced
  by the whole contiguous citation run. Its forward, backward and abstention recalls are kept. Three
  tests are added, on the sibling's pattern:
  - every row's answer is one answer, with the landform bound from its descriptor, the envelope
    primary, its own definition the only corroboration, and no other landform's definition;
  - no row carries a `source` and no `cites` sits at table level;
  - the envelope is not mountain's definition and names no landform.

  The per-row loop binds the landform from the start. A fully ground query is ranked as a hypothesis
  with no citations, which is what the sibling's version of this loop tripped on.

  **12 of 12 mutants killed, two controls.** The mutants:
  - mountain's row citing valley's definition;
  - the old envelope restored;
  - a table-level `cites` re-added;
  - a row `cites` promoted to `source`;
  - a row `source` added at twelve spaces;
  - one row's `cites` deleted;
  - the canyon definition given a full stop;
  - mountain's definition extended with its `[USGS Circ 1048]` reference;
  - a locator repointed;
  - the trust tier flipped;
  - a descriptor atom rebound;
  - a landform appended to the envelope.

  The controls are the unmutated suite, green before the first mutant and after the last. The file
  was byte-identical afterwards. It's a local scratch harness, so that count can't be reproduced
  from the repo.

  **One arm was never observed to fire on its own.** As in the sibling, the "envelope names no
  landform" loop checks the test's `ENVELOPE` constant. The mutant that put a landform into the
  envelope was caught first by the envelope-equality assertion. It is kept as a guard for an edit
  that changes both, and claimed for nothing more.
