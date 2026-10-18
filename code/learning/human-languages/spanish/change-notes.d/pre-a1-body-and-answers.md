## Unreleased — Pre-A1: the body, what it feels, and the small words that answer

Adds chapters 314-317 — twenty `type: word` lessons at sequences 4820-5010, all
pre-A1, all `variety: american-neutral`.

**314 — The Body, Further Down** (`SPINE-CHECK-WELLBEING`): *el brazo*, *la
pierna*, *el pie*, *el diente*, *el oído*. Extends the body thread already
running through chapters 24, 286, 293 and 300 (*la mano*, *la boca*, *el ojo*,
*el cuello*, *el hombro*, *la espalda*, *la rodilla*, *el tobillo*, *la piel*,
*la garganta*), picking up the limbs and the two parts you cannot point at.

**315 — What the Body Feels** (`SPINE-CHECK-WELLBEING`): *el pecho*, *la fuerza*,
*el descanso*, *el miedo*, *la risa*. Turns from parts to states, and lands on
the laughter that chapter 316 answers.

**316 — Never, Always, All, Nothing** (`SPINE-RESPOND-BASIC`): *el llanto*,
*nunca*, *siempre*, *todo*, *nada*.

**317 — Something, Somebody, Nobody** (`SPINE-RESPOND-BASIC`): *algo*,
*alguien*, *nadie*, *otro*, *poco*. Quantifiers and indefinites — the words a
learner needs in order to answer at all, which is why they sit on RESPOND-BASIC
rather than on a vocabulary node.

### Why these words, in this order

The tranche is built around sound laws the track has already taught, so that
each new word confirms a rule instead of adding a fact.

- **The two vowel breaks.** Chapter 300 taught stressed Latin *e* → *ie*
  (*piel*) and *o* → *ue* (*sueño*). This tranche walks *pierna*, *pie*,
  *diente*, *miedo* and *siempre* through the first, and *fuerza* through the
  second, then states both together in `ES-C315-fuerza`.
- **Three Grimm pairs, used as bookends.** *pie*/*foot*, *diente*/*tooth* and
  *poco*/*few* are the same Germanic *p* → *f* shift. The run opens on the first
  and closes on the third, and `ES-C317-poco` says so explicitly.
- **Two more regular changes, each anchored on a word already taught.** Latin
  *ct* → Spanish *ch* is introduced through *pecho* and proved with *la noche*
  (chapter 2). Latin *pl-* → *ll-* is introduced through *llanto* and proved with
  *lluvia*, *lleno* and *llama*.
- **The *natus* twins.** *nada* is *res nata*, "a thing born"; *nadie* is
  *homines nati*, "people born". Both are the participle for *born*, left
  carrying a negative meaning after the negation fell off. Chapter 317 sets them
  against *algo* and *alguien* — both built on *ali-*, "other" — so the four
  indefinites arrive as two matched pairs rather than four separate items.
- **Latin's two words for "other", in one chapter.** *ali-* (*alius*) gives
  *algo* and *alguien*; *alter* gives *otro*.
- **The `-o`/`-a` contrast, reinforced rather than restated.** Chapter 304
  established that `-o` switches and `-e` does not. Here *nunca* and *siempre*
  refuse to move while *todo*, *otro* and *poco* agree, and `ES-C316-siempre`
  explains the one case the rule does not cover on its face: *nunca* ends in
  `-a` and still does not change, because it describes *when*, not a person.

### Etymon ledger

Re-spends six existing root slugs rather than minting near-duplicates:
`pie-ped` (cousins with Hindi *pair* and Persian *pâ*), `audire-latin` (with
Latin *audio* and Spanish *oír*), `totus-latin` (with Portuguese *tudo*),
`nada-nata-latin` (with *de nada*, chapter 4), `nascor-natus-born` (with Latin
*annos natus*) and `paucus-latin` (with *tampoco*, chapter 58). Mints
`bracchium-latin`, `perna-latin`, `pedem-latin`, `dentem-latin`, `pie-dent`,
`pectus-latin`, `fortis-latin`, `campsare-latin`, `metus-latin`,
`ridere-latin`, `plangere-latin`, `numquam-latin`, `semper-latin`,
`aliquod-latin`, `aliquem-latin` and `alter-latin`.

`ES-C315-descanso` states plainly that English has no everyday descendant of
Greek *kámptein*, rather than reaching for a cognate that is not there.

### Wiring

- `spanish/chapters.json` — four chapter entries, each with a `production`
  payoff on the chapter's last lesson.
- `spanish/curriculum.json` — paths `ES-PATH-314-01` … `ES-PATH-317-01`,
  extensions `ES-EXT-314-BODY`, `ES-EXT-315-BODY`, `ES-EXT-316-ANSW`,
  `ES-EXT-317-ANSW`, and the four new path ids appended to the
  `SPINE-CHECK-WELLBEING` and `SPINE-RESPOND-BASIC` segment ledgers.
- `core/book-generation.json` — four targets, kept inside the Spanish group.
- `spanish/book/book.tex` — four `\input` lines.

Spanish at-or-below-pre-A1 vocabulary moves 169/300 → 189/300 (347 → 367 total).
