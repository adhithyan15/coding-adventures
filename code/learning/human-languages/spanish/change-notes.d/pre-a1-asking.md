## Unreleased — Pre-A1: the things you have to ask a stranger for (chapters 310-313)

### Added

- **Twenty pre-A1 vocabulary lessons across four chapters**, all hanging on
  `SPINE-POLITE-REQUEST-REPAIR`. The repair kit (chapter 303) taught a learner
  how to keep a stalled exchange alive — *ayuda*, *un momento*, *otra vez*,
  *no sé*, *¿qué significa?* — but gave them nothing to ask **for**. These four
  chapters supply the nouns, so that a request in Spanish becomes a thing a
  learner can actually complete: a noun, plus *por favor*, said to somebody who
  is holding the object.

  - **Chapter 310, *El cuarto y la mesa*** — `la llave`, `la ducha`, `la mesa`,
    `la luz`, `el baño`. The room and the table.
  - **Chapter 311, *La tienda y la cuenta*** — `la tienda`, `la cuenta`,
    `el precio`, `la bolsa`, `el asiento`. The transaction, start to finish.
  - **Chapter 312, *Lo que llevas*** — `el reloj`, `el teléfono`, `la maleta`,
    `el mapa`, `la carta`. What is already in your pocket.
  - **Chapter 313, *El mostrador*** — `el boleto`, `la moneda`, `el pasaporte`,
    `la entrada`, `el cambio`. The counter, where the permissions are handed
    over.

- **An etymological through-line for the whole tranche: almost none of these
  objects is named after its job.** *Llave* is *clavis*, the little key that also
  named your **clavicle**, a musical **clef** and a **conclave**. *Ducha* is
  *ducere*, to lead, which is why a shower is family to a **duct**, an
  **aqueduct** and a **duke**. *Mesa* is a board cut to a measure, and English
  took the Spanish word untouched for the flat-topped hill. *Tienda* is
  *tendere*, to stretch — the shop is the awning. *Bolsa* is Greek *byrsa*, a
  hide, still audible in **purse**, **bursar** and **reimburse**. *Maleta* is a
  little Gaulish *mala*, a bag — which is why the post is called **mail**.
  *Mapa* is *mappa*, a cloth, and so are **napkin** and **apron**. *Cambio* is
  a verb Latin borrowed from Gaulish, so every English **change** is a dead
  language still being spoken.

  Two lessons deliberately teach a non-relationship, because the corpus's
  etymology is only trustworthy if it also says when a resemblance is false:
  English **bath** is not related to `el baño` (the honest relative is
  *balneology*), and English **light** is a distant Germanic cousin of `la luz`
  rather than a loan from it.

- **Roots re-spent rather than minted** where the etymon was already in the
  ledger: `computare-latin` (from *contar*) for `la cuenta`, `sedere-latin` for
  `el asiento`, `bulla` (from `ES-C268-billete`) for `el boleto`, and both
  `passus` and `porta-latin` for `el pasaporte`. `el boleto` names the
  `billete`/`boleto` split explicitly — same seal, two roads, and *boleto* is
  the form to carry across the Americas.

- **Curriculum wiring**: paths `ES-PATH-310-01` … `ES-PATH-313-01` and
  extensions `ES-EXT-310-ASK` … `ES-EXT-313-ASK`, all four appended to the
  `SPINE-POLITE-REQUEST-REPAIR` segment ledger; four `chapters.json` entries
  with production payoffs on `ES-C310-bano`, `ES-C311-asiento`,
  `ES-C312-carta` and `ES-C313-cambio`; four `core/book-generation.json`
  targets and four `book.tex` inputs.

### Changed

- Spanish's at-or-below-pre-A1 vocabulary count rises by exactly twenty, from
  169/300 to 189/300 against the HL09 §3.1 target (347 to 367 headwords in
  total).
