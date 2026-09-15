- **#13934 batch 1: the evidence the headers documented and the tables dropped is now encoded.**
  `anatomy/tooth-parts` (+5 `cites`) and `biology/neuron-parts` (+3), across **six distinct source
  pages**. That page count is the point: each header recorded a per-row evidence table naming several
  pages, and each table encoded exactly ONE. The evidence was found, read and quoted — then left in a
  comment where nothing can check it and no learner ever sees it.

  `tooth-parts` had 6 rows and a `source` covering crown+enamel only; dentin, pulp, cementum and root
  were unsupported. **Cementum is not on the encoded page at all** — it comes from an NCBI page the
  header named and the table never carried. `neuron-parts` had 4 rows drawing on THREE NCBI pages,
  of which only the dendrites page was encoded; it now returns three distinct locators.

  *** BOTH HEADER QUOTES WERE DAMAGED, THE SAME WAY, AND THAT IS NOW THE EXPECTED CONDITION. ***
  `neuron-parts` quoted its cell-body sentence with STRAIGHT quotes around "read out" where the page
  has CURLY ones, and ended at "...synaptic interaction." while the page continues "(see Figures 1." —
  inventing a terminal period the source does not have there. `anatomy/brain-parts`, fixed earlier
  under #13931, had the identical pair: altered quote marks plus a truncation that dropped the clause
  WITHDRAWING the metaphor it quoted. Two of two header quotes checked against source were wrong in
  the same two ways. Re-extraction is not ceremony.

  WHERE THE PAGE'S SENTENCE BOUNDARY IS AWKWARD, THE PAGE WINS. The cell-body sentence runs into a
  figure reference. Trimming to "...synaptic interaction." would read better — and would be exactly
  the edit that produced the two damaged headers above — so the citation is quoted to the source's own
  period, figure reference included.

  Two quotes are WIDENED past a dependent opening: "This area is known as the \"pulp\" of the tooth"
  names an *area*, not the pulp, so it carries the preceding sentence. A citation is read DETACHED
  from its page and must name its own subject.

  *** FIVE MEMBERS OF THE 33 ARE BLOCKED, EACH FOR A DIFFERENT REASON, AND EVERY ONE WAS FOUND BY
  ATTEMPTING THE WORK RATHER THAN BY PLANNING IT: *** `nutrition/food-groups` (myplate.gov is
  JS-rendered — two pages extract to byte-identical site chrome), `money/us-bills` (all seven evidence
  pages are PDFs, and its citation names the DENOMINATION column while the table answers PORTRAITS —
  zero value coverage across all seven rows), `anatomy/kidney-parts` (its unencoded evidence is on
  Wikipedia while the table declares `trust authoritative`, and `cites` has NO trust field, so encoding
  it would misrepresent the tier — a real format gap, measured to affect exactly one library),
  `biology/cell-division-daughter-cells` (both URLs are genome.gov, which fails certificate
  verification here), and `biology/vitamins` (ods.od.nih.gov returns 403). Three NIH subdomains fail
  SSL while ncbi.nlm.nih.gov works, so this is environmental, not a `.gov` policy.

  533 test binaries / 1592 tests green, clippy -D warnings clean. Both `.query.adj` companions parse
  and run. Every sentence re-extracted from its own page and verified verbatim against raw HTML.

