# Spanish A1 vocabulary tranche 4 -- thirty-five more words

Chapters 360-366, sequences 6920-7260, thirty-five lessons of one headword each.
Spanish moves **479 -> 514** distinct headwords at or below A1, against the
HL09 3.1 floor of **600**. Eighty-six to go.

| Ch. | Spine node | Words |
| --- | --- | --- |
| 360 | `SPINE-DEFINITE-REFERENCE` | la ropa, el vestido, el zapato, el pantalon, el boton |
| 361 | `SPINE-DEFINITE-REFERENCE` | la carne, el pescado, la leche, la sal, el aceite |
| 362 | `SPINE-ASK-LOCATION` | la tierra, el mar, el aire, el fuego, la luna |
| 363 | `SPINE-ASK-LOCATION` | el coche, el avion, el barco, el autobus, la bicicleta |
| 364 | `SPINE-DEFINITE-REFERENCE` | la mujer, la cara, la nariz, el dedo, el corazon |
| 365 | `SPINE-COUNT-ONE-TO-FIVE` | alto, largo, ancho, corto, lleno |
| 366 | `SPINE-DEFINITE-REFERENCE` | la caja, la lampara, la almohada, la escalera, el armario |

Seven chapters of five closes thirty-five exactly, with no chapter borrowing a
slot from another node.

## What the duplicate hunt dropped, and what caught it

The matcher compares whole strings, so a near-duplicate inflates the gate rather
than failing it. Every candidate was checked against all 583 existing headwords
after splitting on the compound delimiters, stripping articles, cutting grammar
annotations and ellipses, and then against the root ledger.

| Dropped | Caught by |
| --- | --- |
| `la camisa` | initial stem -- `cami-` is the taught `caminar` / `el camino` |
| `el abrigo` | initial stem -- `abri-` is the taught `abrir` |
| `el hombre` | initial stem -- differs from the taught `el hombro` by one vowel |
| `el cuerpo` | initial stem -- `cuer-` is the taught `la cuerda` |
| `el sombrero` | morphology -- the taught `la sombra` plus a suffix |
| `el bolsillo` | morphology -- the taught `la bolsa` plus a suffix |
| `la rueda` | root ledger -- the taught `la rodilla` is `rotella`, the diminutive of this very `rota` |
| `el rey` | root ledger -- `regere` is already spent on `regular` and `la derecha` |
| `dejar` | root ledger -- `laxus`, the same root as the taught `lejos`, which it resembles not at all |
| `cantar` | root ledger -- `incantare` is already spent on `encantado` |
| `el dueno` | compound + root -- `domingo` sits inside the taught `sabado, domingo` |
| `el sol` | compound -- inside the taught `hace calor, hace frio, hace sol` |
| `el ano`, `el mes` | compound -- inside `?Cuantos anos tienes?` and `los meses` |
| `la manana`, `el pelo` | compound -- inside `hasta manana` and `tomar el pelo` |
| `la lluvia` | morphology -- the taught `llueve` / `llovia` are the same `pluere` |
| `volver`, `entrar`, `subir`, `bajar` | morphology / root -- the taught `la vuelta`, `la entrada`, `ir`, `abajo` |
| `mirar`, `llamar`, `saber` | inflection -- `mira`, `me llamo`, `no se` are taught headwords |
| `buscar` | morphology -- its leading etymology is the taught `el bosque` |
| `escuchar` | root -- `auscultare` shares `auris` with the taught `el oido` |
| `vacio` | the tranche-3 precedent -- shares the visible `vac-` with the taught `la vaca` |
| **`dar`** | **the atom ledger, not the headword list** -- see below |

`dar` is the one worth keeping. It is not a headword anywhere, so every string
rule clears it -- but `ES-C65-di` introduces the atom `ES-LEX-DAR`, and its
`type` is `grammar`, which `vocabularyOf` does not count. Adding `dar` would
have raised the A1 number while re-teaching a word the course already owns.

Three near-misses were checked and **cleared**: `el dedo` sits inside the taught
`alrededor`, which is Latin `retro` and no relation; `el corazon` ends in the
taught `razon`, which is `ratio` and no relation; `la lampara` contains the
taught `para`. All three are non-initial substrings across no morpheme boundary,
which is a letter accident rather than the confusion that matters.

## Etymology, verified rather than pattern-matched

Every hook was checked against current consensus before it was written, and the
check changed four of them and hedged seven more.

**Four that were wrong.** `el zapato` was going to carry the *sabotage* story;
the clog-in-the-machinery tale is a twentieth-century invention and the word's
origin is genuinely untraced, so the lesson now teaches that instead. `la sal`
was going to fan out into four contrasting roots; `salud`, `saludo` and `salvo`
are **one** root, so it teaches three -- salt, whole-or-safe, and the leap of
`salir`. `la nariz` was going to cite Grimm's law, which touches no sound in the
word. `corto` was going to claim *short*, *shirt* and *skirt*; those are
Germanic and meet `curtus` only at reconstruction depth.

**Threads that run across chapters.** Two famous etymologies are taught as
fakes, and deliberately next to each other: *carne vale* for `carnival`, and
soldiers paid in salt for `salary`. Two words are borrowed twice into the same
language a thousand years apart -- `el aceite` from Arabic *az-zayt* beside
Latin *oleum*. Three words are coinages rather than inheritances, and say so:
`el avion` (French, 1875), `la bicicleta`, and `el autobus`, whose *bus* is a
bare Latin dative-plural ending with the stem thrown away.

**A root already in the learner's pocket.** `la luna` is the same PIE root as
the taught `la luz`, and the lesson is built on it -- inherited *light*,
borrowed *lucid*, Greek-routed *leukemia*, and `lunes` as a calque.

## Modality, budgets and pins

All seven chapters narrate as **"All 5 can be done entirely by ear."** No pin was
raised: `ruleStatements` stays at **30 of 30**, `paradigmTables` at 95,
`lessonsWithFindings` at **121** exactly, banned words unchanged, and Spanish
cross-chapter prose references at **zero**.

Spanish `forwardReferences` moves **308 -> 325**, and the provenance was checked
rather than assumed: **zero** of them are inside chapters 360-366. All seventeen
are earlier lessons that had used one of these words in passing and now finally
have somewhere to point.

One class of finding was fixed rather than absorbed. Four sentences told the
reader what "the course" had already taught them -- a claim a reader holding one
volume cannot check. `standalone-book` catches exactly that, and the fix
re-anchored each on the reader's own experience rather than reaching for a
vaguer word.

## The one gate this tranche had to move, and which way it moved

`language-ladder` splits lesson content into lazy batches and fails above a
request-count ceiling. Thirty-five lesson files took the measured count from 397
to **401**, over a ceiling of 399.

The fix was not to raise that ceiling. Lesson batches are grouped by track and
then split by a `maxSize`, and that `maxSize` is a bundler grouping parameter
rather than a budget: raising it 49 kB -> 56 kB takes the count **401 -> 353**.
So the request ceiling was **lowered to the measured 353** in the same commit --
it moved down by 46 while the corpus grew by 35 lessons. The largest emitted
batch goes 47,976 -> 54,688 bytes, about 11% of the 500 kB eager-chunk budget
that is the limit actually protecting the browser, and which did not move.

This is the second time content growth has walked into that ceiling; the first
was answered the same way, at 32 kB -> 49 kB. A third should not happen, and
issue #12918 carries the structural fix -- group batches by a chapter range,
something a reader actually navigates, so the count grows sublinearly.
