- `anatomy/respiratory-parts.adj` — all 8 rows converted to per-row provenance (RS-5e,
  #14986). The NOSE sentence was this table's `source`, the field that carries the tier, for
  every row, so `? part_function(diaphragm, $F)` came back proved by *"Nose hairs at the
  entrance to the nose trap large inhaled particles."* **The header said so in its own
  words** — *"An ADJ `table` carries ONE provenance envelope, so the `source`/`locator`/`trust`
  below hold the single cleanest span: the SEER statement that fixes the FIRST row"* — an
  accurate description of a defect. Seven of the eight rows were in that position, and their
  real evidence was already in the file, in a comment block no query could reach. Each row now
  carries its own `source` + `locator` at the envelope's tier.

  **Six pages for eight rows**: `larynx.html` states both the larynx and the trachea,
  `bronchi.html` states both the bronchi and the alveoli, and one of the six is the module
  index that is ALSO the envelope's locator (the lungs row cites it for a different sentence).
  That coincidence is recorded in the file because it makes one obvious mutation an
  **equivalent** mutant rather than a gap.

  All eight spans were verified against the fetched pages before writing — **read out of the
  shipped header rather than retyped**, since a retyped span has drifted twice in this cascade
  — with a negative arm (one word altered) that was absent from every page. 8/8 verbatim.

  The envelope becomes a FRAMING span (the module's definition of respiration), which names no
  part of the tract and warrants no row; it is pinned through its locator VALUE and tier, not
  just its presence (#15183).

  `facts_respiratoryparts_e2e.rs`: 4 tests. **The citation assertion that let this conversion
  pass unchanged is recorded as the defect in test form** — `out.contains("training.seer.
  cancer.gov")` is satisfied by any page on the site, so it held just as well when all eight
  rows carried the nose sentence. All eight rows are now pinned individually in single-answer
  queries, the two shared pages get a test asserting neither row can stand in for its sibling,
  and the nose sentence is counted reaching exactly its own row (twice — once under
  `citations`, once under `steps`) where it used to reach all eight. 17 of 18 mutants killed;
  the single survivor is **declared in the harness in advance** as equivalent (dropping the
  lungs row's locator leaves it inheriting the same URL).
