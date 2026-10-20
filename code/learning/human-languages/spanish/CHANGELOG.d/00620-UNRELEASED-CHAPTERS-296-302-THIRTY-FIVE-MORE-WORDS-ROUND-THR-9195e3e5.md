## Unreleased — Chapters 296-302: thirty-five more words, round three

Two tranches took Spanish to 118 of the 300 headwords the pre-A1 vocabulary floor
asks for, and that was still the LOWEST of every track in the ramp — behind
Malayalam and Tamil at 119, Hindi at 120, Sanskrit at 139, Telugu at 151 and
Kannada at 152. These seven chapters answer with thirty-five more, on the same
HL14 rule: **one new headword per lesson**, reuse unlimited and deliberate.

```
296 MEET-GREET             el caballo, la vaca, la oveja, la gallina, el burro
297 COURTESY-THANK         la sartén, el horno, el tenedor, el ajo, la sopa
298 RESPOND-BASIC          tal vez, puede ser, en efecto, sin duda, ni hablar
299 EXCHANGE-NAMES         el campo, la granja, el ganado, la cosecha, el pastor
300 CHECK-WELLBEING        la sed, el sueño, la piel, la sangre, el remedio
301 POLITE-REQUEST-REPAIR  la cuerda, la aguja, el hilo, la cesta, el papel
302 TAKE-LEAVE             la calle, la esquina, la huella, el puerto, el faro
```

The vocabulary shortfall falls 182 to 147 — a drop of exactly thirty-five, the
lesson count — and the pre-A1 reinforcement blocker again does not move, holding
at 24. All seven pre-A1 nodes are reused a THIRD time, which is the mechanism
rather than a shortcut: Spanish realised all seven long ago, and a lesson filed
anywhere else moves the vocabulary number by zero.

**The ramp got gentler again.** R1 falls 0.2628 to 0.2523, its numerator holding at
221 while the denominator grew 841 to 876 — so not one of the thirty-five new
atoms misses its R1 window. Every lesson chains to the one before, each chapter's
second lesson practises the previous chapter's final atom, and each fifth lesson is
a payoff practising all five. `forwardReferences` holds at its 502 ceiling and
`ruleStatements` at 30; `scriptClosureViolations` (756) and
`exposureExemptedGlyphs` (2220) are untouched, as is the ZERO ceiling on naming a
chapter number in Spanish prose.

**Screening cost more than writing, and the largest class was invisible to search.**
151 candidates were screened against all 549 existing lessons and 26 were
discarded, in four kinds:

* **2 were already headwords** — `la olla`, `la despedida`.
* **3 sat inside a HEADWORD only.** `oso` is inside the `-oso` ending lesson;
  `sal` is inside the eight-imperative headword *di, haz, ve, pon, ten, sal, sé,
  ven*, where it means "go out"; `tos` is inside *cuántos*.
* **16 sat in a lesson BODY or an etymology gloss**, so teaching them would have
  minted a forward reference against a ceiling with no headroom: `lobo` is glossed
  where *lupus* becomes *lobo*, `harina` is a specimen in the f-to-h lesson,
  `llave` is one in the cl/pl/fl-to-ll lesson, `azúcar` and `aceite` are both in
  the Arabic-borrowing list, and `red` is matched by ENGLISH "red" inside an
  emphasised run. Also `leche`, `vela`, `hambre`, `salud`, `cura`, `huerto`,
  `vale`, `desde luego`, `camino`, `peso`.
* **5 collided with the gloss of a morpheme or a STOCK PHRASE**, which no string
  search reaches (HL-C207) and which was again the sharpest class. `el paso` —
  *passus* is already a declared ROOT of *pasar a mejor vida*. `el descanso` and
  `el cansancio` — *cansar* "to tire" is fully glossed at `ES-C09-cansado`, and
  *descansar* already stands in that lesson's euphemism list. `la fuerza` — "the
  force" is established English metalanguage in this track for the word carrying
  sentence stress, and a headword glossed with the same phrase would be read as
  that. `el aliento` — "in one breath" is a stock Guided Practice instruction
  across the corpus, so glossing a headword as "the breath" collides with a line
  the reader meets in dozens of lessons.

**Seven more were examined under that same rule and KEPT**, because the collision
was with an ETYMON or an English gloss word rather than with a Spanish headword,
and because reuse is the point of the ramp. `la cosecha` reuses the *colligere*
already taken apart under *tomar* instead of re-explaining it. `el puerto` is
written as the twin of the *porta* under *puerta* — the corpus's own "the word
set aside earlier" move, the one *la niebla* used on *nubes*. `la aguja` leans on
*acus* being the sharp that names the acute accent the reader has been writing for
hundreds of lessons. `el sueño` is the thing *dormir* is made of, given as what you
HOLD. `el papel` is set against *el libro*, which is named for the bark people
wrote on before paper arrived. `la granja` is named for the grain *el trigo*
already gave. `la sed` sits next to *beber*.

**Threads that pay across chapters.** The f-to-h decoder pays three times
(`horno`, `hilo`, `huella`) and is stated as a decoder the reader can run on
strangers. The short-o-to-ue break pays four (`sueño`, `cuerda`, `puerto`, and
`puede ser`), the short-e-to-ie break once more in `piel` beside *cielo* and
*miel*, and the *e-* step before s-plus-consonant once in `esquina` beside
*escoba*. The small-and-fond ending links `oveja` and `aguja` by rhyme. Two of the
five short answers are assembled entirely from words the reader already owns.

**Parts of speech are still not named.** Written without "verb" or "noun" from the
first draft, so the metalanguage pins do not move.

Two defects were caught by the author's own re-check after drafting, both by the
tools rather than by reading: `ES-C302-faro` came out at 325 computed seconds
against a 300-second threshold and was cut back, and `ES-C297-tenedor` classified
as `sight` because the phrase "on the table" is a sight cue — reworded, and all
thirty-five lessons and all seven chapters are now voice-drivable.

**The book is now 1337 pages** (1272 before), and the contents-column fix from
round two absorbed the growth: `xelatex` reports missing-character 0, overfull 0
and underfull 0, with the pre-existing `ch:how-are-you` duplicate label (HL-C204)
the only warning and unchanged in count. The seven new chapters add 42 labels,
none of them colliding.

