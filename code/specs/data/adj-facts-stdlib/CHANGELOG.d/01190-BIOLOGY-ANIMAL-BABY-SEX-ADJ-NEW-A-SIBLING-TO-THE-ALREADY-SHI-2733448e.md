- `biology/animal-baby-sex.adj` (new) — a sibling to the already-shipped `animal-babies.adj`
  (`animal_baby(animal, baby)`, ONE generic baby name per animal): a new `animal_baby_sex(animal,
  sex, baby)` table names the SEX-SPECIFIC baby term the source distinguishes, decoded from a
  span already sitting unused inside `animal-babies.adj`'s own `source` field
  (`"colt (male), filly (female), foal, weanling, yearling"`) — no new WebFetch. Of the 24
  animals `animal-babies.adj` ships, `horse` is the ONLY one whose Wikipedia "Young" cell draws a
  clean male/female split, so this table is honestly narrow: one animal, two rows (male → colt,
  female → filly). New e2e test file `facts_animalbabysex_e2e.rs` (2 tests: both-term recall
  with citation, honest abstention on an animal the source does not sex-distinguish). No manifest
  objective, matching `animal-babies.adj`'s own precedent of not having one.
