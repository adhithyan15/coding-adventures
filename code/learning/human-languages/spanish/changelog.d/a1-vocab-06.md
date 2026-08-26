# Spanish A1 vocabulary tranche 6 -- thirty-five more words

Chapters 374-380, sequences 7620-7960, thirty-five lessons of one headword each.
Spanish moves **549 -> 584** distinct headwords at or below A1, against the
HL09 3.1 floor of **600**. Sixteen to go.

| Ch. | Spine node | Words |
| --- | --- | --- |
| 374 | `SPINE-DEFINITE-REFERENCE` | el lapiz, el cepillo, el guante, la vela, el nudo |
| 375 | `SPINE-DEFINITE-REFERENCE` | la una, el labio, el pulgar, el beso, el sobrino |
| 376 | `SPINE-DEFINITE-REFERENCE` | la harina, la uva, el maiz, el chocolate, el pimiento |
| 377 | `SPINE-DEFINITE-REFERENCE` | la letra, la frase, el idioma, la historia, el baile |
| 378 | `SPINE-ASK-LOCATION` | la selva, el prado, el volcan, la cumbre, el edificio |
| 379 | `SPINE-COUNT-ONE-TO-FIVE` | seco, mojado, fresco, tibio, grueso |
| 380 | `SPINE-COUNT-ONE-TO-FIVE` | simpatico, alegre, feo, ultimo, necesario |

Seven chapters of five closes thirty-five exactly, with no chapter borrowing a
slot from another node. Atom runs continue their per-node counters rather than
restarting: `REF` 56-75, `WHERE` 56-60, `COUNT` 31-40.

## The duplicate hunt

Every candidate was screened three ways: against the headword list with
articles stripped and compounds split, against the **atom** ledger across all
lesson types, and against the **root** ledger. Roughly a hundred candidates
were considered to place thirty-five.

### The mechanical screen finally caught something a careful reading missed

Tranche 4 reported the whole-string matcher catching nothing and tranche 5
reported it catching six. This time two drops came from the machine alone,
after a hand pass had already cleared both:

| Dropped | Caught by |
| --- | --- |
| `el codo` | one apart from the very common taught `cómo`, sharing the opening `co-` |
| `la cima` | one apart from **both** `la cama` and `la cita`, sharing the opening `ci-`/`c-` |

`el codo` is the instructive one. It survived a hand pass because the eye
compares it to other body words, and its nearest confusable neighbour is an
interrogative.

### The near-duplicate rule needed narrowing, not widening

A first pass flagged `el beso` against `el peso` and `el codo` against `todo`,
and both are false alarms. The documented drops -- `el hombre`/`el hombro`,
`blando`/`blanco`, `la ropa`/`la roca` -- all share their **opening**. A pair
that differs in its first letter shares nothing a learner keys on.

So the rule is: *a same-length pair differing in exactly one position is a drop
only when the differing position is not the first.* Under it `el beso` and
`el codo` separate correctly -- `beso`/`peso` and `codo`/`todo` are kept,
`codo`/`cómo` is dropped.

### Shared initial stem, unrelated roots

Thirty-odd drops came this way. The ones worth recording:

| Dropped | Beside the taught | Why it is not the same word |
| --- | --- | --- |
| `el hueso` | `el huevo` | `ossum` against `ovum` |
| `la lengua` | `lento` | `lingua` against `lentus` |
| `la cena` | `la ceniza` | `cena` against `cinis` |
| `la hierba` | `el hielo` | `herba` against `gelu` |
| `el pollo` | `el polvo` | `pullus` against `pulvis` |
| `el pato` | `la patata` | Arabic `batt` against a Caribbean loan |
| `el nieto` | `la nieve` | `nepos` against `nix` |
| `el vidrio` | `la vida` | `vitrum` against `vita` |
| `la galleta` | `la gallina` | a French pebble against `gallus` |
| `el conejo` | `el consejo` | `cuniculus` against `consilium` |
| `la playa` | `la plaza` | Greek `plagia` against Greek `plateia` |
| `la ola` | `la olla` | near-homophones under yeismo |
| `el almuerzo` | `la almohada` | `admordium` -- the `al-` is not the Arabic article |
| `la acera` | `el aceite` | `acies` against `oleum` |
| `el camion` | `el camino` | a French loan against the taught Gaulish root |
| `la mejilla` | `mejor`, inside `pasar a mejor vida` | a phrase headword owning the stem |

`el almuerzo` is the one that hurt. Its `al-` looking Arabic and not being
Arabic is a genuinely good lesson, and it is unavailable while `la almohada`
is taught.

### The root ledger

| Dropped | Root already spent on |
| --- | --- |
| `el pelo` | `pilus`, held by the phrase `tomar el pelo` |
| `claro` | `clarus`, held by the phrase `si, claro` |
| `el sol` | `sol`, held inside `hace calor, hace frio, hace sol` |
| `redondo` | `rotundus` -- the corpus files `alrededor` under `rota` |
| `el tren` | `trahere`, spent on `traer` |
| `el jefe` | `caput`, spent on `la cabeza` |
| `el postre` | `post`, spent on `despues` and `pues` |
| `la ensalada` | `sal`, spent on `la sal` |
| `la musica` | the Muses, spent on `el museo` |
| `la pelicula` | `pellis`, spent on `la piel` |
| `el helado` | `gelu`, spent on `el hielo` |
| `amargo` | `amarus` -- `amarillo` is its diminutive |
| `posible` | `posse`, spent on `poder` |
| `facil` | `facere`, spent on `hacer` |
| `ligero` | `levare`, spent on `levantarse` |
| `pobre` | `pau-`, spent on `poco` |
| `morado` | the tag `mora-latin`, held by `la demora` for a different Latin word |

`morado` is a tag collision rather than a real one: Latin had a *mora* meaning
delay and a *mora* meaning mulberry, and they are unrelated. Coining a second
tag beside the first would put two etyma one character apart in an index whose
whole value is that entries join.

### Dropped for a reason no ledger names

`la aldea` was cleared by all three screens and dropped because the Academy's
etymon, Andalusi Arabic *addayaa*, needs two transliteration characters no
Spanish book has ever rendered. Recorded in `BACKLOG.d`; `la cumbre` took the
slot.

## Etymology

Three verification passes ran before any prose was written. **Eleven planned
hooks were killed or rewritten**, four of them outright false.

- **`la vela` was going to be wrong.** The plan had the candle and the sail as
  one word about stretched cloth. They are two words: the Academy's dictionary
  gives separate entries, the candle from `velar` < *vigilare*, "to keep
  watch", and only the sail from *velum*. The lesson now teaches the split, and
  teaches why the tidy version is tempting -- it reasons from the sail toward
  the candle and never checks whether there are two entries.
- **`el pimiento` was circular.** "Named for its colour" reads the earliest
  sense out of *pigmentum* and points at a red pepper as proof, skipping the
  three steps the dictionary actually prints: colouring matter, then drug, then
  ingredient, then condiment. The route runs through the apothecary.
- **`el pulgar` was circular.** "The thumb is the strong one", from *pollere*,
  reads strength out of the proposed ancestor and offers the thumb's obvious
  strength back as evidence for it. De Vaan is unconvinced. The lesson now
  teaches what the Academy actually gives -- *pollicaris* meant **an inch
  long**, which is why Spanish has *pulgada*.
- **`necesario` was circular.** "What cannot be got round" is the disputed
  *ne-* + *ced-* analysis restated, not support for it.
- **`el lapiz`** lost a clause that laundered an Arabic etymology through a
  Latin one: *azul* does not come from *lapis*, it comes from the **other**
  half of *lapis lazuli*. The two words share a stone and not a root, and that
  is now the lesson.
- **`simpatico`** lost its whole frame. The Academy gives it no etymology,
  because Spanish built it on *simpatia*; and English *sympathetic* is a
  different suffixal formation, not the same word.
- **`el chocolate`** now teaches that the famous *xocolatl* is unattested in
  colonial sources and that *chicolatl*, after the stick used to whisk the
  drink, is the live proposal.
- **`el maiz`** lost "the first American word to reach Europe" -- *canoa*
  almost certainly beat it.
- **`el volcan`** lost the island of Vulcano, which travel writing asserts and
  the etymological dictionaries do not.
- **`el edificio`** lost "hearth" from the gloss of *aedes*: that is a
  reconstructed prehistoric sense, not a Latin meaning, and listing it beside
  the real ones dresses a deep guess as direct descent.
- **`grueso`** lost the bridge from *grocer* to "by the gross" -- the
  occupation comes from *grossarius*, and the counting noun is separate.

Three lessons were also hedged harder than planned: `el labio`, where de Vaan
and Boutkan reject the old Indo-European link between *labium* and English
*lip* in favour of a lost pre-Indo-European source; `el nudo`, where Corominas
explicitly rejects the obvious guess that the knot drifted toward *nudus*
"naked"; and `seco`, where English *sack* is filed under *siccus* by habit
though the Oxford dictionary cannot account for the vowel and the wine was
sweet.

## Pins

**No pin was raised.** `ruleStatements` 30 of 30, `paradigmTables` 95,
`lessonsWithFindings` 121, `fullParadigmGrids` 22, banned words unchanged,
Spanish cross-chapter prose references zero. The derived bundle gate reports
284 lesson batches over 283 chapter bands -- the seven chapters add two bands
and two chunks together, which is what that gate exists to see --
`BAND_SPLIT_SLACK` untouched at 1 and the 256 kB backstop untouched.

Spanish `forwardReferences` moves 333 -> 341. The provenance was checked rather
than assumed: **zero of the eight are inside chapters 374-380.** Seven of them
are earlier lessons that had used one of these thirty-five words in passing and
now finally have somewhere to point -- `ES-C57-f-a-h` has always mentioned
*harina* while explaining the f-to-h change, and now the word exists.

All seven chapters narrate as "All 5 can be done entirely by ear", and all
thirty-five lessons resolve to `voice` in the modality manifest.
