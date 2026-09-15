- `biology/cell-division-genetic-outcome.adj` (new) — a sibling to the already-shipped
  `cell-division-daughter-cells.adj` (`cell_division_daughter_cells(process, count)`, HOW MANY
  daughter cells mitosis/meiosis each yield). That table's own header deliberately quotes,
  verbatim, the OTHER clause of each defining NHGRI sentence "for honesty" while explicitly stating
  it is NOT what the count-only schema extracts: mitosis's daughter cells "have identical genomes,"
  meiosis's are "haploid... the gametes." New `cell_division_genetic_outcome(process, outcome)`
  table: mitosis → genetically_identical, meiosis → haploid. Honest abstention on
  `binary_fission` (the prokaryotic process), matching the parent table's own abstention boundary.
  New e2e test file `facts_celldivisiongeneticoutcome_e2e.rs` (3 tests: forward recall with
  citation, backward recall of the haploid process, honest abstention). The parent table DOES carry
  a manifest objective (`adj.science.6to8.cell_division_daughter_cells`) scoped specifically to the
  daughter-cell COUNT competency; this sibling decodes a different leftover fact from the same
  cited spans rather than a new standalone curriculum competency, so — matching this session's
  established sibling-table precedent — no new manifest objective was added for it. Second slice
  from the fresh biology/ sweep tranche, queued after `mammal-origin.adj`.
