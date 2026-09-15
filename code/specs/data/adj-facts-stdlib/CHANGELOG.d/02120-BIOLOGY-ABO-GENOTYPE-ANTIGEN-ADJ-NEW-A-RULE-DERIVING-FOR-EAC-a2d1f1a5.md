- `biology/abo-genotype-antigen.adj` (new) — a `rule` DERIVING, for each of the three ABO
  genotypes `abo-genotype-phenotype.adj` tables, which red-cell antigen(s) it ultimately
  produces: `abo_genotype_antigen(genotype, antigen)`, composing the already-shipped
  `abo_genotype_phenotype` table (`biology/abo-genotype-phenotype.adj`, OpenStax) with the
  already-shipped `blood_type_antigen` table (`biology/blood-groups.adj`, NCBI Bookshelf) on the
  literal, exact phenotype atom (`a`/`b`/`ab`) both tables share — `ia_ia` → `a_antigen`,
  `ib_ib` → `b_antigen`, `ia_ib` → `a_and_b_antigens`. This is the FOURTH instance of the
  "heredity" Major Gap (ADJ-STDLIB-COVERAGE.md §5.1/§5.2), and the FIRST to DERIVE rather than
  merely recall a heredity fact — the exact genotype→phenotype→antigen composition
  `abo-genotype-phenotype.adj`'s own header flagged by name as its most promising unexplored
  angle, rather than shipping it prematurely in that earlier, smaller slice. Zero new sourcing:
  both premises were already shipped and already cited before this file existed. Empirically
  verified in a scratch dir against the real built CLI binary before writing this file: all
  three genotypes join cleanly, and the reverse query (`? abo_genotype_antigen($G,
  a_and_b_antigens)`) correctly recovers `ia_ib`. Honest abstention on `ia_i`, `ib_i`, and `i_i`
  propagates through the join from `abo_genotype_phenotype`'s own already-documented abstention
  on those three genotypes (its cited passage never states their phenotype in continuous prose) —
  `blood_type_antigen` itself covers all four ABO phenotypes with no gap on that side, so every
  phenotype the first table can ever produce joins cleanly; the abstention is inherited, not
  independently discovered. New e2e test `facts_abogenotypeantigen_e2e.rs`; new manifest
  objective `adj.science.9to12.abo_genotype_antigen` (`infer` competency, matching this loop's
  other `rule`-derived facts). Also backfills two pre-existing README.md gaps flagged by a prior
  cycle: `biology/heredity-term.adj` and `biology/blood-groups.adj` had never had a row added to
  `adj-facts-stdlib/README.md`'s "Organized by subject, not by level" table, despite both being
  shipped and cited libraries — both rows added in this same commit.

