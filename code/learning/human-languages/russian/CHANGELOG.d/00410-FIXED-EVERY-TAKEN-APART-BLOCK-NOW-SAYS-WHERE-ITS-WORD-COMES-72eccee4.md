## Fixed — every taken-apart block now says where its word comes from

Fifty-two Russian word lessons had a "taken apart" block that split the word into a prefix, a root and an ending, all Russian, and never said where the root came from (human-language-data `hasEmptyEtymologyBlock`, HL24 §9.4). Each block now says it, from Vasmer's dictionary (the tamila-krashtan/vasmer database, read locally). Vasmer is copyrighted: every sentence is a paraphrase, cited "(Vasmer)" or "Vasmer relates…", and no phrase of his is copied. Words a block names are spelled as the lesson already spells them, or romanized. Empty etymology blocks 52 → 0.

- **New origin sentences (51).** The root's Proto-Slavic form or its Baltic, Latin, Greek, Germanic or Sanskrit kin as Vasmer gives it: *ход* (the root *\*sed-*, "to sit", for four lessons), *давать* (three lessons and *преподаватель*), *говор*, *что*, *когда*, *вина*, *боль*, *рука*, *сей*, *час*, *тот*, *пять*, *десять*, *день*, *стоять*, *мыть*, *путь*, *где*, *куда*, *даве*, *начать* with *конец*, *круг* with Old Norse *hringr*, *этот*, *лежать*, *место*, *иной*, *слово*, *правый*, *нести*, *читать*, *плыть*, *дева*. Uncertain origins are said plainly: *утро*, *рыба*, *пускать*, *внук* (tentative).
- **Rewording (1).** RU-C176-vozvrashchatsya cites Vasmer for *вращать* as the Church Slavonic shape of *ворочать*.
- **Corrected.** RU-C01-pozhaluysta gave only one reading of *пожалуй*. Vasmer records two, the command form and a worn-down "I grant", and does not tie the verb to *жаль*; the block now gives both readings and calls the *жаль* link "often" made.
- **Durations.** RU-C01-pozhaluysta computes above 270 s; it dropped its closing gloss so it did not grow. No `max_seconds` was raised.

