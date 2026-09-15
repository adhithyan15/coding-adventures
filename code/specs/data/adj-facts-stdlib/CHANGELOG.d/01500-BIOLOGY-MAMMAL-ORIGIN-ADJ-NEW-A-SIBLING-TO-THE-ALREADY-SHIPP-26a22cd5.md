- `biology/mammal-origin.adj` (new) — a sibling to the already-shipped `animal-classes.adj`
  (`animal_class(animal, class)`, WHICH vertebrate class an animal belongs to — cat, kangaroo, fox,
  rabbit, bandicoot, quoll, koala are all just `mammal`). That table's own header already quotes,
  verbatim, two Australian Museum spans that split those same seven mammals into two disjoint
  origin groups — a distinction the one-class-per-animal schema had no room for: "introduced
  mammals such as cats, foxes and rabbits" and "marsupials like kangaroos, bandicoots, quolls and
  the Koala". New `mammal_origin(animal, origin)` table covers the full seven-mammal domain with no
  abstention: cat/fox/rabbit → introduced, kangaroo/bandicoot/quoll/koala → marsupial. New e2e test
  file `facts_mammalorigin_e2e.rs` (3 tests: forward recall with citation, backward recall of all
  introduced mammals, full-domain coverage with no abstention). No manifest objective, matching
  `animal-classes.adj`'s own precedent of not having one. First slice from a fresh biology/ sweep
  tranche (of the domain's remaining ~44 tables), discovered after the prior tranche
  (vertebrate-thermoregulation.adj / muscle-striation.adj / insulin-glucagon-trigger.adj)
  completed; two more strong candidates (`cell-division-genetic-outcome.adj`,
  `muscle-nuclei-count.adj`) are queued next.
