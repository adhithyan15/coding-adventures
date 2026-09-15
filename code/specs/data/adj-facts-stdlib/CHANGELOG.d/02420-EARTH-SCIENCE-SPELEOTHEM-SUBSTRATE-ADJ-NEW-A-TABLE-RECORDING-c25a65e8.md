- `earth-science/speleothem-substrate.adj` (new) — a `table` recording what a speleothem grows ON:
  `speleothem_substrate(speleothem, substrate)`, eleven rows over two speleothems and three
  sentences. `helictite` → `cave_ceiling`, `cave_wall`, `cave_floor_less_often`,
  `carbonate_coating`, `crust`, `soda_straw_sometimes`; `frostwork` → `stalactite`, `cave_wall`,
  `cave_ceiling`, `ledge`, `cave_floor_less_occasionally`. EIGHTH cave/karst library.

  IT EXISTS BECAUSE `speleothem-growth-surface.adj` ABSTAINED ON `helictite`, and the abstention
  still stands. That library's header records the reason: the source places helictites on three
  surfaces with a frequency hedge on the third, and "one surface would drop the others while
  three-as-equals would flatten the source's own frequency hedge." That reasoning was right about
  THAT relation — `speleothem_growth_surface` is single-valued and means "the surface a dripstone
  grows FROM", one surface each. It was never an argument that the fact is untableable. This
  relation is multi-valued, means "grows ON" (the source's own verb for these speleothems), and
  carries the hedge the ordinary way: INSIDE THE ATOM of the value it modifies, the placement rule
  `veto-override.adj` and `karst-process-zone.adj` already apply. So bare `cave_floor` is NOT a
  value and a query for it abstains for both speleothems. Same move `speleothem-component.adj` made
  for the `column` abstention: when a table declines, the fix is a new relation shaped to the
  question, never a loosened row in the old one.

  TWO SPELEOTHEMS, NOT ONE. This was nearly built as a helictite-only table, which would have been
  thin enough to question. Re-reading the page showed the same shape stated twice — a list of
  substrates ending in a hedged floor — so `frostwork` is a second member and the reverse query on
  `cave_wall` returns BOTH, from two different sentences. "What grows on cave walls?" is a question
  nothing in this stdlib could answer before.

  Provenance: the `source` envelope quotes TWO CONTIGUOUS SENTENCES as one string, deliberately. The
  substrate sentence begins "They typically grow on other speleothems…", and cited alone its subject
  is a bare pronoun — a reader could not tell who "They" refers to. The sentences are adjacent in the
  article, so quoting the pair is still verbatim and makes the citation self-contained; a citation
  that cannot be read without the page open is not doing its job. The `cites` corroboration carries
  the frostwork sentence. U.S. National Park Service "Speleothems", `trust authoritative`, taken from
  a RAW TEXT EXTRACTION rather than a fetch summary.

  THREE QUALIFIERS, EACH PLACED BY THE SETTLED RULE. "less often" modifies cave floors only and
  "sometimes" modifies soda straws only, so both ride in their atoms. "typically" modifies the whole
  clause "grow on other speleothems", so it stays in the CITATION and the atoms `carbonate_coating`
  and `crust` remain bare. Frostwork's "less occasionally" rides in its atom too — and is
  deliberately NOT normalised to helictite's "less often", even though the phrase is awkward and
  plainly means the same thing. They are different sentences and each atom carries the word its own
  sentence used; smoothing them into one spelling would be editing a citation to make a table
  tidier. The e2e test asserts frostwork never borrows helictite's wording.

  Two things the table does not claim, stated in its header so nobody reads them in: the substrate
  column MIXES KINDS on purpose (cave parts like `cave_ceiling` and `ledge` alongside other
  speleothems like `stalactite` and `crust`), because the relation means "the thing it grows on" and
  the source itself mixes them in one list; and THE LISTS ARE NOT CLOSED, since the helictite
  substrate sentence says "SUCH AS carbonate coatings, crusts" — exemplary, not exhaustive, so
  absence of a substrate means the source did not name it rather than that the speleothem does not
  grow there. Honest abstention also on `stalagmite`, `column`, `drapery` and the rest, which this
  page never places on a substrate, and on `slanted_surface` (cave bacon "forms on slanted
  surfaces", but that describes the shape of a surface rather than an identifiable thing in a cave).

  New `speleothem-substrate.query.adj` and `facts_speleothemsubstrate_e2e.rs` (6 tests: all six
  helictite substrates with the citation, the pronoun antecedent travelling with the citation, the
  reverse lookup finding both speleothems, each speleothem keeping its own hedge wording with a
  negative assertion that frostwork never borrows helictite's, the unhedged-floor abstention with a
  negative assertion that neither speleothem is asserted unqualified, and abstention where the source
  names no substrate). Every assertion was mutation-tested: normalising frostwork's wording and
  baring helictite's hedge each make the forbidden needle appear.

  ONE TEST WAS FOUND VACUOUS AND FIXED BEFORE SHIPPING, which is worth recording because it is the
  second instance of the same mistake in this series. The pronoun-antecedent test originally asserted
  the two sentences SEPARATELY, which passes just as happily when they are split into two citations —
  `source` holding the first and a `cites` holding the second — the exact arrangement the test exists
  to rule out. Mutation-verified: under the split, both original needles remain present and the test
  still passes, while the property it names is false. The assertion is now a SINGLE needle straddling
  the sentence boundary, which disappears under that mutation. The general error both times was
  asserting on the whole output blob rather than on the structural property; a needle that any
  successful run would satisfy is decoration, not a test. New manifest objective
  `adj.science.3to5.speleothem_substrate`.

