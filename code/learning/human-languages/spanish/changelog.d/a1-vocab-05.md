# Spanish A1 vocabulary tranche 5 -- thirty-five more words

Chapters 367-373, sequences 7270-7610, thirty-five lessons of one headword each.
Spanish moves **514 -> 549** distinct headwords at or below A1, against the
HL09 3.1 floor of **600**. Fifty-one to go.

| Ch. | Spine node | Words |
| --- | --- | --- |
| 367 | `SPINE-DEFINITE-REFERENCE` | el desayuno, la fruta, la naranja, la patata, el azucar |
| 368 | `SPINE-DEFINITE-REFERENCE` | la abeja, la mosca, el cerdo, el lobo, el toro |
| 369 | `SPINE-ASK-LOCATION` | el rio, la arena, el polvo, el humo, la ceniza |
| 370 | `SPINE-ASK-LOCATION` | la tormenta, el trueno, el hielo, la cueva, la costa |
| 371 | `SPINE-COUNT-ONE-TO-FIVE` | pequeno, gordo, joven, lento, dulce |
| 372 | `SPINE-DEFINITE-REFERENCE` | gris, rosa, oscuro, palido, suave |
| 373 | `SPINE-DEFINITE-REFERENCE` | el oro, la botella, la alfombra, la falda, el peine |

Seven chapters of five closes thirty-five exactly, with no chapter borrowing a
slot from another node. Atom runs continue their per-node counters rather than
restarting: `REF` 36-45 and 46-55, `WHERE` 46-55, `COUNT` 26-30.

## The duplicate hunt

Every candidate was screened three ways: against all 785 existing headwords
after stripping articles and splitting compounds, against the **atom** ledger
across all lesson types, and against the **root** ledger. The string matcher
caught six of the drops. It missed nine, and those nine are the interesting
ones.

| Dropped | Caught by |
| --- | --- |
| `la mantequilla` | initial stem -- `mant-` is the taught `la manta`, and the roots are unrelated |
| `marron` | the taught `el mar` is its whole initial stem |
| `la mariposa` | same -- `el mar`, plus `mari-` against the taught `maria` |
| `el raton` | the taught `el rato` is its whole initial stem, and `raptus` is nothing to do with `rata` |
| `el espejo` | initial stem -- `espe-` is the taught `esperar`, and `specere` is a second hit besides |
| `blando` | initial stem -- `blan-` is the taught `blanco`, Germanic against Latin `blandus` |
| `la piedra` | the taught `el pie` is its whole initial stem; `petra` is not `pedem` |
| `la cortina` | initial stem -- `cort-` is the taught `corto`, and `cortina` is not `curtus` |
| `la sabana` | initial stem -- `saba-` is the taught `sabado` |
| `nadar` | the taught `nada` is its whole initial stem, and `natare` is not `nata` |
| `el gallo` | the taught `la gallina` -- same root |
| `ensenar` | initial stem -- `ense-` is the taught `enseguida`, `insignare` against `sequi` |
| `el suelo` | one consonant from the taught `el sueno`, and `solum` is not `somnium` |
| `caro` | one vowel from the taught `la cara`, and `carus` is not `kara` |
| `la madera` | root ledger -- `mater-latin` is already spent |
| `la plata` | root ledger -- `platea-latin` is spent on `la plaza`; the taught `el plato` catches it too |
| `rapido` | root ledger -- `raptus-latin` is spent on `el rato`, a seized moment |
| `duro` | root ledger -- `durare` is spent on `durante` |
| `claro` | compound -- inside the taught `si, claro` |
| `recordar` | root ledger -- a second helping of the `cor` spent on `el corazon` |
| `caer` | root ledger -- `cadere` is spent on `la ocasion` |
| `rico` | root ledger -- Germanic `rikja` is the same ancient root as the `regere` already spent three times |
| `el hogar` | root ledger -- `focus-latin` is spent on `el fuego` |
| `perder` | root ledger -- `per` plus `dare-latin`, and `dare` is spent |
| `llevar` | root ledger -- `levare-latin` is spent on `levantarse` |
| `pagar` | root ledger -- `pax` and the `pagus-latin` behind `el pais` are one PIE root |
| `ganar` | the taught `el ganado` is its own past participle |
| `cocinar` | the taught `el cocinero` is the same stem |
| `facil` | root ledger -- `facere-latin` is spent on `hacer` |
| `pobre` | root ledger -- `paucus-latin` is spent on `poco` |
| `bonito` | root ledger -- `bonus` is spent on `bueno` |
| `caliente`, `pesado`, `el trabajo`, `el juego`, `la lluvia`, `el amor` | root ledger -- each names a root a taught word already holds |

Two false alarms were cleared rather than dropped, on the tranche-4 rule that a
taught word appearing as a **non-initial** substring across no morpheme boundary
is not a collision: `olvidar` contains the taught `vida` (it is `oblitare`, and
the `d` is the regular voicing of a `t`), and `la costa` shares three letters
with the taught `la cosa` (`causa`, unrelated). Three-letter initial overlaps
were kept throughout, on the tranche-4 precedent that shipped `corto` and
`el corazon` in one chapter.

## The etymology pass, which changed eleven lessons before they were written

Every hook was checked against current scholarship first. Two independent
verification passes ran over the same claim list, and between them they killed
or rewrote eleven planned stories.

- **`la alfombra` was going to be "the red one."** That etymology is superseded.
  The Academy's dictionary now gives Andalusi Arabic `alhanbal`, a kind of
  tapestry; Corominas objected that the redness had been read *out of* the
  proposed etymology and then offered back as evidence *for* it, and that the
  word had been tangled in the record with `alhamar`, a coverlet, which really
  does come from the Arabic root for red. The lesson now teaches the correction
  and the reasoning, which is a better lesson than the false one.
- **`la cueva` was going to claim English *cove*.** *Cove* is Old English
  `cofa`, Germanic, and its seaside sense is sixteenth-century. It is now taught
  as the look-alike to refuse.
- **`la tormenta` was going to claim `torno` and `turno`.** Those are a Greek
  word for a lathe, a different root -- so they are taught as the trap instead.
- **`el cerdo` was going to say Spanish lost `porcus`.** It did not: `puerco` is
  the regular reflex and is alive. The claim is now that `cerdo` took the
  neutral slot, which is what is true.
- **`oscuro` was going to bring in *sky*, *scum* and *hide*.** The standard
  modern dictionary of Latin origins rejects that root for `-scurus` outright.
  The lesson now hands the reader the live disagreement.
- **`el humo` was going to claim *dust* and *thyme*.** The Germanic formation is
  different, and the Greek word for thyme is not the Greek word for breath. Both
  cut.
- **`el polvo` was going to claim *pollen*,** **`el hielo` was going to claim
  *glacier*,** and **`el peine` was going to state the comb-to-money bridge as
  fact.** All three are doubted by specialists; the first two are cut and the
  third is taught as a tradition that the current specialists no longer endorse.
- **`el toro` was going to name Akkadian and Hebrew.** The Semitic-loan proposal
  is not the account now printed; the entry says non-Indo-European substrate,
  and the lesson teaches the wandering word instead.
- **`pequeno` was going to be called untraced.** The dictionary of record is more
  precise than that: an *expressive* word common to all the Romance languages,
  which is not the same as a gap in the record.

The strongest survivor is `el rio`. English *river* is not this word: it is
`ripa`, a bank, the same noun inside *arrive*, and `rivus` and `ripa` are
unrelated even at reconstructed depth. Spanish `rio` belongs with *rival* and
*derive*; English *river* belongs with *arrive*.

## Modality, budgets and pins

All seven chapters narrate as **"All 5 can be done entirely by ear."** --
`drivablePrefix: 5`, every lesson `voice`. Spanish moves 832 -> 867 voice with
`sight` and `pen` unchanged at 50 and 14.

**No pin was raised.** `ruleStatements` stays at **30 of 30**, `paradigmTables`
at 95, `lessonsWithFindings` at **121**, `fullParadigmGrids` at 22, banned words
unchanged, and Spanish cross-chapter prose references at zero. The info-dump
report records **zero** findings in chapters 367-373.

Spanish `forwardReferences` moves **325 -> 333**, and the provenance was checked
rather than assumed: **zero** of the eight are inside chapters 367-373. All
eight are earlier lessons that had used one of these words in passing and now
finally have somewhere to point -- `ES-C05-hasta` on `alfombra`, `azucar` and
`naranja`, `ES-C290-arroz` on `azucar`, `ES-C306-hijo` on `humo`,
`ES-C37-abrir` on `lobo`, `ES-C45-os` on `rio`, `ES-C67-uno-otro` on `pequeno`.
