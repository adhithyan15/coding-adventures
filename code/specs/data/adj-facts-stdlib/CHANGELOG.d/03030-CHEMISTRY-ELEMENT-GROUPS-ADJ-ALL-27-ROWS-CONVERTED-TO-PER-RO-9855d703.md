- `chemistry/element-groups.adj` — all 27 rows converted to per-row provenance (RS-5e,
  #14986). The ALKALI-METAL sentence was this table's `source`, the field that carries the
  tier, for every row; it names lithium through francium and no other element, so
  `row (helium, noble_gas)` came back warranted by a sentence that mentions no gas, on a page
  about a different family. Twenty-one of 27 rows were in that position. Their evidence was
  already in the file — three families as untiered `cites`, the transition metals only in a
  comment — and is now each row's own `source` + `locator`, at the envelope's tier. The
  envelope becomes a FRAMING span (the definition of a periodic-table group) which names no
  element and warrants no row; it is pinned in the test through its locator VALUE and tier,
  not just its presence.

  FOUR SPAN DEFECTS, each measured against the rendered page this cycle rather than argued
  from the file:

  1. **The alkali span did not occur on its own page.** The lead renders
     `... potassium (K),[note 1] rubidium (Rb), caesium (Cs),[note 2] and francium (Fr).` —
     the shipped span dropped BOTH note markers, making it text the page never displays as one
     run. This file's own header already recorded that defect twice, for the noble-gas span
     ("ELIDED the parenthetical with `...` — a constructed span") and the halogen one ("The
     `[1]` is a real rendered footnote marker and stays"). Caught on two spans, missed on the
     one warranting the whole table.
  2. **The transition-metal span was truncated mid-claim.** The page reads `... are transition
     metals (iron, cobalt and nickel) or inner transition metals (gadolinium).` Cutting at the
     first disjunct turns the page's "A or B" into a bare "A" — a stronger claim than it makes.
     A shipped test comment said this sentence "is not on the live page under any extractor
     fix"; measured, the FULL sentence is verbatim on the page and the TRUNCATED one is not.
     The earlier probe searched for the truncation, so the truncation is what it failed to
     find — the defect reporting itself as evidence that no source existed.
  3. **The alkaline-earth span opened on a pronoun** — `They are beryllium (Be), ...` — whose
     antecedent sat off the span, so the quoted run never said which group those six elements
     are. The preceding sentence is contiguous on the page; both are now quoted as one run.
  4. **The header's row count was wrong in three places.** It said twenty-eight. The table has
     27 rows (6+6+6+6+3), which the header's own truth table also lists as 27 lines.

  `facts_elementgroups_e2e.rs`: 6 tests. All 27 rows pinned individually in single-answer
  queries (binding the element, since binding the family returns six rows and a whole-stdout
  `contains` is then satisfied by an intact sibling's copy — the masking defect from #15164).
  Two whole-list corroboration pins are replaced: the tennessine caveat is now asserted on the
  tennessine ROW the caveat is about, and each family's sentence is counted, reaching exactly
  its own rows twice each (12/12/12/12/6) where one sentence used to reach all 27. 37 of 37
  mutants killed, baseline green — every row's copy broken individually, plus re-dropping the
  note markers, re-truncating the transition span, re-orphaning the pronoun, repointing the
  envelope locator, and a fabricate-the-envelope arm.
