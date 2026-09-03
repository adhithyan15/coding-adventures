### Added — the Russian A1 exam inventory, and the first track measured with no caveat to answer

- `core/exam-inventory-russian-a1.json` enumerates **228** A1 points and the
  corpus covers **73**, 32%. Every one of the proxy's 273 Spanish points is
  accounted for and **none is dropped** — the first file in this series with an
  empty `notTransferred`. A1-O1-06, superscript letters in Spanish
  abbreviations, is the one that looked untransferable and is really the demand
  that a numeral carry a written grammatical ending, which Russian answers with
  the hyphenated ordinal `1-y`; it is derived at `RU-A1-L-09`.
- **Russian's `exam-levels.json` entry carries no caveat**, so nothing external
  told this file which column the Spanish proxy could not supply. It was
  measured instead: walking the proxy's noun, pronoun and verb-phrase columns
  against the corpus showed that every one resolved to a question about **case**,
  and the verb column to **aspect**. Both became categories of their own, and ten
  of the file's fourteen `russianSpecific` grammar points live in them.
  `Padezh` reads **3 of 10** — the object pronouns `menya`/`vas` and the genitive
  after `do`, and nothing else. There is no plural anywhere in the track.
- **The joining column is 0 of 13**, the sixth track running and the first
  outside South Asia. Not one coordinator and not one subordinator is
  introduced. `i` ("and") is *printed as a conjunction* in two lesson bodies —
  `Ya chitayu i ponimayu`, `Moloko, syr, sok i sup, pozhaluysta` — and carried by
  no atom, which is a hair better than Gujarati's `ane` at zero occurrences and
  worse in one way: the commonest word in the language is on the page doing
  grammatical work the reader is never told about. `ili`, `no`, `a`, `chto`,
  `potomu chto`, `kogda`, `kotoryy` and `chtoby` are absent outright.
- **The repair column is half closed, which no percentage shows.** `Ya ne
  ponimayu` and `Ya ne znayu` are both taught in full, with stress marked. There
  is no way to apologise and no way to ask for a repeat: `izvinite`, `prostite`,
  `povtorite` and `medlenno` each return zero across all 88 lesson files,
  checked in Cyrillic and in romanization. A learner can report that the
  conversation has failed and can do nothing about it.
- **The letters are the closest thing to a finished column: 29 of 33**, counted
  against `data/scripts/cyrillic.json`, which lists all 33 with a cited stroke
  order. `yo` and the hard sign are shown in 5 and 4 lesson files and taught by
  nobody; `shcha` and `e-oborotnoe` appear nowhere at all. The category still
  reads 5 of 10, because the letter *names* are never given, the cursive hand —
  a second alphabet, not a style — is never taught, and the spelling rule that
  decides half of Russian morphology is never stated.
- Fourteen verbs are taught and **every one of them is an infinitive**. No
  adjective and no numeral is taught at all, so `Prilagatelnoe` and
  `Chislitelnye` are both empty and the whole evaluation column empties with
  them.
- Six farewells cover the proxy's single leave-taking point, which is the
  clearest picture of the track's shape: it can leave a conversation six ways
  and cannot open one about anything.
- `tests/exam-inventory.test.ts` gains the Russian block: every probe atom is
  checked against the atoms the corpus introduces, the derivation is totalled in
  both directions, the empty `notTransferred` is asserted as a claim rather than
  an omission, and the coverage figure is pinned by shape — case, joining,
  letters — rather than by size alone.
