- `geology/fossil-formation-type.adj` (new) -- a new `fossil_formation_type(type, description)`
  table names three ways a fossil can form and what each actually is
  (amber->preserved_in_hardened_tree_sap, cast_or_mold->impression_of_a_living_organism,
  permineralization->mineral_deposits_form_a_cast_of_the_organism), quoted verbatim from
  Ducksters' "Earth Science for Kids: Fossils" page -- `trust consensus`, the same tier
  `ocean-zones.adj` uses for its non-.gov WHOI citation. Grounds NGSS 3-5 earth-science
  standards. Picked using the mandatory full-tree-grep-before-scoping discipline --
  `grep -rilE "\bfossil\b|fossil_formation_type|\bpermineralization\b|\bamber\b|cast_or_mold"
  code/specs/data/adj-facts-stdlib/` found only two incidental unrelated matches
  (`astronomy/spectral-classes.adj` and `biology/genetic-code.adj`'s "amber" STOP-codon
  nickname), confirming a completely fresh topic before this file was written.
  WebFetch-verified before writing (twice). Honest abstention on "freezing" (a real
  preservation method the same page mentions, but only in a single bare sentence rather
  than the fuller description style used for the three tabled here). NPS's "How Fossils
  Form" page was investigated first and deprioritized -- it mentions fossil types but lacks
  clean one-sentence definitions per type, unlike Ducksters. New manifest objective
  `adj.science.3to5.fossil_formation_type` (band 3-5, `recall` competency, `ngss` coverage
  root). New e2e test `facts_fossilformationtype_e2e.rs` (3 tests: direct recall, reverse
  binding, honest abstention on an untabled formation type).
