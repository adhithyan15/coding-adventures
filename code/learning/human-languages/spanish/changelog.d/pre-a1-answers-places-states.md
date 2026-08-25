## Unreleased — Pre-A1: weighing an answer, naming a place, reporting a state

Adds chapters 324-327 — twenty `type: word` lessons at sequences 5320-5510, all
pre-A1, all `variety: american-neutral`.

**324 — True, False, and Not Even** (`SPINE-RESPOND-BASIC`): *la verdad*,
*falso*, *acaso*, *menos*, *siquiera*. The track can already say *sí*, *no*,
*claro*, *exacto* and *de acuerdo*. These five let a learner put a weight on
somebody else's claim instead of only agreeing or refusing.

**325 — Country, City, Neighbourhood** (`SPINE-MEET-GREET`): *el país*, *la
ciudad*, *el barrio*, *la plaza*, *el lugar*. Five nested circles, widest first,
so that *soy de…* finally has somewhere to land.

**326 — Five Ways to Be** (`SPINE-CHECK-WELLBEING`): *nervioso*, *valiente*,
*solo*, *libre*, *listo*. States of mind, ending on the word where *ser* and
*estar* stop being a rule and start changing the meaning.

**327 — Awake, Asleep, Strong, Weak** (`SPINE-CHECK-WELLBEING`): *despierto*,
*dormido*, *fuerte*, *débil*, *sano*. Two pairs and a closer, all of them
reports from the body rather than the mind.

### Why these words, in this order

- **Two lessons built around a fact rather than a gloss.** `ES-C326-solo` says
  plainly that *alone* and *only* are one word, and that the accent Spanish once
  used to separate them — *sólo* — was abolished by the RAE in 2010, so both are
  now written *solo*; the lesson then notes the difference spelling never showed,
  which is that the *alone* one switches to *sola* and the *only* one never
  moves. `ES-C326-listo` cashes in the *ser*/*estar* split: *ser listo* is
  clever, *estar listo* is ready, and choosing wrong does not sound odd, it says
  something else.
- **Phrases opened into their parts, again.** *¿verdad?* has been a tag question
  since the negation chapter; `ES-C324-verdad` hands over the noun inside it.
  *Hasta luego* and *en primer lugar* both carry Latin *locus*; `ES-C325-lugar`
  shows that *luego* and *lugar* are one word that split into time and space.
  And `ES-C324-siquiera` is built entirely from *si* and *quiera*, two words the
  learner already owns, welded until the seam closed.
- **One sound rule, proved twice in a row.** The stressed-vowel break was taught
  long ago as *e → ie* and *o → ue*. `ES-C327-despierto` shows the *e* of
  *despertar* cracking because the stress moves onto it, and `ES-C327-dormido`
  shows the *o* of *dormir* NOT cracking because the stress has moved off it.
  Together they turn a memorised pair into a condition.
- **The Latin `-tātem` road, walked twice in two chapters.** *vēritātem* gives
  *verdad*, *cīvitātem* gives *ciudad*, and English took the same ending as *-ty*
  in *verity* and *city*. `ES-C324-verdad` opens the road and `ES-C325-ciudad`
  re-walks it, so the *-dad* ending becomes predictive rather than decorative.
- **Two Latin words spelled the same, kept apart.** `ES-C326-libre` names the
  collision English hides: *liber* the tree bark gave *libro* and *library*,
  *liber* the unowned person gave *libre* and *liberty*. Spanish kept them
  visibly distinct; English filed them on different shelves.
- **Where English split a word Spanish kept whole.** `ES-C327-sano` shows
  *sānus* covering body and mind at once, and English sending *sane* and
  *sanity* to the mind while *sanitary* and *sanatorium* went to the body.

### Etymon ledger

Re-spends six roots the track has already paid for, so the words land as payoffs
rather than as new facts: `magis-minus-latin` (with `ES-C04-regular`),
`si-latin` and `quaerere-latin` (with `ES-C55-si` and `ES-C18-querer-subjuntivo`),
`locus-latin` (with `ES-C38-luego`), `valere` (with `ES-C274-mas-vale-pajaro`),
`dormire-latin` (with `ES-C36-dormir`) and `fortis-latin` (with
`ES-C315-fuerza`).

Mints `veritas-latin`, `fallere-latin`, `casus-latin`, `pagus-latin`,
`civitas-latin`, `barri-arabic`, `platea-latin`, `nervus-latin`, `solus-latin`,
`liber-free-latin`, `lists-gothic`, `expergisci-latin`, `debilis-latin` and
`sanus-latin`.

`liber-free-latin` is deliberately NOT the existing `liber-latin` that
`ES-C43-libro` spends. Latin had two unrelated words spelled *liber* — the inner
bark of a tree and the person nobody owns — and fusing them into one slug would
record a false claim in the ledger to save a string.

Three etymologies are worth naming because they are not obvious. *El barrio* is
not Latin at all: it is Andalusi Arabic *barrī*, *of the outside*, the district
that had spilled past the town wall, and it joins *hasta*, *azul* and *café* as
the fourth Arabic word in the track. *El país* reaches English twice from one
Latin word — *pāgānus*, the country-dweller, arrives through Church Latin as
**pagan** and through French *paisant* as **peasant**. And *listo* is Gothic
*lists*, *cunning*, from the Germanic layer the Visigoths left in Spanish — the
layer already met in *blanco*, and one of the places where Spanish kept a
Germanic word English dropped.

### Choices made against the planned allocation

The pool held twenty-two verified-free candidates for twenty slots, but four
chapters of five across three spine nodes needs one node to supply ten, and no
node in the pool held that many. `SPINE-CHECK-WELLBEING` was taken to ten by
adding *valiente*, which pairs against *nervioso* the way *despierto* pairs
against *dormido* and re-spends `valere` — a root the track already owns through
*más vale pájaro en mano* and the everyday *vale*.

Three pool words were dropped:

- ***ojalá*** is already taught. `ES-C18-ojala` carries it as
  `headword: ojalá + subjunctive` at sequence 1488, together with the Arabic
  etymon `ES-ETYMON-OJALA-HISPANIC-ARABIC`. A compound-aware matcher that splits
  on `·`, `/` and `,` does not split on `+`, which is how it read as free. The
  Arabic highlight the tranche wanted is carried by *el barrio* instead.
- ***el camino*** and ***la parada*** are free, and were dropped for coherence:
  a road and a bus stop are travel words, and chapter 325 is about the nested
  places you name when you meet somebody rather than the ones you pass through.

*Falso*, *la verdad* and *el lugar* were kept despite near-collisions, and each
is turned into a payoff rather than a duplicate: `ES-C09-falsos-amigos` teaches
the metalinguistic term and not the adjective, `ES-C267-verdad-no` teaches the
tag question and not the noun, and `ES-C273-en-primer-lugar` teaches the phrase
with the noun sealed inside it.

### Wiring

- `spanish/chapters.json` — four chapter entries, each with a `production`
  payoff on the chapter's last lesson.
- `spanish/curriculum.json` — paths `ES-PATH-324-01` … `ES-PATH-327-01`,
  extensions `ES-EXT-324-TRUE`, `ES-EXT-325-PLACE`, `ES-EXT-326-STATE` and
  `ES-EXT-327-STATE` (one per chapter, because an extension cannot span
  segments), and the four path ids appended to the `SPINE-RESPOND-BASIC`,
  `SPINE-MEET-GREET` and `SPINE-CHECK-WELLBEING` segment ledgers.
- `core/book-generation.json` — four targets, kept inside the Spanish group.
- `spanish/book/book.tex` — no hand-edit; regenerated from the targets above.

All twenty lessons are `voice` and drivable, so each of the four chapters
narrates as "All 5 can be done entirely by ear."

Spanish at-or-below-pre-A1 vocabulary moves 249/300 → **269/300** (427 → 447
total), measured on top of the courtesy-and-when tranche.
