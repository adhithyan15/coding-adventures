- `biology/animal-classes.adj` (extended) -- extended the already-shipped
  `animal_class(animal, class)` table from 8 to 18 rows, adding fox, rabbit,
  bandicoot, quoll, koala (mammal), cassowary, hummingbird (bird), lizard,
  crocodile (reptile), and ray (fish). Every added animal is drawn from
  material ALREADY quoted in the table's own header ("introduced mammals such
  as cats, foxes and rabbits", "marsupials like kangaroos, bandicoots, quolls
  and the Koala", "the Emu and Southern Cassowary", "tiny hummingbirds up to
  huge ostriches", "turtles, lizards, snakes and crocodiles", "sharks and
  rays") -- zero new WebFetch needed, mirroring the "extend an existing
  table" pattern already used for `cloud-types.adj` and `noun-type.adj`.
  Picked after checking plant-tropisms.adj, vertebrate-groups.adj,
  blood-groups.adj, kingdoms.adj, energy-sources.adj, sound-properties.adj,
  em-spectrum.adj, light-colors.adj, flame-colors.adj, and ph-scale.adj this
  window -- none as directly extendable as animal-classes.adj's own
  already-quoted list sentences. `bat` remains the honest-abstention target
  (a real mammal, deliberately excluded as a surprising borderline case for
  beginners). Extended e2e test `facts_animalclasses_e2e.rs` to 2 tests (the
  original + a new extension test covering fox/ray/cassowary/bat). No new
  manifest objective (same library, same objective).
