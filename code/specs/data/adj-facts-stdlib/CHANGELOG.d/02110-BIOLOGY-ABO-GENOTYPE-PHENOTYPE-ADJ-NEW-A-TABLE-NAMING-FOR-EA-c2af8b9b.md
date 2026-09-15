- `biology/abo-genotype-phenotype.adj` (new) — a `table` naming, for each of three named ABO
  blood-type genotype combinations (the two homozygotes, and the one heterozygote a self-cross
  between them produces), the blood-type PHENOTYPE it produces, quoted verbatim from OpenStax
  "Concepts of Biology" §8.3 "Extensions of the Laws of Inheritance" (CC-BY 4.0, Rice University;
  WebFetch-verified against the raw server-rendered page text before writing this file, a new
  source never before cited in this stdlib): `abo_genotype_phenotype(genotype, blood_type)`,
  `ia_ia` → `a`, `ib_ib` → `b`, `ia_ib` → `ab`. This is the THIRD instance of the "heredity" Major
  Gap (ADJ-STDLIB-COVERAGE.md §5.1/§5.2), after `biology/dna-base-pairs.adj` and
  `biology/heredity-term.adj`, and the first to ground it with a real, textbook-solid worked
  example of multiple alleles and codominance rather than either a molecular fact (base-pairing)
  or curated abstract vocabulary. Composition with `heredity-term.adj` itself was checked again
  this cycle (a fresh full-tree grep confirms no shipped table anywhere has `gene`, `allele`,
  `dominant`, `recessive`, `genotype`, or `phenotype` as a row value) and remains dry, matching
  that table's own header, which already documents this exact check running dry twice before it
  shipped; this table instead grounds the SAME underlying concepts with fresh sourcing. The NCBI
  Bookshelf "The ABO blood group" chapter (already cited by `blood-groups.adj`) was tried first
  and rejected for this specific purpose: it states the general facts (three alleles, "inherited
  codominantly over O", "six possible genotypes and four possible blood types") but never
  enumerates the genotype→phenotype mapping in continuous prose (curl-verified: no "AA"/"AO"/
  "BO"/"OO" genotype-label substring anywhere in the chapter text). Deliberately reuses
  `blood-groups.adj`'s own `a`/`b`/`ab` phenotype atoms in its `blood_type` column (rather than
  inventing new ones) so a later rule could chain genotype → phenotype → antigen into a single
  derived fact — that follow-on rule is NOT shipped here, kept as a separate, smaller slice.
  Runs the relation BACKWARD as a genuine recall (binding a blood type recalls its genotype).
  Honest abstention on `ia_i`, `ib_i`, and `i_i` — three more real ABO genotypes NCBI's chapter
  confirms exist, but whose phenotype the cited OpenStax passage never states in continuous
  prose (only a separately-stated general dominance rule implies it, which this table does not
  treat as a substitute for a directly quoted fact). New e2e test
  `facts_abogenotypephenotype_e2e.rs`; new manifest objective
  `adj.science.9to12.abo_genotype_phenotype`.

