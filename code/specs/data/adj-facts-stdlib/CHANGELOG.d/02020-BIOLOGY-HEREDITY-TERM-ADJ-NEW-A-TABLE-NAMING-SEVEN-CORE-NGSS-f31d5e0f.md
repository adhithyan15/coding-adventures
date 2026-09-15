- `biology/heredity-term.adj` (new) — a `table` naming seven core NGSS MS-LS3 heredity vocabulary
  terms (gene, allele, dominant, recessive, genotype, phenotype, trait), each definition quoted
  verbatim from NHGRI's (National Human Genome Research Institute, genome.gov) Talking Glossary of
  Genomic and Genetic Terms: `heredity_term(term, definition)`. The FOURTH fresh-WebFetch slice
  into this item, and the first to ground "heredity" via a genuinely new primary source rather than
  the causal-composition rule-joining technique (two prior cycles had only ever checked, and
  rejected, whether an already-shipped heredity table joins another on a literal key — `biology/
  dna-base-pairs.adj` and `biology/cell-division-genetic-outcome.adj` each have no honest
  composable second table). FIRST CANDIDATE REJECTED: the classic K-8 "dominant/recessive human
  trait" worksheets (widow's peak, earlobe attachment, tongue rolling, dimples) — University of
  Utah's Genetic Science Learning Center and a University of Delaware genetics-myths page each
  independently confirm no published study supports single-gene Mendelian inheritance for any of
  these classroom staples, so shipping it would have encoded a documented genetics-education myth
  as fact. Shipped instead: NHGRI's own glossary, sidestepping the myth trap by grounding the
  VOCABULARY a correct heredity claim is built from rather than a specific (and wrong)
  trait-inheritance claim. `trust authoritative` (genome.gov). New manifest objective
  `adj.science.6to8.heredity_term`. New e2e test `facts_heredityterm_e2e.rs`.

