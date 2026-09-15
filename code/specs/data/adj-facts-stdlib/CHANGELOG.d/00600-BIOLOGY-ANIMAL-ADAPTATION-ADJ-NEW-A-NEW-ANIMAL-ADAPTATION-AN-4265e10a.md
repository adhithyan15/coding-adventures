- `biology/animal-adaptation.adj` (new) -- a new `animal_adaptation(animal, adaptation)` table
  names three animals and the one survival adaptation each is known for (arctic_fox->camouflage,
  groundhog->hibernation, canada_goose->migration), each row quoted verbatim from a DIFFERENT
  nationalgeographic.com animal-facts page -- `trust consensus`, the same tier this stdlib
  already reserves for other National Geographic sources (`animal-habitat.adj`,
  `rainforest-layer.adj`). Grounds NGSS 3-LS4-3. Picked using the mandatory
  full-tree-grep-before-scoping discipline -- `grep -rilE "\bgroundhog\b|\bcanada.goose\b|
  \barctic.fox\b" code/specs/data/adj-facts-stdlib/` found ZERO hits, confirming a completely
  fresh topic before this file was written. Each of the three citations was independently
  WebFetch-verified with an explicit keyword search confirming the adaptation term ("camouflage",
  "hibernation", "migrate") appears in a clean quotable sentence on its own page -- a genuinely
  different source family from an earlier, unsuccessful attempt to cite NPS teacher-lesson-plan
  pages for this same topic, which turned out to be activity prompts rather than pages that state
  concrete animal-to-adaptation facts. Since each row's animal comes from a DIFFERENT source page
  and an ADJ `table` carries only ONE table-level `source`/`locator`/`trust` block, the table's
  own citation is the arctic_fox row's (the primary/first-listed source), and the other two rows'
  own distinct citations are documented in header prose -- mirroring
  `ocean-observing-instruments.adj`'s established multi-source discipline. Honest abstention on
  "penguin" (a real, well-known animal, but not one of these three tabled here). New manifest
  objective `adj.science.3to5.animal_adaptation` (band 3-5, `recall` competency, `ngss` coverage
  root). New e2e test `facts_animaladaptation_e2e.rs` (3 tests: direct recall, reverse binding,
  honest abstention on an untabled animal).
